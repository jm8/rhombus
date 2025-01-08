use rhombus::{
    axum::Router,
    internal::{database::provider::Connection, grpc::get_api_key, router::RouterState},
    plugin::{PluginMeta, RunContext},
    Plugin, Result,
};
use tonic::async_trait;
use tracing_subscriber::EnvFilter;

pub mod proto {
    use tonic::include_proto;

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("myplugin_descriptor");
    include_proto!("myplugin");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("rhombus=trace,plugin=trace"))
                .unwrap(),
        )
        .init();

    let app = rhombus::Builder::default()
        .load_env()
        .config_source(rhombus::config::File::with_name("config"))
        .upload_provider(rhombus::LocalUploadProvider::new("uploads".into()))
        .plugin(MyPlugin)
        .build()
        .await
        .unwrap();

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    app.serve(listener).await;
}

struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: env!("CARGO_PKG_DESCRIPTION").into(),
        }
    }

    async fn run(&self, context: &mut RunContext<'_>) -> Result<Router<RouterState>> {
        context
            .grpc_builder
            .add_service(proto::my_plugin_server::MyPluginServer::new(GrpcImpl {
                db: context.db.clone(),
                root_api_key: context.settings.read().await.root_api_key.clone(),
            }))
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET);

        Ok(Router::new())
    }
}

pub struct GrpcImpl {
    pub db: Connection,
    pub root_api_key: Option<String>,
}

#[async_trait]
impl proto::my_plugin_server::MyPlugin for GrpcImpl {
    async fn reverse_user_name(
        &self,
        request: tonic::Request<proto::ReverseUserNameRequest>,
    ) -> std::result::Result<tonic::Response<proto::ReverseUserNameReply>, tonic::Status> {
        let key = get_api_key(&request)?.to_owned();

        let user_id = request.into_inner().user_id;

        let is_authorized = if self
            .root_api_key
            .as_ref()
            .is_some_and(|root_api_key| &key == root_api_key)
        {
            true
        } else {
            self.db
                .get_user_from_api_key(&key)
                .await
                .is_ok_and(|db_user| db_user.is_admin || db_user.id == user_id)
        };

        if !is_authorized {
            return Err(tonic::Status::unauthenticated(
                "Must be admin to reverse another user's name",
            ));
        }

        let user = self
            .db
            .get_user_from_id(user_id)
            .await
            .map_err(|_| tonic::Status::not_found("failed to get user"))?;
        let new_user_name = user.name.chars().rev().collect::<String>();
        self.db
            .set_account_name(user.id, user.team_id, &new_user_name, 0)
            .await
            .map_err(|_| tonic::Status::internal("failed to update user name"))?
            .map_err(|_| tonic::Status::internal("failed to update user name"))?;
        Ok(tonic::Response::new(proto::ReverseUserNameReply {
            new_user_name,
        }))
    }
}
