/*
 * rhombus_api
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: 0.1.0
 * 
 * Generated by: https://openapi-generator.tech
 */

use crate::models;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChallengeDataPatchActionOneOf6 {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "patch")]
    pub patch: Box<models::CategoryPatch>,
    #[serde(rename = "type")]
    pub r#type: Type,
}

impl ChallengeDataPatchActionOneOf6 {
    pub fn new(id: String, patch: models::CategoryPatch, r#type: Type) -> ChallengeDataPatchActionOneOf6 {
        ChallengeDataPatchActionOneOf6 {
            id,
            patch: Box::new(patch),
            r#type,
        }
    }
}
/// 
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "patch_category")]
    PatchCategory,
}

impl Default for Type {
    fn default() -> Type {
        Self::PatchCategory
    }
}

