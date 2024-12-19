use crate::grpc::proto::rhombus_server::{Rhombus, RhombusServer};
use crate::grpc::proto::{
    self, Author, Category, Challenge,
    ChallengeAttachment, ChallengeData, ChallengeDataPatch, HelloReply, HelloRequest,
};
use crate::internal::database::provider::Connection;
use tonic::{transport::server::Router, transport::Server, Request, Response, Status};

use super::challenge_diff::diff_challenge_data;

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
