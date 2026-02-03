use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: Uuid,
    pub name_given: Option<String>,
    pub name_family: Option<String>,
    pub name_middle: Option<String>,
    pub name_prefix: Option<String>,
    pub name_suffix: Option<String>,
    pub name_nickname: Option<String>,
    pub preferred_name: Option<String>,
    pub display_name: Option<String>,
    pub sort_name: Option<String>,
    pub search_name: Option<String>,
    pub name_order: NameOrder,
    pub person_type: PersonType,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_dirty: bool,
    pub external_ids: Option<String>,
    pub checkin_date: Option<DateTime<Utc>>,
    pub ai_contact_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NameOrder {
    #[default]
    Western,
    Eastern,
    Latin,
}

impl NameOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Western => "western",
            Self::Eastern => "eastern",
            Self::Latin => "latin",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "eastern" => Self::Eastern,
            "latin" => Self::Latin,
            _ => Self::Western,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PersonType {
    #[default]
    Personal,
    Business,
    Prospect,
    Connector,
}

impl PersonType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Business => "business",
            Self::Prospect => "prospect",
            Self::Connector => "connector",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "business" => Self::Business,
            "prospect" => Self::Prospect,
            "connector" => Self::Connector,
            _ => Self::Personal,
        }
    }
}

impl Person {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name_given: None,
            name_family: None,
            name_middle: None,
            name_prefix: None,
            name_suffix: None,
            name_nickname: None,
            preferred_name: None,
            display_name: None,
            sort_name: None,
            search_name: None,
            name_order: NameOrder::default(),
            person_type: PersonType::default(),
            notes: None,
            is_active: true,
            created_at: now,
            updated_at: now,
            is_dirty: false,
            external_ids: None,
            checkin_date: None,
            ai_contact_allowed: true,
        }
    }

    /// Compute all name fields. Call after changing any name component.
    pub fn compute_names(&mut self) {
        self.display_name = Some(self.compute_display_name());
        self.sort_name = Some(self.compute_sort_name());
        self.search_name = Some(self.compute_search_name());
    }

    fn compute_display_name(&self) -> String {
        match self.name_order {
            NameOrder::Eastern => {
                // Family Given (e.g., "Tanaka Taro")
                let mut parts = Vec::new();
                if let Some(ref f) = self.name_family {
                    parts.push(f.as_str());
                }
                if let Some(ref g) = self.name_given {
                    parts.push(g.as_str());
                }
                parts.join(" ")
            }
            NameOrder::Western | NameOrder::Latin => {
                // [Prefix] Given [Middle] Family [Suffix]
                let mut parts = Vec::new();
                if let Some(ref p) = self.name_prefix {
                    parts.push(p.as_str());
                }
                if let Some(ref g) = self.name_given {
                    parts.push(g.as_str());
                }
                if let Some(ref m) = self.name_middle {
                    parts.push(m.as_str());
                }
                if let Some(ref f) = self.name_family {
                    parts.push(f.as_str());
                }
                if let Some(ref s) = self.name_suffix {
                    parts.push(s.as_str());
                }
                parts.join(" ")
            }
        }
    }

    fn compute_sort_name(&self) -> String {
        // "Family, Given" for sorting
        match (&self.name_family, &self.name_given) {
            (Some(f), Some(g)) => format!("{}, {}", f, g),
            (Some(f), None) => f.clone(),
            (None, Some(g)) => g.clone(),
            (None, None) => self
                .name_nickname
                .clone()
                .or_else(|| self.preferred_name.clone())
                .unwrap_or_default(),
        }
    }

    fn compute_search_name(&self) -> String {
        // All name parts lowercase, space-separated
        let parts: Vec<&str> = [
            self.name_given.as_deref(),
            self.name_family.as_deref(),
            self.name_middle.as_deref(),
            self.name_nickname.as_deref(),
            self.preferred_name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();

        parts.join(" ").to_lowercase()
    }
}

impl Default for Person {
    fn default() -> Self {
        Self::new()
    }
}
