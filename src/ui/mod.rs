use anyhow::Result;

use crate::store::{SecretKind, SecretValue};

/// User closed the editor without saving.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// Trait for editing a secret value interactively.
///
/// Takes the secret kind (to determine UI layout) and an optional current value
/// (None for new secrets). Returns the new value or an error (e.g. user cancelled).
pub trait SecretEditor {
    fn edit(&self, kind: &SecretKind, current: Option<SecretValue>) -> Result<SecretValue>;
}

pub mod xilem_editor;

pub use xilem_editor::XilemEditor;
