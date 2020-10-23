use bb8_redis_stuck::errors::*;
use mobc::{Connection, Pool};
use mobc_redis::redis::{AsyncCommands, FromRedisValue};
use mobc_redis::{redis, RedisConnectionManager};

use anyhow::anyhow;
use bb8_redis_stuck::api::some_service_server::SomeService;
use bb8_redis_stuck::api::{GetRequest, GetResponse, SetRequest, SetResponse};
use tonic::{Request, Response, Status};

use bb8_redis_stuck::{Result, StdResult};

use tonic::transport::Server;

use bb8_redis_stuck::api::some_service_server::SomeServiceServer;

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "mobc_server=debug");
    env_logger::init();

    let client = redis::Client::open("redis://127.0.0.1:6379")?;
    let redis_conn_manager = RedisConnectionManager::new(client);
    // Ok(Pool::builder()
    //     .get_timeout(Some(Duration::from_secs(CACHE_POOL_TIMEOUT_SECONDS)))
    //     .max_open(CACHE_POOL_MAX_OPEN)
    //     .max_idle(CACHE_POOL_MAX_IDLE)
    //     .max_lifetime(Some(Duration::from_secs(CACHE_POOL_EXPIRE_SECONDS)))
    //     .build(manager))
    let redis_pool = Pool::builder().build(redis_conn_manager);
    let some_service_impl = SomeServiceImpl { redis_pool };

    log::info!("starting server...");

    let grpc_service = SomeServiceServer::new(some_service_impl);
    Server::builder()
        .add_service(grpc_service)
        .serve("0.0.0.0:50061".parse()?)
        .await;

    Ok(())
}

struct SomeServiceImpl {
    pub redis_pool: Pool<RedisConnectionManager>,
}

#[tonic::async_trait]
impl SomeService for SomeServiceImpl {
    async fn get(&self, request: Request<GetRequest>) -> StdResult<Response<GetResponse>, Status> {
        log::debug!("Got a get request: {:?}", request);
        let response = get_handler(&self.redis_pool, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn set(&self, request: Request<SetRequest>) -> StdResult<Response<SetResponse>, Status> {
        log::debug!("Got a set request: {:?}", request);
        let response = set_handler(&self.redis_pool, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}

async fn get_handler(
    redis_pool: &Pool<RedisConnectionManager>,
    request: GetRequest,
) -> Result<GetResponse> {
    let mut conn = redis_pool.get().await?;
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

async fn set_handler(
    redis_pool: &Pool<RedisConnectionManager>,
    request: SetRequest,
) -> Result<SetResponse> {
    let mut conn = redis_pool.get().await?;
    let response: redis::RedisResult<()> = conn.set(request.id, request.status).await;
    match response {
        Ok(_) => Ok(SetResponse {}),
        Err(err) => {
            log::error!("fail to set status to redis, reason: '{:?}'", err);
            Err(anyhow!("redis error").into())
        }
    }
}
