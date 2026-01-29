use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InteractionType {
    #[default]
    Note,
    Call,
    Email,
    Meeting,
    Text,
    Social,
    Other,
}

impl InteractionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Call => "call",
            Self::Email => "email",
            Self::Meeting => "meeting",
            Self::Text => "text",
            Self::Social => "social",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "call" => Self::Call,
            "email" => Self::Email,
            "meeting" => Self::Meeting,
            "text" | "sms" => Self::Text,
            "social" => Self::Social,
            "other" => Self::Other,
            _ => Self::Note,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interaction {
    pub id: Uuid,
    pub person_id: Uuid,
    pub interaction_type: InteractionType,
    pub occurred_at: DateTime<Utc>,
    pub summary: Option<String>,
    pub notes: Option<String>,
    pub sentiment: Option<String>,
}

impl Interaction {
    pub fn new(person_id: Uuid, interaction_type: InteractionType) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            interaction_type,
            occurred_at: Utc::now(),
            summary: None,
            notes: None,
            sentiment: None,
        }
    }
}
