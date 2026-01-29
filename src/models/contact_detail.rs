use serde::{Deserialize, Serialize};

use super::{
    Address, Email, Interaction, Note, Organization, Person, PersonOrganization, Phone,
    SpecialDate, Tag,
};

/// Full contact detail - aggregates person with all related data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactDetail {
    pub person: Person,
    pub emails: Vec<Email>,
    pub phones: Vec<Phone>,
    pub addresses: Vec<Address>,
    pub organizations: Vec<(PersonOrganization, Organization)>,
    pub tags: Vec<Tag>,
    pub special_dates: Vec<SpecialDate>,
    pub notes: Vec<Note>,
    pub interactions: Vec<Interaction>,
}

impl ContactDetail {
    /// Get the primary email address, if any
    pub fn primary_email(&self) -> Option<&str> {
        self.emails
            .iter()
            .find(|e| e.is_primary)
            .or_else(|| self.emails.first())
            .map(|e| e.email_address.as_str())
    }

    /// Get the primary phone number, if any
    pub fn primary_phone(&self) -> Option<&str> {
        self.phones
            .iter()
            .find(|p| p.is_primary)
            .or_else(|| self.phones.first())
            .map(|p| p.phone_number.as_str())
    }

    /// Get the primary address location (city, state), if any
    pub fn primary_location(&self) -> Option<String> {
        self.addresses
            .iter()
            .find(|a| a.is_primary)
            .or_else(|| self.addresses.first())
            .and_then(|a| a.city_state())
    }

    /// Get the current primary organization with title
    pub fn current_org_title(&self) -> Option<String> {
        self.organizations
            .iter()
            .find(|(po, _)| po.is_current && po.is_primary)
            .or_else(|| self.organizations.iter().find(|(po, _)| po.is_current))
            .map(|(po, org)| {
                match &po.title {
                    Some(title) => format!("{} at {}", title, org.name),
                    None => org.name.clone(),
                }
            })
    }
}
