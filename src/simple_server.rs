use anyhow::anyhow;
use bb8_redis_stuck::api::some_service_server::{SomeService, SomeServiceServer};
use bb8_redis_stuck::api::{GetRequest, GetResponse, SetRequest, SetResponse};
use bb8_redis_stuck::{Result, StdResult};
use redis::AsyncCommands;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "simple_server=debug");
    env_logger::init();

    let redis_client = redis::Client::open("redis://127.0.0.1:6379")?;
    let some_service_impl = SomeServiceImpl { redis_client };

    log::info!("starting server...");

    let grpc_service = SomeServiceServer::new(some_service_impl);
    Server::builder()
        .add_service(grpc_service)
        .serve("0.0.0.0:50061".parse()?)
        .await;

    Ok(())
}

#[derive(Debug)]
struct SomeServiceImpl {
    pub redis_client: redis::Client,
}

#[tonic::async_trait]
impl SomeService for SomeServiceImpl {
    async fn get(&self, request: Request<GetRequest>) -> StdResult<Response<GetResponse>, Status> {
        log::debug!("Got a get request: {:?}", request);
        let response = get_handler(&self.redis_client, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn set(&self, request: Request<SetRequest>) -> StdResult<Response<SetResponse>, Status> {
        log::debug!("Got a set request: {:?}", request);
        let response = set_handler(&self.redis_client, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}

async fn get_handler(redis_client: &redis::Client, request: GetRequest) -> Result<GetResponse> {
    let mut conn = redis_client.get_async_connection().await?;
    let response: redis::RedisResult<Option<i64>> = conn.get(request.id).await;
    match response {
        Ok(result) => Ok(GetResponse {
            id: request.id,
            status: result.unwrap_or(-1),
        }),
        Err(err) => {
            log::error!("fail to get camera status from redis, reason: '{:?}'", err);
            Err(anyhow!("redis error").into())
        }
    }
}

async fn set_handler(redis_client: &redis::Client, request: SetRequest) -> Result<SetResponse> {
    let mut conn = redis_client.get_async_connection().await?;
    let response: redis::RedisResult<()> = conn.set(request.id, request.status).await;
    match response {
        Ok(_) => Ok(SetResponse {}),
        Err(err) => {
            log::error!("fail to set status to redis, reason: '{:?}'", err);
            Err(anyhow!("redis error").into())
        }
    }
}
