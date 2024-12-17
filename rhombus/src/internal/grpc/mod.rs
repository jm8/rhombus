use std::collections::HashMap;

use crate::internal::database::provider::Connection;
use proto::rhombus_server::{Rhombus, RhombusServer};
use proto::{Author, ChallengeData, ChallengeDataPatch, HelloReply, HelloRequest};
use tonic::{transport::server::Router, transport::Server, Request, Response, Status};

pub mod proto {
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("rhombus_descriptor");
    tonic::include_proto!("rhombus");
}

pub struct MyGreeter {
    db: Connection,
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
        request: tonic::Request<()>,
    ) -> Result<tonic::Response<ChallengeData>, Status> {
        let a = request.into_inner();
        println!("{:?}", a);

        let chals = self
            .db
            .get_challenges()
            .await
            .map_err(|_| Status::internal("failed to get challenges"))?;

        Ok(Response::new(ChallengeData {
            challenges: HashMap::new(),
            categories: HashMap::new(),
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
        }))
    }

    async fn diff_challenges(
        &self,
        request: tonic::Request<ChallengeData>,
    ) -> Result<tonic::Response<ChallengeDataPatch>, Status> {
        let a = request.into_inner();
        println!("{:?}", a);
        let reply = ChallengeDataPatch { actions: vec![] };
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
