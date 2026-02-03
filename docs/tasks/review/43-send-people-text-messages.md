# Task 43: Send people text messages

**Feature**: none
**Created**: 2026-01-28
**Depends on**: none
**Blocks**: none

## Problem

ContactCMD can search and display iMessage history but cannot send messages. The messages view (`[m]` from contact card) shows conversation history but has no way to continue the conversation. Users want to complete the communication loop without switching to Messages.app.

## Current flow

```
Contact Card
  [e]dit [m]essages [d]elete [q]uit
           ↓
Messages List (show_messages_screen)
  > → Jan 15 "Hey, are you free tomorrow?"
    ← Jan 15 "Yeah, what's up?"
    → Jan 14 "Thanks for lunch"

  12/47  [↑/↓] select [enter] view [q]uit
```

## Proposed flow

```
Contact Card
  [e]dit [m]essages [d]elete [q]uit
           ↓
Messages List (show_messages_screen)
  > → Jan 15 "Hey, are you free tomorrow?"
    ← Jan 15 "Yeah, what's up?"
    → Jan 14 "Thanks for lunch"

  12/47  [↑/↓] select [enter] view [s]end [q]uit
                                    ↓
                              Compose Screen
                              To: John Smith (+1 555-123-4567)

                              message: _

                              [enter] send [esc] cancel
                                    ↓
                              "Sent." (then return to messages list)
```

## Success criteria

- [ ] Messages screen shows `[s]end` option in footer
- [ ] Pressing `s` opens compose screen with recipient pre-filled
- [ ] User types message and presses Enter to send
- [ ] User can press Esc to cancel without sending
- [ ] After sending, user sees "Sent." confirmation and returns to messages list
- [ ] Errors display clearly (no phone, Messages not configured, permission denied)
- [ ] First run prompts for Accessibility permissions with clear instructions

## Implementation approach

### AppleScript method

macOS provides no public API for sending iMessages. The reliable method is AppleScript:

```applescript
tell application "Messages"
    set targetBuddy to buddy "+15551234567" of service "iMessage"
    send "Hello from CLI" to targetBuddy
end tell
```

This approach:
- Works with both iMessage and SMS (depends on recipient capability)
- Requires Messages.app to be configured with an iMessage account
- Needs Accessibility permissions for `osascript` to control Messages.app

### Compose screen behavior

1. Show recipient name and phone number (or email if no phone)
2. Multi-line text input with `inquire::Text` or raw mode editor
3. Enter sends, Esc cancels
4. While sending, show "Sending..."
5. On success: "Sent." then return to messages list
6. On error: Show error, press any key to return to compose

### Message escaping

User messages must be escaped for AppleScript:
- Replace `"` with `\"`
- Replace `\` with `\\`
- Handle newlines appropriately

## Error handling

| Scenario | User sees |
|----------|-----------|
| Contact has no phone or email | "No phone or email for this contact." (don't show [s]end option) |
| Messages.app not configured | "Messages.app is not set up. Open Messages and sign in first." |
| Accessibility denied | "Permission required. System Settings > Privacy > Accessibility > Terminal" |
| Send failed | "Could not send message. Check Messages.app." |

## Files to modify

**Modify:**
- `src/cli/show.rs` - Add `[s]end` to messages screen, add compose flow
- `src/cli/mod.rs` - May need new helper module for AppleScript execution

**New file (optional):**
- `src/cli/imessage.rs` - AppleScript iMessage functions (could also be in show.rs)

## Implementation details

### Changes to show_messages_screen

```rust
// In the footer, add [s]end option
print!("\n{}/{}  [↑/↓] select [enter] view [s]end [q]uit", selected + 1, total_msgs);

// In the match block, add:
KeyCode::Char('s') | KeyCode::Char('S') => {
    // Get phone number (prefer) or email
    let recipient = get_send_address(&detail)?;
    if let Some(addr) = recipient {
        match compose_and_send(&addr, display_name)? {
            SendResult::Sent => {
                // Optionally refresh messages list
            }
            SendResult::Cancelled => {}
            SendResult::Error(msg) => {
                show_error(&msg)?;
            }
        }
    } else {
        show_error("No phone or email for this contact.")?;
    }
}
```

### Compose screen

```rust
fn compose_and_send(recipient: &str, display_name: &str) -> Result<SendResult> {
    clear_screen()?;

    println!("To: {} ({})\n", display_name, recipient);

    let message = Text::new("message:")
        .with_render_config(minimal_render_config())
        .prompt_skippable()?;

    let Some(message) = message else {
        return Ok(SendResult::Cancelled);
    };

    if message.trim().is_empty() {
        return Ok(SendResult::Cancelled);
    }

    print!("Sending...");
    io::stdout().flush()?;

    match send_imessage(recipient, &message) {
        Ok(()) => {
            println!(" Sent.");
            std::thread::sleep(std::time::Duration::from_millis(800));
            Ok(SendResult::Sent)
        }
        Err(e) => Ok(SendResult::Error(e.to_string()))
    }
}
```

### AppleScript execution

```rust
fn send_imessage(recipient: &str, message: &str) -> Result<()> {
    // Escape message for AppleScript
    let escaped = message
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let script = format!(r#"
        tell application "Messages"
            set targetBuddy to buddy "{}" of service "iMessage"
            send "{}" to targetBuddy
        end tell
    "#, recipient, escaped);

    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not authorized") {
            anyhow::bail!("Permission required. System Settings > Privacy > Accessibility");
        }
        anyhow::bail!("Send failed: {}", stderr.trim());
    }

    Ok(())
}
```

## Security considerations

- Messages appear in Messages.app history (audit trail)
- No bulk sending capability
- User explicitly presses `[s]end` then `[enter]` - two deliberate actions
- Esc cancels at any point

## Testing approach

1. **Unit tests**: Message escaping, recipient address extraction
2. **Manual testing**: Send real test messages during development
3. **Error paths**: Test with Messages closed, no account, permissions denied

## Notes

- Messages.app launches automatically if closed (may need brief delay)
- Very long messages may be split by carrier (SMS ~160 char limit)
- Group messages out of scope (single recipient only)
- Attachments out of scope
- The send option only appears if contact has phone or email

## Future enhancements (not this task)

- Quick reply from contact card without going to messages screen
- Standalone `contactcmd text "John" "message"` command
- Show delivery status (delivered, read)
- Reply to specific message thread
