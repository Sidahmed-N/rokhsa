//! Turns the owner's unlock code into the hash the app is built with.
//!
//!   cargo run --bin unlock_hash > unlock.hash
//!   (then type the code and press Enter)
//!
//!   cargo run --bin unlock_hash -- --check unlock.hash
//!   (type a code: says whether it is the one that hash was made from)
//!
//! The code is read from standard input so it is never written to a file or
//! left in the shell history. Only the hash is printed; see src/license and
//! build.rs for where it goes. Changing the code later: move the old hash
//! into retired.hash first, so schools already unlocked stay unlocked.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

fn read_code() -> String {
    let mut code = String::new();
    std::io::stdin()
        .read_line(&mut code)
        .expect("could not read the code");
    // Windows PowerShell can put an invisible byte-order mark in front of
    // piped text; hashed with it, the real code would never match.
    let code = code.trim().trim_start_matches('\u{feff}').trim().to_string();
    if code.is_empty() {
        eprintln!("No code given.");
        std::process::exit(1);
    }
    code
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--check") {
        let path = args.get(2).map(String::as_str).unwrap_or("unlock.hash");
        let stored = std::fs::read_to_string(path).expect("could not read the hash file");
        let parsed = PasswordHash::new(stored.trim()).expect("not a valid hash");
        let code = read_code();
        let ok = Argon2::default().verify_password(code.as_bytes(), &parsed).is_ok();
        println!("{}", if ok { "MATCH" } else { "NO MATCH" });
        std::process::exit(if ok { 0 } else { 2 });
    }

    let code = read_code();
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(code.as_bytes(), &salt)
        .expect("could not hash the code");
    println!("{hash}");
}
