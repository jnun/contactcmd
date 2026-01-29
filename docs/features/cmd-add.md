# Feature: add Command

**Status:** BACKLOG
**Created:** 2026-01-27
**Updated:** 2026-01-27

## Overview

Create a new contact interactively or with command-line options.

## User Stories

- As a user, I want to add contacts quickly so I can capture new people I meet
- As a user, I want interactive prompts so I don't forget fields
- As a user, I want direct options so I can script contact creation

## Requirements

### Functional Requirements

- [ ] Interactive mode with prompts
- [ ] Direct mode with CLI options
- [ ] Validate email format
- [ ] Detect potential duplicates
- [ ] Return new contact ID

### Non-Functional Requirements

- [ ] Create contact in <50ms

## Technical Design

### CLI Interface

```
contactcmd add [OPTIONS]

Options:
  -f, --first <NAME>    First name
  -l, --last <NAME>     Last name
  -e, --email <EMAIL>   Email address
  -p, --phone <PHONE>   Phone number
  -c, --company <NAME>  Company name
  -t, --title <TITLE>   Job title
  -n, --notes <TEXT>    Notes
```

### Interactive Mode

When no options provided:

```
contactcmd add

First name: John
Last name: Smith
Email (optional): john@example.com
Phone (optional): 555-123-4567
Company (optional): Acme Corp
Title (optional): Software Engineer
Notes (optional): Met at conference

Contact created: John Smith
ID: 550e8400-e29b-41d4-a716-446655440000
```

### Direct Mode

```bash
contactcmd add -f John -l Smith -e john@example.com

Contact created: John Smith
ID: 550e8400-e29b-41d4-a716-446655440000
```

### Validation

1. At least first OR last name required
2. Email format validation (contains @, has domain)
3. Duplicate check: warn if similar name+email exists

### Duplicate Detection

```
Warning: Similar contact exists:
  John Smith (johnsmith@other.com)

Continue anyway? [y/N]:
```

### Database Operations

1. Generate UUID
2. Compute display_name, sort_name, search_name
3. Insert person record
4. Insert email record (if provided)
5. Insert phone record (if provided)
6. Insert/find organization (if company provided)
7. Insert person_organization (if title/company)

## Acceptance Criteria

- [ ] Interactive mode prompts for all fields
- [ ] Direct mode accepts CLI options
- [ ] At least one name required
- [ ] Email format validated
- [ ] Duplicate warning shown
- [ ] Returns UUID on success
- [ ] Creates related records (email, phone, org)
