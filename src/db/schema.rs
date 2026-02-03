pub const SCHEMA_VERSION: i32 = 14;

pub const MIGRATION_V2: &str = r#"
ALTER TABLE persons ADD COLUMN photo_path TEXT;
"#;

/// V3 migration: Drop photo_path column (photos now derived from UUID)
/// SQLite 3.35.0+ supports DROP COLUMN directly.
/// For older versions, we rebuild the table.
pub const MIGRATION_V3_DROP_COLUMN: &str = r#"
ALTER TABLE persons DROP COLUMN photo_path;
"#;

/// V4 migration: Add app_settings table for email config, signature, etc.
pub const MIGRATION_V4: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

/// V5 migration: Add oauth_tokens table for Google OAuth2 and other providers
pub const MIGRATION_V5: &str = r#"
CREATE TABLE IF NOT EXISTS oauth_tokens (
    provider TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    access_token TEXT,
    expires_at INTEGER,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
"#;

/// V6 migration: Add tasks table for Eisenhower matrix task management
pub const MIGRATION_V6: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    quadrant INTEGER NOT NULL DEFAULT 4,
    deadline TEXT,
    completed_at TEXT,
    person_id TEXT,
    parent_id TEXT,
    privacy_level TEXT NOT NULL DEFAULT 'personal',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE SET NULL,
    FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_task_quadrant ON tasks(quadrant);
CREATE INDEX IF NOT EXISTS idx_task_deadline ON tasks(deadline);
CREATE INDEX IF NOT EXISTS idx_task_completed ON tasks(completed_at);
CREATE INDEX IF NOT EXISTS idx_task_person ON tasks(person_id);
CREATE INDEX IF NOT EXISTS idx_task_parent ON tasks(parent_id);
"#;

/// V7 migration: Add gateway tables for communication approval system
pub const MIGRATION_V7: &str = r#"
-- API keys for agent authentication
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    key_prefix TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
);

-- Communication queue for pending messages
CREATE TABLE IF NOT EXISTS communication_queue (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    channel TEXT NOT NULL,
    recipient_address TEXT NOT NULL,
    recipient_name TEXT,
    subject TEXT,
    body TEXT NOT NULL,
    priority TEXT DEFAULT 'normal',
    status TEXT DEFAULT 'pending',
    agent_context TEXT,
    created_at TEXT NOT NULL,
    reviewed_at TEXT,
    sent_at TEXT,
    error_message TEXT,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id)
);

CREATE INDEX IF NOT EXISTS idx_queue_status ON communication_queue(status);
CREATE INDEX IF NOT EXISTS idx_queue_created ON communication_queue(created_at);
CREATE INDEX IF NOT EXISTS idx_queue_api_key ON communication_queue(api_key_id);
"#;

/// V8 migration: Add checkin_date column to persons for follow-up tracking
pub const MIGRATION_V8: &str = r#"
ALTER TABLE persons ADD COLUMN checkin_date TEXT;
CREATE INDEX IF NOT EXISTS idx_person_checkin ON persons(checkin_date);
"#;

/// V9 migration: Add learn_something table for progressive feature discovery
/// Single table design - times_learned tracks user progress per feature
pub const MIGRATION_V9: &str = r#"
CREATE TABLE IF NOT EXISTS learn_something (
    id TEXT PRIMARY KEY,
    feature_name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,
    tutorial_json TEXT NOT NULL,
    times_learned INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_learn_times ON learn_something(times_learned);
CREATE INDEX IF NOT EXISTS idx_learn_category ON learn_something(category);
"#;

/// V10 migration: Add rate limiting columns to api_keys table
pub const MIGRATION_V10: &str = r#"
ALTER TABLE api_keys ADD COLUMN rate_limit_per_hour INTEGER NOT NULL DEFAULT 10;
ALTER TABLE api_keys ADD COLUMN rate_limit_per_day INTEGER NOT NULL DEFAULT 50;
"#;

/// V11 migration: Add recipient allowlist table for per-key recipient restrictions
/// Patterns support exact match (email/phone) and wildcards (*@company.com)
/// Empty allowlist = unrestricted (backward compatible)
pub const MIGRATION_V11: &str = r#"
CREATE TABLE IF NOT EXISTS api_key_allowlists (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    recipient_pattern TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_allowlist_api_key ON api_key_allowlists(api_key_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_allowlist_unique ON api_key_allowlists(api_key_id, recipient_pattern);
"#;

/// V12 migration: Add content_filters table for message content safety screening
/// Filters can auto-deny or flag messages containing sensitive patterns (SSN, credit cards, etc.)
pub const MIGRATION_V12: &str = r#"
CREATE TABLE IF NOT EXISTS content_filters (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    pattern_type TEXT NOT NULL CHECK(pattern_type IN ('regex', 'literal')),
    action TEXT NOT NULL CHECK(action IN ('deny', 'flag')),
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_content_filter_enabled ON content_filters(enabled);
CREATE INDEX IF NOT EXISTS idx_content_filter_action ON content_filters(action);
"#;

/// V13 migration: Add webhook_url column to api_keys for status change notifications
pub const MIGRATION_V13: &str = r#"
ALTER TABLE api_keys ADD COLUMN webhook_url TEXT;
"#;

/// V14 migration: Add ai_contact_allowed column to persons for per-contact AI consent
pub const MIGRATION_V14: &str = r#"
ALTER TABLE persons ADD COLUMN ai_contact_allowed INTEGER NOT NULL DEFAULT 1;
"#;

/// Fallback for older SQLite: rebuild table without photo_path
pub const MIGRATION_V3_REBUILD: &str = r#"
CREATE TABLE persons_new (
    id TEXT PRIMARY KEY,
    name_given TEXT,
    name_family TEXT,
    name_middle TEXT,
    name_prefix TEXT,
    name_suffix TEXT,
    name_nickname TEXT,
    preferred_name TEXT,
    display_name TEXT,
    sort_name TEXT,
    search_name TEXT,
    name_order TEXT NOT NULL DEFAULT 'western',
    person_type TEXT NOT NULL DEFAULT 'personal',
    notes TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_dirty INTEGER NOT NULL DEFAULT 0,
    external_ids TEXT
);

INSERT INTO persons_new SELECT
    id, name_given, name_family, name_middle, name_prefix, name_suffix,
    name_nickname, preferred_name, display_name, sort_name, search_name,
    name_order, person_type, notes, is_active, created_at, updated_at,
    is_dirty, external_ids
FROM persons;

DROP TABLE persons;
ALTER TABLE persons_new RENAME TO persons;

CREATE INDEX idx_person_search ON persons(search_name);
CREATE INDEX idx_person_sort ON persons(sort_name);
CREATE INDEX idx_person_active ON persons(is_active);
"#;

pub const SCHEMA_V1: &str = r#"
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    id INTEGER PRIMARY KEY,
    version INTEGER NOT NULL
);

-- Core person table
CREATE TABLE IF NOT EXISTS persons (
    id TEXT PRIMARY KEY,
    name_given TEXT,
    name_family TEXT,
    name_middle TEXT,
    name_prefix TEXT,
    name_suffix TEXT,
    name_nickname TEXT,
    preferred_name TEXT,
    display_name TEXT,
    sort_name TEXT,
    search_name TEXT,
    name_order TEXT NOT NULL DEFAULT 'western',
    person_type TEXT NOT NULL DEFAULT 'personal',
    notes TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_dirty INTEGER NOT NULL DEFAULT 0,
    external_ids TEXT
);

CREATE TABLE IF NOT EXISTS emails (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    email_address TEXT NOT NULL,
    email_type TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS phones (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    phone_number TEXT NOT NULL,
    phone_type TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS addresses (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    street TEXT,
    street2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    country TEXT,
    address_type TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    org_type TEXT,
    industry TEXT,
    website TEXT,
    city TEXT,
    state TEXT,
    country TEXT
);

CREATE TABLE IF NOT EXISTS person_organizations (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    title TEXT,
    department TEXT,
    relationship_type TEXT NOT NULL DEFAULT 'employee',
    start_date TEXT,
    end_date TEXT,
    is_current INTEGER NOT NULL DEFAULT 1,
    is_primary INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    UNIQUE(person_id, organization_id)
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT
);

CREATE TABLE IF NOT EXISTS person_tags (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    added_at TEXT NOT NULL,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    UNIQUE(person_id, tag_id)
);

CREATE TABLE IF NOT EXISTS special_dates (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    date TEXT NOT NULL,
    date_type TEXT NOT NULL,
    label TEXT,
    year_known INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    content TEXT NOT NULL,
    note_type TEXT,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS interactions (
    id TEXT PRIMARY KEY,
    person_id TEXT NOT NULL,
    interaction_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    summary TEXT,
    notes TEXT,
    sentiment TEXT,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_person_search ON persons(search_name);
CREATE INDEX IF NOT EXISTS idx_person_sort ON persons(sort_name);
CREATE INDEX IF NOT EXISTS idx_person_active ON persons(is_active);
CREATE INDEX IF NOT EXISTS idx_email_person ON emails(person_id);
CREATE INDEX IF NOT EXISTS idx_email_person_primary ON emails(person_id, is_primary);
CREATE INDEX IF NOT EXISTS idx_email_address ON emails(email_address);
CREATE INDEX IF NOT EXISTS idx_phone_person ON phones(person_id);
CREATE INDEX IF NOT EXISTS idx_phone_person_primary ON phones(person_id, is_primary);
CREATE INDEX IF NOT EXISTS idx_address_person ON addresses(person_id);
CREATE INDEX IF NOT EXISTS idx_address_person_primary ON addresses(person_id, is_primary);
CREATE INDEX IF NOT EXISTS idx_address_city ON addresses(city);
CREATE INDEX IF NOT EXISTS idx_person_org_person ON person_organizations(person_id);
CREATE INDEX IF NOT EXISTS idx_person_org_current ON person_organizations(person_id, is_current);
CREATE INDEX IF NOT EXISTS idx_tag_name ON tags(name);
CREATE INDEX IF NOT EXISTS idx_person_tag ON person_tags(person_id);
CREATE INDEX IF NOT EXISTS idx_special_date_person ON special_dates(person_id);
CREATE INDEX IF NOT EXISTS idx_note_person ON notes(person_id);
CREATE INDEX IF NOT EXISTS idx_interaction_person ON interactions(person_id);
CREATE INDEX IF NOT EXISTS idx_interaction_date ON interactions(occurred_at);
"#;
