pub const SCHEMA_VERSION: i32 = 3;

pub const MIGRATION_V2: &str = r#"
ALTER TABLE persons ADD COLUMN photo_path TEXT;
"#;

/// V3 migration: Drop photo_path column (photos now derived from UUID)
/// SQLite 3.35.0+ supports DROP COLUMN directly.
/// For older versions, we rebuild the table.
pub const MIGRATION_V3_DROP_COLUMN: &str = r#"
ALTER TABLE persons DROP COLUMN photo_path;
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
