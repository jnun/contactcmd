# Task 44: Tab through every contact with a phone number to send each one a message

**Feature**: none
**Created**: 2026-01-29
**Depends on**: Task 45
**Blocks**: none

## Problem

Users want to efficiently send messages (SMS or email) to multiple contacts in sequence. The current browse function allows navigating through contacts with left/right arrow keys, but there's no streamlined workflow for sending a message to each contact as you browse through them. Users need to be able to tab through contacts and quickly send personalized messages via SMS or email without returning to the main menu between each contact.

## Success criteria

- [ ] User can browse contacts using left/right arrow keys (existing functionality)
- [ ] User can initiate message composition from browse view for current contact
- [ ] User can choose between SMS and email when messaging from browse view
- [ ] After sending a message, user can continue to next contact with arrow key
- [ ] Browse filters contacts to only those with phone numbers (for SMS workflow)
- [ ] Browse filters contacts to only those with email addresses (for email workflow)
- [ ] User can send messages to multiple contacts in a single browsing session

## Notes

- Depends on Task 45 (email service connector) being completed first
- Should integrate with existing browse functionality in the menu system
- Consider adding a "bulk message mode" indicator in the UI
- May want separate browse modes: "all contacts", "contacts with phone", "contacts with email"
- Related to Task 41 (browse option for all contacts) and Task 43 (send text messages)

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
