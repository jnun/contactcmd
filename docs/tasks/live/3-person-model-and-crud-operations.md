# Task 3: Person Model and CRUD Operations

**Feature:** /docs/features/data-model.md
**Created:** 2026-01-27
**Depends on:** Task 2
**Blocks:** Tasks 4-10

## User Story

As a developer, I need Rust structs for all entities and database CRUD operations so that commands can create, read, update, and delete contact data.

## Acceptance Criteria

- [x] Model structs for all entities:
  - Person with computed name fields (display_name, sort_name, search_name)
  - Email, Phone, Address with type enums
  - Organization, PersonOrganization
  - Tag, PersonTag
  - SpecialDate, Note, Interaction with type enums
- [x] Type-safe enums: NameOrder, PersonType, EmailType, PhoneType, AddressType, DateType, InteractionType
- [x] CRUD operations for Person (insert, get_by_id, update, delete)
- [x] CRUD operations for related entities (emails, phones, addresses, special_dates, notes, interactions)
- [x] List view query (single query, no N+1)
- [x] Search query with multi-word AND logic
- [x] Contact detail view (get_contact_detail returns person + all related data)
- [x] `cargo test` passes (30 tests)

## References

- /docs/features/data-model.md for entity relationships
