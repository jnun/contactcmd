# AI Agent Gateway: Protecting Contacts from Automated Messaging

**Status:** MVP Complete, Hardening Needed
**Last Updated:** 2026-02-02

## Problem Statement

AI agents (N8N, OpenClaw, etc.) with access to contact databases can impersonate users by sending messages to trusted contacts. Even well-intentioned agents may:
- Send messages phrased inappropriately
- Contact people at wrong times
- Misunderstand context
- Leak sensitive information

The core issue: **AI retains PII access but should not have unsupervised send capability**.

## What We Built

### Communication Gateway (Implemented)

A human-in-the-loop approval system for AI-initiated messages.

**Architecture:**
```
AI Agent ──► Gateway Server ──► Approval Queue ──► Human Review ──► Send
   │              │                                     │
   │              │ returns action_id                   │
   │              │                                     │
   └──────────────┴── poll status ◄─────────────────────┘
```

**Components:**
- `src/db/gateway.rs` - Queue and API key storage
- `src/cli/gateway/server.rs` - HTTP API for agents
- `src/cli/gateway/approve.rs` - TUI for human review
- `src/cli/gateway/execute.rs` - Send via Gmail/Messages.app
- `src/cli/gateway/keys.rs` - API key management

**API Endpoints:**
| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| POST | `/gateway/send` | API Key | Queue message |
| GET | `/gateway/actions/{id}` | API Key | Poll status |
| GET | `/gateway/health` | None | Health check |
| GET | `/gateway/queue` | Local only | List pending |
| POST | `/gateway/queue/{id}/approve` | Local only | Approve + send |
| POST | `/gateway/queue/{id}/deny` | Local only | Deny |

**CLI Commands:**
```bash
contactcmd gateway start [--port 9810]
contactcmd gateway stop
contactcmd gateway status
contactcmd gateway approve          # TUI in main menu
contactcmd gateway keys add <name>
contactcmd gateway keys list
contactcmd gateway keys revoke <id>
```

**OpenClaw Plugin:**
- `extensions/contactcmd-gateway/` - Registers `contactcmd_gateway` tool
- Agents can queue messages and poll status
- Config: `gatewayUrl` + `apiKey` in `~/.openclaw/config.json`

## What This Protects Against

| Threat | Mitigation |
|--------|------------|
| AI impersonation | Human verifies every message |
| Spam/harassment | Human can deny |
| Wrong recipient | Human sees recipient before send |
| Poor phrasing | Human can deny, rewrite manually |
| Accidental sends | Nothing sends without approval |

## Gaps: PII Still Exposed

The gateway only gates **outbound messages**. Agents still have full access to:
- Contact names, emails, phones
- Addresses, organizations
- Notes, interaction history
- Message history (iMessage search)

An agent could:
1. Exfiltrate PII through other channels (logs, webhooks, other tools)
2. Use PII to craft convincing phishing attempts (even if queued)
3. Correlate contacts with external data

## Future Improvements Needed

### 1. Rate Limiting
```
Per API key:
- Max messages per hour: 10
- Max messages per day: 50
- Cooldown after denial: 1 hour
```
Prevents queue flooding.

### 2. Recipient Allowlists
```
API key "N8N Agent" can only message:
- john@example.com
- +15551234567
```
Limits blast radius of compromised keys.

### 3. Content Filtering
```
Auto-deny if body contains:
- SSN patterns
- Credit card numbers
- "password" or "credential"
```
Prevents accidental PII leakage in messages.

### 4. PII Access Tiers

Instead of full database access, provide tiered APIs:

**Tier 1: Aggregate Only**
```
"You have 3 contacts in San Francisco"
"John's birthday is in 2 weeks"
```
No raw PII returned.

**Tier 2: Masked Access**
```
"John D." / "j***@example.com" / "+1***567"
```
Enough to reference, not enough to exfiltrate.

**Tier 3: Full Access (Audited)**
```
Full PII with logging:
- What was accessed
- When
- By which agent
- For what stated purpose
```

### 5. Audit Log UI
```
contactcmd gateway history
```
Shows:
- All messages sent (approved)
- All messages denied
- Which agent, when, to whom

### 6. Webhook Callbacks
```json
{
  "action_id": "...",
  "status": "approved",
  "sent_at": "..."
}
```
Agent doesn't need to poll; gets notified.

### 7. Contact Consent Flags
```sql
ALTER TABLE persons ADD COLUMN ai_contact_allowed INTEGER DEFAULT 0;
```
Per-contact opt-in for AI-initiated messages.

### 8. Read Access Gateway

Similar queue for **reading** sensitive fields:
```
Agent: "I need John's phone number to send reminder"
Human: [approve] / [deny] / [provide masked]
```

## Implementation Priority

| # | Feature | Effort | Impact | Status |
|---|---------|--------|--------|--------|
| 0 | Core gateway (queue/approve/send) | High | High | **DONE** |
| 1 | Rate limiting | Low | High | Not started |
| 2 | Audit log UI | Low | High | Not started |
| 3 | Recipient allowlists | Medium | High | Not started |
| 4 | Content filtering | Medium | Medium | Not started |
| 5 | PII access tiers | High | Highest | Not started |

## Related Files

- `src/db/schema.rs` - MIGRATION_V7 (gateway tables)
- `src/cli/gateway/` - All gateway code
- `src/cli/menu.rs` - Gateway in main menu
- `extensions/contactcmd-gateway/` - OpenClaw plugin (in devops repo)
