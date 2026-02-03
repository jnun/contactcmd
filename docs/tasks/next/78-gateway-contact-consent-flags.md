# Task 78: Gateway contact consent flags

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Depends on**: none
**Blocks**: none

## Problem

Even with human approval, some contacts might never want to receive AI-initiated messages. A per-contact consent flag lets users mark contacts as "off limits" to AI agents entirely, providing an additional layer of protection.

## Success criteria

- [ ] `persons` table has `ai_contact_allowed` column (INTEGER, default 1 = allowed)
- [ ] POST `/gateway/send` returns HTTP 403 if recipient matches a contact with ai_contact_allowed=0
- [ ] Error response indicates contact has opted out
- [ ] Contact edit TUI shows toggle for "Allow AI contact"
- [ ] `contactcmd show <person>` displays AI contact status

## Notes

- Reference: docs/ideas/ai-agent-gateway.md lines 160-165
- Default to allowed (1) for backward compatibility
- Must match recipient address to contacts (fuzzy match on phone/email)
- Consider: should flagged contacts still appear in queue for manual override?
