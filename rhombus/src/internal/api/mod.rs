mod challenges;

use super::router::RouterState;
use aide::{
    axum::{
        routing::{get, get_with},
        IntoApiResponse,
    },
    openapi::{Info, OpenApi},
    redoc::Redoc,
    transform::{TransformOperation, TransformParameter},
    NoApi,
};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use challenges::{get_challenges, get_challenges_diff};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, JsonSchema)]
#[serde(tag = "type")]
pub enum GetAttachmentResult {
    Exists { url: String },
    DoesNotExist,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetAttachmentParams {
    /// The hash of the chicken
    pub hash: String,
}

pub async fn get_attachment(
    Path(params): Path<GetAttachmentParams>,
    state: State<RouterState>,
) -> Json<GetAttachmentResult> {
    Json(match state.db.get_attachment(&params.hash).await.unwrap() {
        Some(url) => GetAttachmentResult::Exists { url },
        None => GetAttachmentResult::DoesNotExist,
    })
}
