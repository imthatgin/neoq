use std::collections::HashMap;

use neo4rs::{BoltString, BoltType, Query, query};
use tracing::warn;

pub trait QueryCombinerExt {
    fn with(self, query: impl Into<Query>) -> Query;
}

impl QueryCombinerExt for Query {
    fn with(self, query: impl Into<Query>) -> Query {
        combine_queries([self, query.into()])
    }
}

/// Takes an iterator of queries to combine by string appending with newlines.
/// This does not take care of syntax issues.
pub fn combine_queries(queries: impl IntoIterator<Item = Query>) -> Query {
    let mut parameter_values: HashMap<BoltString, BoltType> = HashMap::new();

    let mut query_string = String::new();

    // Iterate over the provided queries, and insert their parameters into the final map.
    for q in queries {
        let parameters = q.get_params().value.clone();

        for (key, value) in parameters {
            // Send a warning if the key is duplicated.
            if parameter_values.contains_key(&key) {
                warn!("Combining queries duplicated key '{}'", &key);
                continue;
            }
            parameter_values.insert(key, value);
        }

        query_string += format!("{}\n", q.query()).as_str();
    }

    let output = query(&query_string).params(parameter_values);
    output
}
