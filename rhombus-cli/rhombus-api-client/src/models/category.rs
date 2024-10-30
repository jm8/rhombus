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
pub struct Category {
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "name")]
    pub name: String,
}

impl Category {
    pub fn new(color: String, name: String) -> Category {
        Category {
            color,
            name,
        }
    }
}

