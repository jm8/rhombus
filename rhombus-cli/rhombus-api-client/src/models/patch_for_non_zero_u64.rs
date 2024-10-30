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
pub struct PatchForNonZeroU64 {
    #[serde(rename = "new")]
    pub new: i32,
    #[serde(rename = "old")]
    pub old: i32,
}

impl PatchForNonZeroU64 {
    pub fn new(new: i32, old: i32) -> PatchForNonZeroU64 {
        PatchForNonZeroU64 {
            new,
            old,
        }
    }
}

