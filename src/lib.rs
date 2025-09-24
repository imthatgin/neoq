use async_trait::async_trait;
use neo4rs::{BoltType, Graph, RowStream, Txn};
use serde::{de::DeserializeOwned, Serialize};
use sha2::Digest;

use crate::migrations::MigrationError;

pub mod index_file;
pub mod migrations;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Neo4rsError(#[from] neo4rs::Error),

    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    MigrationError(#[from] MigrationError),
}

#[async_trait]
pub trait NeoResultSet {
    async fn value<T: DeserializeOwned>(
        &mut self,
        tx: &mut Txn,
    ) -> Result<Option<T>, neo4rs::Error>;

    async fn values<T: DeserializeOwned + Send>(
        &mut self,
        tx: &mut Txn,
    ) -> Result<Vec<T>, neo4rs::Error>;
}

#[async_trait]
impl NeoResultSet for neo4rs::RowStream {
    /// Retrieves a single value from a `RowStream` and converts the
    /// row to an instance of `T` if possible
    async fn value<T: DeserializeOwned>(
        &mut self,
        mut tx: &mut Txn,
    ) -> Result<Option<T>, neo4rs::Error> {
        let row = self.next(&mut tx).await?;
        if let Some(value) = row {
            match value.to::<T>() {
                Ok(val) => return Ok(Some(val)),
                Err(err) => return Err(neo4rs::Error::DeserializationError(err)),
            };
        } else {
            return Ok(None);
        }
    }

    /// Retrieves a list of values from a `RowStream` and converts each
    /// row to an instance of `T` if possible
    async fn values<T: DeserializeOwned + Send>(
        &mut self,
        mut tx: &mut Txn,
    ) -> Result<Vec<T>, neo4rs::Error> {
        let mut output = Vec::new();

        while let Some(ref row) = self.next(&mut tx).await? {
            match row.to::<T>() {
                Ok(entry) => {
                    let deserialized: T = entry;
                    output.push(deserialized);
                }
                Err(err) => return Err(neo4rs::Error::DeserializationError(err)),
            };
        }

        Ok(output)
    }
}

#[async_trait]
pub trait NeoQueryExt {
    async fn execute_value<T: DeserializeOwned>(
        self,
        graph: &Graph,
    ) -> Result<Option<T>, neo4rs::Error>;

    async fn execute_values<T: DeserializeOwned + Send>(
        self,
        graph: &Graph,
    ) -> Result<Vec<T>, neo4rs::Error>;

    async fn execute_value_tx<T: DeserializeOwned>(
        self,
        tx: &mut Txn,
    ) -> Result<Option<T>, neo4rs::Error>;

    async fn execute_values_tx<T: DeserializeOwned + Send>(
        self,
        tx: &mut Txn,
    ) -> Result<Vec<T>, neo4rs::Error>;

    async fn execute_stream(self, tx: &mut Txn) -> Result<RowStream, neo4rs::Error>;
}

#[async_trait]
impl NeoQueryExt for neo4rs::Query {
    async fn execute_value<T: DeserializeOwned>(
        self,
        graph: &Graph,
    ) -> Result<Option<T>, neo4rs::Error> {
        let mut stream = graph.execute(self).await?;
        let row = stream.next().await?;
        if let Some(value) = row {
            match value.to::<T>() {
                Ok(val) => return Ok(Some(val)),
                Err(err) => return Err(neo4rs::Error::DeserializationError(err)),
            };
        } else {
            return Ok(None);
        }
    }

    async fn execute_values<T: DeserializeOwned + Send>(
        self,
        graph: &Graph,
    ) -> Result<Vec<T>, neo4rs::Error> {
        let mut output = Vec::new();

        let mut stream = graph.execute(self).await?;
        while let Some(ref row) = stream.next().await? {
            match row.to::<T>() {
                Ok(entry) => {
                    let deserialized: T = entry;
                    output.push(deserialized);
                }
                Err(err) => return Err(neo4rs::Error::DeserializationError(err)),
            };
        }

        Ok(output)
    }

    async fn execute_value_tx<T: DeserializeOwned>(
        self,
        tx: &mut Txn,
    ) -> Result<Option<T>, neo4rs::Error> {
        let mut stream = tx.execute(self).await?;

        let row = stream.next(tx.handle()).await?;
        if let Some(value) = row {
            match value.to::<T>() {
                Ok(val) => return Ok(Some(val)),
                Err(err) => return Err(neo4rs::Error::DeserializationError(err)),
            };
        } else {
            return Ok(None);
        }
    }

    async fn execute_values_tx<T: DeserializeOwned + Send>(
        self,
        tx: &mut Txn,
    ) -> Result<Vec<T>, neo4rs::Error> {
        let mut output = Vec::new();

        let mut stream = tx.execute(self).await?;
        while let Some(ref row) = stream.next(tx.handle()).await? {
            match row.to::<T>() {
                Ok(entry) => {
                    let deserialized: T = entry;
                    output.push(deserialized);
                }
                Err(err) => return Err(neo4rs::Error::DeserializationError(err)),
            };
        }

        Ok(output)
    }

    async fn execute_stream(self, tx: &mut Txn) -> Result<RowStream, neo4rs::Error> {
        let stream = tx.execute(self).await?;
        Ok(stream)
    }
}

/// Used to convert an instance of T into a parameterizable map of properties that neo4rs can use.
pub fn parameterize<T: Serialize>(instance: T) -> BoltType {
    struct_to_hashmap(&instance).unwrap()
}

/// Internal function that converts the type to a JSON serializable, and then tries
/// to get a [`BoltType`] from it.
fn struct_to_hashmap<T: Serialize>(instance: &T) -> Result<BoltType, crate::Error> {
    let value = serde_json::to_value(instance)?;
    let bolt_type = BoltType::try_from(value)?;

    Ok(bolt_type)
}

/// Fetches the shasum of the index file script
fn get_str_shasum(index_script: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(index_script);

    let hash_result = hasher.finalize();

    format!("{:x}", hash_result)
}
