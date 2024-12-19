use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use figment::{
    providers::{Format, Yaml},
    Figment,
};
use grpc::proto::{
    challenge_data_patch_action::Action, rhombus_client::RhombusClient, Challenge,
    ChallengeAttachmentsPatch, ChallengeData, ChallengeDataPatch, ChallengePatch, CreateChallenge,
    HelloRequest, OptionalStringPatch, PatchChallenge, StringPatch,
};
use serde::{Deserialize, Serialize};
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
        .await?
        .into_inner();

    print_diff(&x);

    println!("{:#?}", x);
    Ok(())
}

// enum DiffElement {
//     PatchObject(HashMap<String, DiffElement>),
//     CreateObject(HashMap<String, String>),
//     Delete,
//     PatchValue {
//         old: Option<String>,
//         new: Option<String>,
//     },
// }

trait RenderPatch {
    fn render(&self, name: &str, indent: usize);
}

impl RenderPatch for StringPatch {
    fn render(&self, name: &str, indent: usize) {
        println!(
            "{:indent$}{}: {:?} -> {:?}",
            "",
            name,
            self.old.red(),
            self.new.green(),
            indent = indent
        );
    }
}

impl RenderPatch for OptionalStringPatch {
    fn render(&self, name: &str, indent: usize) {
        println!(
            "{:indent$}{}: {:?} -> {:?}",
            "",
            name,
            self.old,
            self.new,
            indent = indent
        );
    }
}

impl RenderPatch for ChallengeAttachmentsPatch {
    fn render(&self, name: &str, indent: usize) {
        println!(
            "{:indent$}{}: {:?} -> {:?}",
            "",
            name,
            self.old,
            self.new,
            indent = indent
        );
    }
}
impl RenderPatch for ChallengePatch {
    fn render(&self, name: &str, indent: usize) {
        println!("{:indent$}{:?}:", "", name, indent = indent);
        if let Some(p) = &self.name {
            p.render("name", indent + 2);
        }
        if let Some(p) = &self.description {
            p.render("description", indent + 2);
        }
        if let Some(p) = &self.category {
            p.render("category", indent + 2);
        }
        if let Some(p) = &self.author {
            p.render("author", indent + 2);
        }
        if let Some(p) = &self.ticket_template {
            p.render("ticket_template", indent + 2);
        }
        if let Some(p) = &self.files {
            p.render("files", indent + 2);
        }
        if let Some(p) = &self.flag {
            p.render("flag", indent + 2);
        }
        if let Some(p) = &self.healthscript {
            p.render("healthscript", indent + 2);
        }
    }
}

fn print_diff(patch: &ChallengeDataPatch) {
    // let challenges: HashMap<String, DiffElement> = HashMap::new();

    let actions = patch.actions.iter().filter_map(|x| x.action.as_ref());

    for action in actions {
        match action {
            Action::PatchChallenge(patch_challenge) => {
                if let Some(patch) = &patch_challenge.patch {
                    patch.render(&patch_challenge.id, 2);
                }
            }
            Action::DeleteChallenge(delete_challenge) => todo!(),
            Action::CreateChallenge(create_challenge) => todo!(),
            Action::PatchAuthor(patch_author) => todo!(),
            Action::DeleteAuthor(delete_author) => todo!(),
            Action::CreateAuthor(create_author) => todo!(),
            Action::PatchCategory(patch_category) => todo!(),
            Action::DeleteCategory(delete_category) => todo!(),
            Action::CreateCategory(create_category) => todo!(),
        }
    }
}
