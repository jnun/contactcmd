# Guide: Recent Contacts

View contacts you've messaged via iMessage/SMS to quickly follow up on conversations.

> **Note:** This feature requires macOS and reads from the local Messages database.

## Quick Start

```
/recent        # Show last 7 days
/r             # Same (shortcut)
/recent 30     # Show last 30 days
```

## Output

The command shows each contact with:
- **Name** - From your contacts database, or phone/email if unknown
- **Time** - How long ago the last message was
- **Service** - iMessage or SMS

```
Recent contacts (last 7 days):

  Sarah Chen         3 days ago    iMessage
  Alex Rivera        today         SMS
  +1 555-123-4567   5 days ago    iMessage  (unknown)

2 contacts, 1 unknown. /browse to view details.
```

## Workflow

1. **Find recent contacts**: `/recent` or `/r`
2. **Browse the list**: `/browse` to view details
3. **Take action**: Message, call, or view full contact info

## Unknown Numbers

Numbers not in your contacts appear with "(unknown)". These are valid message handles that couldn't be matched to any contact.

To add an unknown number:
1. Note the phone number
2. Use `/add` to create a new contact with that number

## Troubleshooting

### "Error reading Messages database"

The app needs **Full Disk Access** permission to read your iMessage history.

**Fix:**
1. Open System Settings → Privacy & Security → Full Disk Access
2. Enable access for Terminal (or your terminal app)
3. Restart your terminal

### No messages found

If `/recent` shows no results:
- Check that you have iMessage/SMS conversations in the last N days
- Try increasing the time range: `/recent 30`

## Related Commands

| Command | Description |
|---------|-------------|
| `/browse` | Browse matched contacts in TUI |
| `/messages <name>` | View full message history with a contact |
| `/search` | Find contacts by name, email, or company |
