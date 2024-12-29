use crate::grpc::proto::rhombus_server::{Rhombus, RhombusServer};
use crate::grpc::proto::{
    self, Author, Category, Challenge, ChallengeAttachment, ChallengeData, ChallengeDataPatch,
    GetAttachmentByHashRequest, GetAttachmentByHashResponse, HelloReply, HelloRequest,
};
use crate::internal::database::provider::Connection;
use tonic::{transport::server::Router, transport::Server, Request, Response, Status};

use super::challenge_diff::diff_challenge_data;

pub struct MyGreeter {
    db: Connection,
    grpc_psk: Option<String>,
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
                            metadata: serde_json::to_string(&v.metadata).ok(),
                            points: Some(v.points),
                            score_type: Some(v.score_type.clone()),
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

    pub fn require_admin<T>(&self, req: Request<T>) -> Result<T, Status> {
        let (metadata, _, inner) = req.into_parts();
        let Some(authorization) = metadata.get("authorization") else {
            return Err(Status::unauthenticated("missing authorization header"));
        };
        eprintln!("AUTHORIZATION == {:?}", authorization);
        Ok(inner)
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
        request: Request<()>,
    ) -> Result<Response<ChallengeData>, Status> {
        self.require_admin(request)?;
        Ok(Response::new(self.get_challenges_from_db().await?))
    }

    async fn diff_challenges(
        &self,
        request: Request<ChallengeData>,
    ) -> Result<Response<ChallengeDataPatch>, Status> {
        let new = self.require_admin(request)?;
        let old = self.get_challenges_from_db().await?;
        let reply = diff_challenge_data(&old, &new);
        Ok(Response::new(reply))
    }

    async fn get_attachment_by_hash(
        &self,
        request: Request<GetAttachmentByHashRequest>,
    ) -> Result<Response<GetAttachmentByHashResponse>, Status> {
        let hash = self.require_admin(request)?.hash;

        let url = self
            .db
            .get_attachment_url_by_hash(&hash)
            .await
            .map_err(|_| Status::internal("failed to lookup attachment"))?;

        Ok(Response::new(GetAttachmentByHashResponse { url }))
    }
}

pub fn make_server(db: Connection, grpc_psk: Option<String>) -> Router {
    let rhombus = MyGreeter { db, grpc_psk };
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let service = RhombusServer::new(rhombus);

    Server::builder()
        .add_service(reflection_service)
        .add_service(service)
}
