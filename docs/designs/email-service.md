# Email Service Design

Email integration for contactcmd via macOS Mail.app.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        contactcmd                           │
├─────────────────────────────────────────────────────────────┤
│  Setup Screen          │  Email Compose       │  Messages   │
│  - Account selection   │  - To/From/Subject   │  - [@]email │
│  - Signature config    │  - Body with sig     │  - [t]ext   │
│  - Default subject     │  - Send confirmation │             │
├─────────────────────────────────────────────────────────────┤
│                    app_settings table                       │
│  - email_account       - email_signature      - email_default_subject
├─────────────────────────────────────────────────────────────┤
│                    AppleScript / osascript                  │
│  - Query Mail.app accounts                                  │
│  - Send email via Mail.app                                  │
└─────────────────────────────────────────────────────────────┘
```

## Data Model

### app_settings Table (Schema V4)

```sql
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

Keys used:
- `email_account` - Selected sender email address
- `email_signature` - Multi-line signature text
- `email_default_subject` - Optional default subject line

## User Flows

### Initial Setup

```
Main Menu → Setup → Email
                      ↓
            Check Mail.app accounts (osascript)
                      ↓
            ┌─────────┴─────────┐
            ↓                   ↓
       No accounts         Accounts found
            ↓                   ↓
    Show instructions    Select account
            ↓                   ↓
         [q]uit          Configure signature
                                ↓
                         Configure default subject (optional)
                                ↓
                         "Email configured successfully!"
```

### Sending Email

```
Contact Card → [m]essages → [@]email
                              ↓
                    ┌─────────┴─────────┐
                    ↓                   ↓
            Not configured      Configured
                    ↓                   ↓
            Show error         Compose screen
            "Use Setup..."            ↓
                              Enter subject
                                    ↓
                              Enter body (with signature)
                                    ↓
                              Confirmation screen
                                    ↓
                              [s]end → Mail.app sends
```

## Screen Specifications

### Setup Main

```
Setup

External Services:
  [e] Email - configured (jason@example.com)

[e]mail [q]uit:
```

Status shows:
- `not configured` - No account selected
- `configured (email@example.com)` - Account selected

### Email Account Selection

Uses inquire Select with vim mode:

```
Email Setup

Checking Mail.app accounts... done.

Current account: jason@example.com

Select email account:
> jason@example.com
  work@company.com
  Cancel
```

### Signature Edit

```
Email Signature

Current signature:
┌─────────────────────────────┐
│ - Jason Nunnelley
│ CEO, Acme Corp
└─────────────────────────────┘

[e]dit [c]lear [q]uit:
```

Editing mode:
```
Type signature, then 'k' to [k]eep or 'd' to [d]iscard:

- Jason Nunnelley
CEO, Acme Corp
k
Saved.
```

### Email Compose

```
Compose Email

To: John Smith (john@example.com)
From: me@gmail.com

Subject: Meeting follow-up

┌─ signature ─────────────────┐
│ --
│ - Jason Nunnelley
└─────────────────────────────┘

Type message, then 's' to [s]end or 'c' to [c]ancel:

Hey John,

Great meeting you yesterday!
s
```

### Confirmation

```
Ready to send:

To: John Smith (john@example.com)
From: me@gmail.com
Subject: Meeting follow-up

  Hey John,

  Great meeting you yesterday!
  ...

[s]end [q]uit:
```

## Key Bindings

| Context | Key | Action |
|---------|-----|--------|
| Messages screen | `@` | Open email compose |
| Messages screen | `t` | Open SMS compose |
| Setup screen | `e` | Configure email |
| Setup screen | `q` | Exit setup |
| Signature view | `e` | Edit signature |
| Signature view | `c` | Clear signature |
| Signature edit | `k` | Keep (save) changes |
| Signature edit | `d` | Discard changes |
| Email compose | `s` | Send email |
| Email compose | `c` | Cancel |
| Confirmation | `s` | Send |
| Confirmation | `q` | Cancel |

## AppleScript Integration

### Query Accounts

```applescript
tell application "Mail" to get email addresses of every account
```

Returns comma-separated list: `"email1@me.com, email2@gmail.com"`

### Send Email

```applescript
tell application "Mail"
    set msg to make new outgoing message with properties {
        subject:"Subject",
        content:"Body",
        sender:"from@example.com",
        visible:false
    }
    tell msg
        make new to recipient with properties {address:"to@example.com"}
        send
    end tell
end tell
```

## Error Handling

### No Mail.app Accounts

```
No email accounts found in Mail.app.

To use email features:
  1. Open Mail.app
  2. Add an email account (Mail > Add Account)
  3. Return here to configure

[q]uit:
```

### Email Not Configured

When user presses [@] without setup:

```
Error: Email not configured. Use Setup from main menu to configure.

[q]uit:
```

### Permission Error

```
Error: Permission required.

Grant access in: System Settings > Privacy & Security > Automation
Enable access for your terminal to control Mail.

[q]uit:
```

### Send Failure

```
Error: Send failed: [specific error from Mail.app]

[q]uit:
```

## Security Considerations

### AppleScript Escaping

All user input is escaped before embedding in AppleScript:
- `\` → `\\`
- `"` → `\"`
- `\n` → `\\n`
- `\r` → `\\r`
- `\t` → `\\t`
- Control characters (0x00-0x08, 0x0b, 0x0c, 0x0e-0x1f) → removed

### No Credential Storage

- No passwords or OAuth tokens stored
- Mail.app handles all authentication
- Only stores email address (for display) and user preferences

## Platform Support

| Platform | Status |
|----------|--------|
| macOS | Full support via Mail.app |
| Linux | Not supported (no Mail.app) |
| Windows | Not supported (no Mail.app) |

Non-macOS platforms show:
```
Email sending is only available on macOS.
```

## Future Considerations

- Multiple recipient support (CC, BCC)
- Attachments
- HTML email formatting
- Draft saving
- Alternative SMTP backend for cross-platform
