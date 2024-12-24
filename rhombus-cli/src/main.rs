use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use figment::{
    providers::{Format, Yaml},
    Figment,
};
use grpc::proto::{
    rhombus_client::RhombusClient, Author, Category, Challenge, ChallengeAttachment, ChallengeData,
    GetAttachmentByHashRequest, HelloRequest,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    path::{Path, PathBuf},
};

mod grpc {
    pub mod proto {
        // tonic::include_proto!("rhombus");
        include!("./rhombus.rs");
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
    pub points: Option<i64>,
    pub score_type: Option<String>,
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

    let config: LoaderYaml = Figment::new()
        .merge(Yaml::file_exact("loader.yaml"))
        .extract()?;

    let challenge_yamls = ChallengeYamlWalker::new(&PathBuf::from("."))
        .into_iter()
        .map(|p| {
            let text = fs::read_to_string(&p)?;
            let parsed = Figment::new()
                .merge(Yaml::string(&text))
                .extract::<ChallengeYaml>()?;
            let json = serde_json::to_string(&serde_yml::from_str::<serde_json::Value>(&text)?)?;

            Ok((p, parsed, json))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut src_to_url: HashMap<PathBuf, String> = HashMap::new();

    for (challenge_yaml_path, challenge_yaml, _metadata) in &challenge_yamls {
        for file in &challenge_yaml.files {
            match file {
                ChallengeAttachmentYaml::Url { url: _, dst: _ } => continue,
                ChallengeAttachmentYaml::File { src, dst: _ } => {
                    let file_path = challenge_yaml_path.parent().unwrap().join(src);
                    let data = fs::read(file_path.as_path())?;
                    let digest = ring::digest::digest(&ring::digest::SHA256, &data);
                    let hash = slice_to_hex_string(digest.as_ref());
                    let url = client
                        .get_attachment_by_hash(tonic::Request::new(GetAttachmentByHashRequest {
                            hash: hash.clone(),
                        }))
                        .await?
                        .into_inner()
                        .url;
                    src_to_url.insert(file_path, url.expect("uploading isn't implemented"));
                }
            }
        }
    }

    let x = client
        .diff_challenges(tonic::Request::new(ChallengeData {
            challenges: challenge_yamls
                .iter()
                .map(|(p, chal, metadata)| -> Result<(String, Challenge)> {
                    Ok((
                        chal.stable_id.clone(),
                        Challenge {
                            name: chal.name.clone().unwrap_or_else(|| chal.stable_id.clone()),
                            description: markdown::to_html_with_options(
                                &chal.description,
                                &markdown::Options {
                                    compile: markdown::CompileOptions {
                                        allow_dangerous_html: true,
                                        allow_dangerous_protocol: true,
                                        ..markdown::CompileOptions::default()
                                    },
                                    ..markdown::Options::default()
                                },
                            )
                            .map_err(|err| {
                                anyhow!("failed to convert markdown in {}: {}", chal.stable_id, err)
                            })?,
                            category: chal.category.clone(),
                            author: chal.author.clone(),
                            ticket_template: chal.ticket_template.clone(),
                            files: chal
                                .files
                                .iter()
                                .map(|file| match file {
                                    ChallengeAttachmentYaml::Url { url, dst } => {
                                        ChallengeAttachment {
                                            name: dst.clone(),
                                            url: url.clone(),
                                        }
                                    }
                                    ChallengeAttachmentYaml::File { src, dst } => {
                                        ChallengeAttachment {
                                            name: dst.clone(),
                                            url: src_to_url
                                                .get(&p.parent().unwrap().join(&src))
                                                .unwrap()
                                                .clone(),
                                        }
                                    }
                                })
                                .collect(),
                            flag: chal.flag.clone(),
                            healthscript: chal.healthscript.clone(),
                            points: chal.points,
                            metadata: Some(metadata.clone()),
                            score_type: chal.score_type.clone(),
                        },
                    ))
                })
                .collect::<Result<HashMap<String, Challenge>>>()?,
            authors: config
                .authors
                .iter()
                .map(|author| {
                    (
                        author.stable_id.clone(),
                        Author {
                            name: author
                                .name
                                .clone()
                                .unwrap_or_else(|| author.stable_id.clone()),
                            avatar_url: author.avatar.clone(),
                            discord_id: author.discord_id.to_string(),
                        },
                    )
                })
                .collect(),
            categories: config
                .categories
                .iter()
                .map(|category| {
                    (
                        category.stable_id.clone(),
                        Category {
                            name: category
                                .name
                                .clone()
                                .unwrap_or_else(|| category.stable_id.clone()),
                            color: category.color.clone(),
                        },
                    )
                })
                .collect(),
        }))
        .await?
        .into_inner();

    println!("{:#?}", x);

    Ok(())
}

fn slice_to_hex_string(slice: &[u8]) -> String {
    slice.iter().fold(String::new(), |mut output, b| {
        let _ = write!(output, "{b:02x}");
        output
    })
}
