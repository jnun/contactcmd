# Task 73: Outlook Integration

## Summary

Add Microsoft Graph API integration to fetch Outlook email metadata.

## Dependency

Requires Task 70 (schema) to store results.

## Blocked By

- Task 70: Email interactions schema

## Blocks

- Task 74: Contact matching
- Task 76: Sync command

## Scope

New OAuth flow for Microsoft + Graph API queries.

## Microsoft Graph API

| Endpoint | Purpose |
|----------|---------|
| `GET /me/messages` | List messages with filters |
| Query params | `$select=subject,from,toRecipients,receivedDateTime` |

## Auth Flow

New OAuth 2.0 flow for Microsoft identity platform:
- Register app in Azure AD
- Scopes: `Mail.Read`, `User.Read`
- Store tokens similar to Gmail

## Files

| File | Change |
|------|--------|
| `src/cli/microsoft_auth.rs` | New - OAuth flow |
| `src/cli/outlook.rs` | New - Graph API client |

## Acceptance

- Can authenticate with Microsoft account
- Fetch email metadata from Outlook
- Extracts: from, to, subject, date, message_id
- Inserts into email_interactions table

## Notes

- Lower priority than Spotlight/Gmail (more users)
- Could be skipped for v1 if Spotlight covers Outlook locally
