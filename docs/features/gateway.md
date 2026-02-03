# Feature: Communication Gateway

**Status:** COMPLETE (MVP)
**Created:** 2026-02-02
**Updated:** 2026-02-02

## Overview

HTTP gateway that allows external AI agents to queue messages (email, SMS, iMessage) for human approval before sending. Prevents AI impersonation to trusted contacts.

## User Stories

- As a user, I want AI agents to request permission before messaging my contacts
- As a user, I want to review queued messages before they send
- As a user, I want to approve or deny messages from a simple TUI
- As a user, I want to manage API keys for different agents

## Requirements

### Functional Requirements (Completed)

- [x] HTTP server on configurable port (default 9810)
- [x] API key authentication for agents
- [x] Queue messages with channel, recipient, body, priority
- [x] Pending/approved/denied/sent/failed status tracking
- [x] TUI for reviewing and approving messages
- [x] Send via Gmail API (email) or AppleScript (SMS/iMessage)
- [x] CLI commands: start, stop, status, approve, keys

### Functional Requirements (Future)

- [ ] Rate limiting per API key
- [ ] Recipient allowlists per key
- [ ] Content filtering (auto-deny patterns)
- [ ] Audit log UI
- [ ] Webhook callbacks on status change

### Non-Functional Requirements

- [x] Local-only endpoints for approve/deny (security)
- [x] API keys hashed with SHA-256 (never stored plaintext)
- [ ] Sub-100ms queue operations

## Technical Design

### Database Schema (V7)

```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    key_prefix TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
);

CREATE TABLE communication_queue (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    channel TEXT NOT NULL,           -- sms, imessage, email
    recipient_address TEXT NOT NULL,
    recipient_name TEXT,
    subject TEXT,
    body TEXT NOT NULL,
    priority TEXT DEFAULT 'normal',
    status TEXT DEFAULT 'pending',
    agent_context TEXT,              -- JSON
    created_at TEXT NOT NULL,
    reviewed_at TEXT,
    sent_at TEXT,
    error_message TEXT,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id)
);
```

### API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/gateway/send` | API Key | Queue a message |
| GET | `/gateway/actions/{id}` | API Key | Poll status |
| GET | `/gateway/health` | None | Health check |
| GET | `/gateway/queue` | Local | List pending |
| POST | `/gateway/queue/{id}/approve` | Local | Approve + send |
| POST | `/gateway/queue/{id}/deny` | Local | Deny |

### CLI Commands

```bash
contactcmd gateway start [--port 9810] [--foreground]
contactcmd gateway stop
contactcmd gateway status
contactcmd gateway approve        # Also in main menu as "Gateway"

contactcmd gateway keys add <name>
contactcmd gateway keys list
contactcmd gateway keys revoke <id>
```

## Implementation

### Files

- `src/db/schema.rs` - MIGRATION_V7
- `src/db/gateway.rs` - Queue and key CRUD
- `src/cli/gateway/mod.rs` - CLI dispatch
- `src/cli/gateway/server.rs` - HTTP server
- `src/cli/gateway/types.rs` - Request/response types
- `src/cli/gateway/keys.rs` - Key generation/validation
- `src/cli/gateway/approve.rs` - TUI
- `src/cli/gateway/execute.rs` - Send logic
- `src/cli/menu.rs` - "Gateway" menu option

### OpenClaw Integration

Plugin at `~/Projects/devops/experiments/openclaw/extensions/contactcmd-gateway/`:
- Registers `contactcmd_gateway` tool
- Config: `gatewayUrl` + `apiKey`

## Acceptance Criteria

- [x] `contactcmd gateway start` starts HTTP server
- [x] Agent can POST to `/gateway/send` with valid API key
- [x] Agent can poll `/gateway/actions/{id}` for status
- [x] `contactcmd gateway approve` shows pending messages
- [x] Approving a message sends it (email/SMS/iMessage)
- [x] Denying a message marks it denied
- [x] Main menu shows "Gateway (N)" with pending count
- [x] API keys can be created, listed, revoked

## Security Considerations

- API keys are SHA-256 hashed before storage
- Approve/deny endpoints only accept connections from 127.0.0.1
- Revoked keys are rejected immediately
- Agent context is logged but not trusted

## Known Limitations

- No rate limiting (agent can flood queue)
- No recipient restrictions (any address allowed)
- PII read access is not gated (only sends are gated)
- No audit log UI yet
