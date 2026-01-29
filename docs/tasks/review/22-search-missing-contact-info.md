# Task 22: Search for Contacts Missing Phone or Email

**Feature**: none
**Created**: 2026-01-27

## Problem

Users need a way to find contacts that have incomplete information, specifically those missing a phone number or email address. This helps with contact data hygiene and allows users to review and update contacts that may need attention. The search results should use the same interactive display with edit/delete/notes features so users can immediately fix the missing information.

## Success criteria

- [x] Add `--missing` flag to search command that accepts "phone" or "email" as values
- [x] `contactcmd search --missing phone` returns all contacts without a phone number
- [x] `contactcmd search --missing email` returns all contacts without an email address
- [x] Results display in interactive review mode with edit/delete/notes actions
- [x] Database query efficiently finds contacts with missing fields using LEFT JOIN / IS NULL pattern
- [x] Help text documents the new `--missing` flag with examples

## Notes

Similar implementation pattern to the existing multi-word search. The database query should use LEFT JOIN on emails/phones tables and filter for NULL to find contacts missing that data type.

Example usage:
```
contactcmd search --missing phone    # Find contacts without phone numbers
contactcmd search --missing email    # Find contacts without email addresses
```
