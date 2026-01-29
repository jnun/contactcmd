use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PhoneType {
    #[default]
    Mobile,
    Home,
    Work,
    Fax,
    Other,
}

impl PhoneType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mobile => "mobile",
            Self::Home => "home",
            Self::Work => "work",
            Self::Fax => "fax",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "home" => Self::Home,
            "work" => Self::Work,
            "fax" => Self::Fax,
            "other" => Self::Other,
            "cell" | "cellular" => Self::Mobile,
            _ => Self::Mobile,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phone {
    pub id: Uuid,
    pub person_id: Uuid,
    pub phone_number: String,
    pub phone_type: PhoneType,
    pub is_primary: bool,
}

impl Phone {
    pub fn new(person_id: Uuid, phone_number: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            phone_number,
            phone_type: PhoneType::default(),
            is_primary: false,
        }
    }
}
