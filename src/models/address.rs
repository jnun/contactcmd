use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AddressType {
    #[default]
    Home,
    Work,
    Other,
}

impl AddressType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Work => "work",
            Self::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "work" => Self::Work,
            "other" => Self::Other,
            _ => Self::Home,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub id: Uuid,
    pub person_id: Uuid,
    pub street: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub address_type: AddressType,
    pub is_primary: bool,
}

impl Address {
    pub fn new(person_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            street: None,
            street2: None,
            city: None,
            state: None,
            postal_code: None,
            country: None,
            address_type: AddressType::default(),
            is_primary: false,
        }
    }

    pub fn city_state(&self) -> Option<String> {
        match (&self.city, &self.state) {
            (Some(c), Some(s)) => Some(format!("{}, {}", c, s)),
            (Some(c), None) => Some(c.clone()),
            (None, Some(s)) => Some(s.clone()),
            (None, None) => None,
        }
    }
}
