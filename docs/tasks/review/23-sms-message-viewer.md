# Task 23: SMS message viewer

**Feature**: /docs/features/cmd-show.md
**Created**: 2026-01-28

## Problem

The Messages integration in ContactCMD has three issues that prevent it from being useful:

1. **`attributedBody` extraction is broken.** The current code queries `WHERE m.text IS NOT NULL AND m.text != ''`, but modern macOS stores message content in the `attributedBody` blob column, not `text`. In practice ~99% of messages have NULL `text`. The existing "last message" feature returns nothing for most contacts.

2. **No message search.** There is no way to search message content across all conversations. A common use case is "I was texting someone about X but can't remember who" -- the current code only supports looking up messages for a contact you already know.

3. **No message viewing screen.** The `show` command displays a single last message inline. There is no way to view a conversation thread or browse recent messages with a contact.

This task addresses all three by fixing the extraction bug, adding a message content search, and adding an interactive `[M]essages` option to the show command's action menu that enters a new screen showing the SMS conversation with that contact. This is the first step toward a broader communication viewer that will later support email and other channels.

## Success criteria

- [ ] `attributedBody` blob is parsed to extract plain text when `text` column is NULL
- [ ] Existing "last message" feature in show/search works correctly with `attributedBody` messages
- [ ] New message search command (`contactcmd messages "search terms"`) searches SMS content across all conversations
- [ ] Message search results display: matching message text, contact phone/name (resolved from ContactCMD DB or macOS Contacts), timestamp, and direction (sent/received)
- [ ] Message search matches against ContactCMD contacts when possible (show name instead of just phone number)
- [ ] `[M]essages` option added to the show command's interactive action menu
- [ ] `[M]essages` enters a new screen displaying the SMS conversation thread with that contact (most recent messages, scrollable or paginated)
- [ ] Messages screen shows direction (sent/received), timestamp, and message text for each message
- [ ] All message reading is read-only (no writes to chat.db)
- [ ] Graceful handling when Messages DB is inaccessible (permissions, not macOS, etc.)
- [ ] Unit tests for `attributedBody` text extraction

## Notes

- SMS/iMessage data lives in `~/Library/Messages/chat.db` (requires Full Disk Access)
- The `attributedBody` column contains an NSAttributedString in Apple's typedstream binary format; readable text segments can be extracted by finding printable byte sequences between binary markers
- The `[M]essages` screen is view-only for now; sending messages or other interaction will come in a future task
- Starting with SMS only; email, social, and other channels are future scope
- The existing `messages/macos.rs` module and its phone normalization / timestamp conversion utilities should be extended rather than replaced
- The current query scans the last 1000 messages and filters client-side; consider querying by handle directly for the conversation view
