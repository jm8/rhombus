use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU64};

use super::database;

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct ChallengeData {
    pub challenges: BTreeMap<String, Challenge>,
    pub categories: BTreeMap<String, Category>,
    pub authors: BTreeMap<String, Author>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ChallengeDataPatch {
    actions: Vec<ChallengeDataPatchAction>,
}

impl ChallengeDataPatch {
    pub fn diff(old: &ChallengeData, new: &ChallengeData) -> Self {
        let mut actions = vec![];
        for (id, new_challenge) in new.challenges.iter() {
            match old.challenges.get(id) {
                Some(old_challenge) => {
                    if let Some(patch) = ChallengePatch::diff(old_challenge, &new_challenge) {
                        actions.push(ChallengeDataPatchAction::PatchChallenge {
                            id: id.clone(),
                            patch,
                        });
                    }
                }
                None => actions.push(ChallengeDataPatchAction::CreateChallenge {
                    id: id.clone(),
                    value: new_challenge.clone(),
                }),
            }
        }
        for id in old.challenges.keys() {
            if !new.challenges.contains_key(id) {
                actions.push(ChallengeDataPatchAction::DeleteChallenge { id: id.clone() })
            }
        }
        Self { actions }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ChallengeDataPatchAction {
    PatchChallenge { id: String, patch: ChallengePatch },
    DeleteChallenge { id: String },
    CreateChallenge { id: String, value: Challenge },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Challenge {
    pub name: String,
    pub description: String,
    pub category: String,
    pub author: String,
    pub ticket_template: Option<String>,
    pub files: Vec<ChallengeAttachment>,
    pub flag: String,
    pub healthscript: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, PartialEq)]
pub struct ChallengeAttachment {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Patch<T: PartialEq + Clone> {
    old: T,
    new: T,
}

impl<T: PartialEq + Clone> Patch<T> {
    pub fn diff(old: &T, new: &T) -> Option<Self> {
        if old == new {
            None
        } else {
            Some(Self {
                old: old.clone(),
                new: new.clone(),
            })
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ChallengePatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_template: Option<Patch<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Patch<Vec<ChallengeAttachment>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthscript: Option<Patch<Option<String>>>,
}

impl ChallengePatch {
    pub fn diff(old: &Challenge, new: &Challenge) -> Option<Self> {
        let result = Self {
            name: Patch::diff(&old.name, &new.name),
            description: Patch::diff(&old.description, &new.description),
            category: Patch::diff(&old.category, &new.category),
            author: Patch::diff(&old.author, &new.author),
            ticket_template: Patch::diff(&old.ticket_template, &new.ticket_template),
            files: Patch::diff(&old.files, &new.files),
            flag: Patch::diff(&old.flag, &new.flag),
            healthscript: Patch::diff(&old.healthscript, &new.healthscript),
        };

        result.has_change().then_some(result)
    }

    pub fn has_change(&self) -> bool {
        self.name.is_some()
            || self.description.is_some()
            || self.category.is_some()
            || self.author.is_some()
            || self.ticket_template.is_some()
            || self.files.is_some()
            || self.flag.is_some()
            || self.healthscript.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Category {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct CategoryPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Patch<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Author {
    pub name: String,
    pub avatar_url: String,
    pub discord_id: NonZeroU64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct AuthorPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<Patch<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_id: Option<Patch<NonZeroU64>>,
}

impl From<&database::provider::ChallengeData> for ChallengeData {
    fn from(db_data: &database::provider::ChallengeData) -> Self {
        let challenges = db_data
            .challenges
            .iter()
            .map(|(id, db_challenge)| {
                (
                    id.clone(),
                    Challenge {
                        name: db_challenge.name.clone(),
                        description: db_challenge.description.clone(),
                        category: db_challenge.category_id.clone(),
                        author: db_challenge.author_id.clone(),
                        ticket_template: db_challenge.ticket_template.clone(),
                        files: db_challenge
                            .attachments
                            .iter()
                            .map(|attachment| ChallengeAttachment {
                                name: attachment.name.clone(),
                                url: attachment.url.clone(),
                            })
                            .collect(),
                        flag: db_challenge.flag.clone(),
                        healthscript: db_challenge.healthscript.clone(),
                    },
                )
            })
            .collect();

        let categories = db_data
            .categories
            .iter()
            .map(|(id, db_category)| {
                (
                    id.clone(),
                    Category {
                        name: db_category.name.clone(),
                        color: db_category.color.clone(),
                    },
                )
            })
            .collect();

        let authors = db_data
            .authors
            .iter()
            .map(|(id, db_author)| {
                (
                    id.clone(),
                    Author {
                        name: db_author.name.clone(),
                        avatar_url: db_author.avatar_url.clone(),
                        discord_id: db_author.discord_id,
                    },
                )
            })
            .collect();

        ChallengeData {
            challenges,
            categories,
            authors,
        }
    }
}

#[cfg(test)]
mod test {
    use super::ChallengeDataPatch;
    use expect_test::{expect, Expect};

    fn check(old: &str, new: &str, expected: Expect) {
        let old = serde_json::from_str(old).unwrap();
        let new = serde_json::from_str(new).unwrap();
        let actual = ChallengeDataPatch::diff(&old, &new);
        let actual = serde_json::to_string_pretty(&actual).unwrap();
        expected.assert_eq(&actual);
    }

    #[test]
    fn test_add_challenge() {
        check(
            r#"
            {
                "challenges": {},
                "categories": {},
                "authors": {}
            }
            "#,
            r#"
            {
              "challenges": {
                "test": {
                  "name": "Test challenge",
                  "description": "Test",
                  "category": "abc",
                  "author": "john daker",
                  "files": [],
                  "flag": "rhombusctf{abc}"
                }
              },
              "categories": {},
              "authors": {}
            }
            "#,
            expect![[r#"
                {
                  "actions": [
                    {
                      "type": "create_challenge",
                      "id": "test",
                      "value": {
                        "name": "Test challenge",
                        "description": "Test",
                        "category": "abc",
                        "author": "john daker",
                        "ticket_template": null,
                        "files": [],
                        "flag": "rhombusctf{abc}",
                        "healthscript": null
                      }
                    }
                  ]
                }"#]],
        );
    }

    #[test]
    fn test_modify_challenge() {
        check(
            r#"
            {
              "challenges": {
                "test": {
                  "name": "Test challenge",
                  "description": "Test",
                  "category": "abc",
                  "author": "john daker",
                  "files": [],
                  "flag": "rhombusctf{abc}"
                }
              },
              "categories": {},
              "authors": {}
            }
            "#,
            r#"
            {
              "challenges": {
                "test": {
                  "name": "Test challenge",
                  "description": "Test",
                  "category": "abc",
                  "author": "john daker",
                  "files": [],
                  "flag": "rhombusctf{def}"
                }
              },
              "categories": {},
              "authors": {}
            }
            "#,
            expect![[r#"
                {
                  "actions": [
                    {
                      "type": "patch_challenge",
                      "id": "test",
                      "patch": {
                        "flag": {
                          "old": "rhombusctf{abc}",
                          "new": "rhombusctf{def}"
                        }
                      }
                    }
                  ]
                }"#]],
        );
    }

    #[test]
    fn test_change_challenge_id() {
        check(
            r#"
            {
              "challenges": {
                "test": {
                  "name": "Test challenge",
                  "description": "Test",
                  "category": "abc",
                  "author": "john daker",
                  "files": [],
                  "flag": "rhombusctf{abc}"
                }
              },
              "categories": {},
              "authors": {}
            }
            "#,
            r#"
            {
              "challenges": {
                "quiz": {
                  "name": "Test challenge",
                  "description": "Test",
                  "category": "abc",
                  "author": "john daker",
                  "files": [],
                  "flag": "rhombusctf{abc}"
                }
              },
              "categories": {},
              "authors": {}
            }
            "#,
            expect![[r#"
                {
                  "actions": [
                    {
                      "type": "create_challenge",
                      "id": "quiz",
                      "value": {
                        "name": "Test challenge",
                        "description": "Test",
                        "category": "abc",
                        "author": "john daker",
                        "ticket_template": null,
                        "files": [],
                        "flag": "rhombusctf{abc}",
                        "healthscript": null
                      }
                    },
                    {
                      "type": "delete_challenge",
                      "id": "test"
                    }
                  ]
                }"#]],
        );
    }
}
