use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub person_id: Uuid,
    pub content: String,
    pub note_type: Option<String>,
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
}

impl Note {
    pub fn new(person_id: Uuid, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            content,
            note_type: None,
            is_pinned: false,
            created_at: Utc::now(),
        }
    }
}
