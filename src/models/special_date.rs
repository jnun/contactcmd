use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DateType {
    #[default]
    Birthday,
    Anniversary,
    Custom,
}

impl DateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Birthday => "birthday",
            Self::Anniversary => "anniversary",
            Self::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "anniversary" => Self::Anniversary,
            "custom" => Self::Custom,
            _ => Self::Birthday,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialDate {
    pub id: Uuid,
    pub person_id: Uuid,
    pub date: String,
    pub date_type: DateType,
    pub label: Option<String>,
    pub year_known: bool,
}

impl SpecialDate {
    pub fn new(person_id: Uuid, date: String, date_type: DateType) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            date,
            date_type,
            label: None,
            year_known: true,
        }
    }
}
