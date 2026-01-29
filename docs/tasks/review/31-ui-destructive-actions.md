# Task 31: UI Destructive Actions and Confirmations

**Feature**: none
**Created**: 2026-01-28
**Depends on**: Tasks 24, 25
**Design reference**: [ui-system.md](/docs/designs/ui-system.md) - Confirmation format, [design-system.md](/docs/guides/design-system.md) - CLI Conventions

## Problem

Destructive actions (delete) need confirmation to prevent accidents, but the confirmation shouldn't be so onerous that users bypass it or get frustrated.

Current implementation requires typing "yes" which is:
- Slow (4 keystrokes vs 1)
- Error-prone (typos)
- Inconsistent with other CLI tools (`rm -i` uses y/n)

UX research: The best confirmations show what will happen and require minimal but intentional input.

## Success criteria

- [ ] Refactor delete confirmation in `src/cli/delete.rs`
- [ ] Use inquire Confirm for y/n prompt
- [ ] Show contact summary before confirm
- [ ] Prompt: `Delete John Smith? (y/N)` (default No, per design system)
- [ ] y/n then Enter to confirm (inquire standard behavior)
- [ ] Success: `Deleted.`
- [ ] Cancel: (user pressed Escape or 'n')
- [ ] Force flag (`-f`) skips confirmation (for scripting)
- [ ] In review mode, `[d]` keypress is the confirmation (immediate delete)

## Notes

Delete flow:
```
John Smith
  john@example.com
  Acme Corp

Delete? [y/N] y
Deleted.
```

Using inquire Confirm:
```rust
let confirmed = Confirm::new("Delete John Smith?")
    .with_default(false)
    .prompt()?;
```

The default-No is a critical safety feature. Accidentally hitting enter doesn't delete.

For review mode, the `[d]` keypress is itself the intent signal, so we delete immediately and show feedback:
```
Deleted: John Smith
```

Then advance to next contact.

---
