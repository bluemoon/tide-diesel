use async_std::sync::{Mutex, MutexGuard};
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use std::sync::Arc;
use tide::{utils::async_trait, Middleware, Next, Request};

pub type PooledPgConn = PooledConnection<ConnectionManager<PgConnection>>;
pub type PoolPgConn = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct DieselMiddleware {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DieselMiddleware {
    pub fn new(db_uri: &'_ str) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let manager = ConnectionManager::<PgConnection>::new(db_uri);
        let pg_conn = diesel::r2d2::Builder::<ConnectionManager<PgConnection>>::new()
            .build(manager)
            .map_err(|e| Box::new(e))?;
        Ok(Self { pool: pg_conn })
    }
}

impl AsRef<Pool<ConnectionManager<PgConnection>>> for DieselMiddleware {
    fn as_ref(&self) -> &Pool<ConnectionManager<PgConnection>> {
        &self.pool
    }
}

impl From<Pool<ConnectionManager<PgConnection>>> for DieselMiddleware {
    fn from(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<State> Middleware<State> for DieselMiddleware
where
    State: Clone + Send + Sync + 'static,
{
    async fn handle(&self, mut req: Request<State>, next: Next<'_, State>) -> tide::Result {
        if req.ext::<Arc<Mutex<PoolPgConn>>>().is_some() {
            return Ok(next.run(req).await);
        }

        let conn: Arc<PoolPgConn> = Arc::new(self.pool.clone());
        req.set_ext(conn.clone());
        let res = next.run(req).await;
        Ok(res)
    }
}

#[async_trait]
pub trait DieselRequestExt {
    async fn pg_conn<'req>(
        &'req self,
    ) -> std::result::Result<PooledPgConn, Box<dyn std::error::Error + Send + Sync + 'static>>;
    async fn pg_pool_conn<'req>(&'req self) -> MutexGuard<'req, PoolPgConn>;
}

#[async_trait]
impl<T: Send + Sync + 'static> DieselRequestExt for Request<T> {
    async fn pg_conn<'req>(
        &'req self,
    ) -> std::result::Result<PooledPgConn, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let pg_conn: &Arc<PoolPgConn> = self.ext().expect("You must install Diesel middleware");
        Ok(pg_conn.get().map_err(|e| Box::new(e))?)
    }

    async fn pg_pool_conn<'req>(&'req self) -> MutexGuard<'req, PoolPgConn> {
        let pg_conn: &Arc<Mutex<PoolPgConn>> =
            self.ext().expect("You must install Diesel middleware");
        pg_conn.lock().await
    }
}
