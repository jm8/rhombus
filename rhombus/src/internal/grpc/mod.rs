use proto::rhombus_server::{Rhombus, RhombusServer};
use proto::{HelloReply, HelloRequest};
use tonic::{transport::server::Router, transport::Server, Request, Response, Status};

pub mod proto {
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("rhombus_descriptor");
    tonic::include_proto!("rhombus");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

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
}

pub fn make_server() -> Router {
    let rhombus = MyGreeter::default();
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let service = RhombusServer::new(rhombus);

    Server::builder()
        .add_service(reflection_service)
        .add_service(service)
}
