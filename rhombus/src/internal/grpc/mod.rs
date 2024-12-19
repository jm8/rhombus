use crate::internal::database::provider::Connection;
use proto::rhombus_server::{Rhombus, RhombusServer};
use proto::{
    challenge_data_patch_action, Author, AuthorPatch, Category, CategoryPatch, Challenge,
    ChallengeAttachment, ChallengeAttachmentsPatch, ChallengeData, ChallengeDataPatch,
    ChallengeDataPatchAction, ChallengePatch, CreateAuthor, CreateCategory, CreateChallenge,
    DeleteAuthor, DeleteCategory, DeleteChallenge, HelloReply, HelloRequest, OptionalStringPatch,
    PatchAuthor, PatchCategory, PatchChallenge, StringPatch,
};
use tonic::{transport::server::Router, transport::Server, Request, Response, Status};

pub mod proto {
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("rhombus_descriptor");
    tonic::include_proto!("rhombus");
}

pub struct MyGreeter {
    db: Connection,
}

impl MyGreeter {
    async fn get_challenges_from_db(&self) -> Result<ChallengeData, Status> {
        let chals = self
            .db
            .get_challenges()
            .await
            .map_err(|_| Status::internal("failed to get challenges"))?;

        Ok(ChallengeData {
            challenges: chals
                .challenges
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        Challenge {
                            author: v.author_id.clone(),
                            name: v.name.clone(),
                            description: v.description.clone(),
                            category: v.category_id.clone(),
                            ticket_template: v.ticket_template.clone(),
                            files: v
                                .attachments
                                .iter()
                                .map(|a| ChallengeAttachment {
                                    name: a.name.clone(),
                                    url: a.url.clone(),
                                })
                                .collect(),
                            flag: v.flag.clone(),
                            healthscript: v.healthscript.clone(),
                        },
                    )
                })
                .collect(),
            categories: chals
                .categories
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        Category {
                            color: v.color.clone(),
                            name: v.name.clone(),
                        },
                    )
                })
                .collect(),
            authors: chals
                .authors
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        Author {
                            avatar_url: v.avatar_url.clone(),
                            discord_id: v.discord_id.clone().to_string(),
                            name: v.name.clone(),
                        },
                    )
                })
                .collect(),
        })
    }
}

#[tonic::async_trait]
impl Rhombus for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }

    async fn get_challenges(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<ChallengeData>, Status> {
        Ok(Response::new(self.get_challenges_from_db().await?))
    }

    async fn diff_challenges(
        &self,
        request: tonic::Request<ChallengeData>,
    ) -> Result<tonic::Response<ChallengeDataPatch>, Status> {
        let new = request.into_inner();
        let old = self.get_challenges_from_db().await?;
        let reply = diff_challenge_data(&old, &new);
        Ok(Response::new(reply))
    }
}
pub fn diff_challenge_data(old: &ChallengeData, new: &ChallengeData) -> ChallengeDataPatch {
    let mut patch = ChallengeDataPatch { actions: vec![] };

    for (id, challenge) in &old.challenges {
        match new.challenges.get(id) {
            Some(new_challenge) => {
                if challenge != new_challenge {
                    let challenge_patch = diff_challenge(challenge, new_challenge);
                    patch.actions.push(ChallengeDataPatchAction {
                        action: Some(challenge_data_patch_action::Action::PatchChallenge(
                            PatchChallenge {
                                id: id.clone(),
                                patch: Some(challenge_patch),
                            },
                        )),
                    });
                }
            }
            None => {
                patch.actions.push(ChallengeDataPatchAction {
                    action: Some(challenge_data_patch_action::Action::DeleteChallenge(
                        DeleteChallenge { id: id.clone() },
                    )),
                });
            }
        }
    }

    for (id, challenge) in &new.challenges {
        if !old.challenges.contains_key(id) {
            patch.actions.push(ChallengeDataPatchAction {
                action: Some(challenge_data_patch_action::Action::CreateChallenge(
                    CreateChallenge {
                        id: id.clone(),
                        value: Some(challenge.clone()),
                    },
                )),
            });
        }
    }

    for (id, author) in &old.authors {
        match new.authors.get(id) {
            Some(new_author) => {
                if author != new_author {
                    let author_patch = diff_author(author, new_author);
                    patch.actions.push(ChallengeDataPatchAction {
                        action: Some(challenge_data_patch_action::Action::PatchAuthor(
                            PatchAuthor {
                                id: id.clone(),
                                patch: Some(author_patch),
                            },
                        )),
                    });
                }
            }
            None => {
                patch.actions.push(ChallengeDataPatchAction {
                    action: Some(challenge_data_patch_action::Action::DeleteAuthor(
                        DeleteAuthor { id: id.clone() },
                    )),
                });
            }
        }
    }

    for (id, author) in &new.authors {
        if !old.authors.contains_key(id) {
            patch.actions.push(ChallengeDataPatchAction {
                action: Some(challenge_data_patch_action::Action::CreateAuthor(
                    CreateAuthor {
                        id: id.clone(),
                        value: Some(author.clone()),
                    },
                )),
            });
        }
    }

    for (id, category) in &old.categories {
        match new.categories.get(id) {
            Some(new_category) => {
                if category != new_category {
                    let category_patch = diff_category(category, new_category);
                    patch.actions.push(ChallengeDataPatchAction {
                        action: Some(challenge_data_patch_action::Action::PatchCategory(
                            PatchCategory {
                                id: id.clone(),
                                patch: Some(category_patch),
                            },
                        )),
                    });
                }
            }
            None => {
                patch.actions.push(ChallengeDataPatchAction {
                    action: Some(challenge_data_patch_action::Action::DeleteCategory(
                        DeleteCategory { id: id.clone() },
                    )),
                });
            }
        }
    }

    for (id, category) in &new.categories {
        if !old.categories.contains_key(id) {
            patch.actions.push(ChallengeDataPatchAction {
                action: Some(challenge_data_patch_action::Action::CreateCategory(
                    CreateCategory {
                        id: id.clone(),
                        value: Some(category.clone()),
                    },
                )),
            });
        }
    }

    patch
}

pub fn diff_challenge(old: &Challenge, new: &Challenge) -> ChallengePatch {
    let mut patch = ChallengePatch::default();

    if old.name != new.name {
        patch.name = Some(StringPatch {
            old: old.name.clone(),
            new: new.name.clone(),
        });
    }
    if old.description != new.description {
        patch.description = Some(StringPatch {
            old: old.description.clone(),
            new: new.description.clone(),
        });
    }
    if old.category != new.category {
        patch.category = Some(StringPatch {
            old: old.category.clone(),
            new: new.category.clone(),
        });
    }
    if old.author != new.author {
        patch.author = Some(StringPatch {
            old: old.author.clone(),
            new: new.author.clone(),
        });
    }
    if old.ticket_template != new.ticket_template {
        patch.ticket_template = Some(OptionalStringPatch {
            old: old.ticket_template.clone(),
            new: new.ticket_template.clone(),
        });
    }
    if old.files != new.files {
        patch.files = Some(ChallengeAttachmentsPatch {
            old: old.files.clone(),
            new: new.files.clone(),
        });
    }
    if old.flag != new.flag {
        patch.flag = Some(StringPatch {
            old: old.flag.clone(),
            new: new.flag.clone(),
        });
    }
    if old.healthscript != new.healthscript {
        patch.healthscript = Some(OptionalStringPatch {
            old: old.healthscript.clone(),
            new: new.healthscript.clone(),
        });
    }

    patch
}

pub fn diff_author(old: &Author, new: &Author) -> AuthorPatch {
    let mut patch = AuthorPatch::default();

    if old.name != new.name {
        patch.name = Some(StringPatch {
            old: old.name.clone(),
            new: new.name.clone(),
        });
    }
    if old.avatar_url != new.avatar_url {
        patch.avatar_url = Some(StringPatch {
            old: old.avatar_url.clone(),
            new: new.avatar_url.clone(),
        });
    }
    if old.discord_id != new.discord_id {
        patch.discord_id = Some(StringPatch {
            old: old.discord_id.clone(),
            new: new.discord_id.clone(),
        });
    }

    patch
}

pub fn diff_category(old: &Category, new: &Category) -> CategoryPatch {
    let mut patch = CategoryPatch::default();

    if old.name != new.name {
        patch.name = Some(StringPatch {
            old: old.name.clone(),
            new: new.name.clone(),
        });
    }
    if old.color != new.color {
        patch.color = Some(StringPatch {
            old: old.color.clone(),
            new: new.color.clone(),
        });
    }

    patch
}

#[cfg(test)]
mod test {
    use super::*;
    use expect_test::{expect, Expect};
    use std::collections::HashMap;

    fn check(old: ChallengeData, new: ChallengeData, expected: Expect) {
        let actual = diff_challenge_data(&old, &new);
        expected.assert_debug_eq(&actual);
    }

    #[test]
    fn test_modify_challenge() {
        let old = ChallengeData {
            challenges: HashMap::from([(
                "test".to_string(),
                Challenge {
                    name: "Test challenge".to_string(),
                    description: "Test".to_string(),
                    category: "abc".to_string(),
                    author: "john daker".to_string(),
                    files: vec![],
                    flag: "rhombusctf{abc}".to_string(),
                    ticket_template: None,
                    healthscript: None,
                },
            )]),
            categories: HashMap::new(),
            authors: HashMap::new(),
        };

        let new = ChallengeData {
            challenges: HashMap::from([(
                "test".to_string(),
                Challenge {
                    flag: "rhombusctf{def}".to_string(),
                    ..old.challenges["test"].clone()
                },
            )]),
            categories: HashMap::new(),
            authors: HashMap::new(),
        };

        check(
            old,
            new,
            expect![[r#"
                ChallengeDataPatch {
                    actions: [
                        ChallengeDataPatchAction {
                            action: Some(
                                PatchChallenge(
                                    PatchChallenge {
                                        id: "test",
                                        patch: Some(
                                            ChallengePatch {
                                                name: None,
                                                description: None,
                                                category: None,
                                                author: None,
                                                ticket_template: None,
                                                files: None,
                                                flag: Some(
                                                    StringPatch {
                                                        old: "rhombusctf{abc}",
                                                        new: "rhombusctf{def}",
                                                    },
                                                ),
                                                healthscript: None,
                                            },
                                        ),
                                    },
                                ),
                            ),
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn test_change_challenge_id() {
        let old = ChallengeData {
            challenges: HashMap::from([(
                "test".to_string(),
                Challenge {
                    name: "Test challenge".to_string(),
                    description: "Test".to_string(),
                    category: "abc".to_string(),
                    author: "john daker".to_string(),
                    files: vec![],
                    flag: "rhombusctf{abc}".to_string(),
                    ticket_template: None,
                    healthscript: None,
                },
            )]),
            categories: HashMap::new(),
            authors: HashMap::new(),
        };

        let new = ChallengeData {
            challenges: HashMap::from([(
                "quiz".to_string(),
                Challenge {
                    ..old.challenges["test"].clone()
                },
            )]),
            categories: HashMap::new(),
            authors: HashMap::new(),
        };

        check(
            old,
            new,
            expect![[r#"
                ChallengeDataPatch {
                    actions: [
                        ChallengeDataPatchAction {
                            action: Some(
                                DeleteChallenge(
                                    DeleteChallenge {
                                        id: "test",
                                    },
                                ),
                            ),
                        },
                        ChallengeDataPatchAction {
                            action: Some(
                                CreateChallenge(
                                    CreateChallenge {
                                        id: "quiz",
                                        value: Some(
                                            Challenge {
                                                name: "Test challenge",
                                                description: "Test",
                                                category: "abc",
                                                author: "john daker",
                                                ticket_template: None,
                                                files: [],
                                                flag: "rhombusctf{abc}",
                                                healthscript: None,
                                            },
                                        ),
                                    },
                                ),
                            ),
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn test_add_challenge_and_category_and_author() {
        let old = ChallengeData {
            challenges: HashMap::new(),
            categories: HashMap::new(),
            authors: HashMap::new(),
        };

        let new = ChallengeData {
            challenges: HashMap::from([(
                "twoplustwo".to_string(),
                Challenge {
                    name: "2+2".to_string(),
                    description: "solve it".to_string(),
                    category: "math".to_string(),
                    author: "jdaker".to_string(),
                    files: vec![ChallengeAttachment {
                        name: "equation.pdf".to_string(),
                        url: "https://example.com/equation.pdf".to_string(),
                    }],
                    flag: "rhombusctf{abc}".to_string(),
                    ticket_template: None,
                    healthscript: None,
                },
            )]),
            categories: HashMap::from([(
                "math".to_string(),
                Category {
                    name: "Mathematics".to_string(),
                    color: "blue".to_string(),
                },
            )]),
            authors: HashMap::from([(
                "jdaker".to_string(),
                Author {
                    name: "John Daker".to_string(),
                    avatar_url: "https://www.gravatar.com/avatar/23463b99b62a72f26ed677cc556c44e8?s=200&d=identicon&r=g"
                        .to_string(),
                    discord_id: 12345678.to_string(),
                },
            )]),
        };

        check(
            old,
            new,
            expect![[r#"
                ChallengeDataPatch {
                    actions: [
                        ChallengeDataPatchAction {
                            action: Some(
                                CreateChallenge(
                                    CreateChallenge {
                                        id: "twoplustwo",
                                        value: Some(
                                            Challenge {
                                                name: "2+2",
                                                description: "solve it",
                                                category: "math",
                                                author: "jdaker",
                                                ticket_template: None,
                                                files: [
                                                    ChallengeAttachment {
                                                        name: "equation.pdf",
                                                        url: "https://example.com/equation.pdf",
                                                    },
                                                ],
                                                flag: "rhombusctf{abc}",
                                                healthscript: None,
                                            },
                                        ),
                                    },
                                ),
                            ),
                        },
                        ChallengeDataPatchAction {
                            action: Some(
                                CreateAuthor(
                                    CreateAuthor {
                                        id: "jdaker",
                                        value: Some(
                                            Author {
                                                name: "John Daker",
                                                avatar_url: "https://www.gravatar.com/avatar/23463b99b62a72f26ed677cc556c44e8?s=200&d=identicon&r=g",
                                                discord_id: "12345678",
                                            },
                                        ),
                                    },
                                ),
                            ),
                        },
                        ChallengeDataPatchAction {
                            action: Some(
                                CreateCategory(
                                    CreateCategory {
                                        id: "math",
                                        value: Some(
                                            Category {
                                                name: "Mathematics",
                                                color: "blue",
                                            },
                                        ),
                                    },
                                ),
                            ),
                        },
                    ],
                }
            "#]],
        );
    }

    #[test]
    fn test_modify_author() {
        let old = ChallengeData {
            challenges: HashMap::from([(
                "twoplustwo".to_string(),
                Challenge {
                    name: "2+2".to_string(),
                    description: "solve it".to_string(),
                    category: "math".to_string(),
                    author: "jdaker".to_string(),
                    files: vec![ChallengeAttachment {
                        name: "equation.pdf".to_string(),
                        url: "https://example.com/equation.pdf".to_string(),
                    }],
                    flag: "rhombusctf{abc}".to_string(),
                    ticket_template: None,
                    healthscript: None,
                },
            )]),
            categories: HashMap::from([(
                "math".to_string(),
                Category {
                    name: "Mathematics".to_string(),
                    color: "blue".to_string(),
                },
            )]),
            authors: HashMap::from([(
                "jdaker".to_string(),
                Author {
                    name: "John Daker".to_string(),
                    avatar_url: "https://www.gravatar.com/avatar/23463b99b62a72f26ed677cc556c44e8?s=200&d=identicon&r=g"
                        .to_string(),
                    discord_id: 12345678.to_string(),
                },
            )]),
        };

        let new = ChallengeData {
            authors: HashMap::from([(
                "jdaker".to_string(),
                Author {
                    name: "John Baker".to_string(),
                    discord_id: 87654321.to_string(),
                    ..old.authors["jdaker"].clone()
                },
            )]),
            ..old.clone()
        };

        check(
            old,
            new,
            expect![[r#"
                ChallengeDataPatch {
                    actions: [
                        ChallengeDataPatchAction {
                            action: Some(
                                PatchAuthor(
                                    PatchAuthor {
                                        id: "jdaker",
                                        patch: Some(
                                            AuthorPatch {
                                                name: Some(
                                                    StringPatch {
                                                        old: "John Daker",
                                                        new: "John Baker",
                                                    },
                                                ),
                                                avatar_url: None,
                                                discord_id: Some(
                                                    StringPatch {
                                                        old: "12345678",
                                                        new: "87654321",
                                                    },
                                                ),
                                            },
                                        ),
                                    },
                                ),
                            ),
                        },
                    ],
                }
            "#]],
        );
    }
}

pub fn make_server(db: Connection) -> Router {
    let rhombus = MyGreeter { db };
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let service = RhombusServer::new(rhombus);

    Server::builder()
        .add_service(reflection_service)
        .add_service(service)
}
