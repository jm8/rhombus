use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::{Config, FileFormat};
use rhombus_api_client::{
    apis::{configuration::Configuration as ApiConfiguration, default_api::challenges_get},
    models::{self},
};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
}

#[derive(Subcommand)]
enum AdminCommands {
    Apply,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct ChallengesYaml {
    authors: Vec<AuthorWithStableId>,
    categories: Vec<CategoryWithStableId>,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct ChallengesWithStableId {
    stable_id: String,
    #[serde(flatten)]
    challenge: models::Challenge,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct AuthorWithStableId {
    stable_id: Option<String>,
    #[serde(flatten)]
    author: models::Author,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct CategoryWithStableId {
    stable_id: Option<String>,
    #[serde(flatten)]
    category: models::Category,
}

fn load_challenges() -> Result<models::ChallengeData> {
    let config = Config::builder()
        .add_source(config::File::new("loader.yaml", FileFormat::Yaml))
        .build()?
        .try_deserialize::<ChallengesYaml>()
        .context("Failed to deserializing loader.yaml")?;

    let authors = config
        .authors
        .into_iter()
        .map(|AuthorWithStableId { stable_id, author }| {
            (stable_id.unwrap_or_else(|| author.name.clone()), author)
        })
        .collect();

    let categories = config
        .categories
        .into_iter()
        .map(
            |CategoryWithStableId {
                 stable_id,
                 category,
             }| { (stable_id.unwrap_or_else(|| category.name.clone()), category) },
        )
        .collect();

    let challenges = ChallengeYamlWalker::new(&PathBuf::from("."))
        .into_iter()
        .map(|path| {
            Ok(Config::builder()
                .add_source(config::File::from(path.as_path()))
                .build()
                .unwrap()
                .try_deserialize::<ChallengesWithStableId>()
                .with_context(|| format!("Failed to deserialize {}", path.display()))?)
        })
        .map(|res| {
            res.map(
                |ChallengesWithStableId {
                     stable_id,
                     challenge,
                 }| { (stable_id, challenge) },
            )
        })
        .collect::<Result<HashMap<String, models::Challenge>>>()?;

    Ok(models::ChallengeData {
        authors,
        categories,
        challenges,
    })
}

struct ChallengeYamlWalker {
    stack: Vec<ReadDir>,
}

impl ChallengeYamlWalker {
    fn new(root: &Path) -> Self {
        let mut stack = Vec::new();
        if root.is_dir() {
            stack.push(fs::read_dir(root).unwrap());
        }
        ChallengeYamlWalker { stack }
    }
}

impl Iterator for ChallengeYamlWalker {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(dir_iter) = self.stack.last_mut() {
            if let Some(entry) = dir_iter.next() {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_dir() {
                            self.stack.push(fs::read_dir(path).unwrap());
                        } else if path.is_file() && path.file_name().unwrap() == "challenge.yaml" {
                            return Some(path);
                        }
                    }
                    Err(_) => continue,
                }
            } else {
                self.stack.pop();
            }
        }
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let configuration = ApiConfiguration {
        base_path: "http://localhost:3000/api/v1".to_string(),
        ..Default::default()
    };

    match cli.command {
        Commands::Admin {
            command: AdminCommands::Apply,
        } => {
            let x = load_challenges()?;
            println!("{:?}", x);
        }
    }

    let challenges = challenges_get(&configuration).await;

    println!("{:?}", challenges);

    Ok(())
}
