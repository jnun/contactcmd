//! Content filter matching for message safety screening.
//!
//! Filters can either deny messages outright or flag them for extra review.
//! Patterns can be regex or literal (case-insensitive) strings.

use regex::Regex;
use std::sync::RwLock;

use crate::db::gateway::ContentFilter;
use crate::db::Database;

/// Result of checking content against filters.
#[derive(Debug, Clone)]
pub enum FilterResult {
    /// Content passed all filters.
    Passed,
    /// Content should be denied - contains blocked pattern.
    Denied {
        filter_name: String,
        description: String,
    },
    /// Content should be flagged for extra review.
    Flagged {
        filter_name: String,
        description: String,
    },
}

/// Compiled filter pattern for efficient matching.
struct CompiledFilter {
    id: String,
    pattern: CompiledPattern,
    action: FilterAction,
    description: String,
}

enum CompiledPattern {
    Regex(Regex),
    Literal(String), // Stored lowercase for case-insensitive matching
}

#[derive(Clone, Copy)]
enum FilterAction {
    Deny,
    Flag,
}

/// Content filter matcher with compiled and cached patterns.
///
/// Thread-safe via RwLock for concurrent access.
pub struct ContentFilterMatcher {
    filters: RwLock<Vec<CompiledFilter>>,
}

impl ContentFilterMatcher {
    /// Create a new matcher (empty, must call reload to load filters).
    pub fn new() -> Self {
        Self {
            filters: RwLock::new(Vec::new()),
        }
    }

    /// Load/reload filters from database.
    ///
    /// Only loads enabled filters. Compiles regex patterns once.
    /// Invalid regex patterns are skipped with a warning.
    pub fn reload(&self, db: &Database) -> anyhow::Result<usize> {
        let db_filters = db.list_enabled_content_filters()?;
        let mut compiled = Vec::with_capacity(db_filters.len());

        for filter in db_filters {
            match compile_filter(&filter) {
                Ok(cf) => compiled.push(cf),
                Err(e) => {
                    eprintln!(
                        "Warning: Skipping invalid filter '{}': {}",
                        filter.description.as_deref().unwrap_or(&filter.id),
                        e
                    );
                }
            }
        }

        let count = compiled.len();
        let mut filters = self.filters.write().unwrap();
        *filters = compiled;

        Ok(count)
    }

    /// Check content against all loaded filters.
    ///
    /// Returns the first match found (deny filters are checked first).
    /// For emails, both subject and body should be checked by calling this
    /// method twice or using `check_email`.
    pub fn check(&self, content: &str) -> FilterResult {
        let filters = self.filters.read().unwrap();

        // Check deny filters first (they're sorted first from the DB query)
        for filter in filters.iter() {
            if pattern_matches(&filter.pattern, content) {
                match filter.action {
                    FilterAction::Deny => {
                        return FilterResult::Denied {
                            filter_name: filter.id.clone(),
                            description: filter.description.clone(),
                        };
                    }
                    FilterAction::Flag => {
                        return FilterResult::Flagged {
                            filter_name: filter.id.clone(),
                            description: filter.description.clone(),
                        };
                    }
                }
            }
        }

        FilterResult::Passed
    }

    /// Check email content (subject + body) against filters.
    ///
    /// Checks subject first, then body. Returns first match.
    pub fn check_email(&self, subject: Option<&str>, body: &str) -> FilterResult {
        // Check subject first if present
        if let Some(subj) = subject {
            let result = self.check(subj);
            if !matches!(result, FilterResult::Passed) {
                return result;
            }
        }

        // Then check body
        self.check(body)
    }

    /// Check SMS/iMessage content (body only).
    pub fn check_message(&self, body: &str) -> FilterResult {
        self.check(body)
    }

    /// Get number of loaded filters.
    pub fn filter_count(&self) -> usize {
        self.filters.read().unwrap().len()
    }
}

impl Default for ContentFilterMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Compile a database filter into an efficient matcher.
fn compile_filter(filter: &ContentFilter) -> anyhow::Result<CompiledFilter> {
    let pattern = match filter.pattern_type.as_str() {
        "regex" => {
            // Build case-insensitive regex
            let regex = Regex::new(&format!("(?i){}", filter.pattern))
                .map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
            CompiledPattern::Regex(regex)
        }
        "literal" => {
            // Store lowercase for case-insensitive matching
            CompiledPattern::Literal(filter.pattern.to_lowercase())
        }
        other => {
            return Err(anyhow::anyhow!("Unknown pattern type: {}", other));
        }
    };

    let action = match filter.action.as_str() {
        "deny" => FilterAction::Deny,
        "flag" => FilterAction::Flag,
        other => {
            return Err(anyhow::anyhow!("Unknown action: {}", other));
        }
    };

    Ok(CompiledFilter {
        id: filter
            .description
            .clone()
            .unwrap_or_else(|| filter.id.clone()),
        pattern,
        action,
        description: filter
            .description
            .clone()
            .unwrap_or_else(|| format!("Filter {}", filter.id)),
    })
}

/// Check if content matches a compiled pattern.
fn pattern_matches(pattern: &CompiledPattern, content: &str) -> bool {
    match pattern {
        CompiledPattern::Regex(re) => re.is_match(content),
        CompiledPattern::Literal(literal) => content.to_lowercase().contains(literal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_result_types() {
        let passed = FilterResult::Passed;
        assert!(matches!(passed, FilterResult::Passed));

        let denied = FilterResult::Denied {
            filter_name: "ssn".to_string(),
            description: "SSN detected".to_string(),
        };
        assert!(matches!(denied, FilterResult::Denied { .. }));

        let flagged = FilterResult::Flagged {
            filter_name: "password".to_string(),
            description: "Password mention".to_string(),
        };
        assert!(matches!(flagged, FilterResult::Flagged { .. }));
    }

    #[test]
    fn test_literal_pattern_case_insensitive() {
        let pattern = CompiledPattern::Literal("password".to_string());
        assert!(pattern_matches(&pattern, "your PASSWORD is"));
        assert!(pattern_matches(&pattern, "Password123"));
        assert!(pattern_matches(&pattern, "ENTER PASSWORD HERE"));
        assert!(!pattern_matches(&pattern, "pass word"));
    }

    #[test]
    fn test_regex_pattern_case_insensitive() {
        // SSN pattern
        let regex = Regex::new(r"(?i)\b\d{3}-\d{2}-\d{4}\b").unwrap();
        let pattern = CompiledPattern::Regex(regex);

        assert!(pattern_matches(&pattern, "SSN: 123-45-6789"));
        assert!(pattern_matches(&pattern, "my ssn is 999-99-9999 ok"));
        assert!(!pattern_matches(&pattern, "1234567890"));
        assert!(!pattern_matches(&pattern, "123-456-7890")); // Phone format, not SSN
    }

    #[test]
    fn test_credit_card_pattern() {
        let regex = Regex::new(r"(?i)\b(?:\d{4}[- ]?){3}\d{4}\b").unwrap();
        let pattern = CompiledPattern::Regex(regex);

        assert!(pattern_matches(&pattern, "card: 1234567890123456"));
        assert!(pattern_matches(&pattern, "card: 1234-5678-9012-3456"));
        assert!(pattern_matches(&pattern, "card: 1234 5678 9012 3456"));
        assert!(!pattern_matches(&pattern, "123456789012345")); // 15 digits
        assert!(!pattern_matches(&pattern, "12345678901234567")); // 17 digits
    }

    #[test]
    fn test_matcher_with_db() {
        let db = Database::open_memory().unwrap();
        let matcher = ContentFilterMatcher::new();

        // Load filters from seeded DB
        let count = matcher.reload(&db).unwrap();
        assert!(count >= 4, "Should have loaded at least 4 default filters");

        // Test SSN detection
        let result = matcher.check("My SSN is 123-45-6789");
        assert!(
            matches!(result, FilterResult::Denied { .. }),
            "SSN should be denied"
        );

        // Test password flag
        let result = matcher.check("Your password is secret123");
        assert!(
            matches!(result, FilterResult::Flagged { .. }),
            "Password mention should be flagged"
        );

        // Test clean content
        let result = matcher.check("Hello, how are you?");
        assert!(
            matches!(result, FilterResult::Passed),
            "Clean content should pass"
        );
    }

    #[test]
    fn test_check_email() {
        let db = Database::open_memory().unwrap();
        let matcher = ContentFilterMatcher::new();
        matcher.reload(&db).unwrap();

        // SSN in subject
        let result = matcher.check_email(Some("Your SSN 123-45-6789"), "Clean body");
        assert!(matches!(result, FilterResult::Denied { .. }));

        // SSN in body
        let result = matcher.check_email(Some("Normal subject"), "SSN: 123-45-6789");
        assert!(matches!(result, FilterResult::Denied { .. }));

        // Password in body (flagged)
        let result = matcher.check_email(Some("Login info"), "Your password is abc123");
        assert!(matches!(result, FilterResult::Flagged { .. }));

        // Clean email
        let result = matcher.check_email(Some("Hello"), "How are you doing?");
        assert!(matches!(result, FilterResult::Passed));
    }

    #[test]
    fn test_filter_count() {
        let db = Database::open_memory().unwrap();
        let matcher = ContentFilterMatcher::new();

        assert_eq!(matcher.filter_count(), 0);
        matcher.reload(&db).unwrap();
        assert!(matcher.filter_count() >= 4);
    }
}
