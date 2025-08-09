# neoq

Neo4j helper for Rust

## Migrations

```rust
let migrator = GraphMigrator::new();

let migrations = migrator.gather_migrations(Path::new("./migrations"));
let migrations_result = migrator
    .run_migrations(your_db, your_graph, migrations)
    .await;
```

## Result helpers

```rust
let mut results = tx
    .execute(query)
    .await?;

let session = results
    .value::<YourQueryResult>(&mut tx)
    .await

```

```rust
let mut results = tx
    .execute(query)
    .await?;

let session = results
    .values::<YourQueryResult>(&mut tx)
    .await
```
