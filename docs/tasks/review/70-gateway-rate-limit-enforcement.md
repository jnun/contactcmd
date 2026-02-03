# Task 70: Gateway rate limit enforcement

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Completed**: 2026-02-03
**Depends on**: Task 69
**Blocks**: none

## Problem

Once rate limit columns exist in the database, the gateway server needs to check them before accepting new messages. Without enforcement, the schema is useless.

When an agent exceeds their rate limit, the API should return a clear error with retry-after information.

## Success criteria

- [x] POST `/gateway/send` returns HTTP 429 when hourly limit exceeded
- [x] POST `/gateway/send` returns HTTP 429 when daily limit exceeded
- [x] Response includes `retry_after_seconds` in JSON body
- [x] Rate check happens before message is queued (not after)
- [x] Legitimate requests within limits still succeed

## Notes

- Check both hourly and daily limits
- Use `created_at` timestamps from `communication_queue` to count recent messages
- Error response format: `{"error": "rate_limit_exceeded", "retry_after_seconds": 3600}`

## Implementation

- Added `RateLimitErrorResponse` struct in `src/cli/gateway/types.rs`
- Added HTTP 429 "Too Many Requests" status handling in `send_json_response()`
- Added rate limit checking in `handle_send()` after authentication, before parsing
- Uses `count_queue_since()` from Task 69 to count messages in time windows
- Response includes: error, retry_after_seconds, limit_type, current_count, limit

## Test Results

```
=== Request 1 (within limit) ===
{"success":true,"data":{"action_id":"...","status":"pending"}}
HTTP Status: 200

=== Request 4 (over limit) ===
{"error":"rate_limit_exceeded","retry_after_seconds":3600,"limit_type":"hourly","current_count":3,"limit":2}
HTTP Status: 429
```
