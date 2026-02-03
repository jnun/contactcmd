# UI Design System

contactcmd's visual language for terminal interfaces.

## Principles

- **Minimal**: Show only what's needed
- **Clean**: Whitespace is the design, no decorative elements
- **Reliable**: Same patterns everywhere
- **Fast**: Instant feedback, no delays
- **Antifragile**: Works on any terminal

## Text Hierarchy

### Headers
The primary focus of the screen. Plain text, no decoration.

```
John Smith
```

### Indented Content
Details beneath a header use 2-space indent.

```
John Smith

  Engineer at Acme Corp
  john@example.com
  (555) 123-4567
  Austin, TX
```

### Labels and Prompts
Lowercase, colon, space. No trailing punctuation on the label itself.

```
search:
name:
```

### Hints
Navigation hints in square brackets, lowercase keys.

```
[e]dit [d]elete [q]uit
[↑/↓] scroll [q]uit
[enter]
```

## Spacing Rules

### Between Sections
One blank line separates logical sections.

```
John Smith

  Engineer at Acme Corp
  john@example.com

  > Yesterday at 3:42pm "Thanks for the intro"
```

### Within Sections
No blank lines between related items.

```
  john@example.com
  jane@example.com
  work@example.com
```

### Screen Boundaries
- No leading banners or titles (except screen-specific headers like "Messages: John Smith")
- No trailing decorations
- No box borders or horizontal rules

## Prompt Formats

### Text Input
Use inquire Text with minimal render config.

```
search: _
name: John Smith_
```

### Selection Menu
Numbered list with 2-space indent. Optional context in parentheses.

```
Select:

  1. John Smith (john@example.com)
  2. Jane Smith (jane@example.com)
  3. John Doe

[1-3] or [q]: _
```

### Single-Key Actions
For interactive displays where immediate response matters.

```
[e]dit [m]essages [d]elete [q]uit: _
```

### Browse-Style Navigation
The standard pattern for action screens after the main menu:

```
1/2378  [e]dit [m]essages [d]elete [←/→] [q]uit:
```

Components:
- Position indicator: `1/2378` (current/total)
- Actions with hotkeys: `[e]dit`, `[m]essages`, `[@]email`
- Navigation hints: `[←/→]` for horizontal, `[↑/↓]` for vertical
- Exit: `[q]uit:`

### Settings Actions
For configuration screens, use `[k]eep` and `[d]iscard` to avoid conflicts with `[s]`:

```
[e]dit [c]lear [q]uit:
```

When editing settings:
```
Type signature, then 'k' to [k]eep or 'd' to [d]iscard:
```

### Email Actions
For email compose, use `[s]end` since there's no conflict:

```
[s]end [q]uit:
```

During message composition:
```
Type message, then 's' to [s]end or 'c' to [c]ancel:
```

### Confirmation
Use inquire Confirm. Default to "no" for destructive actions.

```
Delete John Smith? (y/N): _
```

## Feedback Patterns

### Success
Action word (capitalized), colon, result.

```
Created: John Smith
Deleted.
Saved.
```

### Error
Capital "Error:", specific message.

```
Error: invalid email format
Error: contact not found
```

### Warning
Capital "Warning:", explanation.

```
Warning: Could not delete from macOS Contacts
```

### Info
Plain statement, no prefix.

```
No matches.
No messages.
3 contacts synced.
```

### Counts
Numbers only, no verbose labels.

```
1-15 of 42
3 results
```

## Navigation Conventions

### Vim-Style Movement
Enable vim mode in inquire Select. Support j/k for scrolling.

```
[↑/↓] scroll
```

### Escape and Quit
Escape or 'q' exits the current screen. Enter confirms or continues.

```
[q]uit
[enter]
```

### Back Navigation
Escape returns to previous screen. No explicit "back" option needed.

### Pagination Display
Show current range and total.

```
1-15 of 42  [↑/↓] scroll [q]uit
```

## Date Formatting

Relative dates for recent, absolute for older. Always lowercase am/pm.

| Context | Format | Example |
|---------|--------|---------|
| Today | `Today at H:MMam/pm` | `Today at 3:42pm` |
| Yesterday | `Yesterday at H:MMam/pm` | `Yesterday at 9:15am` |
| This year | `Mon D at H:MMam/pm` | `Jan 15 at 11:00am` |
| Other years | `Mon D, YYYY at H:MMam/pm` | `Jan 15, 2024 at 2:30pm` |

## Text Truncation

When space is limited, truncate with `...` suffix. Never lose critical information.

| Content | Max Length | Example |
|---------|------------|---------|
| Notes in contact view | 60 chars | `Met at tech conference 2024...` |
| Message preview | 50 chars | `Thanks for the intro to...` |
| Snippet with context | 52 chars | `...looking forward to the...` |

For snippets around a search match, center the window on the match term:
- Match at start: `Text starts here and continues...`
- Match in middle: `...context before match here and after...`
- Match at end: `...the text ends with match`

## Message Direction

Use arrow symbols for message flow:

```
> Today at 3:42pm "Outgoing message"    <- you sent (> points right)
< Today at 9:15am "Incoming message"    <- they sent (< points left)
```

## Empty States

Consistent phrasing, no punctuation variations:

```
No matches.
No messages.
No contacts.
No results.
```

## Progress States

Action in progress, then result:

```
Syncing from macOS Contacts...
142 contacts synced.
```

## Output Streams

### stdout vs stderr

```
stdout (println!):
  John Smith
  3 contacts synced.
  1-15 of 42

stderr (eprintln!):
  Error: contact not found
  Warning: could not delete from macOS Contacts
```

All errors and warnings go to stderr. Normal output goes to stdout.
This allows piping output while still seeing errors:

```bash
contactcmd list --json > contacts.json  # errors still visible
```

### TTY vs Piped Behavior

When stdout is a terminal (interactive):
- Show prompts and menus
- Use cursor movement for updates
- Wait for user input

When stdout is piped (scripted):
- Output plain text, one item per line
- Skip interactive prompts
- Exit with error if input is required

## Color Usage

**Decision: No colors.**

Rationale:
- Works on all terminals without configuration
- No accessibility concerns (colorblind users)
- Cleaner output, easier to read
- Can pipe output without ANSI escape codes
- Consistent appearance everywhere

We respect the `NO_COLOR` environment variable standard, though we have no colors to disable.

**No exceptions.** Search highlighting uses text positioning (centering snippet on match) rather than color. Selection highlighting uses the terminal's native cursor/selection, not ANSI inverse.

## Terminal Resilience

### Narrow Terminal Handling

```
80+ columns: Full display
  John Smith          john@example.com          Austin, TX

60-79 columns: Compact display
  John Smith          john@example.com

<60 columns: Minimal display
  John Smith
  john@example.com
```

Never wrap mid-word. Truncate with `...` before breaking layout.

### Terminal Size Detection

```rust
fn get_term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)  // Safe default
}
```

### Resize Handling

For interactive screens, redraw on SIGWINCH:
```rust
// Simplified: just redraw on next input
loop {
    clear_screen()?;
    render_at_width(get_term_width())?;
    wait_for_input()?;
}
```

### Broken Pipe Handling

```rust
// Don't panic on broken pipe (e.g., `contactcmd list | head`)
fn main() {
    if let Err(e) = run() {
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```

## Raw Mode Guidelines

Use inquire for all prompts. Raw mode (crossterm) is acceptable only for:

1. **Screen clearing** via `clear_screen()`
2. **Single-key immediate actions** where waiting for Enter would feel sluggish

When using raw mode:
- Always use RAII guard pattern (`RawModeGuard`) to ensure cleanup
- Handle Escape as cancel/quit
- Support both lowercase and uppercase keys
- Handle terminal resize gracefully

## Screen Archetypes

Every screen follows one of four patterns:

| Archetype | Purpose | Example |
|-----------|---------|---------|
| **Menu** | Navigate options | Main menu, action prompts |
| **Detail** | View single item | Contact view, message view |
| **List** | Browse multiple items | Search results, message list |
| **Form** | Collect input | Add contact, edit fields |

### Archetype: Menu

```
[title]

> [selected option]
  [option]
  [option]
```

- inquire Select handles rendering
- Vim keys for navigation
- Enter to select, Escape to cancel

### Archetype: Detail

```
[Header]

  [field]: [value]
  [field]: [value]

[action hints]: _
```

- Header is the focus (name, title)
- 2-space indent for fields
- Actions at bottom

### Archetype: List

```
[item]    [context]
[item]    [context]
[item]    [context]

[range] of [total]  [navigation hints]
```

- One item per line
- Pagination at bottom
- Optional selection indicator

### Archetype: Form

```
[field]: [value]_
[field]: [value]
[field]: [value]

[feedback]
```

- Sequential prompts
- Feedback after completion
- Escape cancels entire form

---

## Screen Mockups

### Main Menu

```
contactcmd

> List
  Search
  Show
  Add
  Note
  Update
  Delete
  Sync
  Messages
  Quit
```

### Contact Detail

```
John Smith

  Engineer at Acme Corp
  john@example.com
  (555) 123-4567
  Austin, TX
  Met at tech conference 2024...

  > Yesterday at 3:42pm "Thanks for the intro"

[e]dit [m]essages [d]elete [q]uit: _
```

### Search Results (Single Match)

```
John Smith

  john@example.com
  (555) 123-4567

[e]dit [d]elete [q]uit: _
```

### Search Results (Multiple Matches)

```
Select:

  1. John Smith (john@example.com)
  2. Jane Smith (jane@example.com)

[1-2] or [q]: _
```

### Messages Screen (Contact View)

```
Messages: John Smith

> Today at 10:15am "Sure, let's meet Thursday"
< Today at 9:30am "Are you free this week?"
> Yesterday at 3:42pm "Thanks for the intro"
< Yesterday at 2:15pm "I'd like you to meet Sarah"
> Jan 15 at 11:00am "Happy new year!"

1-5 of 23  [↑/↓] scroll [q]uit
```

### Messages Search

```
Messages: John Smith

  Austin, TX
  +15551234567

> > Today at 10:15am "...the project looks great..."
  < Yesterday at 3:42pm "...looking forward to the..."
  > Jan 15 at 11:00am "...great meeting you..."

1 of 3  5 message(s)  1-3 of 5
[←/→] contact [↑/↓] select [enter] expand [q]uit: _
```

The `>` prefix on a line indicates the selected message for expansion.

### List View

```
John Smith          john@example.com
Jane Smith          jane@example.com
Bob Johnson         bob@acme.com

1-3 of 3
```

### Add/Edit Form

```
first name: John_
last name: Smith
email: john@example.com
phone: (555) 123-4567

Saved.
```

### Delete Confirmation

```
John Smith

  john@example.com
  (555) 123-4567

Delete John Smith? (y/N): _
```

### Sync Output

```
Syncing from macOS Contacts...
142 contacts synced.
```

### Setup Screen

```
Setup

External Services:
  [e] Email - configured (jason@example.com)

[e]mail [q]uit:
```

### Email Signature Setup

```
Email Signature

Current signature:
┌─────────────────────────────┐
│ - Jason Nunnelley
│ CEO, Acme Corp
└─────────────────────────────┘

[e]dit [c]lear [q]uit:
```

### Email Compose

```
Compose Email

To: John Smith (john@example.com)
From: me@gmail.com

Subject: Catching up

┌─ signature ─────────────────┐
│ --
│ - Jason Nunnelley
└─────────────────────────────┘

Type message, then 's' to [s]end or 'c' to [c]ancel:

Hey John,

Great meeting you at the conference!
s
```

### Email Confirmation

```
Ready to send:

To: John Smith (john@example.com)
From: me@gmail.com
Subject: Catching up

  Hey John,

  Great meeting you at the conference!
  ...

[s]end [q]uit:
```

### Messages Screen with Email Option

When contact has email, show [@] option:

```
No messages for this contact.

[t]ext [@]email [q]uit:
```

With messages:

```
Messages: John Smith

> Today at 10:15am "Sure, let's meet Thursday"
< Today at 9:30am "Are you free this week?"

1/5  [↑/↓] select [enter] view [t]ext [@]email [q]uit:
```

### Error States

```
search: xyznonexistent

No matches.
```

```
name:

Error: name cannot be empty
```

---

References:
- [Command Line Interface Guidelines](https://clig.dev/)
- [GNU Coding Standards (user interfaces)](https://www.gnu.org/prep/standards/html_node/User-Interfaces.html)
- Apple Human Interface Guidelines
