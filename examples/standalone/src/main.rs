use std::{net::IpAddr, path::PathBuf};

use rhombus::{builder::RhombusApp, challenge_loader_plugin::ChallengeLoaderPlugin};
use tokio::{join, spawn, try_join};
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

    let RhombusApp { web, grpc } = rhombus::Builder::default()
        .load_env()
        .config_source(rhombus::config::File::with_name("config"))
        .plugin(ChallengeLoaderPlugin::new(PathBuf::from("challenges")))
        .extractor(move |_, _| Some(ip))
        .build()
        .await
        .unwrap();

    let listener = tokio::net::TcpListener::bind(":::3000").await.unwrap();
    let a = spawn(async { grpc.serve("[::]:3001".parse().unwrap()).await.unwrap() });
    let b = spawn(async move { web.serve(listener).await });
    try_join!(a, b).unwrap();
}
