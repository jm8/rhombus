use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::{Config, FileFormat};
use rhombus_api_client::{
    apis::{
        configuration::Configuration as ApiConfiguration,
        default_api::{attachment_hash_get, challenges_get},
    },
    models::{self, challenge},
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ChallengesYaml {
    pub authors: Vec<AuthorWithStableId>,
    pub categories: Vec<CategoryWithStableId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum ChallengeAttachment {
    Url { url: String, dst: String },
    File { src: String, dst: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ChallengeWithStableId {
    pub stable_id: String,
    pub author: String,
    pub category: String,
    pub description: String,
    pub files: Vec<ChallengeAttachment>,
    pub flag: String,
    pub healthscript: Option<String>,
    pub name: String,
    pub ticket_template: Option<String>,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct AuthorWithStableId {
    pub stable_id: String,
    #[serde(flatten)]
    pub author: models::Author,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct CategoryWithStableId {
    pub stable_id: String,
    #[serde(flatten)]
    pub category: models::Category,
}

pub fn slice_to_hex_string(slice: &[u8]) -> String {
    slice.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{b:02x}");
        output
    })
}

async fn load_challenges(api: &ApiConfiguration) -> Result<models::ChallengeData> {
    let config = Config::builder()
        .add_source(config::File::new("loader.yaml", FileFormat::Yaml))
        .build()?
        .try_deserialize::<ChallengesYaml>()
        .context("Failed to deserializing loader.yaml")?;

    let authors = config
        .authors
        .into_iter()
        .map(|AuthorWithStableId { stable_id, author }| (stable_id, author))
        .collect();

    let categories = config
        .categories
        .into_iter()
        .map(
            |CategoryWithStableId {
                 stable_id,
                 category,
             }| { (stable_id, category) },
        )
        .collect();

    let challenge_files = ChallengeYamlWalker::new(&PathBuf::from("."))
        .into_iter()
        .map(|path| {
            Ok((
                path.clone(),
                Config::builder()
                    .add_source(config::File::from(path.as_path()))
                    .build()
                    .unwrap()
                    .try_deserialize::<ChallengeWithStableId>()
                    .with_context(|| format!("Failed to deserialize {}", path.display()))?,
            ))
        })
        .collect::<Result<Vec<(PathBuf, ChallengeWithStableId)>>>()?;

    let mut hash_to_file_path = BTreeMap::new();

    struct ChallengeIntermediateAttachment {
        name: String,
        source: ChallengeIntermediateAttachmentSource,
    }
    enum ChallengeIntermediateAttachmentSource {
        Hash(String),
        Url(String),
    }

    for (challenge_yaml_path, chal) in challenge_files {
        for file in chal.files {
            match file {
                ChallengeAttachment::File { src, dst: _ } => {
                    let file_path = challenge_yaml_path.parent().unwrap().join(src);
                    let data = tokio::fs::read(file_path.as_path()).await?;
                    let digest = ring::digest::digest(&ring::digest::SHA256, &data);
                    let hash = slice_to_hex_string(digest.as_ref());
                    hash_to_file_path.insert(hash, file_path);
                }
                _ => (),
            }
        }
    }

    println!("{:#?}", hash_to_file_path);

    for (hash, path) in hash_to_file_path {
        let url = attachment_hash_get(api, &hash).await?;
        println!("{} -> {:?}", hash, url);
    }

    Ok(models::ChallengeData {
        authors,
        categories,
        challenges: todo!(),
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

    let base_url = "http://localhost:3000".to_owned();

    let api = ApiConfiguration {
        base_path: format!("{}/api/v1", base_url),
        ..Default::default()
    };

    match cli.command {
        Commands::Admin {
            command: AdminCommands::Apply,
        } => {
            let x = load_challenges(&api).await?;
            println!("{:?}", x);
        }
    }

    let challenges = challenges_get(&api).await?;

    println!("{:?}", challenges);

    Ok(())
}
