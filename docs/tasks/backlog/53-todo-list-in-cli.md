# Task 53: TODO list in CLI

**Feature**: /docs/features/tasks.md
**Created**: 2026-01-30
**Depends on**: none
**Blocks**: none

## Problem

The problem is I often get bogged down in tools, projects, etc. and even my calendars get messy. I need a central CLI interface that helps me track and delegate my tasks appropriately. Some can be handled by autonomous agents, and some need my personal attention. Step one is centralizing my notes and tasks in one place.

ContactCMD is a good starting space.

Create a simple task management that simply collects all my little tasks, then we can use GSD (Get Stuff Done) and autonomous assistant agents later to assist, in a way that protects my personal data but relieves me of the droll.

## Solution

A `task` subcommand integrated into contactcmd with:

### Data Model

```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    quadrant INTEGER NOT NULL DEFAULT 4,      -- Eisenhower matrix: 1-4
    deadline TEXT,                             -- ISO datetime, nullable
    completed_at TEXT,                         -- null = incomplete
    person_id TEXT,                            -- optional contact link
    parent_id TEXT,                            -- for subtasks/checklists
    privacy_level TEXT DEFAULT 'personal',     -- personal/pii/delegable
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (person_id) REFERENCES persons(id) ON DELETE SET NULL,
    FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```

### Eisenhower Quadrants

- **Q1**: Important AND Urgent (do first)
- **Q2**: Important but NOT Urgent (schedule)
- **Q3**: NOT Important but Urgent (delegate/batch)
- **Q4**: NOT Important and NOT Urgent (do later or drop)

### CLI Commands

```bash
# Interactive dashboard (default)
contactcmd task

# List tasks
contactcmd task list                # Incomplete tasks, sorted by quadrant
contactcmd task list --all          # Include completed
contactcmd task list -q 1           # Filter by quadrant
contactcmd task list --today        # Due today
contactcmd task list --overdue      # Past deadline
contactcmd task list --for "John"   # Tasks linked to a contact

# Add tasks (fast capture)
contactcmd task add "Call dentist"
contactcmd task add "Pay rent" -q 1 --due 2026-02-01
contactcmd task add "Review proposal" --for "John Smith"

# Task operations
contactcmd task show <id>           # Full details
contactcmd task done <id>           # Mark complete
contactcmd task undone <id>         # Unmark
contactcmd task edit <id>           # Interactive edit
contactcmd task rm <id>             # Delete

# Subtasks
contactcmd task sub <parent-id> "Subtask title"
```

### Contact Integration

- `contactcmd show "John"` displays tasks linked to that contact
- `--for` flag links a task to a contact by name/id
- Tasks persist if contact is deleted (SET NULL)

### Privacy Levels (for future agent delegation)

- `personal`: Regular tasks (default)
- `pii`: Sensitive data (bills, medical, legal)
- `delegable`: Safe to hand off to an autonomous agent

## Success Criteria

- [ ] `task add "title"` creates a task instantly (no prompts)
- [ ] `task list` shows incomplete tasks grouped/sorted by quadrant
- [ ] `task list --all` includes completed tasks
- [ ] `task list -q <n>` filters by quadrant (1-4)
- [ ] `task list --today` shows tasks due today
- [ ] `task list --overdue` shows past-deadline tasks
- [ ] `task done <id>` marks task complete with timestamp
- [ ] `task undone <id>` clears completion
- [ ] `task show <id>` displays full task details
- [ ] `task edit <id>` allows interactive editing
- [ ] `task rm <id>` deletes a task (with confirmation)
- [ ] `task sub <parent> "title"` creates a subtask
- [ ] `--for "contact"` links task to a person
- [ ] `contactcmd show "person"` displays their linked tasks
- [ ] Interactive `task` mode with keyboard nav (j/k/enter/d)
- [ ] Schema migration V6 adds tasks table

## Deferred (Future Work)

- Natural language date parsing ("tomorrow", "next week")
- Recurring tasks
- Agent delegation / GSD integration
- Sync to external task systems (Todoist, Things, etc.)
- Due time notifications

## Notes

- Follows existing contactcmd patterns (clap, SQLite, crossterm TUI)
- Tasks are first-class entities, contact link is optional
- Minimal friction for capture is the primary UX goal
- Quadrant 4 default encourages triage rather than false urgency
