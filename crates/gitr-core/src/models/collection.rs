use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repo::RepoId;

/// Unique identifier for a collection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollectionId(pub Uuid);

impl CollectionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for CollectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A user-defined grouping of repos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: CollectionId,
    pub name: String,
    pub description: Option<String>,
}

impl Collection {
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            id: CollectionId::new(),
            name,
            description,
        }
    }
}

/// A membership entry linking a collection to a repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMember {
    pub collection_id: CollectionId,
    pub repo_id: RepoId,
}
