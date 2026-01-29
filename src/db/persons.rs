use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Row};
use std::collections::HashMap;
use uuid::Uuid;

use super::Database;
use crate::models::*;

/// Display info for a person: (primary_email, location)
pub type DisplayInfo = HashMap<Uuid, (Option<String>, Option<String>)>;

/// Helper to convert UUID parse errors to rusqlite errors
fn parse_uuid(s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

impl Database {
    // ==================== PERSON CREATE ====================

    pub fn insert_person(&self, person: &Person) -> Result<()> {
        self.conn.execute(
            r#"INSERT INTO persons (
                id, name_given, name_family, name_middle, name_prefix, name_suffix,
                name_nickname, preferred_name, display_name, sort_name, search_name,
                name_order, person_type, notes, is_active, created_at, updated_at,
                is_dirty, external_ids
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            params![
                person.id.to_string(),
                person.name_given,
                person.name_family,
                person.name_middle,
                person.name_prefix,
                person.name_suffix,
                person.name_nickname,
                person.preferred_name,
                person.display_name,
                person.sort_name,
                person.search_name,
                person.name_order.as_str(),
                person.person_type.as_str(),
                person.notes,
                person.is_active as i32,
                person.created_at.to_rfc3339(),
                person.updated_at.to_rfc3339(),
                person.is_dirty as i32,
                person.external_ids,
            ],
        )?;
        Ok(())
    }

    // ==================== PERSON READ ====================

    pub fn get_person_by_id(&self, id: Uuid) -> Result<Option<Person>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM persons WHERE id = ? AND is_active = 1")?;

        let result = stmt.query_row([id.to_string()], Self::row_to_person);

        match result {
            Ok(person) => Ok(Some(person)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Find a person by their email address. Returns the first active match.
    pub fn get_person_by_email(&self, email: &str) -> Result<Option<Person>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT p.* FROM persons p
               INNER JOIN emails e ON e.person_id = p.id
               WHERE e.email_address = ? AND p.is_active = 1
               LIMIT 1"#,
        )?;

        let result = stmt.query_row([email], Self::row_to_person);

        match result {
            Ok(person) => Ok(Some(person)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_persons(&self, limit: u32, offset: u32) -> Result<Vec<Person>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM persons WHERE is_active = 1 ORDER BY sort_name ASC LIMIT ? OFFSET ?",
        )?;

        let persons = stmt
            .query_map([limit, offset], Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    pub fn count_persons(&self) -> Result<u32> {
        let count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM persons WHERE is_active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// List persons with custom sort
    pub fn list_persons_sorted(
        &self,
        limit: u32,
        offset: u32,
        sort_column: &str,
        sort_direction: &str,
    ) -> Result<Vec<Person>> {
        // Whitelist columns to prevent SQL injection
        let column = match sort_column {
            "sort_name" | "created_at" | "updated_at" => sort_column,
            _ => "sort_name",
        };
        let direction = match sort_direction {
            "ASC" | "DESC" => sort_direction,
            _ => "ASC",
        };

        let sql = format!(
            "SELECT * FROM persons WHERE is_active = 1 ORDER BY {} {} LIMIT ? OFFSET ?",
            column, direction
        );

        let mut stmt = self.conn.prepare(&sql)?;

        let persons = stmt
            .query_map([limit, offset], Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    /// Fetch contact rows for list display in a single query (O(1) instead of O(N))
    pub fn list_contact_rows(
        &self,
        limit: u32,
        offset: u32,
        sort_column: &str,
        sort_direction: &str,
    ) -> Result<Vec<crate::cli::list::ContactListRow>> {
        use crate::cli::list::ContactListRow;

        let column = match sort_column {
            "sort_name" | "created_at" | "updated_at" => sort_column,
            _ => "sort_name",
        };
        let direction = match sort_direction {
            "ASC" | "DESC" => sort_direction,
            _ => "ASC",
        };

        let sql = format!(
            r#"SELECT
                p.id,
                p.display_name,
                (SELECT email_address FROM emails WHERE person_id = p.id ORDER BY is_primary DESC LIMIT 1) as primary_email,
                (SELECT phone_number FROM phones WHERE person_id = p.id ORDER BY is_primary DESC LIMIT 1) as primary_phone,
                (SELECT city FROM addresses WHERE person_id = p.id ORDER BY is_primary DESC LIMIT 1) as city,
                (SELECT state FROM addresses WHERE person_id = p.id ORDER BY is_primary DESC LIMIT 1) as state,
                (SELECT po.title FROM person_organizations po WHERE po.person_id = p.id ORDER BY po.is_primary DESC LIMIT 1) as title,
                (SELECT o.name FROM person_organizations po
                    JOIN organizations o ON o.id = po.organization_id
                    WHERE po.person_id = p.id ORDER BY po.is_primary DESC LIMIT 1) as org_name
            FROM persons p
            WHERE p.is_active = 1
            ORDER BY p.{} {}
            LIMIT ? OFFSET ?"#,
            column, direction
        );

        let mut stmt = self.conn.prepare(&sql)?;

        let rows = stmt
            .query_map([limit, offset], |row| {
                let id_str: String = row.get(0)?;
                let display_name: Option<String> = row.get(1)?;
                let primary_email: Option<String> = row.get(2)?;
                let primary_phone: Option<String> = row.get(3)?;
                let city: Option<String> = row.get(4)?;
                let state: Option<String> = row.get(5)?;
                let title: Option<String> = row.get(6)?;
                let org_name: Option<String> = row.get(7)?;

                let location = match (&city, &state) {
                    (Some(c), Some(s)) => Some(format!("{}, {}", c, s)),
                    (Some(c), None) => Some(c.clone()),
                    (None, Some(s)) => Some(s.clone()),
                    (None, None) => None,
                };

                // Use org name as fallback for display_name if person has no name
                let display = match (&display_name, &org_name) {
                    (Some(n), _) if !n.is_empty() => n.clone(),
                    (_, Some(o)) => o.clone(),
                    _ => "Unknown".to_string(),
                };

                // Only show title_and_org if display_name exists (not using org as name)
                let title_and_org = match (&display_name, &title, &org_name) {
                    (Some(n), Some(t), Some(o)) if !n.is_empty() => Some(format!("{} at {}", t, o)),
                    (Some(n), None, Some(o)) if !n.is_empty() => Some(o.clone()),
                    _ => None,
                };

                Ok(ContactListRow {
                    id: parse_uuid(&id_str)?,
                    display_name: display,
                    title_and_org,
                    primary_email,
                    primary_phone,
                    location,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(rows)
    }

    /// Get organizations for a person with the join table data
    pub fn get_organizations_for_person(
        &self,
        person_id: Uuid,
    ) -> Result<Vec<(PersonOrganization, Organization)>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT
                po.id, po.person_id, po.organization_id, po.title, po.department,
                po.relationship_type, po.start_date, po.end_date, po.is_current, po.is_primary,
                o.id, o.name, o.org_type, o.industry, o.website, o.city, o.state, o.country
            FROM person_organizations po
            JOIN organizations o ON o.id = po.organization_id
            WHERE po.person_id = ?
            ORDER BY po.is_primary DESC, po.is_current DESC"#,
        )?;

        let results = stmt
            .query_map([person_id.to_string()], |row| {
                let po = PersonOrganization {
                    id: parse_uuid(&row.get::<_, String>(0)?)?,
                    person_id: parse_uuid(&row.get::<_, String>(1)?)?,
                    organization_id: parse_uuid(&row.get::<_, String>(2)?)?,
                    title: row.get(3)?,
                    department: row.get(4)?,
                    relationship_type: row
                        .get::<_, String>(5)
                        .unwrap_or_else(|_| "employee".to_string()),
                    start_date: row.get(6)?,
                    end_date: row.get(7)?,
                    is_current: row.get::<_, i32>(8).unwrap_or(1) == 1,
                    is_primary: row.get::<_, i32>(9).unwrap_or(0) == 1,
                };
                let org = Organization {
                    id: parse_uuid(&row.get::<_, String>(10)?)?,
                    name: row.get(11)?,
                    org_type: row.get(12)?,
                    industry: row.get(13)?,
                    website: row.get(14)?,
                    city: row.get(15)?,
                    state: row.get(16)?,
                    country: row.get(17)?,
                };
                Ok((po, org))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(results)
    }

    pub fn search_persons(&self, query: &str, limit: u32) -> Result<Vec<Person>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self.conn.prepare(
            r#"SELECT DISTINCT p.* FROM persons p
               LEFT JOIN emails e ON e.person_id = p.id
               WHERE p.is_active = 1
                 AND (p.search_name LIKE ?1 OR p.display_name LIKE ?1 OR e.email_address LIKE ?1)
               ORDER BY p.sort_name ASC
               LIMIT ?2"#,
        )?;

        let persons = stmt
            .query_map(params![pattern, limit], Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    /// Search persons with multi-word AND logic.
    /// Uses GLOB for case-sensitive (default SQLite LIKE is case-insensitive).
    /// Searches across: name, email, notes, city, and organization name.
    pub fn search_persons_multi(
        &self,
        words: &[&str],
        case_sensitive: bool,
        limit: u32,
    ) -> Result<Vec<Person>> {
        if words.is_empty() {
            return Ok(vec![]);
        }

        // Build dynamic WHERE clause for AND logic
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        for (i, word) in words.iter().enumerate() {
            // GLOB uses * wildcard and is case-sensitive; LIKE uses % and is case-insensitive
            let pattern = if case_sensitive {
                format!("*{}*", Self::escape_glob(word))
            } else {
                format!("%{}%", Self::escape_like(&word.to_lowercase()))
            };
            params.push(pattern);

            let param_num = i + 1;
            if case_sensitive {
                // GLOB is case-sensitive in SQLite
                conditions.push(format!(
                    "(p.display_name GLOB ?{0} OR e.email_address GLOB ?{0} OR n.content GLOB ?{0} OR a.city GLOB ?{0} OR o.name GLOB ?{0})",
                    param_num
                ));
            } else {
                // search_name is already lowercase; LIKE is case-insensitive
                // ESCAPE '\' enables backslash escaping for % and _ literals
                conditions.push(format!(
                    "(p.search_name LIKE ?{0} ESCAPE '\\' OR LOWER(e.email_address) LIKE ?{0} ESCAPE '\\' OR LOWER(n.content) LIKE ?{0} ESCAPE '\\' OR LOWER(a.city) LIKE ?{0} ESCAPE '\\' OR LOWER(o.name) LIKE ?{0} ESCAPE '\\')",
                    param_num
                ));
            }
        }

        let where_clause = conditions.join(" AND ");
        let limit_param = params.len() + 1;

        let sql = format!(
            r#"SELECT DISTINCT p.* FROM persons p
               LEFT JOIN emails e ON e.person_id = p.id
               LEFT JOIN notes n ON n.person_id = p.id
               LEFT JOIN addresses a ON a.person_id = p.id
               LEFT JOIN person_organizations po ON po.person_id = p.id
               LEFT JOIN organizations o ON o.id = po.organization_id
               WHERE p.is_active = 1 AND ({})
               ORDER BY p.sort_name ASC
               LIMIT ?{}"#,
            where_clause, limit_param
        );

        let mut stmt = self.conn.prepare(&sql)?;

        // Build parameter slice
        let mut all_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for p in &params {
            all_params.push(p);
        }
        all_params.push(&limit);

        let persons = stmt
            .query_map(rusqlite::params_from_iter(all_params), Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    /// Escape GLOB metacharacters (* ? [ ]) using character class escaping
    fn escape_glob(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '*' => result.push_str("[*]"),
                '?' => result.push_str("[?]"),
                '[' => result.push_str("[[]"),
                ']' => result.push_str("[]]"),
                _ => result.push(c),
            }
        }
        result
    }

    /// Escape LIKE metacharacters (% _ \)
    fn escape_like(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '%' | '_' | '\\' => {
                    result.push('\\');
                    result.push(c);
                }
                _ => result.push(c),
            }
        }
        result
    }

    /// Find persons missing a phone number
    pub fn find_persons_missing_phone(&self, limit: u32) -> Result<Vec<Person>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT p.* FROM persons p
               LEFT JOIN phones ph ON ph.person_id = p.id
               WHERE p.is_active = 1 AND ph.id IS NULL
               ORDER BY p.sort_name ASC
               LIMIT ?"#,
        )?;

        let persons = stmt
            .query_map([limit], Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    /// Find persons missing an email address
    pub fn find_persons_missing_email(&self, limit: u32) -> Result<Vec<Person>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT p.* FROM persons p
               LEFT JOIN emails e ON e.person_id = p.id
               WHERE p.is_active = 1 AND e.id IS NULL
               ORDER BY p.sort_name ASC
               LIMIT ?"#,
        )?;

        let persons = stmt
            .query_map([limit], Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    /// Find persons missing both phone and email
    pub fn find_persons_missing_both(&self, limit: u32) -> Result<Vec<Person>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT p.* FROM persons p
               LEFT JOIN phones ph ON ph.person_id = p.id
               LEFT JOIN emails e ON e.person_id = p.id
               WHERE p.is_active = 1 AND ph.id IS NULL AND e.id IS NULL
               ORDER BY p.sort_name ASC
               LIMIT ?"#,
        )?;

        let persons = stmt
            .query_map([limit], Self::row_to_person)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(persons)
    }

    /// Get display info (email, location) for multiple persons in a single query.
    /// Returns a map from person_id to (primary_email, location).
    pub fn get_display_info_for_persons(
        &self,
        person_ids: &[Uuid],
    ) -> Result<DisplayInfo> {

        if person_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build IN clause with placeholders
        let placeholders: Vec<&str> = person_ids.iter().map(|_| "?").collect();
        let sql = format!(
            r#"SELECT
                p.id,
                (SELECT email_address FROM emails WHERE person_id = p.id AND is_primary = 1 LIMIT 1) as primary_email,
                (SELECT city FROM addresses WHERE person_id = p.id AND is_primary = 1 LIMIT 1) as city,
                (SELECT state FROM addresses WHERE person_id = p.id AND is_primary = 1 LIMIT 1) as state
            FROM persons p
            WHERE p.id IN ({})"#,
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let id_strings: Vec<String> = person_ids.iter().map(|id| id.to_string()).collect();
        let params: Vec<&dyn rusqlite::ToSql> = id_strings
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt
            .query_map(rusqlite::params_from_iter(params), |row| {
                let id_str: String = row.get(0)?;
                let primary_email: Option<String> = row.get(1)?;
                let city: Option<String> = row.get(2)?;
                let state: Option<String> = row.get(3)?;

                let location = match (&city, &state) {
                    (Some(c), Some(s)) => Some(format!("{}, {}", c, s)),
                    (Some(c), None) => Some(c.clone()),
                    (None, Some(s)) => Some(s.clone()),
                    (None, None) => None,
                };

                Ok((id_str, primary_email, location))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut map = HashMap::new();
        for (id_str, email, location) in rows {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                map.insert(id, (email, location));
            }
        }

        Ok(map)
    }

    /// Get tags for a person
    pub fn get_tags_for_person(&self, person_id: Uuid) -> Result<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT t.id, t.name, t.color
               FROM tags t
               JOIN person_tags pt ON pt.tag_id = t.id
               WHERE pt.person_id = ?
               ORDER BY t.name"#,
        )?;

        let tags = stmt
            .query_map([person_id.to_string()], |row| {
                let id: String = row.get(0)?;
                Ok(Tag {
                    id: parse_uuid(&id)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(tags)
    }

    // ==================== PERSON UPDATE ====================

    /// Update a person. Automatically sets `updated_at` to now.
    pub fn update_person(&self, person: &Person) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            r#"UPDATE persons SET
                name_given = ?, name_family = ?, name_middle = ?, name_prefix = ?,
                name_suffix = ?, name_nickname = ?, preferred_name = ?, display_name = ?,
                sort_name = ?, search_name = ?, name_order = ?, person_type = ?,
                notes = ?, is_active = ?, updated_at = ?, is_dirty = ?, external_ids = ?
               WHERE id = ?"#,
            params![
                person.name_given,
                person.name_family,
                person.name_middle,
                person.name_prefix,
                person.name_suffix,
                person.name_nickname,
                person.preferred_name,
                person.display_name,
                person.sort_name,
                person.search_name,
                person.name_order.as_str(),
                person.person_type.as_str(),
                person.notes,
                person.is_active as i32,
                now.to_rfc3339(),
                person.is_dirty as i32,
                person.external_ids,
                person.id.to_string(),
            ],
        )?;
        Ok(())
    }

    // ==================== PERSON DELETE ====================

    /// Hard delete a person and all related records (via CASCADE).
    pub fn delete_person(&self, id: Uuid) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM persons WHERE id = ?", [id.to_string()])?;
        Ok(rows > 0)
    }

    /// Batch delete multiple persons in a single transaction.
    /// Returns the number of successfully deleted persons.
    pub fn delete_persons_batch(&self, ids: &[Uuid]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        // Use a transaction for atomicity and performance
        self.conn.execute("BEGIN IMMEDIATE", [])?;

        let mut deleted = 0;
        for id in ids {
            match self.conn.execute(
                "DELETE FROM persons WHERE id = ?",
                [id.to_string()],
            ) {
                Ok(rows) => deleted += rows,
                Err(e) => {
                    // Rollback on error
                    let _ = self.conn.execute("ROLLBACK", []);
                    return Err(e.into());
                }
            }
        }

        self.conn.execute("COMMIT", [])?;
        Ok(deleted)
    }

    /// Soft delete: deactivate a person (set is_active = false).
    /// The person can be reactivated later. Related records are preserved.
    pub fn deactivate_person(&self, id: Uuid) -> Result<bool> {
        let now = Utc::now();
        let rows = self.conn.execute(
            "UPDATE persons SET is_active = 0, updated_at = ? WHERE id = ?",
            params![now.to_rfc3339(), id.to_string()],
        )?;
        Ok(rows > 0)
    }

    /// Reactivate a previously deactivated person.
    pub fn reactivate_person(&self, id: Uuid) -> Result<bool> {
        let now = Utc::now();
        let rows = self.conn.execute(
            "UPDATE persons SET is_active = 1, updated_at = ? WHERE id = ?",
            params![now.to_rfc3339(), id.to_string()],
        )?;
        Ok(rows > 0)
    }

    // ==================== EMAIL CRUD ====================

    /// Insert an email. If this is the first email for the person, it becomes primary.
    pub fn insert_email(&self, email: &Email) -> Result<()> {
        let is_first = self.conn.query_row(
            "SELECT COUNT(*) FROM emails WHERE person_id = ?",
            [email.person_id.to_string()],
            |row| row.get::<_, i32>(0),
        )? == 0;

        let is_primary = email.is_primary || is_first;

        self.conn.execute(
            "INSERT INTO emails (id, person_id, email_address, email_type, is_primary)
             VALUES (?, ?, ?, ?, ?)",
            params![
                email.id.to_string(),
                email.person_id.to_string(),
                email.email_address,
                email.email_type.as_str(),
                is_primary as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_emails_for_person(&self, person_id: Uuid) -> Result<Vec<Email>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, person_id, email_address, email_type, is_primary
             FROM emails WHERE person_id = ? ORDER BY is_primary DESC",
        )?;

        let emails = stmt
            .query_map([person_id.to_string()], Self::row_to_email)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(emails)
    }

    pub fn update_email(&self, email: &Email) -> Result<()> {
        self.conn.execute(
            "UPDATE emails SET email_address = ?, email_type = ?, is_primary = ? WHERE id = ?",
            params![
                email.email_address,
                email.email_type.as_str(),
                email.is_primary as i32,
                email.id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn delete_email(&self, id: Uuid) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM emails WHERE id = ?", [id.to_string()])?;
        Ok(rows > 0)
    }

    pub fn delete_emails_for_person(&self, person_id: Uuid) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM emails WHERE person_id = ?",
            [person_id.to_string()],
        )?;
        Ok(rows)
    }

    // ==================== PHONE CRUD ====================

    /// Insert a phone. If this is the first phone for the person, it becomes primary.
    pub fn insert_phone(&self, phone: &Phone) -> Result<()> {
        let is_first = self.conn.query_row(
            "SELECT COUNT(*) FROM phones WHERE person_id = ?",
            [phone.person_id.to_string()],
            |row| row.get::<_, i32>(0),
        )? == 0;

        let is_primary = phone.is_primary || is_first;

        self.conn.execute(
            "INSERT INTO phones (id, person_id, phone_number, phone_type, is_primary)
             VALUES (?, ?, ?, ?, ?)",
            params![
                phone.id.to_string(),
                phone.person_id.to_string(),
                phone.phone_number,
                phone.phone_type.as_str(),
                is_primary as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_phones_for_person(&self, person_id: Uuid) -> Result<Vec<Phone>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, person_id, phone_number, phone_type, is_primary
             FROM phones WHERE person_id = ? ORDER BY is_primary DESC",
        )?;

        let phones = stmt
            .query_map([person_id.to_string()], Self::row_to_phone)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(phones)
    }

    pub fn update_phone(&self, phone: &Phone) -> Result<()> {
        self.conn.execute(
            "UPDATE phones SET phone_number = ?, phone_type = ?, is_primary = ? WHERE id = ?",
            params![
                phone.phone_number,
                phone.phone_type.as_str(),
                phone.is_primary as i32,
                phone.id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn delete_phone(&self, id: Uuid) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM phones WHERE id = ?", [id.to_string()])?;
        Ok(rows > 0)
    }

    pub fn delete_phones_for_person(&self, person_id: Uuid) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM phones WHERE person_id = ?",
            [person_id.to_string()],
        )?;
        Ok(rows)
    }

    // ==================== ADDRESS CRUD ====================

    /// Insert an address. If this is the first address for the person, it becomes primary.
    pub fn insert_address(&self, address: &Address) -> Result<()> {
        let is_first = self.conn.query_row(
            "SELECT COUNT(*) FROM addresses WHERE person_id = ?",
            [address.person_id.to_string()],
            |row| row.get::<_, i32>(0),
        )? == 0;

        let is_primary = address.is_primary || is_first;

        self.conn.execute(
            "INSERT INTO addresses (id, person_id, street, street2, city, state, postal_code, country, address_type, is_primary)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                address.id.to_string(),
                address.person_id.to_string(),
                address.street,
                address.street2,
                address.city,
                address.state,
                address.postal_code,
                address.country,
                address.address_type.as_str(),
                is_primary as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_addresses_for_person(&self, person_id: Uuid) -> Result<Vec<Address>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, person_id, street, street2, city, state, postal_code, country, address_type, is_primary
             FROM addresses WHERE person_id = ? ORDER BY is_primary DESC"
        )?;

        let addresses = stmt
            .query_map([person_id.to_string()], Self::row_to_address)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(addresses)
    }

    pub fn update_address(&self, address: &Address) -> Result<()> {
        self.conn.execute(
            "UPDATE addresses SET street = ?, street2 = ?, city = ?, state = ?,
             postal_code = ?, country = ?, address_type = ?, is_primary = ? WHERE id = ?",
            params![
                address.street,
                address.street2,
                address.city,
                address.state,
                address.postal_code,
                address.country,
                address.address_type.as_str(),
                address.is_primary as i32,
                address.id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn delete_address(&self, id: Uuid) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM addresses WHERE id = ?", [id.to_string()])?;
        Ok(rows > 0)
    }

    pub fn delete_addresses_for_person(&self, person_id: Uuid) -> Result<usize> {
        let rows = self.conn.execute(
            "DELETE FROM addresses WHERE person_id = ?",
            [person_id.to_string()],
        )?;
        Ok(rows)
    }

    // ==================== ROW MAPPERS ====================

    fn row_to_person(row: &Row) -> rusqlite::Result<Person> {
        let id: String = row.get("id")?;
        let name_order: String = row.get("name_order")?;
        let person_type: String = row.get("person_type")?;
        let created_at: String = row.get("created_at")?;
        let updated_at: String = row.get("updated_at")?;

        Ok(Person {
            id: parse_uuid(&id)?,
            name_given: row.get("name_given")?,
            name_family: row.get("name_family")?,
            name_middle: row.get("name_middle")?,
            name_prefix: row.get("name_prefix")?,
            name_suffix: row.get("name_suffix")?,
            name_nickname: row.get("name_nickname")?,
            preferred_name: row.get("preferred_name")?,
            display_name: row.get("display_name")?,
            sort_name: row.get("sort_name")?,
            search_name: row.get("search_name")?,
            name_order: NameOrder::parse(&name_order),
            person_type: PersonType::parse(&person_type),
            notes: row.get("notes")?,
            is_active: row.get::<_, i32>("is_active")? == 1,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            is_dirty: row.get::<_, i32>("is_dirty")? == 1,
            external_ids: row.get("external_ids")?,
        })
    }

    fn row_to_email(row: &Row) -> rusqlite::Result<Email> {
        let id: String = row.get("id")?;
        let person_id: String = row.get("person_id")?;
        let email_type_str: Option<String> = row.get("email_type")?;

        Ok(Email {
            id: parse_uuid(&id)?,
            person_id: parse_uuid(&person_id)?,
            email_address: row.get("email_address")?,
            email_type: EmailType::parse(&email_type_str.unwrap_or_default()),
            is_primary: row.get::<_, i32>("is_primary")? == 1,
        })
    }

    fn row_to_phone(row: &Row) -> rusqlite::Result<Phone> {
        let id: String = row.get("id")?;
        let person_id: String = row.get("person_id")?;
        let phone_type_str: Option<String> = row.get("phone_type")?;

        Ok(Phone {
            id: parse_uuid(&id)?,
            person_id: parse_uuid(&person_id)?,
            phone_number: row.get("phone_number")?,
            phone_type: PhoneType::parse(&phone_type_str.unwrap_or_default()),
            is_primary: row.get::<_, i32>("is_primary")? == 1,
        })
    }

    fn row_to_address(row: &Row) -> rusqlite::Result<Address> {
        let id: String = row.get("id")?;
        let person_id: String = row.get("person_id")?;
        let address_type_str: Option<String> = row.get("address_type")?;

        Ok(Address {
            id: parse_uuid(&id)?,
            person_id: parse_uuid(&person_id)?,
            street: row.get("street")?,
            street2: row.get("street2")?,
            city: row.get("city")?,
            state: row.get("state")?,
            postal_code: row.get("postal_code")?,
            country: row.get("country")?,
            address_type: AddressType::parse(&address_type_str.unwrap_or_default()),
            is_primary: row.get::<_, i32>("is_primary")? == 1,
        })
    }

    // ==================== SPECIAL DATE CRUD ====================

    pub fn get_special_dates_for_person(&self, person_id: Uuid) -> Result<Vec<SpecialDate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, person_id, date, date_type, label, year_known
             FROM special_dates WHERE person_id = ? ORDER BY date",
        )?;

        let dates = stmt
            .query_map([person_id.to_string()], |row| {
                let id: String = row.get(0)?;
                let pid: String = row.get(1)?;
                let date_type_str: String = row.get(3)?;

                Ok(SpecialDate {
                    id: parse_uuid(&id)?,
                    person_id: parse_uuid(&pid)?,
                    date: row.get(2)?,
                    date_type: DateType::parse(&date_type_str),
                    label: row.get(4)?,
                    year_known: row.get::<_, i32>(5)? == 1,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(dates)
    }

    // ==================== NOTE CRUD ====================

    pub fn insert_note(&self, note: &Note) -> Result<()> {
        self.conn.execute(
            "INSERT INTO notes (id, person_id, content, note_type, is_pinned, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                note.id.to_string(),
                note.person_id.to_string(),
                note.content,
                note.note_type,
                note.is_pinned as i32,
                note.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_notes_for_person(&self, person_id: Uuid) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, person_id, content, note_type, is_pinned, created_at
             FROM notes WHERE person_id = ? ORDER BY is_pinned DESC, created_at DESC",
        )?;

        let notes = stmt
            .query_map([person_id.to_string()], |row| {
                let id: String = row.get(0)?;
                let pid: String = row.get(1)?;
                let created_at: String = row.get(5)?;

                Ok(Note {
                    id: parse_uuid(&id)?,
                    person_id: parse_uuid(&pid)?,
                    content: row.get(2)?,
                    note_type: row.get(3)?,
                    is_pinned: row.get::<_, i32>(4)? == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(notes)
    }

    // ==================== INTERACTION CRUD ====================

    pub fn get_interactions_for_person(&self, person_id: Uuid, limit: u32) -> Result<Vec<Interaction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, person_id, interaction_type, occurred_at, summary, notes, sentiment
             FROM interactions WHERE person_id = ? ORDER BY occurred_at DESC LIMIT ?",
        )?;

        let interactions = stmt
            .query_map(params![person_id.to_string(), limit], |row| {
                let id: String = row.get(0)?;
                let pid: String = row.get(1)?;
                let interaction_type_str: String = row.get(2)?;
                let occurred_at: String = row.get(3)?;

                Ok(Interaction {
                    id: parse_uuid(&id)?,
                    person_id: parse_uuid(&pid)?,
                    interaction_type: InteractionType::parse(&interaction_type_str),
                    occurred_at: chrono::DateTime::parse_from_rfc3339(&occurred_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    summary: row.get(4)?,
                    notes: row.get(5)?,
                    sentiment: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(interactions)
    }

    // ==================== SPECIAL DATE CRUD ====================

    pub fn insert_special_date(&self, date: &SpecialDate) -> Result<()> {
        self.conn.execute(
            "INSERT INTO special_dates (id, person_id, date, date_type, label, year_known)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                date.id.to_string(),
                date.person_id.to_string(),
                date.date,
                date.date_type.as_str(),
                date.label,
                date.year_known as i32,
            ],
        )?;
        Ok(())
    }

    // ==================== ORGANIZATION CRUD ====================

    pub fn insert_organization(&self, org: &Organization) -> Result<()> {
        self.conn.execute(
            "INSERT INTO organizations (id, name, org_type, industry, website, city, state, country)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                org.id.to_string(),
                org.name,
                org.org_type,
                org.industry,
                org.website,
                org.city,
                org.state,
                org.country,
            ],
        )?;
        Ok(())
    }

    pub fn get_organization_by_name(&self, name: &str) -> Result<Option<Organization>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, org_type, industry, website, city, state, country FROM organizations WHERE name = ?",
        )?;

        let result = stmt.query_row([name], |row| {
            let id: String = row.get(0)?;
            Ok(Organization {
                id: parse_uuid(&id)?,
                name: row.get(1)?,
                org_type: row.get(2)?,
                industry: row.get(3)?,
                website: row.get(4)?,
                city: row.get(5)?,
                state: row.get(6)?,
                country: row.get(7)?,
            })
        });

        match result {
            Ok(org) => Ok(Some(org)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get or create an organization by name
    pub fn get_or_create_organization(&self, name: &str) -> Result<Organization> {
        if let Some(org) = self.get_organization_by_name(name)? {
            return Ok(org);
        }

        let org = Organization::new(name.to_string());
        self.insert_organization(&org)?;
        Ok(org)
    }

    pub fn insert_person_organization(&self, po: &PersonOrganization) -> Result<()> {
        self.conn.execute(
            "INSERT INTO person_organizations (id, person_id, organization_id, title, department, relationship_type, start_date, end_date, is_current, is_primary)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                po.id.to_string(),
                po.person_id.to_string(),
                po.organization_id.to_string(),
                po.title,
                po.department,
                po.relationship_type,
                po.start_date,
                po.end_date,
                po.is_current as i32,
                po.is_primary as i32,
            ],
        )?;
        Ok(())
    }

    /// Delete all organization links for a person
    pub fn delete_person_organizations(&self, person_id: Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM person_organizations WHERE person_id = ?",
            params![person_id.to_string()],
        )?;
        Ok(())
    }

    // ==================== EXTERNAL ID LOOKUPS ====================

    /// Find a person by their external ID (e.g., Apple Contacts identifier)
    pub fn find_person_by_external_id(&self, source: &str, external_id: &str) -> Result<Option<Person>> {
        // external_ids is stored as JSON, e.g., {"apple": "ABC123"}
        let pattern = format!("%\"{}\":\"{}\"%", source, external_id);
        let mut stmt = self.conn.prepare(
            "SELECT * FROM persons WHERE external_ids LIKE ? AND is_active = 1 LIMIT 1",
        )?;

        let result = stmt.query_row([pattern], Self::row_to_person);

        match result {
            Ok(person) => Ok(Some(person)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ==================== CONTACT DETAIL ====================

    /// Get full contact detail with all related data
    pub fn get_contact_detail(&self, id: Uuid) -> Result<Option<ContactDetail>> {
        let person = match self.get_person_by_id(id)? {
            Some(p) => p,
            None => return Ok(None),
        };

        let emails = self.get_emails_for_person(id)?;
        let phones = self.get_phones_for_person(id)?;
        let addresses = self.get_addresses_for_person(id)?;
        let organizations = self.get_organizations_for_person(id)?;
        let tags = self.get_tags_for_person(id)?;
        let special_dates = self.get_special_dates_for_person(id)?;
        let notes = self.get_notes_for_person(id)?;
        let interactions = self.get_interactions_for_person(id, 10)?;

        Ok(Some(ContactDetail {
            person,
            emails,
            phones,
            addresses,
            organizations,
            tags,
            special_dates,
            notes,
            interactions,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get_person() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("John".to_string());
        person.name_family = Some("Smith".to_string());
        person.compute_names();

        db.insert_person(&person).unwrap();

        let retrieved = db.get_person_by_id(person.id).unwrap().unwrap();
        assert_eq!(retrieved.name_given, Some("John".to_string()));
        assert_eq!(retrieved.display_name, Some("John Smith".to_string()));
    }

    #[test]
    fn test_list_persons() {
        let db = Database::open_memory().unwrap();

        for i in 0..5 {
            let mut p = Person::new();
            p.name_given = Some(format!("Person{}", i));
            p.compute_names();
            db.insert_person(&p).unwrap();
        }

        let persons = db.list_persons(10, 0).unwrap();
        assert_eq!(persons.len(), 5);
    }

    #[test]
    fn test_search_persons() {
        let db = Database::open_memory().unwrap();

        let mut p1 = Person::new();
        p1.name_given = Some("John".to_string());
        p1.name_family = Some("Smith".to_string());
        p1.compute_names();
        db.insert_person(&p1).unwrap();

        let mut p2 = Person::new();
        p2.name_given = Some("Jane".to_string());
        p2.name_family = Some("Doe".to_string());
        p2.compute_names();
        db.insert_person(&p2).unwrap();

        let results = db.search_persons("john", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name_given, Some("John".to_string()));
    }

    #[test]
    fn test_get_person_by_email() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Alice".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        let email = Email::new(person.id, "alice@example.com".to_string());
        db.insert_email(&email).unwrap();

        // Find by exact email
        let found = db.get_person_by_email("alice@example.com").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, person.id);

        // Not found
        let not_found = db.get_person_by_email("nobody@example.com").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_delete_person() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("ToDelete".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        assert!(db.delete_person(person.id).unwrap());
        assert!(db.get_person_by_id(person.id).unwrap().is_none());
    }

    #[test]
    fn test_soft_delete_and_reactivate() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("SoftDelete".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        // Deactivate (soft delete)
        assert!(db.deactivate_person(person.id).unwrap());

        // Should not be found by normal queries
        assert!(db.get_person_by_id(person.id).unwrap().is_none());
        assert_eq!(db.count_persons().unwrap(), 0);

        // Reactivate
        assert!(db.reactivate_person(person.id).unwrap());

        // Now should be found
        let found = db.get_person_by_id(person.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name_given, Some("SoftDelete".to_string()));
    }

    #[test]
    fn test_cascade_delete() {
        let db = Database::open_memory().unwrap();

        // Create person with email, phone, and address
        let mut person = Person::new();
        person.name_given = Some("Cascade".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        let email = Email::new(person.id, "cascade@example.com".to_string());
        db.insert_email(&email).unwrap();

        let phone = Phone::new(person.id, "555-CASCADE".to_string());
        db.insert_phone(&phone).unwrap();

        let mut address = Address::new(person.id);
        address.city = Some("Cascade City".to_string());
        db.insert_address(&address).unwrap();

        // Verify they exist
        assert_eq!(db.get_emails_for_person(person.id).unwrap().len(), 1);
        assert_eq!(db.get_phones_for_person(person.id).unwrap().len(), 1);
        assert_eq!(db.get_addresses_for_person(person.id).unwrap().len(), 1);

        // Delete person (hard delete)
        db.delete_person(person.id).unwrap();

        // All related records should be gone (CASCADE)
        assert_eq!(db.get_emails_for_person(person.id).unwrap().len(), 0);
        assert_eq!(db.get_phones_for_person(person.id).unwrap().len(), 0);
        assert_eq!(db.get_addresses_for_person(person.id).unwrap().len(), 0);
    }

    #[test]
    fn test_email_crud() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Test".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        // Create
        let mut email = Email::new(person.id, "test@example.com".to_string());
        email.email_type = EmailType::Work;
        db.insert_email(&email).unwrap();

        // Read
        let emails = db.get_emails_for_person(person.id).unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].email_address, "test@example.com");

        // Update
        let mut updated_email = emails[0].clone();
        updated_email.email_address = "updated@example.com".to_string();
        db.update_email(&updated_email).unwrap();

        let emails = db.get_emails_for_person(person.id).unwrap();
        assert_eq!(emails[0].email_address, "updated@example.com");

        // Delete
        assert!(db.delete_email(email.id).unwrap());
        let emails = db.get_emails_for_person(person.id).unwrap();
        assert_eq!(emails.len(), 0);
    }

    #[test]
    fn test_phone_crud() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Test".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        // Create
        let phone = Phone::new(person.id, "555-1234".to_string());
        db.insert_phone(&phone).unwrap();

        // Read
        let phones = db.get_phones_for_person(person.id).unwrap();
        assert_eq!(phones.len(), 1);
        assert_eq!(phones[0].phone_number, "555-1234");

        // Update
        let mut updated_phone = phones[0].clone();
        updated_phone.phone_number = "555-5678".to_string();
        db.update_phone(&updated_phone).unwrap();

        let phones = db.get_phones_for_person(person.id).unwrap();
        assert_eq!(phones[0].phone_number, "555-5678");

        // Delete
        assert!(db.delete_phone(phone.id).unwrap());
        let phones = db.get_phones_for_person(person.id).unwrap();
        assert_eq!(phones.len(), 0);
    }

    #[test]
    fn test_address_crud() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Test".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        // Create
        let mut address = Address::new(person.id);
        address.city = Some("Seattle".to_string());
        address.state = Some("WA".to_string());
        db.insert_address(&address).unwrap();

        // Read
        let addresses = db.get_addresses_for_person(person.id).unwrap();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].city, Some("Seattle".to_string()));

        // Update
        let mut updated_address = addresses[0].clone();
        updated_address.city = Some("Portland".to_string());
        updated_address.state = Some("OR".to_string());
        db.update_address(&updated_address).unwrap();

        let addresses = db.get_addresses_for_person(person.id).unwrap();
        assert_eq!(addresses[0].city, Some("Portland".to_string()));

        // Delete
        assert!(db.delete_address(address.id).unwrap());
        let addresses = db.get_addresses_for_person(person.id).unwrap();
        assert_eq!(addresses.len(), 0);
    }

    #[test]
    fn test_first_email_becomes_primary() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Test".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        // First email should auto-become primary even if not marked
        let email1 = Email::new(person.id, "first@example.com".to_string());
        assert!(!email1.is_primary); // Not marked as primary
        db.insert_email(&email1).unwrap();

        let emails = db.get_emails_for_person(person.id).unwrap();
        assert!(emails[0].is_primary); // But stored as primary

        // Second email stays non-primary
        let email2 = Email::new(person.id, "second@example.com".to_string());
        db.insert_email(&email2).unwrap();

        let emails = db.get_emails_for_person(person.id).unwrap();
        let primary_count = emails.iter().filter(|e| e.is_primary).count();
        assert_eq!(primary_count, 1);
        assert_eq!(
            emails.iter().find(|e| e.is_primary).unwrap().email_address,
            "first@example.com"
        );
    }

    #[test]
    fn test_first_phone_becomes_primary() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Test".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        let phone1 = Phone::new(person.id, "555-0001".to_string());
        db.insert_phone(&phone1).unwrap();

        let phones = db.get_phones_for_person(person.id).unwrap();
        assert!(phones[0].is_primary);

        let phone2 = Phone::new(person.id, "555-0002".to_string());
        db.insert_phone(&phone2).unwrap();

        let phones = db.get_phones_for_person(person.id).unwrap();
        let primary_count = phones.iter().filter(|p| p.is_primary).count();
        assert_eq!(primary_count, 1);
    }

    #[test]
    fn test_first_address_becomes_primary() {
        let db = Database::open_memory().unwrap();

        let mut person = Person::new();
        person.name_given = Some("Test".to_string());
        person.compute_names();
        db.insert_person(&person).unwrap();

        let addr1 = Address::new(person.id);
        db.insert_address(&addr1).unwrap();

        let addresses = db.get_addresses_for_person(person.id).unwrap();
        assert!(addresses[0].is_primary);

        let addr2 = Address::new(person.id);
        db.insert_address(&addr2).unwrap();

        let addresses = db.get_addresses_for_person(person.id).unwrap();
        let primary_count = addresses.iter().filter(|a| a.is_primary).count();
        assert_eq!(primary_count, 1);
    }
}
