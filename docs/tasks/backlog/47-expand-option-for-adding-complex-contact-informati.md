# Task 47: Expand Option for Complex Contact Information

**Feature**: none
**Created**: 2026-01-29
**Depends on**: Task 46
**Blocks**: none

## Problem

Contact data extends beyond the person record into related tables: emails, phones, addresses, organizations, tags, special dates, notes, and interactions. Currently, users can only edit email/phone values but cannot manage types, add addresses, track employment history, add tags, or record special dates. An `[x] Expand` option should open a dedicated screen for managing all this complex associated data.

## Current State

**Partially editable:**
- Emails - can edit address text, cannot change type or add/remove
- Phones - can edit number text, cannot change type or add/remove
- Organization - can set name/title/department, cannot manage dates or multiple orgs

**Not editable at all:**
- Addresses (home, work, etc.)
- Email/Phone types (Personal, Work, Mobile, etc.)
- Organization employment dates and history
- Multiple organization affiliations
- Tags
- Special dates (birthdays, anniversaries)
- Notes (timestamped note records)
- Interactions (call logs, meetings)

## Design: Expand Screen

From the contact detail view, add `[x] Expand` option:

```
=== John Smith ===
Email: john@example.com (Work)
Phone: 555-1234 (Mobile)
Company: Acme Corp - Software Engineer

[e] Edit  [x] Expand  [m] Message  [b] Back

```

Pressing `[x]` opens the expanded edit screen:

```
=== Expand: John Smith ===

[1] Emails (2)
[2] Phones (1)
[3] Addresses (0)
[4] Organizations (1)
[5] Tags (3)
[6] Special Dates (1)
[7] Notes (5)
[8] Interactions (12)

[b] Back to Contact

```

## Implementation Plan

### Phase 1: Infrastructure

#### 1.1 Create Expand Module
**New file:** `src/cli/expand.rs`

- Main expand menu handler
- Sub-handlers for each data type
- Shared CRUD patterns for list management

#### 1.2 Add Expand Entry Point
**Location:** `src/cli/list.rs` or contact detail handler

- Add `[x] Expand` keybinding
- Navigate to expand module

### Phase 2: Contact Methods (Emails/Phones)

#### 2.1 Email Management Screen
```
=== Emails: John Smith ===

1. john@example.com [Work] *primary
2. johnny@gmail.com [Personal]

[a] Add Email  [d] Delete  [t] Change Type  [p] Set Primary  [b] Back
```

Features:
- Add new email with type selector
- Delete existing email
- Change email type (Personal/Work/School/Other)
- Set primary email
- Edit email address text

#### 2.2 Phone Management Screen
```
=== Phones: John Smith ===

1. 555-1234 [Mobile] *primary
2. 555-5678 [Work]

[a] Add Phone  [d] Delete  [t] Change Type  [p] Set Primary  [b] Back
```

Features:
- Add new phone with type selector
- Delete existing phone
- Change phone type (Mobile/Home/Work/Fax/Other)
- Set primary phone
- Edit phone number text

### Phase 3: Addresses

#### 3.1 Address Management Screen
```
=== Addresses: John Smith ===

1. [Home] *primary
   123 Main St
   Apt 4B
   San Francisco, CA 94102
   USA

2. [Work]
   456 Market St
   Suite 100
   San Francisco, CA 94105

[a] Add Address  [e] Edit  [d] Delete  [p] Set Primary  [b] Back
```

#### 3.2 Address Editor
```
=== Edit Address ===

Type: [Home | Work | Other]
Street: 123 Main St
Street 2: Apt 4B
City: San Francisco
State: CA
Postal: 94102
Country: USA

[s] Save  [c] Cancel
```

### Phase 4: Organizations & Employment

#### 4.1 Organizations Screen
```
=== Organizations: John Smith ===

1. Acme Corp *current *primary
   Software Engineer, Engineering
   Jan 2020 - Present

2. StartupXYZ
   Junior Developer
   Jun 2018 - Dec 2019

[a] Add Organization  [e] Edit  [d] Delete  [p] Set Primary  [b] Back
```

#### 4.2 Organization Editor
```
=== Edit Organization Role ===

Company: Acme Corp
Title: Software Engineer
Department: Engineering
Relationship: [Employee | Contractor | Founder | Board | Advisor | Volunteer]
Start Date: 2020-01-15
End Date: _ (blank = current)
Is Current: [x]

[s] Save  [c] Cancel
```

Features:
- Link to existing organization or create new
- Multiple roles at same company (promotions)
- Track employment dates
- Mark current position
- Set primary organization for display

### Phase 5: Tags

#### 5.1 Tags Screen
```
=== Tags: John Smith ===

Current: [VIP] [SF Bay Area] [Tech Industry]

Available: [Investor] [Speaker] [Customer] [Friend] ...

[a] Add Tag  [r] Remove Tag  [n] New Tag  [b] Back
```

Features:
- Add existing tag to contact
- Remove tag from contact
- Create new tag (with optional color)
- Show all available tags for quick selection

### Phase 6: Special Dates

#### 6.1 Special Dates Screen
```
=== Special Dates: John Smith ===

1. Birthday: March 15, 1985
2. Anniversary: June 20 (year unknown)
3. Work Anniversary: Jan 15, 2020

[a] Add Date  [e] Edit  [d] Delete  [b] Back
```

#### 6.2 Date Editor
```
=== Edit Special Date ===

Type: [Birthday | Anniversary | Custom]
Label: Work Anniversary (for Custom type)
Date: 2020-01-15
Year Known: [x] yes  [ ] no

[s] Save  [c] Cancel
```

### Phase 7: Notes

#### 7.1 Notes Screen
```
=== Notes: John Smith ===

[Pinned]
* Met at tech conference, interested in AI

[Recent]
1. 2026-01-15: Follow up about project proposal
2. 2025-12-01: Had coffee, discussed partnership
3. 2025-11-10: Initial meeting at networking event

[a] Add Note  [e] Edit  [d] Delete  [p] Pin/Unpin  [b] Back
```

Features:
- Add timestamped notes
- Edit existing notes
- Pin important notes
- Note types (General, Meeting, Call, Todo)
- Chronological display

### Phase 8: Interactions

#### 8.1 Interactions Screen
```
=== Interactions: John Smith ===

Filter: [All | Calls | Emails | Meetings | Texts]

1. 2026-01-20 [Call] Discussed Q1 plans (+positive)
2. 2026-01-15 [Email] Sent proposal
3. 2026-01-10 [Meeting] Lunch at Cafe Roma
4. 2025-12-20 [Text] Holiday greetings

[a] Add Interaction  [e] Edit  [d] Delete  [f] Filter  [b] Back
```

#### 8.2 Interaction Editor
```
=== Log Interaction ===

Type: [Note | Call | Email | Meeting | Text | Social | Other]
Date: 2026-01-20
Summary: Discussed Q1 plans
Notes: They're interested in expanding the partnership...
Sentiment: [Positive | Neutral | Negative]

[s] Save  [c] Cancel
```

## Success Criteria

- [ ] User can access Expand screen from contact detail view via `[x]`
- [ ] User can add, edit, delete, and set primary for multiple emails
- [ ] User can change email type (Personal/Work/School/Other)
- [ ] User can add, edit, delete, and set primary for multiple phones
- [ ] User can change phone type (Mobile/Home/Work/Fax/Other)
- [ ] User can manage full addresses with all fields
- [ ] User can track multiple organization affiliations with dates
- [ ] User can add and remove tags from contacts
- [ ] User can record special dates (birthdays, anniversaries)
- [ ] User can add timestamped notes with pin functionality
- [ ] User can log interactions with type, date, and sentiment
- [ ] All changes persist correctly to database
- [ ] Navigation flows smoothly between screens

## Database Operations Needed

### New Functions in `src/db/`

**emails.rs:**
- `add_email(person_id, email, type, is_primary)`
- `update_email(id, email, type)`
- `delete_email(id)`
- `set_primary_email(person_id, email_id)`

**phones.rs:**
- `add_phone(person_id, number, type, is_primary)`
- `update_phone(id, number, type)`
- `delete_phone(id)`
- `set_primary_phone(person_id, phone_id)`

**addresses.rs (new file):**
- `get_addresses(person_id)`
- `add_address(person_id, address_data)`
- `update_address(id, address_data)`
- `delete_address(id)`
- `set_primary_address(person_id, address_id)`

**organizations.rs:**
- `add_person_organization(person_id, org_id, role_data)`
- `update_person_organization(id, role_data)`
- `delete_person_organization(id)`
- `set_primary_organization(person_id, person_org_id)`

**tags.rs (new file):**
- `get_all_tags()`
- `create_tag(name, color)`
- `add_tag_to_person(person_id, tag_id)`
- `remove_tag_from_person(person_id, tag_id)`

**special_dates.rs (new file):**
- `get_special_dates(person_id)`
- `add_special_date(person_id, date_data)`
- `update_special_date(id, date_data)`
- `delete_special_date(id)`

**notes.rs (new file):**
- `get_notes(person_id)`
- `add_note(person_id, content, type)`
- `update_note(id, content, type)`
- `delete_note(id)`
- `toggle_pin(id)`

**interactions.rs (new file):**
- `get_interactions(person_id, filter)`
- `add_interaction(person_id, interaction_data)`
- `update_interaction(id, interaction_data)`
- `delete_interaction(id)`

## Files to Create/Modify

**New files:**
- `src/cli/expand.rs` - Main expand module
- `src/db/addresses.rs` - Address CRUD
- `src/db/tags.rs` - Tag CRUD
- `src/db/special_dates.rs` - Special date CRUD
- `src/db/notes.rs` - Note CRUD
- `src/db/interactions.rs` - Interaction CRUD

**Modify:**
- `src/cli/mod.rs` - Register expand module
- `src/cli/list.rs` - Add [x] Expand keybinding
- `src/db/mod.rs` - Register new db modules
- `src/db/emails.rs` - Add type/primary management
- `src/db/phones.rs` - Add type/primary management
- `src/db/organizations.rs` - Add date/role management

## Technical Notes

- Use consistent CRUD patterns across all sub-modules
- Implement soft navigation (back returns to previous screen)
- Consider batch saves vs immediate saves
- Add confirmation dialogs for destructive actions (delete)
- Handle foreign key constraints gracefully
- Consider undo functionality for accidental deletions
