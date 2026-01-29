use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonTag {
    pub id: Uuid,
    pub person_id: Uuid,
    pub tag_id: Uuid,
    pub added_at: DateTime<Utc>,
}

impl PersonTag {
    pub fn new(person_id: Uuid, tag_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            tag_id,
            added_at: Utc::now(),
        }
    }
}
