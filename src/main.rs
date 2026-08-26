use clap::{Parser, ValueEnum};
use regent_sdk::ExpectedState;
use regent_sdk::hosts::inventory::Inventory;
use regent_sdk::secrets::{SecretProvider, SecretProvidersPoolBuilder};
use regent_sdk::state::compliance::HostStatus;
use std::fs;
use tracing::{error, info, warn};

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Assess,
    Enforce,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the inventory file
    #[clap(short, long)]
    inventory: String,

    /// Path to the expected state description file
    #[clap(short, long)]
    state: String,

    /// Assess compliance or enforce it if needed
    #[clap(short, long)]
    #[arg(long, default_value = "assess")]
    mode: Mode,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Read the inventory file
    let inventory_contents = fs::read_to_string(&args.inventory).unwrap();
    // Read the expected state description file
    let expected_state_description = fs::read_to_string(&args.state).unwrap();

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
        .add_default_provider("env_var", SecretProvider::env_var())
        .add_provider("file", SecretProvider::files())
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
    let compliance_result = match args.mode {
        Mode::Assess => living_inventory.assess_compliance(&expected_state).await,
        Mode::Enforce => living_inventory.reach_compliance(&expected_state).await,
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
