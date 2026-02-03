# Task 68: Learn Something Feature - Progressive feature discovery system with tutorials

**Feature**: none
**Created**: 2026-02-03
**Depends on**: none
**Blocks**: none

## Problem

This app has many features that grow constantly. Users toggle between DOS-style UI and CLI modes, but there's no structured way for them to discover and learn all available functionality. New users may miss powerful features, and existing users forget about capabilities they don't use often. We need a progressive learning system that introduces features one at a time and reinforces them through spaced repetition.

## Success criteria

### Database & Schema
- [x] `learn_something` table with: `id` (UUID PK), `feature_name`, `category`, `tutorial_json` (JSON blob), `times_learned` (int, default 0)
- [x] `tutorial_json` structure: `{ "title": "", "summary": "", "steps": [], "tips": [], "related_features": [] }`
- [x] Single table design - no separate progress table needed (single-user app)
- [x] Seed data built from `docs/guides/` content

### Chat UI Suggestion ("Clippy-lite")
- [x] When chat screen loads, show subtle suggestion: "Learn something new?" (or similar)
- [x] User can dismiss or engage - non-intrusive, not modal
- [x] If engaged, system finds feature with lowest `times_learned` value
- [x] App presents the tutorial from `tutorial_json`
- [x] After viewing, `times_learned` increments by 1

### Refresher Mode
- [x] After all features learned once (all `times_learned` >= 1), system switches to refresher messaging
- [x] Still picks lowest `times_learned` to ensure even coverage over time
- [x] Suggestion changes to "Refresh your memory on a feature?" (or similar)

### AI Tutorial Integration
- [x] User can type `/teach <topic>` to get help on a specific feature
- [x] System looks up matching feature by name/category from `learn_something` table
- [x] System displays `tutorial_json` content in formatted output
- [ ] AI conversational follow-up (future: integrate with AI chat for natural language queries)

### Feature Catalog & Maintenance
- [x] Initial seed data covers major features: sync, search, messages, photos, import, contacts, UI modes
- [x] `docs/guides/` serves as source of truth - when guides update, `tutorial_json` should be updated
- [ ] Future: sync server can push new/updated tutorials to `learn_something` table

## Notes

- **Single-user assumption**: This is a native app, one user per installation. No user_id tracking needed.
- **JSON in database**: Tutorials travel with the app. Sync server can update them dynamically later.
- **docs/guides as source**: Developers maintain `docs/guides/*.md`. These inform the JSON tutorials.
- **Spaced repetition**: `times_learned` counter naturally prioritizes less-reviewed features.
- **Future enhancements**:
  - `last_learned_at` timestamp for time-based refresh logic
  - Category-based learning ("teach me about messaging features")
  - Progress indicator ("You've learned 12 of 20 features")

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
