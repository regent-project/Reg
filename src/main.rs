mod live;

use clap::{CommandFactory, Parser, ValueEnum};
use regent_sdk::ExpectedState;
use regent_sdk::LiveSink;
use regent_sdk::hosts::inventory::Inventory;
use regent_sdk::logging::{JsonLayer, NonBlocking, NonBlockingGuard, non_blocking_stdout};
use regent_sdk::secrets::{SecretProvider, SecretProvidersPoolBuilder};
use regent_sdk::state::compliance::HostStatus;
use std::fs;
use std::io::IsTerminal;
use std::process::ExitCode;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Validate,
    Assess,
    Enforce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LogFormat {
    /// Human-readable logs, and the live view printed for humans
    Text,
    /// One flat JSON object per line (for Grafana Loki, Elasticsearch...), live view included
    Json,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the inventory file
    #[clap(short, long)]
    inventory: Option<String>,

    /// Path to the expected state description file
    #[clap(short, long)]
    state: Option<String>,

    /// Assess compliance, enforce it if needed, or validate the given files
    #[clap(short, long)]
    #[arg(long, default_value = "assess")]
    mode: Mode,

    /// Do not show the live view (attributes, commands and their output, while they run)
    #[arg(long)]
    no_live: bool,

    /// Log format: `text` for a terminal, `json` to ship the logs (live view included)
    /// to a log database. The log level is set by RUST_LOG (default: info)
    #[arg(long, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,
}

/// Capacity of the live view's channel; beyond it, updates are dropped (and
/// counted) rather than slowing the run down.
const LIVE_VIEW_CAPACITY: usize = 4096;

fn validate_inventory_file(inventory_path: &str) -> bool {
    match fs::read_to_string(inventory_path) {
        Ok(contents) => match Inventory::from_raw_yaml(&contents) {
            Ok(_inventory) => {
                info!(inventory_path, "Inventory file is valid");
                true
            }
            Err(error_detail) => {
                error!(inventory_path, "Invalid inventory file : {error_detail}");
                false
            }
        },
        Err(error_detail) => {
            error!(
                inventory_path,
                "Cannot read inventory file : {error_detail}"
            );
            false
        }
    }
}

fn validate_state_file(state_path: &str) -> bool {
    match fs::read_to_string(state_path) {
        Ok(contents) => match ExpectedState::from_raw_yaml(&contents) {
            Ok(_expected_state) => {
                info!(state_path, "Expected state file is valid");
                true
            }
            Err(error_detail) => {
                error!(state_path, "Invalid expected state file : {error_detail}");
                false
            }
        },
        Err(error_detail) => {
            error!(
                state_path,
                "Cannot read expected state file : {error_detail}"
            );
            false
        }
    }
}

/// Install the logs: everything is written by a separate thread, so a slow output
/// never slows a run down (lines are dropped and counted instead).
fn init_logging(format: LogFormat) -> (NonBlocking, NonBlockingGuard) {
    let (writer, guard) = non_blocking_stdout();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    match format {
        LogFormat::Text => {
            // The live view is printed for humans by `live::print`, not as log lines
            let not_live = filter_fn(|metadata| !metadata.target().starts_with("regent_sdk::live"));
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(writer.clone())
                        .with_ansi(std::io::stdout().is_terminal())
                        .with_filter(not_live),
                )
                .init();
        }
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(
                    JsonLayer::new(writer.clone())
                        .with_field("service", "reg")
                        .with_field("version", env!("CARGO_PKG_VERSION")),
                )
                .init();
        }
    }
    (writer, guard)
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let (writer, guard) = init_logging(args.log_format);

    let code = run(&args).await;

    drop(guard); // writes the queued lines
    if writer.dropped_lines() > 0 {
        eprintln!(
            "reg: {} log line(s) dropped (output too slow)",
            writer.dropped_lines()
        );
    }
    code
}

async fn run(args: &Args) -> ExitCode {
    match &args.mode {
        Mode::Validate => {
            if args.inventory.is_none() && args.state.is_none() {
                Args::command()
                    .error(
                        clap::error::ErrorKind::MissingRequiredArgument,
                        "at least one of --inventory or --state is required in validate mode",
                    )
                    .exit();
            }

            let mut all_valid = true;
            if let Some(inventory_path) = &args.inventory
                && !validate_inventory_file(inventory_path)
            {
                all_valid = false;
            }
            if let Some(state_path) = &args.state
                && !validate_state_file(state_path)
            {
                all_valid = false;
            }
            if all_valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Mode::Assess | Mode::Enforce => {
            let (inventory_path, state_path) = match (&args.inventory, &args.state) {
                (Some(inventory), Some(state)) => (inventory, state),
                _ => Args::command()
                    .error(
                        clap::error::ErrorKind::MissingRequiredArgument,
                        "both --inventory and --state are required in assess and enforce modes",
                    )
                    .exit(),
            };
            let live = match (args.no_live, args.log_format) {
                (true, _) => Live::Off,
                (false, LogFormat::Text) => Live::Printed,
                (false, LogFormat::Json) => Live::Logged,
            };
            run_compliance(inventory_path, state_path, &args.mode, live).await
        }
    }
}

/// Where the live view goes.
enum Live {
    Off,
    /// Printed for humans (text logs)
    Printed,
    /// In the logs only (JSON logs)
    Logged,
}

async fn run_compliance(
    inventory_path: &str,
    state_path: &str,
    mode: &Mode,
    live: Live,
) -> ExitCode {
    // Read the inventory file
    let inventory_contents = fs::read_to_string(inventory_path).unwrap();
    // Read the expected state description file
    let expected_state_description = fs::read_to_string(state_path).unwrap();

    let mut inventory = Inventory::from_raw_yaml(&inventory_contents).unwrap();
    let expected_state = match ExpectedState::from_raw_yaml(&expected_state_description) {
        Ok(state) => state,
        Err(error_detail) => {
            error!("Wrong yaml content : {:?}", error_detail);
            return ExitCode::FAILURE;
        }
    };

    // Build a SecretProvidersPool
    let secrets_providers_pool = SecretProvidersPoolBuilder::new()
        .with_default_provider(&SecretProvider::env_var("env_var"))
        .add_provider(&SecretProvider::files("file"))
        .build()
        .unwrap();

    // Open connections within this Inventory
    let mut living_inventory = match inventory.init(Some(secrets_providers_pool)).await {
        Ok(living_inventory) => living_inventory,
        Err(_details) => {
            return ExitCode::FAILURE;
        }
    };

    // Live view: what each host does while the run goes on
    let printer = match live {
        Live::Off => None,
        Live::Printed => {
            let (sink, updates) = LiveSink::channel(LIVE_VIEW_CAPACITY);
            living_inventory.enable_live(Some(sink));
            Some(tokio::spawn(live::print(updates, live::Style::detect())))
        }
        Live::Logged => {
            living_inventory.enable_live(None);
            None
        }
    };

    // Assess or enforce compliance based on mode
    let compliance_result = match mode {
        Mode::Assess => living_inventory.assess_compliance(&expected_state).await,
        Mode::Enforce => living_inventory.reach_compliance(&expected_state).await,
        Mode::Validate => unreachable!("validate mode does not run compliance operations"),
    };

    // Release the hosts' senders so that the printer finishes before the summary
    living_inventory.disable_live();
    if let Some(printer) = printer {
        let _ = printer.await;
    }

    match compliance_result {
        Ok(inventory_compliance) => {
            for (host_id, host_status) in inventory_compliance {
                match host_status.state {
                    HostStatus::AlreadyCompliant => {
                        info!(host_id, "Already compliant");
                    }
                    HostStatus::NotCompliant => {
                        warn!(host_id, "Not compliant");
                    }
                    HostStatus::ReachComplianceSuccess => {
                        info!(host_id, "Compliance reached");
                    }
                    HostStatus::ReachComplianceFailed => {
                        error!(host_id, "Failed to reach compliance");
                    }
                    HostStatus::Aborted => {
                        warn!(host_id, "Aborted");
                    }
                }
            }
        }
        Err(_error_detail) => {
            // Not showing the error details here as they are already logged by regent-sdk
            error!("Compliance not reached");
        }
    }
    ExitCode::SUCCESS
}
