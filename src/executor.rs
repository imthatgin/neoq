use async_trait::async_trait;
use neo4rs::{DetachedRowStream, Graph, Query, Row, RowStream, RunResult, Txn};
use serde::de::DeserializeOwned;

#[async_trait]
pub trait RowStreamable: Send {
    async fn next(&mut self) -> Result<Option<Row>, neo4rs::Error>;
}

pub struct DetachedRowAdapter(DetachedRowStream);

#[async_trait]
impl RowStreamable for DetachedRowAdapter {
    async fn next(&mut self) -> Result<Option<Row>, neo4rs::Error> {
        self.0.next().await
    }
}

pub struct TxnAdapter<'txn> {
    txn: &'txn mut Txn,
    stream: RowStream,
}

#[async_trait]
impl<'txn> RowStreamable for TxnAdapter<'txn> {
    async fn next(&mut self) -> Result<Option<Row>, neo4rs::Error> {
        self.stream.next(self.txn.handle()).await
    }
}

pub type BoxedRowStream<'a> = Box<dyn RowStreamable + Send + 'a>;

#[async_trait]
pub trait QueryExecutor: Send + Sync {
    async fn execute_query<'a>(self, query: Query) -> Result<BoxedRowStream<'a>, neo4rs::Error>
    where
        Self: 'a;
}

#[async_trait]
impl QueryExecutor for Graph {
    async fn execute_query<'a>(self, query: Query) -> Result<BoxedRowStream<'a>, neo4rs::Error>
    where
        Self: 'a,
    {
        let stream = self.execute(query).await?;
        Ok(Box::new(DetachedRowAdapter(stream)))
    }
}

#[async_trait]
impl<'g> QueryExecutor for &'g Graph {
    async fn execute_query<'a>(self, query: Query) -> Result<BoxedRowStream<'a>, neo4rs::Error>
    where
        Self: 'a,
    {
        let stream = (*self).execute(query).await?;
        Ok(Box::new(DetachedRowAdapter(stream)))
    }
}

#[async_trait]
impl<'txn> QueryExecutor for &'txn mut Txn {
    async fn execute_query<'a>(self, query: Query) -> Result<BoxedRowStream<'a>, neo4rs::Error>
    where
        Self: 'a,
    {
        let stream = (*self).execute(query).await?;
        Ok(Box::new(TxnAdapter { txn: self, stream }))
    }
}

#[async_trait]
pub trait QueryExt {
    async fn execute_value<T>(
        self,
        executor: impl QueryExecutor,
    ) -> Result<Option<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send;

    async fn execute_values<T>(self, executor: impl QueryExecutor) -> Result<Vec<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send;
}

#[async_trait]
impl QueryExt for Query {
    async fn execute_value<T>(
        self,
        executor: impl QueryExecutor,
    ) -> Result<Option<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send,
    {
        let mut stream = executor.execute_query(self).await?;
        if let Some(row) = stream.next().await? {
            Ok(Some(
                row.to::<T>().map_err(neo4rs::Error::DeserializationError)?,
            ))
        } else {
            Ok(None)
        }
    }

    async fn execute_values<T>(self, executor: impl QueryExecutor) -> Result<Vec<T>, neo4rs::Error>
    where
        T: DeserializeOwned + Send,
    {
        let mut stream = executor.execute_query(self).await?;
        let mut out = Vec::new();
        while let Some(row) = stream.next().await? {
            out.push(row.to::<T>().map_err(neo4rs::Error::DeserializationError)?);
        }

        Ok(out)
    }
}
