use clap::{CommandFactory, Parser, ValueEnum};
use regent_sdk::ExpectedState;
use regent_sdk::hosts::inventory::Inventory;
use regent_sdk::secrets::{SecretProvider, SecretProvidersPoolBuilder};
use regent_sdk::state::compliance::HostStatus;
use std::fs;
use tracing::{error, info, warn};

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Validate,
    Assess,
    Enforce,
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
}

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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

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
            if !all_valid {
                std::process::exit(1);
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
            run_compliance(inventory_path, state_path, &args.mode).await;
        }
    }
}

async fn run_compliance(inventory_path: &str, state_path: &str, mode: &Mode) {
    // Read the inventory file
    let inventory_contents = fs::read_to_string(inventory_path).unwrap();
    // Read the expected state description file
    let expected_state_description = fs::read_to_string(state_path).unwrap();

    let mut inventory = Inventory::from_raw_yaml(&inventory_contents).unwrap();
    let expected_state = match ExpectedState::from_raw_yaml(&expected_state_description) {
        Ok(state) => state,
        Err(error_detail) => {
            error!("Wrong yaml content : {:?}", error_detail);
            std::process::exit(1);
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
            std::process::exit(1);
        }
    };

    // Assess or enforce compliance based on mode
    let compliance_result = match mode {
        Mode::Assess => living_inventory.assess_compliance(&expected_state).await,
        Mode::Enforce => living_inventory.reach_compliance(&expected_state).await,
        Mode::Validate => unreachable!("validate mode does not run compliance operations"),
    };

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
                }
            }
        }
        Err(error_detail) => {
            error!("Other error : {error_detail}");
        }
    }
}
