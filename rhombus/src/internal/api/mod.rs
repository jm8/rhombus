mod challenges;

use super::router::RouterState;
use aide::{
    axum::{routing::get, IntoApiResponse},
    openapi::{Info, OpenApi},
    redoc::Redoc,
    NoApi,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use challenges::{get_challenges, get_challenges_diff};

async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    NoApi(Json(api))
}

pub fn build_api_router() -> axum::Router<RouterState> {
    let router = aide::axum::ApiRouter::new()
        .api_route("/", get(|| async { "Hello!".to_string() }))
        .api_route("/challenges", get(get_challenges))
        .api_route("/challenges/diff", get(get_challenges_diff))
        .api_route("/attachment/:hash", get(get_attachment))
        .route("/openapi.json", get(serve_api))
        .route("/docs", Redoc::new("/api/v1/openapi.json").axum_route());

    let mut api = OpenApi {
        info: Info {
            title: "rhombus_api".to_string(),
            version: "0.1.0".to_string(),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    router.finish_api(&mut api).layer(Extension(api)).into()
}

// pub enum Attachment {
//     Exists { url: String },
//     Upload { url: String },
// }

pub async fn get_attachment(
    Path(hash): Path<String>,
    state: State<RouterState>,
) -> impl IntoApiResponse {
    Json(state.db.get_attachment(&hash).await.unwrap())
}
