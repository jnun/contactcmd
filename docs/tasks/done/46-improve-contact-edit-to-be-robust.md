# Task 46: Complete Person Field Editing

**Feature**: none
**Created**: 2026-01-29
**Depends on**: none
**Blocks**: Task 47

## Problem

The current contact edit functionality (`handle_edit_all` in `src/cli/list.rs`) only allows editing a subset of person fields. The `persons` table has several fields that cannot be modified through the UI, including `preferred_name`, `name_order`, and `person_type`. Users need complete control over all editable person record fields to maintain accurate contact information.

## Current State

**Already editable:**
- name_given, name_family, name_middle, name_nickname, name_prefix, name_suffix
- notes (inline person.notes field)

**Not yet editable (this task):**
- `preferred_name` - How the person prefers to be addressed
- `name_order` - Western (Given Family), Eastern (Family Given), or Latin format
- `person_type` - Personal, Business, Prospect, or Connector classification

**Read-only (do not expose):**
- id, display_name, sort_name, search_name (computed)
- created_at, updated_at, is_dirty, is_active, external_ids (system-managed)

## Implementation Plan

### 1. Add Preferred Name Field
**Location:** `src/cli/list.rs` in `handle_edit_all()`

Add after the existing name fields:
```
[Preferred Name]: _______________
```
- Text input field
- Used when display_name computation needs override
- Show current value as default

### 2. Add Name Order Selector
**Location:** `src/cli/list.rs` in `handle_edit_all()`

Add selection menu:
```
[Name Order]:
  (1) Western - "Given Family" (John Smith)
  (2) Eastern - "Family Given" (Smith John)
  (3) Latin   - "Given Family" with formal ordering
```
- Affects how display_name is computed
- Default to current value or "Western"

### 3. Add Person Type Selector
**Location:** `src/cli/list.rs` in `handle_edit_all()`

Add selection menu:
```
[Contact Type]:
  (1) Personal   - Friends, family, personal contacts
  (2) Business   - Professional/work contacts
  (3) Prospect   - Potential business leads
  (4) Connector  - Networking contacts who introduce others
```
- Helps with filtering and organization
- Default to current value or "Personal"

### 4. Update Database Layer
**Location:** `src/db/persons.rs`

Ensure `update_person()` handles:
- preferred_name updates
- name_order updates
- person_type updates
- Proper recomputation of display_name when name_order changes

### 5. UI Flow Changes
**Location:** `src/cli/list.rs`

Reorganize edit menu into logical sections:
```
=== Edit Contact: John Smith ===

[Name]
  First: John
  Last: Smith
  Middle: _
  Nickname: Johnny
  Prefix: _
  Suffix: Jr.
  Preferred: JD

[Classification]
  Name Order: Western
  Contact Type: Personal

[Notes]
  General notes here...

[s] Save  [c] Cancel
```

## Success Criteria

- [x] User can edit preferred_name field for any contact
- [x] User can change name_order between Western/Eastern/Latin
- [x] User can classify contacts as Personal/Business/Prospect/Connector
- [x] Display name updates correctly when name_order changes
- [x] All existing edit functionality continues to work
- [x] Changes persist to database correctly

## Technical Notes

- The `name_order` enum values must match: 'western', 'eastern', 'latin'
- The `person_type` enum values must match: 'personal', 'business', 'prospect', 'connector'
- Recompute display_name/sort_name/search_name after any name field change
- Consider adding validation for enum values before database write

## Files to Modify

1. `src/cli/list.rs` - Add new fields to `handle_edit_all()`
2. `src/db/persons.rs` - Ensure update handles new fields
3. `src/models/person.rs` - Verify enum types if applicable
