use std::collections::HashMap;

use neo4rs::{Graph, Node, Path, Query, Relation, Row, Txn, UnboundedRelation};
use serde::de::DeserializeOwned;

#[derive(Clone, Debug, PartialEq)]
pub struct Subgraph {
    pub nodes: HashMap<i64, Node>,
    pub relations: HashMap<i64, Relation>,
    pub unbounded_relations: HashMap<i64, UnboundedRelation>,
}

impl Subgraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            relations: HashMap::new(),
            unbounded_relations: HashMap::new(),
        }
    }

    fn add_node(&mut self, node: Node) {
        self.nodes.entry(node.id()).or_insert(node);
    }

    fn add_relationship(&mut self, rel: Relation) {
        self.relations.entry(rel.id()).or_insert(rel);
    }

    fn add_unbounded_relationship(&mut self, rel: UnboundedRelation) {
        self.unbounded_relations.entry(rel.id()).or_insert(rel);
    }

    fn add_path(&mut self, path: &Path) {
        for node in path.nodes() {
            self.add_node(node);
        }
        for rel in path.rels() {
            self.add_unbounded_relationship(rel);
        }
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn relationships(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }

    pub fn from_rows(rows: impl IntoIterator<Item = Row>) -> Result<Self, neo4rs::DeError> {
        let mut graph = Subgraph::new();

        for row in rows {
            for key in row.keys() {
                // Path can contain nodes, relations and unbounded relations
                if let Ok(path) = row.get::<Path>(&key.value) {
                    graph.add_path(&path);
                }
                // Nodes
                else if let Ok(node) = row.get::<Node>(&key.value) {
                    graph.add_node(node);
                }
                // Full relations
                else if let Ok(rel) = row.get::<Relation>(&key.value) {
                    graph.add_relationship(rel);
                }
                // Unbounded relations have no information about start or end nodes
                else if let Ok(rel) = row.get::<UnboundedRelation>(&key.value) {
                    graph.add_unbounded_relationship(rel);
                }
            }
        }

        Ok(graph)
    }

    pub fn nodes_as<T: DeserializeOwned>(&self, label: &str) -> Vec<T> {
        self.nodes()
            .filter(|n| n.labels().contains(&label))
            .filter_map(|n| n.to::<T>().ok())
            .collect()
    }

    pub fn relationships_as<T: DeserializeOwned>(&self, typ: &str) -> Vec<T> {
        self.relationships()
            .filter(|r| r.typ() == typ)
            .filter_map(|r| r.to::<T>().ok())
            .collect()
    }
}

#[async_trait::async_trait]
pub trait SubgraphRowStreamExt {
    async fn subgraph(&mut self, tx: &mut Txn) -> Result<Subgraph, neo4rs::Error>;
}

#[async_trait::async_trait]
impl SubgraphRowStreamExt for neo4rs::RowStream {
    async fn subgraph(&mut self, tx: &mut Txn) -> Result<Subgraph, neo4rs::Error> {
        let mut rows = Vec::new();
        while let Some(row) = self.next(tx.handle()).await? {
            rows.push(row);
        }

        Ok(Subgraph::from_rows(rows)?)
    }
}

#[async_trait::async_trait]
pub trait SubgraphDetachedRowStreamExt {
    async fn subgraph(&mut self) -> Result<Subgraph, neo4rs::Error>;
}

#[async_trait::async_trait]
impl SubgraphDetachedRowStreamExt for neo4rs::DetachedRowStream {
    async fn subgraph(&mut self) -> Result<Subgraph, neo4rs::Error> {
        let mut rows = Vec::new();
        while let Some(row) = self.next().await? {
            rows.push(row);
        }

        Ok(Subgraph::from_rows(rows)?)
    }
}

#[async_trait::async_trait]
pub trait SubgraphQueryExt {
    async fn subgraph(self, graph: &Graph) -> Result<Subgraph, neo4rs::Error>;
    async fn subgraph_tx(self, tx: &mut Txn) -> Result<Subgraph, neo4rs::Error>;
}

#[async_trait::async_trait]
impl SubgraphQueryExt for Query {
    async fn subgraph(self, graph: &Graph) -> Result<Subgraph, neo4rs::Error> {
        let mut stream = graph.execute(self).await?;
        let subgraph = stream.subgraph().await?;
        Ok(subgraph)
    }

    async fn subgraph_tx(self, tx: &mut Txn) -> Result<Subgraph, neo4rs::Error> {
        let mut stream = tx.execute(self).await?;
        let subgraph = stream.subgraph(tx).await?;
        Ok(subgraph)
    }
}
