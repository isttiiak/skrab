use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

/// Keychain service/account under which the SQLCipher key is stored.
///
/// The service name is deliberately different under `cfg(test)`. A test that reaches
/// the real entry would overwrite the key to a live database and make the user's
/// entire history unopenable — which is exactly what happened once during
/// development. Tests should not touch the keychain at all, and this makes the
/// blast radius zero if one ever does.
#[cfg(not(test))]
const KEYCHAIN_SERVICE: &str = "com.isttiiak.skrab";
#[cfg(test)]
const KEYCHAIN_SERVICE: &str = "com.isttiiak.skrab.test";

const KEYCHAIN_ACCOUNT: &str = "database-key";

/// Legacy location used before keychain storage existed.
const LEGACY_KEY_FILE: &str = ".dbkey";

/// Returns the hex-encoded SQLCipher key, creating it on first run.
///
/// Order of preference:
/// 1. The OS keychain (macOS Keychain Services / Windows Credential Manager).
/// 2. A `.dbkey` file next to the database, migrated into the keychain on sight.
/// 3. A newly generated key, written to the keychain if possible.
///
/// The file fallback exists because the keychain is genuinely unavailable in some
/// environments (a headless CI runner, a Linux box with no secret service). Failing
/// to start would be worse than storing the key with owner-only permissions, but the
/// keychain is always tried first and the fallback is logged loudly.
pub fn get_or_create(app_data_dir: &Path) -> Result<String> {
    if let Some(key) = keychain_get() {
        return Ok(key);
    }

    let legacy_path = app_data_dir.join(LEGACY_KEY_FILE);
    {
        let key = read_legacy_file(&legacy_path)?.unwrap_or_default();
        if !key.is_empty() {
            // Promote to the keychain, and only then drop the plaintext file — losing
            // this key means losing the entire clipboard history.
            if keychain_set(&key) {
                match fs::remove_file(&legacy_path) {
                    Ok(()) => log::info!("migrated the database key into the OS keychain"),
                    Err(e) => log::warn!("key migrated but the old file remains: {e}"),
                }
            } else {
                log::warn!("keychain unavailable; the database key stays in {LEGACY_KEY_FILE}");
            }
            return Ok(key);
        }
    }

    let key = generate_key()?;
    if !keychain_set(&key) {
        log::warn!(
            "could not reach the OS keychain; storing the database key in \
             {LEGACY_KEY_FILE} with owner-only permissions instead"
        );
        fs::create_dir_all(app_data_dir)?;
        fs::write(&legacy_path, &key)?;
        restrict_permissions(&legacy_path)?;
    } else {
        log::info!("stored a new database key in the OS keychain");
    }

    Ok(key)
}

/// Reads the pre-keychain key file, if it exists and holds something usable.
fn read_legacy_file(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)?;
    let trimmed = contents.trim();
    Ok((!trimmed.is_empty()).then(|| trimmed.to_owned()))
}

fn keychain_entry() -> Option<keyring::Entry> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(entry) => Some(entry),
        Err(e) => {
            log::debug!("keychain entry unavailable: {e}");
            None
        }
    }
}

fn keychain_get() -> Option<String> {
    let entry = keychain_entry()?;
    match entry.get_password() {
        Ok(key) if !key.trim().is_empty() => Some(key.trim().to_owned()),
        Ok(_) => None,
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            log::debug!("could not read the key from the keychain: {e}");
            None
        }
    }
}

fn keychain_set(key: &str) -> bool {
    let Some(entry) = keychain_entry() else {
        return false;
    };
    match entry.set_password(key) {
        Ok(()) => true,
        Err(e) => {
            log::debug!("could not write the key to the keychain: {e}");
            false
        }
    }
}

fn generate_key() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| Error::Random(e.to_string()))?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<()> {
    // Windows inherits the app data directory's ACL, which is already user-scoped.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encode_pads_single_digit_bytes() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xff]), "000fff");
    }

    #[test]
    fn generated_keys_are_256_bit_and_unique() {
        let a = generate_key().unwrap();
        let b = generate_key().unwrap();
        assert_eq!(a.len(), 64, "32 random bytes hex-encode to 64 chars");
        assert_ne!(a, b, "two calls must not return the same key");
    }

    /// Deliberately does not exercise `get_or_create`: that reaches the real OS
    /// keychain, which blocks on an authorization prompt in a headless test runner
    /// and turned a 0.3s suite into a 4-minute one. The keychain path is covered by
    /// the manual Phase 1 test plan instead; the pure logic is covered here.
    #[test]
    fn legacy_key_file_is_read_and_trimmed() {
        let dir = std::env::temp_dir().join(format!("skrab-key-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(LEGACY_KEY_FILE);

        assert_eq!(read_legacy_file(&path).unwrap(), None, "absent file");

        fs::write(&path, format!("  {}\n", "a".repeat(64))).unwrap();
        assert_eq!(read_legacy_file(&path).unwrap(), Some("a".repeat(64)));

        fs::write(&path, "   \n").unwrap();
        assert_eq!(
            read_legacy_file(&path).unwrap(),
            None,
            "a blank file must not be mistaken for a real key"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
