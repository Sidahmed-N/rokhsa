//! The one-code lock.
//!
//! The owner installs every copy himself and types one code, known only to
//! him, the first time the app starts on a school's PC. After that the app
//! opens normally on that PC forever: updates, restarts and power cuts never
//! ask again. A copy taken to another PC — the installer, or the whole data
//! folder — asks for the code again, and the school does not know it.
//!
//! How:
//! - The app never contains the code, only an Argon2id hash of it, given at
//!   build time (`ROKHSA_UNLOCK_HASH`, see build.rs) and kept out of the
//!   public repository.
//! - When the right code is typed, the code itself is saved in the data
//!   folder encrypted with Windows DPAPI. DPAPI ties the encryption to this
//!   Windows user on this PC, so the file is unreadable anywhere else, and it
//!   cannot be forged without knowing the code. At each start the app
//!   decrypts it and checks it against the hash again.
//! - A reinstalled Windows is, to DPAPI, another PC: the code is asked again.
//!
//! Locked is read-only, never hidden: a school's records are its legal and
//! financial papers, so a locked copy still opens, shows everything and can
//! still be backed up. It cannot add or change anything — enforced in the
//! command layer (`AppState::require_unlocked`), not in the screen.
//!
//! No protection is unbreakable. This stops what actually happens: the app
//! copied to a friend's PC.

use std::path::Path;

use argon2::{Argon2, PasswordHash, PasswordVerifier};

use crate::error::AppError;

/// The encrypted code, in the data folder next to the database.
const ACTIVATION_FILE: &str = "activation.bin";

/// Mixed into DPAPI so another program of the same user cannot decrypt the
/// file by accident. Not a secret.
const ENTROPY: &[u8] = b"rokhsa-activation-v1";

/// The hash given at build time. `None` only in a debug build made without
/// one, which is then left unlocked for development; a release build without
/// it fails in build.rs, so no school ever gets an app with no lock.
fn expected_hash() -> Option<&'static str> {
    option_env!("ROKHSA_UNLOCK_HASH").filter(|h| !h.trim().is_empty())
}

/// Hashes of codes used before, if the code was ever changed (a leak). A PC
/// already unlocked with one of them stays unlocked — an update must never
/// lock out a school that paid — but none of them unlocks a new PC.
/// Whitespace-separated, given at build time like the current one.
fn retired_hashes() -> Vec<&'static str> {
    option_env!("ROKHSA_RETIRED_HASHES")
        .map(|all| all.split_whitespace().collect())
        .unwrap_or_default()
}

/// Whether this build has a lock at all.
pub fn required() -> bool {
    expected_hash().is_some()
}

/// Whether the app is unlocked on this PC.
pub fn is_unlocked(data_dir: &Path) -> bool {
    match expected_hash() {
        Some(hash) => is_unlocked_with(hash, &retired_hashes(), data_dir),
        None => true,
    }
}

/// Check the typed code and, if right, unlock this PC for good.
pub fn unlock(data_dir: &Path, code: &str) -> Result<(), AppError> {
    match expected_hash() {
        Some(hash) => unlock_with(hash, data_dir, code),
        None => Ok(()),
    }
}

fn verify(hash: &str, code: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash.trim()) else {
        return false;
    };
    Argon2::default()
        .verify_password(code.trim().as_bytes(), &parsed)
        .is_ok()
}

fn is_unlocked_with(hash: &str, retired: &[&str], data_dir: &Path) -> bool {
    let Ok(sealed) = std::fs::read(data_dir.join(ACTIVATION_FILE)) else {
        return false;
    };
    let Some(code) = dpapi::unprotect(&sealed) else {
        return false;
    };
    let Ok(code) = String::from_utf8(code) else {
        return false;
    };
    verify(hash, &code) || retired.iter().any(|old| verify(old, &code))
}

fn unlock_with(hash: &str, data_dir: &Path, code: &str) -> Result<(), AppError> {
    if !verify(hash, code) {
        return Err(AppError::validation("license.wrongCode"));
    }
    let sealed = dpapi::protect(code.trim().as_bytes())
        .ok_or_else(|| AppError::Internal("could not protect the activation".into()))?;

    // Written under another name and renamed, like a backup: a power cut
    // mid-write must not leave a half file that reads as "locked".
    std::fs::create_dir_all(data_dir)
        .map_err(|e| AppError::Internal(format!("cannot create data directory: {e}")))?;
    let partial = data_dir.join(format!("{ACTIVATION_FILE}.partial"));
    std::fs::write(&partial, &sealed)
        .and_then(|_| std::fs::rename(&partial, data_dir.join(ACTIVATION_FILE)))
        .map_err(|e| AppError::Internal(format!("cannot save the activation: {e}")))?;

    if is_unlocked_with(hash, &[], data_dir) {
        Ok(())
    } else {
        Err(AppError::Internal("activation saved but not readable".into()))
    }
}

#[cfg(windows)]
mod dpapi {
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    use super::ENTROPY;

    fn blob(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        }
    }

    /// Copy DPAPI's output and free it.
    ///
    /// SAFETY: `out` must be a blob DPAPI filled in, allocated with LocalAlloc.
    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as _);
        bytes
    }

    pub fn protect(plain: &[u8]) -> Option<Vec<u8>> {
        let input = blob(plain);
        let entropy = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        // SAFETY: every pointer is valid for the call; `out` is read only on
        // success, when DPAPI has allocated it.
        unsafe {
            let ok = CryptProtectData(
                &input,
                null(),
                &entropy,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            );
            (ok != 0).then(|| take(out))
        }
    }

    pub fn unprotect(sealed: &[u8]) -> Option<Vec<u8>> {
        let input = blob(sealed);
        let entropy = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        // SAFETY: as in `protect`.
        unsafe {
            let ok = CryptUnprotectData(
                &input,
                null_mut(),
                &entropy,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            );
            (ok != 0).then(|| take(out))
        }
    }
}

/// The app ships for Windows only; elsewhere (a developer's machine) the file
/// is not bound to the PC.
#[cfg(not(windows))]
mod dpapi {
    pub fn protect(plain: &[u8]) -> Option<Vec<u8>> {
        Some(plain.to_vec())
    }
    pub fn unprotect(sealed: &[u8]) -> Option<Vec<u8>> {
        Some(sealed.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};

    fn hash_of(code: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(code.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rokhsa-license-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_new_pc_is_locked_until_the_right_code() {
        let hash = hash_of("open-sesame-2026");
        let dir = temp_dir("new");

        assert!(!is_unlocked_with(&hash, &[], &dir));
        assert!(unlock_with(&hash, &dir, "wrong").is_err());
        assert!(!is_unlocked_with(&hash, &[], &dir));

        unlock_with(&hash, &dir, "open-sesame-2026").unwrap();
        // And it stays unlocked at every later start.
        assert!(is_unlocked_with(&hash, &[], &dir));
        assert!(is_unlocked_with(&hash, &[], &dir));
    }

    /// Spaces typed around the code by mistake do not count against it.
    #[test]
    fn surrounding_spaces_are_forgiven() {
        let hash = hash_of("open-sesame-2026");
        let dir = temp_dir("spaces");
        unlock_with(&hash, &dir, "  open-sesame-2026 ").unwrap();
        assert!(is_unlocked_with(&hash, &[], &dir));
    }

    /// A file that is not a sealed right code — written by hand, or sealed
    /// on another PC (which DPAPI cannot open here) — leaves the app locked.
    #[test]
    fn a_forged_or_foreign_activation_file_does_not_unlock() {
        let hash = hash_of("open-sesame-2026");
        let dir = temp_dir("forged");

        std::fs::write(dir.join(ACTIVATION_FILE), b"open-sesame-2026").unwrap();
        assert!(!is_unlocked_with(&hash, &[], &dir));

        // Sealed properly, but a wrong code: still locked.
        std::fs::write(dir.join(ACTIVATION_FILE), dpapi::protect(b"guess").unwrap()).unwrap();
        assert!(!is_unlocked_with(&hash, &[], &dir));
    }

    /// After a leak the code changes: schools already unlocked with the old
    /// one stay unlocked, but the old code no longer unlocks a new PC.
    #[test]
    fn a_retired_code_keeps_old_pcs_but_unlocks_no_new_one() {
        let (first, second) = (hash_of("first-code"), hash_of("second-code"));

        let school = temp_dir("rotated-school");
        unlock_with(&first, &school, "first-code").unwrap();
        assert!(is_unlocked_with(&second, &[first.as_str()], &school));
        // Without the old hash kept, that school would have been locked out.
        assert!(!is_unlocked_with(&second, &[], &school));

        let new_pc = temp_dir("rotated-new");
        assert!(unlock_with(&second, &new_pc, "first-code").is_err());
        unlock_with(&second, &new_pc, "second-code").unwrap();
    }
}
