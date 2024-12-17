use std::{net::IpAddr, path::PathBuf};

use rhombus::challenge_loader_plugin::ChallengeLoaderPlugin;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("rhombus=trace"))
                .unwrap(),
        )
        .init();

    let ip = reqwest::get("https://icanhazip.com")
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
        .trim()
        .parse::<IpAddr>()
        .unwrap();

    let app = rhombus::Builder::default()
        .load_env()
        .config_source(rhombus::config::File::with_name("config"))
        .plugin(ChallengeLoaderPlugin::new(PathBuf::from("challenges")))
        .extractor(move |_, _| Some(ip))
        .build()
        .await
        .unwrap();

    app.serve("[::]:3000".parse().unwrap(), "[::]:3001".parse().unwrap())
        .await
        .unwrap();
}
