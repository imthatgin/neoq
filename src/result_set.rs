use async_trait::async_trait;
use neo4rs::{DetachedRowStream, Graph, Row, RowStream, RunResult, Txn};
use serde::de::DeserializeOwned;

/// AnyRowStream exists to unify queries where the context is a [`neo4rs::Graph`] or a [`neo4rs::Txn`].
/// This allows the user end API to be agnostic with regards to the type of context.
pub enum AnyRowStream<'a> {
    DetachedRowStream(DetachedRowStream),
    RowStream(&'a mut Txn, RowStream),
}

impl<'a> AnyRowStream<'a> {
    pub async fn next_row(&mut self) -> Result<Option<Row>, neo4rs::Error> {
        match self {
            AnyRowStream::DetachedRowStream(stream) => stream.next().await,
            AnyRowStream::RowStream(tx, stream) => stream.next(tx.handle()).await,
        }
    }
}

#[async_trait::async_trait]
pub trait QueryExecutorExt<'a> {
    async fn execute_query(self, query: neo4rs::Query) -> Result<AnyRowStream<'a>, neo4rs::Error>;
}

#[async_trait::async_trait]
impl<'a> QueryExecutorExt<'a> for Graph {
    async fn execute_query(self, query: neo4rs::Query) -> Result<AnyRowStream<'a>, neo4rs::Error> {
        let stream = self.execute(query).await?;
        Ok(AnyRowStream::DetachedRowStream(stream))
    }
}

#[async_trait::async_trait]
impl<'a> QueryExecutorExt<'a> for &Graph {
    async fn execute_query(self, query: neo4rs::Query) -> Result<AnyRowStream<'a>, neo4rs::Error> {
        let stream = self.execute(query).await?;
        Ok(AnyRowStream::DetachedRowStream(stream))
    }
}

#[async_trait::async_trait]
impl<'a> QueryExecutorExt<'a> for &'a mut Txn {
    async fn execute_query(self, query: neo4rs::Query) -> Result<AnyRowStream<'a>, neo4rs::Error> {
        let stream = self.execute(query).await?;
        Ok(AnyRowStream::RowStream(self, stream))
    }
}

#[async_trait::async_trait]
pub trait NeoQueryExtGeneric<'a> {
    async fn execute_value<T, E>(self, executor: E) -> Result<Option<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send,
        E: QueryExecutorExt<'a> + Sized + Send;

    async fn execute_values<T, E>(self, executor: E) -> Result<Vec<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send,
        E: QueryExecutorExt<'a> + Sized + Send;
}

#[async_trait::async_trait]
impl<'a> NeoQueryExtGeneric<'a> for neo4rs::Query {
    async fn execute_value<T, E>(self, executor: E) -> Result<Option<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send,
        E: QueryExecutorExt<'a> + Sized + Send,
    {
        let mut stream = executor.execute_query(self).await?;
        if let Some(row) = stream.next_row().await? {
            Ok(Some(
                row.to::<T>().map_err(neo4rs::Error::DeserializationError)?,
            ))
        } else {
            Ok(None)
        }
    }

    async fn execute_values<T, E>(self, executor: E) -> Result<Vec<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send,
        E: QueryExecutorExt<'a> + Sized + Send,
    {
        let mut stream = executor.execute_query(self).await?;
        let mut output = Vec::new();
        while let Some(row) = stream.next_row().await? {
            output.push(row.to::<T>().map_err(neo4rs::Error::DeserializationError)?);
        }
        Ok(output)
    }
}
