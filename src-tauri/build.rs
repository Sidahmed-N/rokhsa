fn main() {
    unlock_hash();
    tauri_build::build()
}

/// Hands the lock's hash to the app at build time (see src/license).
///
/// From the `ROKHSA_UNLOCK_HASH` environment variable (GitHub's build, from a
/// repository secret) or the gitignored file `unlock.hash` next to this one
/// (the owner's PC). Never from the repository: it is public, and the hash of
/// a short code can be attacked offline.
///
/// A release build without it stops here. An installer with no lock would be
/// a free copy, and nobody would notice until a school had one. A debug build
/// without it runs unlocked, for development.
fn unlock_hash() {
    println!("cargo:rerun-if-env-changed=ROKHSA_UNLOCK_HASH");
    println!("cargo:rerun-if-env-changed=ROKHSA_RETIRED_HASHES");
    println!("cargo:rerun-if-changed=unlock.hash");
    println!("cargo:rerun-if-changed=retired.hash");

    let from_file = |name: &str| {
        std::fs::read_to_string(name)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let from_env = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let hash = from_env("ROKHSA_UNLOCK_HASH").or_else(|| from_file("unlock.hash"));
    match &hash {
        Some(hash) => println!("cargo:rustc-env=ROKHSA_UNLOCK_HASH={hash}"),
        None if std::env::var("PROFILE").as_deref() == Ok("release") => panic!(
            "No unlock hash: set ROKHSA_UNLOCK_HASH or create src-tauri/unlock.hash. \
             A release without it would ship with no lock."
        ),
        None => {}
    }

    let retired = from_env("ROKHSA_RETIRED_HASHES").or_else(|| from_file("retired.hash"));
    if let Some(retired) = retired {
        // One line in the environment: hashes are separated by whitespace.
        let joined = retired.split_whitespace().collect::<Vec<_>>().join(" ");
        println!("cargo:rustc-env=ROKHSA_RETIRED_HASHES={joined}");
    }
}
