use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub org_type: Option<String>,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

impl Organization {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            org_type: None,
            industry: None,
            website: None,
            city: None,
            state: None,
            country: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonOrganization {
    pub id: Uuid,
    pub person_id: Uuid,
    pub organization_id: Uuid,
    pub title: Option<String>,
    pub department: Option<String>,
    pub relationship_type: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_current: bool,
    pub is_primary: bool,
}

impl PersonOrganization {
    pub fn new(person_id: Uuid, organization_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            organization_id,
            title: None,
            department: None,
            relationship_type: "employee".to_string(),
            start_date: None,
            end_date: None,
            is_current: true,
            is_primary: false,
        }
    }

    /// Create a link marking this person as the organization's representative/contact.
    pub fn new_representative(person_id: Uuid, organization_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            person_id,
            organization_id,
            title: None,
            department: None,
            relationship_type: "representative".to_string(),
            start_date: None,
            end_date: None,
            is_current: true,
            is_primary: true,
        }
    }
}
