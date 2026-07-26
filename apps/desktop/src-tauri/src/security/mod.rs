//! Decides what must never reach the database.
//!
//! This runs *before* the first write to disk. A clip rejected here leaves no trace:
//! no row, no image file, no FTS entry. That ordering is the whole point — filtering
//! after persistence would still have written the secret down.

use crate::settings::AppSettings;

/// Why a clip was rejected. Logged (without the payload) for debuggability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The OS marked the payload concealed — a password manager set the flag.
    ConcealedByOs,
    /// The source app is on the user's blocklist.
    BlockedApp,
    /// The text matches a high-confidence secret shape.
    LooksLikeSecret,
    /// Nothing to store.
    Empty,
}

impl Rejection {
    pub fn as_str(self) -> &'static str {
        match self {
            Rejection::ConcealedByOs => "concealed by OS marker",
            Rejection::BlockedApp => "source app is blocklisted",
            Rejection::LooksLikeSecret => "matches a secret pattern",
            Rejection::Empty => "empty payload",
        }
    }
}

/// Should this text clip be stored?
pub fn screen_text(
    text: &str,
    source_app: Option<&str>,
    os_concealed: bool,
    settings: &AppSettings,
) -> Result<(), Rejection> {
    if os_concealed {
        return Err(Rejection::ConcealedByOs);
    }
    if text.trim().is_empty() {
        return Err(Rejection::Empty);
    }
    if is_blocked_app(source_app, settings) {
        return Err(Rejection::BlockedApp);
    }
    if settings.skip_secret_patterns && looks_like_secret(text) {
        return Err(Rejection::LooksLikeSecret);
    }
    Ok(())
}

/// Should this image clip be stored?
pub fn screen_image(
    source_app: Option<&str>,
    os_concealed: bool,
    settings: &AppSettings,
) -> Result<(), Rejection> {
    if os_concealed {
        return Err(Rejection::ConcealedByOs);
    }
    if is_blocked_app(source_app, settings) {
        return Err(Rejection::BlockedApp);
    }
    Ok(())
}

fn is_blocked_app(source_app: Option<&str>, settings: &AppSettings) -> bool {
    let Some(app) = source_app else {
        return false;
    };
    let app = app.to_lowercase();
    settings
        .blocked_apps
        .iter()
        .filter(|b| !b.trim().is_empty())
        .any(|blocked| app.contains(&blocked.to_lowercase()))
}

/// High-confidence secret shapes only.
///
/// This is a deliberately narrow net. The OS concealed-type marker is the real
/// defence; this is a second line for apps that do not set it. Being aggressive here
/// would silently swallow ordinary clips — a user who copies a long random string and
/// finds it missing from history has a worse experience than one who copies a token
/// that got recorded. Every rule below keys off a distinctive prefix, not entropy.
pub fn looks_like_secret(text: &str) -> bool {
    let t = text.trim();

    // Multi-line content is essentially never a bare credential, except for keys.
    if t.contains('\n') {
        return t.contains("-----BEGIN") && t.contains("PRIVATE KEY");
    }

    if t.len() > 512 {
        return false;
    }

    // A credential is a single opaque token. Prose that merely *mentions* a prefix
    // ("AKIA is a prefix used by AWS access keys") contains whitespace and must not
    // be flagged — that class of false positive silently eats ordinary clips.
    if t.split_whitespace().count() != 1 {
        return false;
    }

    // AWS key ids are exactly 20 uppercase alphanumerics. Checking the shape rather
    // than the prefix alone avoids flagging words that merely start with AKIA.
    if (t.starts_with("AKIA") || t.starts_with("ASIA"))
        && t.len() == 20
        && t.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return true;
    }

    // Everything else keys off a distinctive prefix plus enough length to be a real
    // token — a bare "sk-" or "npm_" is a fragment, not a credential.
    const MIN_TOKEN_LEN: usize = 16;

    const SECRET_PREFIXES: &[&str] = &[
        "sk-",         // OpenAI / Anthropic style
        "sk_live_",    // Stripe live secret
        "sk_test_",    // Stripe test secret
        "rk_live_",    // Stripe restricted
        "ghp_",        // GitHub personal access token
        "gho_",        // GitHub OAuth
        "ghs_",        // GitHub server-to-server
        "github_pat_", // GitHub fine-grained PAT
        "xoxb-",       // Slack bot token
        "xoxp-",       // Slack user token
        "AIza",        // Google API key
        "glpat-",      // GitLab PAT
        "npm_",        // npm automation token
        "dop_v1_",     // DigitalOcean
        "-----BEGIN",  // PEM block on one line
    ];

    if t.len() >= MIN_TOKEN_LEN && SECRET_PREFIXES.iter().any(|p| t.starts_with(p)) {
        return true;
    }

    // JWTs: three base64url segments separated by dots, starting with a JOSE header.
    if t.starts_with("eyJ") && t.matches('.').count() == 2 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> AppSettings {
        AppSettings::default()
    }

    #[test]
    fn os_concealed_marker_wins_over_everything() {
        let err = screen_text("hello", None, true, &settings()).unwrap_err();
        assert_eq!(err, Rejection::ConcealedByOs);
    }

    #[test]
    fn ordinary_text_is_stored() {
        assert!(screen_text("meeting at 3pm", Some("Slack"), false, &settings()).is_ok());
    }

    #[test]
    fn blank_text_is_rejected() {
        assert_eq!(
            screen_text("   \n ", None, false, &settings()).unwrap_err(),
            Rejection::Empty
        );
    }

    #[test]
    fn blocklist_matches_case_insensitively_on_a_substring() {
        let s = AppSettings {
            blocked_apps: vec!["1password".to_owned()],
            ..Default::default()
        };
        assert_eq!(
            screen_text("anything", Some("1Password 8"), false, &s).unwrap_err(),
            Rejection::BlockedApp
        );
        assert!(screen_text("anything", Some("Safari"), false, &s).is_ok());
    }

    #[test]
    fn empty_blocklist_entries_do_not_match_everything() {
        // A stray blank row in settings must not silently disable all recording.
        let s = AppSettings {
            blocked_apps: vec!["".to_owned(), "  ".to_owned()],
            ..Default::default()
        };
        assert!(screen_text("hello", Some("Safari"), false, &s).is_ok());
    }

    #[test]
    fn recognizes_common_token_shapes() {
        for token in [
            "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
            "sk-proj-abcdef123456",
            "AKIAIOSFODNN7EXAMPLE",
            "xoxb-123456789012-abcdefghijkl",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc123",
        ] {
            assert!(looks_like_secret(token), "should flag: {token}");
        }
    }

    #[test]
    fn does_not_flag_ordinary_text() {
        for text in [
            "https://github.com/isttiiak/skrab",
            "The quick brown fox jumps over the lazy dog",
            "skrab is a clipboard manager",
            "select * from clip_items where id = 1",
            // Prose that merely mentions a token prefix must not be swallowed.
            "AKIA is a prefix used by AWS access keys",
            "use ghp_ tokens for GitHub authentication",
            // Fragments are not credentials.
            "sk-",
            "npm_",
        ] {
            assert!(!looks_like_secret(text), "should NOT flag: {text}");
        }
    }

    #[test]
    fn flags_a_pasted_private_key_block() {
        let key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...\n-----END RSA PRIVATE KEY-----";
        assert!(looks_like_secret(key));
    }

    #[test]
    fn multiline_prose_is_never_a_secret() {
        assert!(!looks_like_secret(
            "dear team,\n\nplease review the PR.\n\nthanks"
        ));
    }

    #[test]
    fn secret_screening_can_be_switched_off() {
        let s = AppSettings {
            skip_secret_patterns: false,
            ..Default::default()
        };
        assert!(screen_text("ghp_abcdef123456", None, false, &s).is_ok());
    }
}
