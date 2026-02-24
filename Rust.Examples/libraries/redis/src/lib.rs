//! This library provides basic functions to connect and interact with Redis

use deadpool_redis::{
    Config,
    Runtime::Tokio1,
    redis::{Cmd, ErrorKind::IoError, FromRedisValue, cmd},
};

pub use deadpool_redis::Connection;
pub use deadpool_redis::CreatePoolError;
pub use deadpool_redis::Pool as RdPool;
pub use deadpool_redis::redis::AsyncCommands;
pub use deadpool_redis::redis::RedisError;

/// This struct provides redis functions
pub struct Redis;

impl Redis {
    /// This function creates Redis pool connection
    pub fn new(url: &str) -> Result<RdPool, CreatePoolError> {
        let config = Config::from_url(url);
        config.create_pool(Some(Tokio1))
    }

    pub async fn _get_connection(pool: &RdPool) -> Result<Connection, RedisError> {
        let mut _client = pool.get().await.map_err(|err| {
            RedisError::from((
                IoError,
                "Error getting redis connection: {}",
                err.to_string(),
            ))
        });
        _client
    }

    /// This is redis `KEYS` command
    pub async fn keys(
        pool: &RdPool,
        filter_pattern: Option<&str>,
    ) -> Result<Vec<String>, RedisError> {
        let mut client = Self::_get_connection(pool).await?;
        // Prepare the filter pattern
        let filter = match filter_pattern {
            None => "*",
            Some(filter) => filter,
        };
        let res: Result<Vec<String>, RedisError> =
            cmd("KEYS").arg(&[&filter]).query_async(&mut client).await;
        res
    }

    /// This is redis `GET` command
    pub async fn get<T>(pool: &RdPool, key: &str) -> Result<T, RedisError>
    where
        T: FromRedisValue,
    {
        let mut client = Self::_get_connection(pool).await?;
        let values: Result<T, RedisError> = cmd("GET").arg(&[&key]).query_async(&mut client).await;
        values
    }

    /// This is redis `MGET` command
    pub async fn mget<T>(pool: &RdPool, keys: Vec<&str>) -> Result<Vec<Vec<T>>, RedisError>
    where
        T: FromRedisValue,
    {
        let mut client = Self::_get_connection(pool).await?;
        let values: Result<Vec<Vec<T>>, RedisError> =
            cmd("MGET").arg(&keys).query_async(&mut client).await;
        values
    }

    /// This is redis `SET` command
    pub async fn set(pool: &RdPool, key: &str, value: &str) -> Result<(), RedisError> {
        let mut client = Self::_get_connection(pool).await?;
        let res: Result<(), RedisError> =
            cmd("SET").arg(&[key, value]).query_async(&mut client).await;
        res
    }

    /// This is redis `DEL` command
    pub async fn del(pool: &RdPool, key: Vec<&str>) -> Result<(), RedisError> {
        let mut client = Self::_get_connection(pool).await?;
        let res: Result<(), RedisError> = cmd("DEL").arg(&key).query_async(&mut client).await;
        res
    }

    /// This is redis `EXISTS` command
    pub async fn exists(pool: &RdPool, key: &str) -> Result<bool, RedisError> {
        let mut client = Self::_get_connection(pool).await?;
        let res: Result<bool, RedisError> =
            cmd("EXISTS").arg(&[&key]).query_async(&mut client).await;
        res
    }

    /// This is redis `EXPIRE` command
    pub async fn expire(pool: &RdPool, key: &str, time: i64) -> Result<(), RedisError> {
        let mut client = Self::_get_connection(pool).await?;
        let res: Result<(), RedisError> = Cmd::expire(key, time).query_async(&mut client).await;
        res
    }
}

// cargo test --package lib_redis check_redis -- --nocapture
#[cfg(test)]
mod test {
    use super::*;
    use app_config::AppConfig;
    #[test]
    fn check_redis() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let config = AppConfig::new();
            let redis_url = config.redis_url.clone();
            let pool = Redis::new(&redis_url).unwrap();
            let key = "test_key";
            let value = "test_value";
            // Set key-value pair
            let res = Redis::set(&pool, key, value).await;
            assert_eq!(res.is_ok(), true);
            // Get value by key
            let res = Redis::get::<String>(&pool, key).await;
            assert_eq!(res.is_ok(), true);
            assert_eq!(res.unwrap(), value);
            // Delete key
            let res = Redis::del(&pool, vec![key]).await;
            assert_eq!(res.is_ok(), true);
            // Check if key exists
            let res = Redis::exists(&pool, key).await.unwrap();
            assert_eq!(res, false);
        })
    }
}
