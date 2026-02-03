use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Privacy level for tasks - controls visibility and delegation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyLevel {
    /// Personal tasks - not shared
    #[default]
    Personal,
    /// Contains personally identifiable information
    Pii,
    /// Can be delegated to others
    Delegable,
}

impl PrivacyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Pii => "pii",
            Self::Delegable => "delegable",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "pii" => Self::Pii,
            "delegable" => Self::Delegable,
            _ => Self::Personal,
        }
    }
}

/// A task in the Eisenhower matrix style
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    /// Quadrant 1-4 (Eisenhower matrix):
    /// 1 = Urgent & Important
    /// 2 = Important (not urgent)
    /// 3 = Urgent (not important)
    /// 4 = Neither (later/delegate)
    pub quadrant: u8,
    pub deadline: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Optional link to a contact
    pub person_id: Option<Uuid>,
    /// Optional parent task for subtasks
    pub parent_id: Option<Uuid>,
    pub privacy_level: PrivacyLevel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description: None,
            quadrant: 4, // Default to Q4 (Later)
            deadline: None,
            completed_at: None,
            person_id: None,
            parent_id: None,
            privacy_level: PrivacyLevel::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if task is completed
    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark task as not completed
    pub fn uncomplete(&mut self) {
        self.completed_at = None;
        self.updated_at = Utc::now();
    }

    /// Get quadrant label
    pub fn quadrant_label(&self) -> &'static str {
        match self.quadrant {
            1 => "Q1: Urgent & Important",
            2 => "Q2: Important",
            3 => "Q3: Urgent",
            _ => "Q4: Later",
        }
    }

    /// Get short quadrant label
    pub fn quadrant_short(&self) -> &'static str {
        match self.quadrant {
            1 => "Q1",
            2 => "Q2",
            3 => "Q3",
            _ => "Q4",
        }
    }
}

impl Default for Task {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new("Test task".to_string());
        assert_eq!(task.title, "Test task");
        assert_eq!(task.quadrant, 4);
        assert!(!task.is_completed());
    }

    #[test]
    fn test_task_complete() {
        let mut task = Task::new("Test".to_string());
        assert!(!task.is_completed());

        task.complete();
        assert!(task.is_completed());

        task.uncomplete();
        assert!(!task.is_completed());
    }

    #[test]
    fn test_quadrant_labels() {
        let mut task = Task::new("Test".to_string());

        task.quadrant = 1;
        assert_eq!(task.quadrant_label(), "Q1: Urgent & Important");
        assert_eq!(task.quadrant_short(), "Q1");

        task.quadrant = 2;
        assert_eq!(task.quadrant_label(), "Q2: Important");

        task.quadrant = 3;
        assert_eq!(task.quadrant_label(), "Q3: Urgent");

        task.quadrant = 4;
        assert_eq!(task.quadrant_label(), "Q4: Later");
    }

    #[test]
    fn test_privacy_level_parse() {
        assert_eq!(PrivacyLevel::parse("personal"), PrivacyLevel::Personal);
        assert_eq!(PrivacyLevel::parse("pii"), PrivacyLevel::Pii);
        assert_eq!(PrivacyLevel::parse("delegable"), PrivacyLevel::Delegable);
        assert_eq!(PrivacyLevel::parse("unknown"), PrivacyLevel::Personal);
    }
}
