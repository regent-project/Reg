use std::fs;
use clap::Parser;
use regent_sdk::ExpectedState;
use regent_sdk::hosts::inventory::Inventory;
use regent_sdk::secrets::{SecretProvider, SecretProvidersPoolBuilder};
use tracing::{info, error};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the inventory file
    #[clap(short, long)]
    inventory: String,

    /// Path to the expected state description file
    #[clap(short, long)]
    state: String,
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
    let mut living_inventory = inventory.init(Some(secrets_providers_pool)).await.unwrap();

    // Try reach compliance if not already there
    match living_inventory.reach_compliance(&expected_state).await {
        Ok(inventory_compliance) => {
            info!("{inventory_compliance:?}");
        }
        Err(error_detail) => {
            error!("{error_detail}");
        }
    }
}