use std::{path::PathBuf, str::FromStr};

use aptos_sdk::{
    move_types::{ident_str, language_storage::ModuleId},
    rest_client::Client,
    types::transaction::{EntryFunction, TransactionPayload},
};
use clap::Parser;
use url::Url;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long, value_parser)]
    pub deployer_address: String,
    #[clap(short = 'k', long, value_parser)]
    pub private_key: String,
    #[clap(short ='u', long, default_value_t = String::from("http://127.0.0.1:8080"), value_parser)]
    pub node_url: String,
    #[clap(short = 'n', long, value_parser)]
    pub package_name: String,
    #[clap(short = 'p', long, value_parser, default_value_t = String::from("."))]
    pub package_path: String,
    #[clap(short = 's', long, value_parser)]
    pub seed: String,
}

const DEPLOYER_NAME: &str = "deployer";

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let deployer_addr = args.deployer_address.as_str();
    let human_private_key = args.private_key.as_str();
    let node_url = Url::from_str(&args.node_url).unwrap();
    let build_path = PathBuf::from_str(&args.package_path).expect("failed to get the package path");
    let package_name = args.package_name;
    let seed = args.seed.as_str();

    let rest_client = Client::new(node_url.clone());
    let deployer_address = util_for_aptos::hex_to_account_address(deployer_addr);
    let human_account_address = util_for_aptos::get_account_address_from_private_key(
        util_for_aptos::get_private_key_from_str(human_private_key),
    );

    let resource_account_address =
        util_for_aptos::get_resource_address(human_account_address, seed);

    let mut build_options = util_for_aptos::get_none_artifact_build_options();

    build_options
        .named_addresses
        .insert(DEPLOYER_NAME.to_string(), deployer_address);
    build_options
        .named_addresses
        .insert(package_name, resource_account_address);

    let built_package = framework::BuiltPackage::build(build_path, build_options).unwrap();

    let metadata = bcs::to_bytes(&built_package.extract_metadata().unwrap()).unwrap();
    let codes = built_package.extract_code();

    let entry_function_call = EntryFunction::new(
        ModuleId::new(deployer_address, ident_str!(DEPLOYER_NAME).to_owned()),
        ident_str!("create_resource_account").to_owned(),
        vec![],
        vec![bcs::to_bytes(seed).unwrap()],
    );

    let payload = TransactionPayload::EntryFunction(entry_function_call);
    let mut human_account =
        util_for_aptos::get_local_account_from_private_key_str(human_private_key, &rest_client)
            .await;

    util_for_aptos::submit_transaction_with_much_gas(&rest_client, &mut human_account, payload)
        .await;

    let resource_account = util_for_aptos::get_resource_address(human_account.address(), seed);
    println!("resource account is: {}", resource_account);

    deployer::upload_and_publish_from_repo_to_resource_account(
        &rest_client,
        &mut human_account,
        &resource_account,
        "abc",
        &metadata,
        &codes,
    )
    .await;
}
