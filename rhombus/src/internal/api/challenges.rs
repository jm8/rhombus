use std::collections::BTreeMap;

use crate::internal::{
    database::{
        self,
        provider::{self, ChallengeData},
    },
    router::RouterState,
};
use aide::axum::IntoApiResponse;
use axum::{extract::State, Json};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Serialize, JsonSchema, Clone)]
struct Challenge {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub author: String,
    pub ticket_template: Option<String>,
    pub files: Vec<ChallengeAttachment>,

    pub flag: Option<String>,
    pub healthscript: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Clone)]
pub struct ChallengeAttachment {
    pub name: String,
    pub url: String,
}

impl Challenge {
    fn from(challenge: &provider::Challenge, data: &ChallengeData) -> Self {
        let category_id_to_name: BTreeMap<_, _> = data
            .categories
            .iter()
            .map(|category| (category.id, category.name.clone()))
            .collect();
        Self {
            id: challenge.id.to_string(),
            name: challenge.name.clone(),
            description: challenge.description.clone(),
            category: category_id_to_name
                .get(&challenge.category_id)
                .unwrap()
                .clone(),
            author: data.authors.get(&challenge.author_id).unwrap().name.clone(),
            ticket_template: challenge.ticket_template.clone(),
            files: challenge
                .attachments
                .iter()
                .map(|attachment| ChallengeAttachment {
                    name: attachment.name.clone(),
                    url: attachment.url.clone(),
                })
                .collect(),
            flag: None,
            healthscript: None,
        }
    }
}

#[derive(Debug, Serialize, JsonSchema, Clone)]
pub struct Attachment {
    pub url: String,
    pub dst: String,
}

pub async fn get_challenges(state: State<RouterState>) -> impl IntoApiResponse {
    let challenges = state.db.get_challenges().await.unwrap();

    Json(
        challenges
            .challenges
            .iter()
            .map(|challenge| Challenge::from(challenge, &challenges))
            .collect::<Vec<_>>(),
    )
}
