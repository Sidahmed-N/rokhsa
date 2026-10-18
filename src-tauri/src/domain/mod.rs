//! Business rules.
//!
//! Pure functions only: no database, no IO, no Tauri. That is what makes the
//! highest-risk logic in the product cheap to test exhaustively. See CLAUDE.md.

pub mod candidate;
pub mod money;
pub mod text;
