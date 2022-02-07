# tide-diesel

[Tide][] middleware for [Diesel][] pooled connections &amp; transactions.

----

A [Tide][] middleware which holds a pool of Diesel database connections, and automatically hands
each [tide::Request][] a connection, which may transparently be either a database transaction,
or a direct pooled database connection.

When using this, use the `DieselRequestExt` extenstion trait to get the connection.

## Examples

### Basic
```rust
#[async_std::main]
async fn main() -> anyhow::Result<()> {
    use tide_diesel::DieselRequestExt;

    let mut app = tide::new();
    app.with(DieselMiddleware::new("postgres://localhost/a_database").await?);

    app.at("/").post(|req: tide::Request<()>| async move {
        let mut pg_conn = req.pg_conn().await;

        Ok("")
    });
    Ok(())
}
```
