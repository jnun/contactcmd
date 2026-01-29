use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EmailType {
    #[default]
    Personal,
    Work,
    School,
    Other,
}

impl EmailType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Work => "work",
            Self::School => "school",
            Self::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "work" => Self::Work,
            "school" => Self::School,
            "other" => Self::Other,
            _ => Self::Personal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email {
    pub id: Uuid,
    pub person_id: Uuid,
    pub email_address: String,
    pub email_type: EmailType,
    pub is_primary: bool,
}

impl Email {
    pub fn new(person_id: Uuid, email_address: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            email_address,
            email_type: EmailType::default(),
            is_primary: false,
        }
    }
}
