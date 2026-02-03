# Task 51: Google OAuth2 email authentication

**Feature**: email
**Created**: 2026-01-29
**Depends on**: none
**Blocks**: none

## Problem

Currently, sending email requires opening Mail.app and manually clicking send. Users want to send emails directly from contactcmd without leaving the terminal. Gmail/Google Workspace is the most common email provider, and OAuth2 is the secure standard - no passwords stored locally, authentication happens in the browser, and the app only stores a refresh token.

## Success criteria

- [ ] User can run a setup command to authenticate with Google via browser
- [ ] OAuth flow opens browser to Google consent screen
- [ ] After consent, refresh token is stored securely in local database
- [ ] User can send email directly with `[s]end` instead of `[o]pen in Mail`
- [ ] Emails sent via Gmail SMTP with XOAUTH2 authentication
- [ ] Token auto-refreshes; user only re-authenticates if revoked
- [ ] User can disconnect/revoke Google auth from Setup menu

## Technical approach

### OAuth2 flow for CLI apps

```
1. User initiates auth
2. App opens browser: https://accounts.google.com/o/oauth2/v2/auth
   - client_id (from Google Cloud Console)
   - redirect_uri=http://127.0.0.1:PORT (local callback)
   - scope=https://www.googleapis.com/auth/gmail.send
   - response_type=code
3. User consents in browser
4. Google redirects to localhost with ?code=AUTH_CODE
5. App exchanges code for tokens via POST to /token endpoint
6. App stores refresh_token in database (encrypted)
7. For sending: use access_token with Gmail SMTP + XOAUTH2
```

### Rust crates

```toml
oauth2 = "4"           # OAuth2 client
lettre = "0.11"        # SMTP with XOAUTH2 support
tokio = { version = "1", features = ["rt-multi-thread"] }  # async for HTTP server
webbrowser = "0.8"     # Open browser cross-platform
```

### Gmail SMTP settings

```
Host: smtp.gmail.com
Port: 587 (STARTTLS) or 465 (SSL)
Auth: XOAUTH2 with access_token
```

### Database schema addition

```sql
-- Add to app_settings or new oauth_tokens table
CREATE TABLE IF NOT EXISTS oauth_tokens (
    provider TEXT PRIMARY KEY,  -- 'google'
    email TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    access_token TEXT,
    expires_at INTEGER,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### UX flow

**Setup:**
```
Setup > Email > Authenticate with Google

Opening browser for Google sign-in...

[Browser: Google consent screen]

Waiting for authorization... Done!
Authenticated as jason@jnun.com
```

**Sending:**
```
Ready to send:

To: Laura Nunnelley (lauran@rocketmail.com)
From: jason@jnun.com
Subject: Let's chat!

  Testing something

[s]end [o]pen in Mail [q]uit: s
Sending... Sent.
```

## Implementation steps

1. **Google Cloud setup** (manual, document in README)
   - Create project in Google Cloud Console
   - Enable Gmail API
   - Create OAuth2 credentials (Desktop app type)
   - Download client_id and client_secret

2. **Add OAuth module** (`src/cli/oauth.rs`)
   - Local HTTP server for callback (port 8080 or random available)
   - Browser launch with auth URL
   - Code exchange for tokens
   - Token storage/retrieval from DB

3. **Add SMTP sending** (`src/cli/email.rs`)
   - `send_via_gmail()` function using lettre + XOAUTH2
   - Token refresh before sending if expired
   - Fallback to Mail.app if not authenticated

4. **Update email compose UI**
   - Add `[s]end` option when Google auth is configured
   - Keep `[o]pen in Mail` as fallback

5. **Setup menu integration**
   - "Authenticate with Google" option
   - "Disconnect Google" option
   - Show current auth status

## Security considerations

- Refresh token stored locally (consider encryption with keychain on macOS)
- Client secret embedded in binary (acceptable for desktop apps per Google)
- Tokens scoped to `gmail.send` only (not full Gmail access)
- HTTPS for all Google API calls

## Notes

- Google Cloud project is free for personal use
- May need to configure OAuth consent screen as "External" and add test users during development
- Production apps need Google verification for non-sensitive scopes
- Consider supporting multiple Google accounts in future

## References

- [Google OAuth2 for Desktop Apps](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Gmail SMTP with XOAUTH2](https://developers.google.com/gmail/imap/xoauth2-protocol)
- [lettre SMTP crate](https://docs.rs/lettre/latest/lettre/)
- [oauth2 crate](https://docs.rs/oauth2/latest/oauth2/)
