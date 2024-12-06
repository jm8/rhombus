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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetAttachmentResult {
    GetAttachmentResultOneOf(Box<models::GetAttachmentResultOneOf>),
    GetAttachmentResultOneOf1(Box<models::GetAttachmentResultOneOf1>),
}

impl Default for GetAttachmentResult {
    fn default() -> Self {
        Self::GetAttachmentResultOneOf(Default::default())
    }
}
/// 
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "Exists")]
    Exists,
    #[serde(rename = "DoesNotExist")]
    DoesNotExist,
}

impl Default for Type {
    fn default() -> Type {
        Self::Exists
    }
}

