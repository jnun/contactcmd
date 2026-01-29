# Feature: Data Model

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

SQLite database schema for storing contacts with normalized relationships.

## Requirements

### Functional Requirements

- [ ] Store person with international name support
- [ ] Multiple emails per person with type and primary flag
- [ ] Multiple phones per person with type and primary flag
- [ ] Multiple addresses per person
- [ ] Organization history with titles and dates
- [ ] Tags for categorization
- [ ] Special dates (birthdays, anniversaries)
- [ ] Notes and interactions
- [ ] External ID tracking for sync

### Non-Functional Requirements

- [ ] Database <50MB for 10,000 contacts
- [ ] Queries complete in <50ms
- [ ] Support concurrent reads

## Technical Design

### Entity Relationship

```
Person (1) ──< Email (*)
       (1) ──< Phone (*)
       (1) ──< Address (*)
       (1) ──< PersonOrganization (*) >── Organization (1)
       (1) ──< PersonTag (*) >── Tag (1)
       (1) ──< SpecialDate (*)
       (1) ──< Note (*)
       (1) ──< Interaction (*)
```

### Tables

#### persons

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| name_given | TEXT | YES | First name |
| name_family | TEXT | YES | Last name |
| name_middle | TEXT | YES | Middle name |
| name_prefix | TEXT | YES | Dr., Mr., etc. |
| name_suffix | TEXT | YES | Jr., III, PhD |
| name_nickname | TEXT | YES | Nickname |
| preferred_name | TEXT | YES | "Call me..." |
| display_name | TEXT | YES | Computed full name |
| sort_name | TEXT | YES | "Family, Given" |
| search_name | TEXT | YES | All names lowercase |
| name_order | TEXT | NO | western/eastern/latin |
| person_type | TEXT | NO | personal/business |
| notes | TEXT | YES | Freeform notes |
| is_active | INTEGER | NO | Soft delete (1=active) |
| created_at | TEXT | NO | ISO 8601 timestamp |
| updated_at | TEXT | NO | ISO 8601 timestamp |
| is_dirty | INTEGER | NO | Needs sync (0/1) |
| external_ids | TEXT | YES | JSON {"apple": "..."} |

#### emails

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| person_id | TEXT | NO | FK to persons |
| email_address | TEXT | NO | The email |
| email_type | TEXT | YES | personal/work/other |
| is_primary | INTEGER | NO | Primary flag (0/1) |

#### phones

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| person_id | TEXT | NO | FK to persons |
| phone_number | TEXT | NO | The phone number |
| phone_type | TEXT | YES | mobile/home/work/fax |
| is_primary | INTEGER | NO | Primary flag (0/1) |

#### addresses

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| person_id | TEXT | NO | FK to persons |
| street | TEXT | YES | Street line 1 |
| street2 | TEXT | YES | Street line 2 |
| city | TEXT | YES | City |
| state | TEXT | YES | State/Province |
| postal_code | TEXT | YES | ZIP/Postal |
| country | TEXT | YES | Country |
| address_type | TEXT | YES | home/work/other |
| is_primary | INTEGER | NO | Primary flag (0/1) |

#### organizations

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| name | TEXT | NO | Company name |
| org_type | TEXT | YES | company/nonprofit/etc |
| industry | TEXT | YES | Industry |
| website | TEXT | YES | URL |
| city | TEXT | YES | HQ city |
| state | TEXT | YES | HQ state |
| country | TEXT | YES | HQ country |

#### person_organizations

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| person_id | TEXT | NO | FK to persons |
| organization_id | TEXT | NO | FK to organizations |
| title | TEXT | YES | Job title |
| department | TEXT | YES | Department |
| relationship_type | TEXT | NO | employee/founder/etc |
| start_date | TEXT | YES | When started |
| end_date | TEXT | YES | When ended |
| is_current | INTEGER | NO | Currently active (0/1) |
| is_primary | INTEGER | NO | Main job (0/1) |

#### tags

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| name | TEXT | NO | Tag name (unique) |
| color | TEXT | YES | Display color |

#### person_tags

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| person_id | TEXT | NO | FK to persons |
| tag_id | TEXT | NO | FK to tags |
| added_at | TEXT | NO | When tagged |

#### special_dates

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | TEXT | NO | UUID primary key |
| person_id | TEXT | NO | FK to persons |
| date | TEXT | NO | ISO date |
| date_type | TEXT | NO | birthday/anniversary/custom |
| label | TEXT | YES | Custom label |
| year_known | INTEGER | NO | Is year significant (0/1) |

### Indexes

```sql
CREATE INDEX idx_person_search ON persons(search_name);
CREATE INDEX idx_person_sort ON persons(sort_name);
CREATE INDEX idx_person_active ON persons(is_active);
CREATE INDEX idx_email_person ON emails(person_id);
CREATE INDEX idx_email_address ON emails(email_address);
CREATE INDEX idx_phone_person ON phones(person_id);
CREATE INDEX idx_address_person ON addresses(person_id);
CREATE INDEX idx_address_city ON addresses(city);
CREATE INDEX idx_person_org_person ON person_organizations(person_id);
CREATE INDEX idx_person_org_current ON person_organizations(person_id, is_current);
CREATE INDEX idx_tag_name ON tags(name);
CREATE INDEX idx_person_tag ON person_tags(person_id);
```

### Foreign Keys

All child tables use:
```sql
FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE CASCADE
```

## Acceptance Criteria

- [ ] All tables created with correct schema
- [ ] Foreign keys with CASCADE delete
- [ ] Indexes created for query performance
- [ ] Migrations run on first launch
- [ ] Schema version tracked
