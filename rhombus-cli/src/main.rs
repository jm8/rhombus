use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use figment::{
    providers::{Format, Yaml},
    Figment,
};
use grpc::proto::{rhombus_client::RhombusClient, Challenge, ChallengeData, HelloRequest};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};

mod grpc {
    pub mod proto {
        tonic::include_proto!("rhombus");
        // include!("./rhombus.rs");
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct LoaderYaml {
    pub authors: Vec<AuthorYaml>,
    pub categories: Vec<CategoryYaml>,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct AuthorYaml {
    pub stable_id: String,
    pub name: Option<String>,
    pub avatar: String,
    pub discord_id: u64,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct CategoryYaml {
    pub stable_id: String,
    pub name: Option<String>,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ChallengeYaml {
    pub stable_id: String,
    pub author: String,
    pub category: String,
    pub description: String,
    pub files: Vec<ChallengeAttachmentYaml>,
    pub flag: String,
    pub healthscript: Option<String>,
    pub name: Option<String>,
    pub ticket_template: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum ChallengeAttachmentYaml {
    Url { url: String, dst: String },
    File { src: String, dst: String },
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
    let mut client = RhombusClient::connect("http://[::0]:3001").await?;
    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await?;
    println!("{:?}", response);

    let config: LoaderYaml = Figment::new()
        .merge(Yaml::file_exact("loader.yaml"))
        .extract()?;

    let challenge_yamls = ChallengeYamlWalker::new(&PathBuf::from("."))
        .into_iter()
        .map(|p| {
            Figment::new()
                .merge(Yaml::file_exact(&p))
                .extract::<ChallengeYaml>()
                .with_context(|| format!("failed to load {}", p.display()))
        })
        .collect::<Result<Vec<_>>>()?;

    // let challenge_data = todo!();

    let x = client
        .diff_challenges(tonic::Request::new(ChallengeData {
            challenges: challenge_yamls
                .iter()
                .map(|chal| {
                    (
                        chal.stable_id.clone(),
                        Challenge {
                            name: chal.name.clone().unwrap_or_else(|| chal.stable_id.clone()),
                            description: chal.description.clone(),
                            category: chal.category.clone(),
                            author: chal.author.clone(),
                            ticket_template: chal.ticket_template.clone(),
                            files: vec![],
                            flag: chal.flag.clone(),
                            healthscript: chal.healthscript.clone(),
                        },
                    )
                })
                .collect(),
            categories: HashMap::new(),
            authors: HashMap::new(),
        }))
        .await?;

    println!("{:#?}", x);

    Ok(())
}
