use neo4rs::BoltType;
use serde::Serialize;
use sha2::Digest;

use crate::migrations::MigrationError;

pub mod index_file;
pub mod migrations;

mod executor;
pub use executor::*;

mod subgraphs;
pub use subgraphs::*;

mod queries;
pub use queries::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Neo4rsError(#[from] neo4rs::Error),

    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    MigrationError(#[from] MigrationError),
}

/// Used to convert an instance of T into a parameterizable map of properties that neo4rs can use.
pub fn parameterize<T: Serialize>(instance: T) -> BoltType {
    let value = serde_json::to_value(instance).unwrap();
    let bolt_type = BoltType::try_from(value).unwrap();

    bolt_type
}

/// Gets a SHA256 hash for a given input.
fn get_str_shasum(input: impl Into<String>) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.into());

    let hash_result = hasher.finalize();

    format!("{:x}", hash_result)
}
