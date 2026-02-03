# Task 52: Sync moltbot to cmd

**Feature**: none
**Created**: 2026-01-29
**Depends on**: none
**Blocks**: none

## Problem

I want to use Moltbot's AI gateway for processing messages from multiple channels
(WhatsApp, Signal, Telegram, etc.) while keeping iMessage handling on my trusted
Mac via contactcmd. The challenge is connecting these systems safely without
exposing my Apple credentials or giving the AI access to my local filesystem.
Moltbot runs isolated in Docker; contactcmd runs natively on macOS with iMessage
access.

## Success criteria

- [ ] Moltbot runs in Docker with WhatsApp/Signal/Telegram connected
- [ ] contactcmd bridges iMessage to Moltbot via HTTP API
- [ ] Credentials never leave their respective systems (iMessage keys stay on Mac)
- [ ] Messages can be blocked/filtered before reaching Moltbot
- [ ] Connection can be severed instantly (kill switch)
- [ ] All bridge traffic is logged for audit

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  macOS (trusted)                                                        │
│                                                                         │
│  ┌─────────────────────┐                                                │
│  │  contactcmd         │                                                │
│  │  - iMessage access  │                                                │
│  │  - Credential store │◄── Credentials NEVER leave this box            │
│  │  - Message filter   │                                                │
│  │  - Audit log        │                                                │
│  └──────────┬──────────┘                                                │
│             │ localhost:9800 (bridge API)                               │
│             ▼                                                           │
│  ┌─────────────────────┐         ┌─────────────────────────────────┐   │
│  │  Bridge Process     │◄───────►│  Docker: Moltbot Gateway        │   │
│  │  (Rust)             │  :9801  │  - AI processing                │   │
│  │  - mTLS handshake   │         │  - WhatsApp (Baileys)           │   │
│  │  - Rate limiting    │         │  - Signal (signal-cli)          │   │
│  │  - Kill switch      │         │  - Telegram, Discord, etc.      │   │
│  └─────────────────────┘         │  - NO filesystem access         │   │
│                                  │  - NO credential storage        │   │
│                                  └─────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## Security Model

### Principle: Message-only bridge

The bridge passes **messages only** - never credentials, tokens, or keys.

| Data type          | Allowed to cross bridge? |
|--------------------|--------------------------|
| Message text       | Yes                      |
| Sender ID (phone)  | Yes (pseudonymized opt.) |
| Timestamps         | Yes                      |
| Media (photos/etc) | Yes (sanitized)          |
| iMessage auth      | NO - never               |
| Apple ID tokens    | NO - never               |
| Keychain data      | NO - never               |
| Local file paths   | NO - never               |

### Safeguards

1. **Credential isolation**
   - iMessage keys live only in macOS Keychain
   - contactcmd never serializes or exports credentials
   - Moltbot has zero access to Keychain or Apple APIs

2. **Network isolation**
   - Bridge listens on localhost only (or Tailscale for remote)
   - Docker container has no host network access
   - mTLS with pre-shared certs for authentication

3. **Message filtering (contactcmd side)**
   - Allowlist/blocklist by sender
   - Regex filters for sensitive patterns (SSN, credit card, etc.)
   - Rate limiting per sender
   - Max message size cap

4. **Kill switch**
   - Single command to sever bridge: `contactcmd bridge disconnect`
   - Auto-disconnect on anomaly detection (unusual volume, etc.)
   - Bridge process can be killed without affecting either system

5. **Audit trail**
   - All messages logged locally before forwarding
   - Log includes: timestamp, direction, sender, message hash
   - Logs stored on Mac only (not in Docker)

6. **Moltbot sandboxing**
   - Docker container runs as non-root
   - No volume mounts to host filesystem (except config)
   - Read-only config mount where possible
   - Network egress limited to known APIs

## API Design

### contactcmd -> Moltbot (inbound message)

```
POST http://localhost:9801/bridge/inbound
X-Bridge-Token: <rotating-token>
X-Bridge-Timestamp: <unix-ms>
X-Bridge-Signature: <hmac-sha256>

{
  "channel": "imessage",
  "sender": "+1234567890",
  "conversation_id": "abc123",
  "content": "Hello from iMessage",
  "media": [],  // optional base64 attachments
  "timestamp": 1706500000000
}
```

### Moltbot -> contactcmd (outbound reply)

```
POST http://localhost:9800/bridge/outbound
X-Bridge-Token: <rotating-token>
X-Bridge-Timestamp: <unix-ms>
X-Bridge-Signature: <hmac-sha256>

{
  "channel": "imessage",
  "recipient": "+1234567890",
  "conversation_id": "abc123",
  "content": "Reply from AI",
  "media": []
}
```

### Handshake (startup)

```
POST http://localhost:9801/bridge/handshake
{
  "bridge_id": "contactcmd-macbook-pro",
  "supported_channels": ["imessage"],
  "public_key": "<ed25519-pubkey>",
  "capabilities": ["text", "media", "reactions"]
}

Response:
{
  "session_id": "sess_abc123",
  "moltbot_public_key": "<ed25519-pubkey>",
  "token_rotation_interval": 3600
}
```

## Implementation Phases

### Phase 1: Docker Moltbot (no bridge yet)
- [ ] Build Docker image from moltbot repo
- [ ] Configure WhatsApp via QR code
- [ ] Verify headless operation
- [ ] Test message send/receive via WhatsApp

### Phase 2: Bridge skeleton (Rust)
- [ ] Create contactcmd bridge module
- [ ] Implement HTTP server on :9800
- [ ] Implement HTTP client for :9801
- [ ] Add mTLS or HMAC signing
- [ ] Add kill switch command

### Phase 3: Moltbot extension
- [ ] Create Moltbot extension for bridge channel
- [ ] Register bridge as custom channel type
- [ ] Handle inbound webhook at /bridge/inbound
- [ ] Send outbound via HTTP to contactcmd

### Phase 4: iMessage integration
- [ ] Wire contactcmd iMessage monitor to bridge
- [ ] Wire bridge outbound to contactcmd send
- [ ] Test end-to-end: iMessage -> Moltbot -> iMessage

### Phase 5: Hardening
- [ ] Add message filtering rules
- [ ] Add rate limiting
- [ ] Add anomaly detection
- [ ] Add audit logging
- [ ] Security review

## Notes

- Moltbot repo: https://github.com/moltbot/moltbot
- Moltbot supports custom channel extensions in `extensions/` directory
- WhatsApp uses Baileys (WebSocket) - no browser needed
- Signal uses signal-cli (Java) - can run in same or separate container
- Consider Tailscale for secure remote bridge if Mac isn't always on same network

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
