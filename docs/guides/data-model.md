# Data Model

SQLite with 11 tables and 16 indexes.

**Schema source of truth:** `src/db/schema.rs`

## Entity Relationship

```
Person
  ├── emails (1:N)
  ├── phones (1:N)
  ├── addresses (1:N)
  ├── notes (1:N)
  ├── interactions (1:N)
  ├── special_dates (1:N)
  ├──< person_organizations >── organizations (N:M)
  └──< person_tags >── tags (N:M)
```

## Tables

| Table | Description |
|-------|-------------|
| persons | Core contact with computed name fields |
| emails | Email addresses (FK to persons, CASCADE) |
| phones | Phone numbers (FK to persons, CASCADE) |
| addresses | Physical addresses (FK to persons, CASCADE) |
| organizations | Companies/orgs |
| person_organizations | Junction: person↔org with title, dates |
| tags | Labels with optional color |
| person_tags | Junction: person↔tag |
| special_dates | Birthdays, anniversaries |
| notes | Timestamped notes |
| interactions | Meeting/call/email logs |

Junction tables have `UNIQUE(person_id, *_id)` constraints.

## CRUD Operations

All in `src/db/persons.rs`:

**Person:** `insert`, `get_by_id`, `get_by_email`, `list`, `list_sorted`, `count`, `search`, `update`, `delete`, `deactivate`, `reactivate`

**Email/Phone/Address:** `insert`, `get_for_person`, `update`, `delete`, `delete_for_person`

**Organization:** `get_organizations_for_person` (with join data)

## Migrations

Schema versioning in `schema_version` table. Migrations run automatically on database open, wrapped in transactions.

Current version: **1**
