use neo4rs::query;

use crate::get_str_shasum;

pub async fn update_indexes(
    driver: neo4rs::Graph,
    db: neo4rs::Database,
    index_script: String,
) -> Result<(), neo4rs::Error> {
    let sum = get_str_shasum(&index_script);

    let current_index_query = query(
        "MATCH (metaNode:DbIndexTracker {shaSum: $currentSum}) RETURN metaNode.shaSum AS sum LIMIT 1",
    )
    .param("currentSum", sum.clone());
    let mut current_index_hash = driver.execute_on(db.clone(), current_index_query).await?;

    // If the query returns a sum, it means that it matches. Otherwise
    // we just need to
    if let Some(row) = current_index_hash.next().await? {
        let sum_in_db = match row.get::<String>("sum") {
            Ok(sum) => sum,
            Err(err) => {
                tracing::error!("Unable to deserialize shasum from database: {}", err);
                return Err(neo4rs::Error::DeserializationError(err));
            }
        };

        tracing::debug!(
            expected_sum = sum,
            actual_sum = sum_in_db,
            "No changes needed to indexes"
        );
        return Ok(());
    }

    // First drop and then reindex
    drop_indexes(driver.clone(), db.clone()).await?;
    reindex_database(driver, db.clone(), &index_script, sum.clone()).await?;

    tracing::debug!(index_sum = sum, "Index file has been updated");
    tracing::info!("Database indexes have been updated");

    Ok(())
}

/// Uses `apoc.schema.assert` to nuke indexes and constraints.
async fn drop_indexes(driver: neo4rs::Graph, db: neo4rs::Database) -> Result<(), neo4rs::Error> {
    driver
        .run_on(
            db.clone(),
            query("CALL apoc.schema.assert({},{},true) YIELD label, key RETURN *"),
        )
        .await?;
    driver
        .run_on(
            db.clone(),
            query("MATCH (metaNode:DbIndexTracker) DETACH DELETE metaNode RETURN *"),
        )
        .await?;

    tracing::debug!("Indexes have been dropped as part of reindexing");

    Ok(())
}

async fn reindex_database(
    driver: neo4rs::Graph,
    db: neo4rs::Database,
    index_script: &str,
    index_sum: String,
) -> Result<(), neo4rs::Error> {
    let index_statements = index_script.split(";").map(|statement| query(statement));

    tracing::trace!("Sending index queries to db");
    let mut tx = driver.start_txn_on(db.clone()).await?;
    tx.run_queries(index_statements).await?;

    tx.commit().await?;
    tracing::trace!("Committed index query statements");

    let create_index_meta =
        query("CREATE (metaNode:DbIndexTracker {shaSum: $currentSum}) RETURN metaNode")
            .param("currentSum", index_sum.clone());
    driver.run_on(db.clone(), create_index_meta).await?;

    tracing::trace!(?index_sum, "Reindexing");

    Ok(())
}
