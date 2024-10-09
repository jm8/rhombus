use clap::{Parser, Subcommand};
use config::{Config, ConfigError, FileFormat};
use rhombus_api_client::apis::{
    configuration::{self, Configuration},
    default_api::challenges_get,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
enum RhombusCliError {
    #[error("error loading challenges config")]
    ConfigError(#[from] ConfigError),
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Challenge {
        #[command(subcommand)]
        command: ChallengeCommands,
    },
}

#[derive(Subcommand)]
enum ChallengeCommands {
    Sync,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Category {
    pub stable_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Author {
    pub stable_id: Option<String>,
    pub name: String,
    pub avatar: String,
    pub discord_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChallengeLoaderConfiguration {
    pub categories: Vec<Category>,
    pub authors: Vec<Author>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = Config::builder()
        .add_source(config::File::new("loader.yaml", FileFormat::Yaml))
        .build()
        .unwrap()
        .try_deserialize::<ChallengeLoaderConfiguration>()
        .unwrap();

    eprintln!("{:#?}", config);

    match cli.command {
        Commands::Challenge {
            command: ChallengeCommands::Sync,
        } => {}
    }

    let configuration = Configuration {
        base_path: "http://localhost:3000/api/v1".to_string(),
        ..Default::default()
    };
    let challenges = challenges_get(&configuration).await;

    println!("{:?}", challenges);
}
