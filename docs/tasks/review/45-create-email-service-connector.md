# Task 45: Create email service connector

**Feature**: none
**Created**: 2026-01-29
**Depends on**: none
**Blocks**: Task 44

## Problem

Users need the ability to send emails directly from the contact management app. Currently, messaging is limited to SMS/iMessage. Adding email support allows users to reach contacts who may not have phone numbers or when email is the preferred communication method. The email service should integrate with macOS Mail.app to leverage existing email configurations rather than requiring separate SMTP setup.

## Implementation Plan

### 1. Setup Screen for External Services

Add a new Setup screen accessible from the main menu. This screen manages external service connections:

- Main menu gains [s] Setup option
- Setup screen shows list of configurable services (Email is first)
- Each service shows connection status (configured / not configured)
- User selects a service to configure it

### 2. Email Account Selection

When user configures email in Setup:

- App queries Mail.app for all configured accounts via AppleScript
- Displays list of available email addresses (e.g., "work@company.com", "personal@gmail.com")
- User selects their preferred sending account
- Selection is saved to app configuration
- User can change this anytime from Setup screen

Technical: Uses `osascript` to run `tell application "Mail" to get email addresses of every account`

### 3. Signature Configuration

After selecting email account, user sets their signature:

- Simple text box for entering signature
- Supports multiple lines (carriage returns allowed)
- Signature appends to all outbound emails from contactcmd
- Example: "Sent from contactcmd" or full business signature
- Stored in app configuration alongside selected account

### 4. Message Screen Navigation

Update the existing messages screen action bar:

**Current:**
```
{SMS/iMessage history displayed}

[s]end [enter] back:
```

**New:**
```
{SMS/iMessage history displayed}

[t]ext [@]email [enter] back:
```

- [t] → existing SMS/iMessage compose flow (renamed from [s]end)
- [@] → new email compose flow (only shown if contact has email address)
- [enter] → back to contact card

**Email Compose Screen (when [@] pressed):**
```
To: contact@example.com (from contact card)
From: youraddress@gmail.com (from setup config)
Subject: [text input]
────────────────────────
[multi-line editable text box]

--
Your saved signature here
(pre-inserted, user can edit)

[enter] Send | [esc] Cancel
```

The body text box is pre-populated with the signature at the bottom. User types their message above it and can modify/delete the signature if needed for this particular email.

### 5. Send Email Flow

When user sends:

- App calls AppleScript to create outgoing message in Mail.app
- Sets sender to configured account
- Sets recipient to contact's email
- Appends signature to body
- Mail.app handles authentication and delivery
- User sees confirmation or error

## Success Criteria

- [x] Main menu has [s]etup option
- [x] Setup screen lists external services with connection status
- [x] User can select email account from Mail.app configured accounts
- [x] User can set a multi-line text signature for outbound email
- [x] Configuration stored in SQLite `app_settings` table
- [x] Configuration persists between sessions
- [x] Messages screen action bar shows `[t]ext [@]email [enter] back:`
- [x] [t] triggers existing SMS/iMessage compose flow
- [x] [@] only appears when contact has email address
- [x] [@] opens email compose screen with To, From, Subject, Body
- [x] User can type multi-line email body
- [x] Signature pre-inserted in body text box (editable by user)
- [x] Send executes via Mail.app AppleScript
- [x] User sees send confirmation or error message

## Notes

- Keybinding: [@] for email (avoids conflict with [e]dit)
- Keybinding: [t] for text replaces [s]end on messages screen
- Mail.app must be configured with at least one account for email to work
- If no Mail.app accounts found, show helpful message in Setup
- Signature is optional but encouraged
- Subject can have a default (configurable in Setup) or be entered per-email
- Config storage: SQLite `app_settings` table with key/value pairs
- If contact has multiple emails, let user pick which one to send to

**Key files to modify:**
- `src/cli/menu.rs` - add [s]etup to main menu
- `src/cli/show.rs` - update messages screen action bar, add email compose
- `src/db/schema.rs` - add `app_settings` table migration
- New: `src/cli/setup.rs` - setup screen for external services
- New: `src/cli/email.rs` - email compose and send via AppleScript

<!--
AI TASK CREATION GUIDE

Write as you'd explain to a colleague:
- Problem: describe what needs solving and why
- Success criteria: "User can [do what]" or "App shows [result]"
- Notes: dependencies, links, edge cases

Patterns that work well:
  Filename:    120-add-login-button.md (ID + kebab-case description)
  Title:       # Task 120: Add login button (matches filename ID)
  Feature:     **Feature**: /docs/features/auth.md (or "none" or "multiple")
  Created:     **Created**: 2026-01-28 (YYYY-MM-DD format)
  Depends on:  **Depends on**: Task 42 (or "none")
  Blocks:      **Blocks**: Task 101 (or "none")

Success criteria that verify easily:
  - [ ] User can reset password via email
  - [ ] Dashboard shows total for selected date range
  - [ ] Search returns results within 500ms

Get next ID: docs/STATE.md (5DAY_TASK_ID field + 1)
Full protocol: docs/5day/ai/task-creation.md
-->
