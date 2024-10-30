use std::borrow::Borrow;

use crate::internal::{
    challenge_diff::{ChallengeData, ChallengeDataPatch},
    router::RouterState,
};
use aide::axum::IntoApiResponse;
use axum::{extract::State, Json};

pub async fn get_challenges(state: State<RouterState>) -> impl IntoApiResponse {
    let db_challenges = state.db.get_challenges().await.unwrap();
    let old_challenges = ChallengeData::from(db_challenges.borrow());

    Json(old_challenges)
}

pub async fn get_challenges_diff(
    state: State<RouterState>,
    Json(new_challenges): Json<ChallengeData>,
) -> impl IntoApiResponse {
    let db_challenges = state.db.get_challenges().await.unwrap();
    let old_challenges = ChallengeData::from(db_challenges.borrow());

    Json(ChallengeDataPatch::diff(&old_challenges, &new_challenges))
}
