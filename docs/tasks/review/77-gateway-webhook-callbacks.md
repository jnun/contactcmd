# Task 77: Gateway webhook callbacks

**Feature**: docs/features/gateway.md
**Created**: 2026-02-03
**Depends on**: none
**Blocks**: none

## Problem

Currently agents must poll `/gateway/actions/{id}` to check message status. This is inefficient and adds latency. Webhooks allow agents to be notified immediately when status changes, closing the feedback loop faster.

## Success criteria

- [x] `api_keys` table has `webhook_url` column (TEXT, nullable)
- [x] When message status changes to sent/denied/failed, POST to webhook URL
- [x] Webhook payload includes: action_id, status, sent_at (if sent), error_message (if failed)
- [x] Webhook failures are logged but don't block status update
- [x] `contactcmd gateway keys webhook <key-id> <url>` sets webhook URL
- [x] `contactcmd gateway keys webhook <key-id> --remove` clears webhook

## Implementation

**Schema** (`src/db/schema.rs`):
- MIGRATION_V13 adds `webhook_url` column to `api_keys` table
- SCHEMA_VERSION bumped to 13

**Database** (`src/db/gateway.rs`):
- Added `webhook_url` field to `ApiKey` struct
- `set_api_key_webhook()` - set or clear webhook URL
- `get_api_key_webhook()` - get webhook URL for an API key
- Updated all ApiKey queries to include webhook_url

**Webhook module** (`src/cli/gateway/webhook.rs`):
- `WebhookPayload` struct with: action_id, status, sent_at, error_message, recipient, channel
- `WebhookResult` enum: Delivered, NoWebhook, Failed
- `notify_status_change()` - sends webhook notification (non-blocking for errors)
- HTTP POST with 10-second timeout, Content-Type: application/json

**Integration**:
- `approve.rs` - calls webhook on sent/denied/failed from TUI
- `server.rs` - calls webhook from HTTP approve/deny endpoints

**CLI** (`src/cli/gateway/mod.rs`):
- `gateway keys webhook <key-id>` - show current webhook
- `gateway keys webhook <key-id> <url>` - set webhook URL
- `gateway keys webhook <key-id> --remove` - clear webhook
- `gateway keys list` - shows webhook URL if configured

**Tests**: 5 new tests for webhook functionality

## Notes

- Webhook payload format:
  ```json
  {"action_id": "...", "status": "sent", "sent_at": "...", "recipient": "...", "channel": "..."}
  ```
- Consider retry logic with exponential backoff in future
- Consider HMAC signature for webhook authenticity
