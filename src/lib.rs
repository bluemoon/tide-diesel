use std::sync::Arc;

use async_std::sync::{Mutex, MutexGuard};
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use tide::{utils::async_trait, Middleware, Next, Request};

pub type PooledPgConn = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct DieselMiddleware {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DieselMiddleware {
    pub fn new(db_uri: &'_ str) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let manager = ConnectionManager::<PgConnection>::new(db_uri);
        let pg_conn = diesel::r2d2::Builder::<ConnectionManager<PgConnection>>::new()
            .max_size(2)
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
        if req.ext::<Arc<Mutex<PooledPgConn>>>().is_some() {
            return Ok(next.run(req).await);
        }

        let conn: Arc<Mutex<PooledPgConn>> = Arc::new(Mutex::new(self.pool.get()?));
        req.set_ext(conn.clone());
        let res = next.run(req).await;
        Ok(res)
    }
}

#[async_trait]
pub trait DieselRequestExt {
    async fn pg_conn<'req>(&'req self) -> MutexGuard<'req, PooledPgConn>;
}

#[async_trait]
impl<T: Send + Sync + 'static> DieselRequestExt for Request<T> {
    async fn pg_conn<'req>(&'req self) -> MutexGuard<'req, PooledPgConn> {
        let pg_conn: &Arc<Mutex<PooledPgConn>> =
            self.ext().expect("You must install Diesel middleware");

        pg_conn.lock().await
    }
}
