# Agent Gateway Integration Guide

How to build an AI agent server that sends messages through the contactcmd gateway.

## Overview

The contactcmd gateway provides human-in-the-loop approval for AI-initiated communications. Your agent queues messages via HTTP API, the user reviews them in a TUI, and approved messages are sent via email/SMS/iMessage.

```
Your Agent ──► POST /gateway/send ──► Queue ──► User Review ──► Send
                     │                              │
                     └── GET /actions/{id} ◄────────┘
```

## Setup

### 1. Start the Gateway Server

```bash
# Start gateway (default port 9810)
contactcmd gateway start

# Or with custom port
contactcmd gateway start --port 8080

# Run in foreground (see logs)
contactcmd gateway start --foreground
```

### 2. Create an API Key

```bash
contactcmd gateway keys add "openclaw-agent"
# Output: gw_abc123...xyz (save this - shown only once)
```

### 3. Configure Your Agent

Store the API key securely (environment variable, secrets manager, etc.):

```bash
export GATEWAY_API_KEY="gw_abc123..."
export GATEWAY_URL="http://localhost:9810"
```

## API Reference

### Authentication

All agent endpoints require the `X-Gateway-Key` header:

```
X-Gateway-Key: gw_abc123...
```

### Send a Message

**POST /gateway/send**

Queue a message for approval.

```json
{
  "channel": "email",
  "recipient_address": "alice@example.com",
  "recipient_name": "Alice Smith",
  "subject": "Meeting Follow-up",
  "body": "Hi Alice,\n\nThanks for meeting today...",
  "priority": "normal",
  "context": {
    "reason": "calendar follow-up",
    "meeting_id": "mtg_123"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `channel` | string | yes | `email`, `sms`, or `imessage` |
| `recipient_address` | string | yes | Email address or phone number |
| `recipient_name` | string | no | Display name for recipient |
| `subject` | string | email only | Required for email channel |
| `body` | string | yes | Message content |
| `priority` | string | no | `urgent`, `high`, `normal` (default), `low` |
| `context` | object | no | Metadata for audit trail (not sent to recipient) |

**Success Response (200)**

```json
{
  "success": true,
  "data": {
    "action_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "pending"
  }
}
```

### Check Message Status

**GET /gateway/actions/{action_id}**

Poll for status updates.

```json
{
  "success": true,
  "data": {
    "action_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "sent",
    "sent_at": "2026-02-03T15:30:00Z"
  }
}
```

| Status | Meaning |
|--------|---------|
| `pending` | Awaiting user review |
| `flagged` | Content filter triggered, needs review |
| `approved` | User approved, sending in progress |
| `sent` | Successfully delivered |
| `denied` | User rejected the message |
| `failed` | Send attempted but failed (see `error_message`) |

### Health Check

**GET /gateway/health**

No authentication required.

```json
{
  "success": true,
  "data": {
    "status": "ok",
    "uptime_secs": 3600,
    "pending_count": 2,
    "version": "1.0"
  }
}
```

## Error Handling

The gateway returns structured errors. Your agent should handle these gracefully.

### 401 Unauthorized

Invalid or missing API key.

```json
{
  "success": false,
  "error": "Invalid API key"
}
```

### 403 Forbidden - Recipient Not Allowed

The API key has an allowlist and this recipient isn't on it.

```json
{
  "error": "recipient_not_allowed",
  "allowed_patterns": ["*@company.com", "+1555*"]
}
```

### 403 Forbidden - Contact Consent Denied

The recipient is a known contact who has opted out of AI communication.

```json
{
  "error": "contact_consent_denied",
  "recipient": "alice@example.com"
}
```

**Important:** When you receive this error, do NOT retry. The user has explicitly marked this contact as off-limits to AI agents. Log the refusal and inform the user that the contact cannot be reached via AI.

### 400 Bad Request - Content Blocked

Message content matched a safety filter (e.g., credit card numbers, SSN patterns).

```json
{
  "error": "content_blocked",
  "filter": "credit_card_number",
  "description": "Detected credit card number pattern"
}
```

### 429 Too Many Requests

Rate limit exceeded.

```json
{
  "error": "rate_limit_exceeded",
  "retry_after_seconds": 3600,
  "limit_type": "hourly",
  "current_count": 10,
  "limit": 10
}
```

## Example Implementation (Python)

```python
import os
import time
import requests

GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:9810")
API_KEY = os.environ["GATEWAY_API_KEY"]

def send_message(channel, recipient, body, subject=None, context=None):
    """Queue a message for approval. Returns action_id or raises."""

    payload = {
        "channel": channel,
        "recipient_address": recipient,
        "body": body,
    }
    if subject:
        payload["subject"] = subject
    if context:
        payload["context"] = context

    resp = requests.post(
        f"{GATEWAY_URL}/gateway/send",
        json=payload,
        headers={"X-Gateway-Key": API_KEY},
    )

    if resp.status_code == 200:
        data = resp.json()
        return data["data"]["action_id"]

    # Handle specific errors
    if resp.status_code == 403:
        error_data = resp.json()
        if error_data.get("error") == "contact_consent_denied":
            raise ContactConsentError(f"Contact {recipient} has opted out of AI contact")
        if error_data.get("error") == "recipient_not_allowed":
            raise RecipientNotAllowedError(f"Recipient not in allowlist")

    if resp.status_code == 400:
        error_data = resp.json()
        if error_data.get("error") == "content_blocked":
            raise ContentBlockedError(f"Content blocked: {error_data.get('filter')}")

    if resp.status_code == 429:
        error_data = resp.json()
        raise RateLimitError(f"Rate limited, retry after {error_data['retry_after_seconds']}s")

    resp.raise_for_status()


def poll_status(action_id, timeout=300, interval=5):
    """Poll until terminal status or timeout."""

    terminal = {"sent", "denied", "failed"}
    start = time.time()

    while time.time() - start < timeout:
        resp = requests.get(
            f"{GATEWAY_URL}/gateway/actions/{action_id}",
            headers={"X-Gateway-Key": API_KEY},
        )
        resp.raise_for_status()

        status = resp.json()["data"]["status"]
        if status in terminal:
            return resp.json()["data"]

        time.sleep(interval)

    raise TimeoutError(f"Message {action_id} still pending after {timeout}s")


# Custom exceptions
class ContactConsentError(Exception):
    """Contact has opted out of AI communication."""
    pass

class RecipientNotAllowedError(Exception):
    """Recipient not in API key's allowlist."""
    pass

class ContentBlockedError(Exception):
    """Message content blocked by safety filter."""
    pass

class RateLimitError(Exception):
    """Rate limit exceeded."""
    pass
```

## Example Implementation (TypeScript)

```typescript
const GATEWAY_URL = process.env.GATEWAY_URL || "http://localhost:9810";
const API_KEY = process.env.GATEWAY_API_KEY!;

interface SendResult {
  action_id: string;
  status: string;
}

interface StatusResult {
  action_id: string;
  status: "pending" | "flagged" | "approved" | "sent" | "denied" | "failed";
  error_message?: string;
  sent_at?: string;
}

async function sendMessage(
  channel: "email" | "sms" | "imessage",
  recipient: string,
  body: string,
  options?: { subject?: string; context?: Record<string, unknown> }
): Promise<string> {
  const resp = await fetch(`${GATEWAY_URL}/gateway/send`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Gateway-Key": API_KEY,
    },
    body: JSON.stringify({
      channel,
      recipient_address: recipient,
      body,
      subject: options?.subject,
      context: options?.context,
    }),
  });

  const data = await resp.json();

  if (resp.status === 403) {
    if (data.error === "contact_consent_denied") {
      throw new Error(`Contact ${recipient} has opted out of AI contact`);
    }
    if (data.error === "recipient_not_allowed") {
      throw new Error("Recipient not in allowlist");
    }
  }

  if (resp.status === 400 && data.error === "content_blocked") {
    throw new Error(`Content blocked by filter: ${data.filter}`);
  }

  if (resp.status === 429) {
    throw new Error(`Rate limited, retry after ${data.retry_after_seconds}s`);
  }

  if (!resp.ok) {
    throw new Error(data.error || "Unknown error");
  }

  return data.data.action_id;
}

async function pollStatus(
  actionId: string,
  timeoutMs = 300000,
  intervalMs = 5000
): Promise<StatusResult> {
  const terminal = new Set(["sent", "denied", "failed"]);
  const start = Date.now();

  while (Date.now() - start < timeoutMs) {
    const resp = await fetch(`${GATEWAY_URL}/gateway/actions/${actionId}`, {
      headers: { "X-Gateway-Key": API_KEY },
    });

    const data = await resp.json();
    if (terminal.has(data.data.status)) {
      return data.data;
    }

    await new Promise((r) => setTimeout(r, intervalMs));
  }

  throw new Error(`Timeout waiting for action ${actionId}`);
}
```

## Best Practices

### 1. Handle Consent Denial Gracefully

When you receive `contact_consent_denied`, inform the user clearly:

> "I can't send that message - Alice has marked herself as off-limits to AI contact. You'll need to reach out to her directly."

Never retry or attempt to work around this restriction.

### 2. Provide Context

Include meaningful `context` so the user understands why the message is being sent:

```json
{
  "context": {
    "reason": "follow-up from yesterday's meeting",
    "triggered_by": "calendar reminder",
    "agent": "scheduling-assistant"
  }
}
```

### 3. Use Appropriate Priority

- `urgent` - Time-sensitive, needs immediate attention
- `high` - Important but not time-critical
- `normal` - Default, standard messages
- `low` - Batch/bulk, can wait

### 4. Poll with Backoff

Don't poll aggressively. Start at 5s intervals, back off to 30s for long waits:

```python
intervals = [5, 5, 10, 10, 30, 30, 30, ...]
```

### 5. Respect Rate Limits

Track your usage and stay well under limits. If rate limited, back off for the full `retry_after_seconds`.

### 6. Handle Webhook Callbacks (Optional)

If configured, the gateway will POST status updates to a webhook URL:

```json
{
  "action_id": "...",
  "status": "sent",
  "recipient": "alice@example.com",
  "channel": "email",
  "sent_at": "2026-02-03T15:30:00Z"
}
```

Configure webhooks when creating the API key (future feature) or contact the gateway administrator.

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| 401 on every request | Bad API key | Regenerate key with `gateway keys add` |
| 403 consent denied | Contact opted out | Don't contact this person via AI |
| 403 not allowed | Allowlist restriction | Contact gateway admin to update allowlist |
| Message stuck pending | User hasn't reviewed | Prompt user to check `contactcmd gateway approve` |
| 429 rate limited | Too many requests | Wait `retry_after_seconds` before retrying |

## Security Notes

- API keys are shown only once at creation - store securely
- Keys can be revoked instantly: `contactcmd gateway keys revoke <id>`
- All sends are logged with full audit trail
- Only the gateway host machine can approve/deny (local-only endpoints)
