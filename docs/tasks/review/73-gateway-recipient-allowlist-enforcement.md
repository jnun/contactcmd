# Task 73: Gateway recipient allowlist enforcement

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: Task 72
**Blocks**: none

## Problem

Once allowlist table exists, the gateway server must check it before accepting messages. If an API key has allowlist entries, only matching recipients should be allowed. Keys without allowlist entries remain unrestricted (backward compatible).

## Success criteria

- [x] POST `/gateway/send` returns HTTP 403 if recipient not in allowlist
- [x] Error response includes which patterns are allowed
- [x] Wildcard patterns (`*@domain.com`) match correctly
- [x] Keys with empty allowlist can still message anyone
- [x] Phone number matching normalizes formats (+1, spaces, dashes)

## Notes

- Check allowlist after authentication, before rate limiting
- Error format: `{"error": "recipient_not_allowed", "allowed_patterns": ["*@acme.com"]}`
- Consider case-insensitive email matching

## Implementation

- Added `AllowlistErrorResponse` struct in `src/cli/gateway/types.rs`
- Added allowlist enforcement in `handle_send()` in `src/cli/gateway/server.rs`:
  - Check happens after request parsing (need recipient address)
  - Only applies if key has allowlist entries (empty = unrestricted)
  - Returns HTTP 403 with list of allowed patterns on rejection
- Added pattern matching functions:
  - `recipient_matches_allowlist()` - main entry point
  - `normalize_recipient()` - lowercase emails, strip formatting from phones
  - `is_phone_number()` - detect phone vs email
  - `normalize_phone()` - remove spaces, dashes, parentheses
  - `pattern_matches()` - exact match + wildcard domain support
- Pattern matching supports:
  - Exact email match (case-insensitive)
  - Wildcard domain: `*@domain.com` matches any email at that domain
  - Phone normalization: `+1 (555) 123-4567` matches `+15551234567`
- Added 5 unit tests covering all matching scenarios
