mod challenges;

use super::router::RouterState;
use aide::{
    axum::{routing::get, IntoApiResponse},
    openapi::{Info, OpenApi},
    redoc::Redoc,
    NoApi,
};
use axum::{Extension, Json};
use challenges::get_challenges;

async fn serve_api(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    NoApi(Json(api))
}

pub fn build_api_router() -> axum::Router<RouterState> {
    let router = aide::axum::ApiRouter::new()
        .api_route("/", get(|| async { "Hello!".to_string() }))
        .api_route("/challenges", get(get_challenges))
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
