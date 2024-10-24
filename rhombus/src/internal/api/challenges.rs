// use crate::internal::{database::provider, router::RouterState};
// use aide::axum::IntoApiResponse;
// use axum::{extract::State, Json};
// use schemars::JsonSchema;
// use serde::Serialize;
// use std::{collections::BTreeMap, num::NonZeroU64};

// pub async fn get_challenges_diff(
//     state: State<RouterState>,
//     Json(new_challenges): Json<Vec<Challenge>>,
// ) -> impl IntoApiResponse {
//     let old_challenges = state.db.get_challenges().await.unwrap();
// }
