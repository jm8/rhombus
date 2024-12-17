use anyhow::Result;
use clap::{Parser, Subcommand};
use grpc::proto::{rhombus_client::RhombusClient, HelloRequest};
use std::path::PathBuf;

mod grpc {
    pub mod proto {
        tonic::include_proto!("rhombus");
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[arg(short, long)]
        list: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = RhombusClient::connect("http://[::0]:3001").await?;
    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await?;
    println!("{:?}", response);

    Ok(())
}
