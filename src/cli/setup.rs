//! Setup screen for external service configuration (email, AI, etc.)

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use inquire::{Select, Text};
use std::io::{self, Write};
use std::process::Command;

use crate::db::Database;
use super::ai::{AiConfig, AiProviderType};
#[cfg(feature = "local-ai")]
use super::ai::LocalModel;
use super::google_auth::{
    is_google_auth_configured, get_google_email, run_google_auth_flow, disconnect_google,
};
use super::ui::{clear_screen, minimal_render_config, visible_lines, wait_for_key, RawModeGuard};

// Setting keys for email configuration
pub const SETTING_EMAIL_ACCOUNT: &str = "email_account";
pub const SETTING_EMAIL_SIGNATURE: &str = "email_signature";
pub const SETTING_EMAIL_DEFAULT_SUBJECT: &str = "email_default_subject";

/// Setup menu option types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupOption {
    EmailMail,
    EmailGoogle,
    AiRemote,
    #[cfg(feature = "local-ai")]
    AiLocal,
    Back,
}

impl SetupOption {
    #[cfg(feature = "local-ai")]
    const ALL: &'static [SetupOption] = &[
        SetupOption::EmailMail,
        SetupOption::EmailGoogle,
        SetupOption::AiRemote,
        SetupOption::AiLocal,
        SetupOption::Back,
    ];

    #[cfg(not(feature = "local-ai"))]
    const ALL: &'static [SetupOption] = &[
        SetupOption::EmailMail,
        SetupOption::EmailGoogle,
        SetupOption::AiRemote,
        SetupOption::Back,
    ];

    fn base_label(self) -> &'static str {
        match self {
            SetupOption::EmailMail => "Email [Using Mail]",
            SetupOption::EmailGoogle => "Email [Google]",
            SetupOption::AiRemote => "AI [Remote API]",
            #[cfg(feature = "local-ai")]
            SetupOption::AiLocal => "AI [Local Model]",
            SetupOption::Back => "Back",
        }
    }
}

/// Build display label with status for a setup option
fn build_option_label(option: SetupOption, db: &Database) -> String {
    match option {
        SetupOption::EmailMail => {
            let status = match db.get_setting(SETTING_EMAIL_ACCOUNT).ok().flatten() {
                Some(account) => format!("configured ({})", account),
                None => "not configured".to_string(),
            };
            format!("{} - {}", option.base_label(), status)
        }
        SetupOption::EmailGoogle => {
            let status = if is_google_auth_configured(db) {
                let email = get_google_email(db).unwrap_or_default();
                format!("connected ({})", email)
            } else {
                "not connected".to_string()
            };
            format!("{} - {}", option.base_label(), status)
        }
        SetupOption::AiRemote => {
            let config = AiConfig::load(db).ok().unwrap_or_default();
            let status = if config.provider_type == AiProviderType::Remote && config.api_key.is_some() {
                let model = config.effective_model();
                format!("configured ({})", model)
            } else {
                "not configured".to_string()
            };
            format!("{} - {}", option.base_label(), status)
        }
        #[cfg(feature = "local-ai")]
        SetupOption::AiLocal => {
            let config = AiConfig::load(db).ok().unwrap_or_default();
            let status = if config.provider_type == AiProviderType::Local {
                if let Some(model_id) = config.local_model {
                    format!("configured ({})", model_id.as_str())
                } else {
                    "not configured".to_string()
                }
            } else {
                "not configured".to_string()
            };
            format!("{} - {}", option.base_label(), status)
        }
        SetupOption::Back => option.base_label().to_string(),
    }
}

/// Run the setup screen
pub fn run_setup(db: &Database) -> Result<()> {
    loop {
        clear_screen()?;

        // Build labels with current status
        let labels: Vec<String> = SetupOption::ALL
            .iter()
            .map(|opt| build_option_label(*opt, db))
            .collect();

        let selection = Select::new("Setup", labels.clone())
            .with_render_config(minimal_render_config())
            .with_page_size(visible_lines().min(labels.len()))
            .with_vim_mode(true)
            .prompt_skippable();

        let selection = match selection {
            Ok(sel) => sel,
            Err(_) => return Ok(()),
        };

        let Some(choice_label) = selection else {
            // User pressed Escape
            return Ok(());
        };

        // Find which option was selected by matching the start of the label
        let choice = SetupOption::ALL
            .iter()
            .find(|opt| choice_label.starts_with(opt.base_label()))
            .copied();

        match choice {
            Some(SetupOption::EmailMail) => {
                setup_email_mail(db)?;
            }
            Some(SetupOption::EmailGoogle) => {
                setup_email_google(db)?;
            }
            Some(SetupOption::AiRemote) => {
                setup_ai_remote(db)?;
            }
            #[cfg(feature = "local-ai")]
            Some(SetupOption::AiLocal) => {
                setup_ai_local(db)?;
            }
            Some(SetupOption::Back) | None => {
                return Ok(());
            }
        }
    }
}

/// Setup email via Mail.app
fn setup_email_mail(db: &Database) -> Result<()> {
    clear_screen()?;

    println!("Email [Using Mail] Setup\n");

    // Check if already configured - offer to change or disconnect
    let current_account = db.get_setting(SETTING_EMAIL_ACCOUNT)?;
    if let Some(ref account) = current_account {
        println!("Currently configured: {}\n", account);
        print!("[c]hange account [d]isconnect [s]ignature [q]uit: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                println!("\n");
                // Fall through to account selection below
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                db.delete_setting(SETTING_EMAIL_ACCOUNT)?;
                println!("\nDisconnected.\n");
                std::thread::sleep(std::time::Duration::from_millis(800));
                return Ok(());
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                println!();
                setup_signature(db)?;
                return Ok(());
            }
            _ => {
                return Ok(());
            }
        }
    }

    // Get available Mail.app accounts (with loading indicator)
    print!("Checking Mail.app accounts...");
    io::stdout().flush()?;
    let accounts = get_mail_accounts();
    println!(" done.\n");

    if accounts.is_empty() {
        println!("No email accounts found in Mail.app.\n");
        println!("To use email features:");
        println!("  1. Open Mail.app");
        println!("  2. Add an email account (Mail > Add Account)");
        println!("  3. Return here to configure\n");
        print!("[q]uit: ");
        io::stdout().flush()?;
        wait_for_key()?;
        return Ok(());
    }

    // Let user select an account
    let mut options: Vec<&str> = accounts.iter().map(|s| s.as_str()).collect();
    options.push("Cancel");

    let selection = Select::new("Select email account:", options.clone())
        .with_render_config(minimal_render_config())
        .with_vim_mode(true)
        .prompt_skippable()?;

    let Some(selected) = selection else {
        return Ok(());
    };

    if selected == "Cancel" {
        return Ok(());
    }

    // Save selected account
    db.set_setting(SETTING_EMAIL_ACCOUNT, selected)?;

    // Now configure signature
    setup_signature(db)?;

    // Optionally configure default subject
    setup_default_subject(db)?;

    clear_screen()?;
    println!("Email configured successfully!\n");
    println!("  Account: {}", selected);
    if let Some(sig) = db.get_setting(SETTING_EMAIL_SIGNATURE)? {
        if !sig.is_empty() {
            println!("  Signature: (set)");
        }
    }
    if let Some(subj) = db.get_setting(SETTING_EMAIL_DEFAULT_SUBJECT)? {
        if !subj.is_empty() {
            println!("  Default subject: {}", subj);
        }
    }
    println!();
    print!("[enter]/[q] to continue");
    io::stdout().flush()?;
    wait_for_key()?;

    Ok(())
}

/// Setup email via Google OAuth2
fn setup_email_google(db: &Database) -> Result<()> {
    clear_screen()?;

    println!("Email [Google] Setup\n");

    // Check current status
    if is_google_auth_configured(db) {
        let email = get_google_email(db).unwrap_or_else(|| "(unknown)".to_string());
        println!("Currently connected as: {}\n", email);
        print!("[r]econnect [d]isconnect [s]ignature [q]uit: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                println!("\n");
                match run_google_auth_flow(db) {
                    Ok(email) => {
                        clear_screen()?;
                        println!("Connected as: {}\n", email);
                        print!("[enter]/[q] to continue");
                        io::stdout().flush()?;
                        wait_for_key()?;
                    }
                    Err(e) => {
                        println!("\nError: {}\n", e);
                        print!("[enter]/[q] to continue");
                        io::stdout().flush()?;
                        wait_for_key()?;
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                disconnect_google(db)?;
                println!("\nDisconnected.\n");
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                println!();
                setup_signature(db)?;
            }
            _ => {}
        }
    } else {
        println!("Send emails directly from the command line via Gmail.\n");
        println!("This uses secure OAuth2 authentication - no password stored locally.\n");
        print!("[c]onnect with Google [q]uit: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => {
                println!("\n");
                match run_google_auth_flow(db) {
                    Ok(email) => {
                        // Optionally configure signature
                        setup_signature(db)?;

                        clear_screen()?;
                        println!("Google Email configured successfully!\n");
                        println!("  Account: {}", email);
                        if let Some(sig) = db.get_setting(SETTING_EMAIL_SIGNATURE)? {
                            if !sig.is_empty() {
                                println!("  Signature: (set)");
                            }
                        }
                        println!();
                        print!("[enter]/[q] to continue");
                        io::stdout().flush()?;
                        wait_for_key()?;
                    }
                    Err(e) => {
                        println!("Error: {}\n", e);
                        print!("[enter]/[q] to continue");
                        io::stdout().flush()?;
                        wait_for_key()?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Setup email signature
fn setup_signature(db: &Database) -> Result<()> {
    clear_screen()?;

    println!("Email Signature\n");

    // Get current signature if any
    let current = db.get_setting(SETTING_EMAIL_SIGNATURE)?.unwrap_or_default();

    if !current.is_empty() {
        println!("Current signature:");
        println!("┌─────────────────────────────┐");
        for line in current.lines() {
            println!("│ {}", line);
        }
        println!("└─────────────────────────────┘");
    } else {
        println!("No signature set.");
    }

    println!();
    print!("[e]dit [c]lear [q]uit: ");
    io::stdout().flush()?;

    let action = {
        let _guard = RawModeGuard::new()?;
        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                break code;
            }
        }
    };

    match action {
        KeyCode::Char('e') | KeyCode::Char('E') => {
            println!("\n");
            edit_signature(db)?;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            db.set_setting(SETTING_EMAIL_SIGNATURE, "")?;
            println!("\nSignature cleared.");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        _ => {}
    }

    Ok(())
}

/// Edit signature with multi-line input
fn edit_signature(db: &Database) -> Result<()> {
    use super::ui::multiline_input_raw;

    let result = multiline_input_raw(
        "Signature: ([esc] for commands)",
        "keep"
    )?;

    match result {
        Some(sig) if !sig.is_empty() => {
            db.set_setting(SETTING_EMAIL_SIGNATURE, &sig)?;
            println!("Saved.");
        }
        Some(_) => {
            println!("Nothing to save.");
        }
        None => {
            println!("Discarded.");
        }
    }

    Ok(())
}

/// Setup default email subject
fn setup_default_subject(db: &Database) -> Result<()> {
    clear_screen()?;

    println!("Default Subject (optional)\n");
    println!("Set a default subject line for new emails.");
    println!("Leave empty to always enter subject manually.\n");

    let current = db.get_setting(SETTING_EMAIL_DEFAULT_SUBJECT)?.unwrap_or_default();

    let subject = Text::new("Default subject:")
        .with_render_config(minimal_render_config())
        .with_initial_value(&current)
        .prompt_skippable()?;

    if let Some(subj) = subject {
        db.set_setting(SETTING_EMAIL_DEFAULT_SUBJECT, &subj)?;
    }

    Ok(())
}

/// Query Mail.app for configured email accounts
#[cfg(target_os = "macos")]
fn get_mail_accounts() -> Vec<String> {
    let script = "tell application \"Mail\" to get email addresses of every account";

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
fn get_mail_accounts() -> Vec<String> {
    Vec::new()
}

/// Check if email is configured
pub fn is_email_configured(db: &Database) -> bool {
    db.get_setting(SETTING_EMAIL_ACCOUNT)
        .ok()
        .flatten()
        .is_some()
}

/// Get the configured email account
pub fn get_email_account(db: &Database) -> Option<String> {
    db.get_setting(SETTING_EMAIL_ACCOUNT).ok().flatten()
}

/// Get the configured email signature
pub fn get_email_signature(db: &Database) -> Option<String> {
    db.get_setting(SETTING_EMAIL_SIGNATURE).ok().flatten()
}

/// Get the configured default subject
pub fn get_default_subject(db: &Database) -> Option<String> {
    db.get_setting(SETTING_EMAIL_DEFAULT_SUBJECT).ok().flatten()
}

// ==================== AI SETUP ====================

/// Setup AI via remote API (OpenAI-compatible)
fn setup_ai_remote(db: &Database) -> Result<()> {
    clear_screen()?;

    println!("AI [Remote API] Setup\n");
    println!("Configure an OpenAI-compatible API for AI chat features.\n");
    println!("Works with: OpenAI, Groq, Together AI, local vLLM, etc.\n");

    let config = AiConfig::load(db)?;

    // Check if already configured
    if config.provider_type == AiProviderType::Remote && config.api_key.is_some() {
        println!("Current configuration:");
        println!("  URL: {}", config.effective_api_url());
        println!("  Endpoint: {}", config.effective_api_endpoint());
        println!("  Model: {}", config.effective_model());
        println!();

        print!("[c]hange [d]isconnect [q]uit: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                println!("\n");
                // Fall through to configuration
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                AiConfig::clear(db)?;
                println!("\nAI configuration cleared.\n");
                std::thread::sleep(std::time::Duration::from_millis(800));
                return Ok(());
            }
            _ => {
                return Ok(());
            }
        }
    }

    // API Key (required, but keep existing if empty)
    let current_key = config.api_key.as_deref().unwrap_or("");
    let key_hint = if current_key.is_empty() {
        "Your API key (e.g., sk-...)".to_string()
    } else {
        // Show masked key hint
        let masked = if current_key.len() > 8 {
            format!("{}...{} (Enter to keep)", &current_key[..4], &current_key[current_key.len()-4..])
        } else {
            "(configured, Enter to keep)".to_string()
        };
        masked
    };

    let api_key_input = Text::new("API Key:")
        .with_render_config(minimal_render_config())
        .with_help_message(&key_hint)
        .prompt_skippable()?;

    let Some(api_key_input) = api_key_input else {
        return Ok(());
    };

    // Use new key if provided, otherwise keep existing
    let api_key = if api_key_input.is_empty() {
        if current_key.is_empty() {
            println!("\nAPI key is required.");
            std::thread::sleep(std::time::Duration::from_millis(800));
            return Ok(());
        }
        current_key.to_string()
    } else {
        api_key_input
    };

    // API URL (optional, defaults to OpenAI) - keep existing if empty
    let api_url_input = Text::new("API URL (optional):")
        .with_render_config(minimal_render_config())
        .with_help_message("Base URL, default: https://api.openai.com")
        .with_initial_value(config.api_url.as_deref().unwrap_or(""))
        .prompt_skippable()?;

    let api_url = match api_url_input {
        None => return Ok(()),
        Some(s) if s.is_empty() => config.api_url.clone(),
        Some(s) => Some(s),
    };

    // API Endpoint (optional, defaults to /v1/chat/completions) - keep existing if empty
    let api_endpoint_input = Text::new("API Endpoint (optional):")
        .with_render_config(minimal_render_config())
        .with_help_message("Endpoint path, default: /v1/chat/completions")
        .with_initial_value(config.api_endpoint.as_deref().unwrap_or(""))
        .prompt_skippable()?;

    let api_endpoint = match api_endpoint_input {
        None => return Ok(()),
        Some(s) if s.is_empty() => config.api_endpoint.clone(),
        Some(s) => Some(s),
    };

    // Model name (optional) - keep existing if empty
    let model_input = Text::new("Model name (optional):")
        .with_render_config(minimal_render_config())
        .with_help_message("Model to use, default: gpt-4o-mini")
        .with_initial_value(config.model.as_deref().unwrap_or(""))
        .prompt_skippable()?;

    let model = match model_input {
        None => return Ok(()),
        Some(s) if s.is_empty() => config.model.clone(),
        Some(s) => Some(s),
    };

    // Save configuration
    let new_config = AiConfig {
        provider_type: AiProviderType::Remote,
        api_key: Some(api_key),
        api_url,
        api_endpoint,
        model,
        local_model: None,
    };

    new_config.save(db)?;

    clear_screen()?;
    println!("AI Remote API configured successfully!\n");
    println!("  URL: {}", new_config.effective_api_url());
    println!("  Endpoint: {}", new_config.effective_api_endpoint());
    println!("  Model: {}", new_config.effective_model());
    println!();
    println!("You can now use AI features in the Chat interface.");
    println!();
    print!("[enter]/[q] to continue");
    io::stdout().flush()?;
    wait_for_key()?;

    Ok(())
}

/// Setup AI via local model (requires local-ai feature)
#[cfg(feature = "local-ai")]
fn setup_ai_local(db: &Database) -> Result<()> {
    use super::ai::{check_ram_for_model, local::download_model};

    clear_screen()?;

    println!("AI [Local Model] Setup\n");
    println!("Run AI models locally on your machine.\n");

    let config = AiConfig::load(db)?;

    // Check current status
    if config.provider_type == AiProviderType::Local && config.local_model.is_some() {
        let model_id = config.local_model.unwrap();
        let model_info = LocalModel::get(model_id);
        let downloaded = model_info.map(|m| m.is_downloaded()).unwrap_or(false);

        println!("Current model: {}", model_id.display_name());
        println!("Downloaded: {}", if downloaded { "Yes" } else { "No" });
        println!();

        print!("[c]hange [d]ownload [r]emove [x]disconnect [q]uit: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                println!("\n");
                // Fall through to model selection
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                println!("\n");
                if let Some(info) = model_info {
                    if !info.is_downloaded() {
                        download_model(model_id)?;
                    } else {
                        println!("Model already downloaded.");
                    }
                }
                print!("\n[enter]/[q] to continue");
                io::stdout().flush()?;
                wait_for_key()?;
                return Ok(());
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                println!("\n");
                super::ai::local::delete_model(model_id)?;
                print!("\n[enter]/[q] to continue");
                io::stdout().flush()?;
                wait_for_key()?;
                return Ok(());
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                AiConfig::clear(db)?;
                println!("\nAI configuration cleared.\n");
                std::thread::sleep(std::time::Duration::from_millis(800));
                return Ok(());
            }
            _ => {
                return Ok(());
            }
        }
    }

    // Show available models
    println!("Available models:\n");
    let models = LocalModel::all();
    let labels: Vec<String> = models
        .iter()
        .map(|m| {
            let downloaded = if m.is_downloaded() { " [downloaded]" } else { "" };
            format!("{} - {}{}", m.name, m.size_description, downloaded)
        })
        .collect();

    let mut options = labels.clone();
    options.push("Cancel".to_string());

    let selection = Select::new("Select a model:", options)
        .with_render_config(minimal_render_config())
        .with_vim_mode(true)
        .prompt_skippable()?;

    let Some(selected) = selection else {
        return Ok(());
    };

    if selected == "Cancel" {
        return Ok(());
    }

    // Find selected model
    let model_idx = labels.iter().position(|l| l == &selected);
    let Some(idx) = model_idx else {
        return Ok(());
    };

    let model = models[idx];

    // Check RAM requirements
    let (has_enough, available, required) = check_ram_for_model(model.id);

    if model.requires_warning || !has_enough {
        clear_screen()?;
        println!("Warning: {} requires at least {} GB RAM.\n", model.name, required);
        println!("Your system has {} GB RAM available.\n", available);

        if !has_enough {
            println!("This model may not run properly on your system.");
            println!("Consider choosing a smaller model.\n");
        }

        print!("[c]ontinue anyway [b]ack: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                println!("\n");
            }
            _ => {
                return Ok(());
            }
        }
    }

    // Download if not already downloaded
    if !model.is_downloaded() {
        clear_screen()?;
        println!("Model not downloaded yet.\n");
        print!("[d]ownload now [l]ater [c]ancel: ");
        io::stdout().flush()?;

        let action = {
            let _guard = RawModeGuard::new()?;
            loop {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    break code;
                }
            }
        };

        match action {
            KeyCode::Char('d') | KeyCode::Char('D') => {
                println!("\n");
                download_model(model.id)?;
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                println!("\nModel will be configured but not downloaded.");
                println!("Run setup again to download.\n");
            }
            _ => {
                return Ok(());
            }
        }
    }

    // Save configuration
    let new_config = AiConfig {
        provider_type: AiProviderType::Local,
        api_key: None,
        api_url: None,
        api_endpoint: None,
        model: None,
        local_model: Some(model.id),
    };

    new_config.save(db)?;

    clear_screen()?;
    println!("AI Local Model configured successfully!\n");
    println!("  Model: {}", model.name);
    println!("  Size: {}", model.size_description);
    println!("  Downloaded: {}", if model.is_downloaded() { "Yes" } else { "No" });
    println!();
    println!("You can now use AI features in the Chat interface.");
    println!();
    print!("[enter]/[q] to continue");
    io::stdout().flush()?;
    wait_for_key()?;

    Ok(())
}

/// Check if AI is configured
pub fn is_ai_configured(db: &Database) -> bool {
    AiConfig::load(db)
        .map(|c| c.is_configured())
        .unwrap_or(false)
}
