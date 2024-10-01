use super::{auth::MaybeUser, router::RouterState};
use aide::{
    axum::{routing::get, IntoApiResponse},
    openapi::{Info, OpenApi},
};
use axum::{extract::State, Extension, Json};
use schemars::JsonSchema;
use serde::Serialize;

async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api)
}

#[derive(Serialize, JsonSchema)]
struct Challenge {
    name: String,
}

async fn challenges_route(state: State<RouterState>) -> impl IntoApiResponse {
    let challenges = state.db.get_challenges().await.unwrap();
    Json(
        challenges
            .challenges
            .iter()
            .map(|chal| Challenge {
                name: chal.name.clone(),
            })
            .collect::<Vec<_>>(),
    )
}

pub fn build_api_router() -> axum::Router<RouterState> {
    let router = aide::axum::ApiRouter::new()
        .api_route("/", get(|| async { "Hello!".to_string() }))
        .api_route("/challenges", get(challenges_route))
        .route("/api.json", get(serve_api));

    let mut api = OpenApi {
        info: Info {
            description: Some("an example API".to_string()),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    router.finish_api(&mut api).layer(Extension(api)).into()
}
