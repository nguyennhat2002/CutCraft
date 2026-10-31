//! Session-level state for native project dialogs and navigation guards.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    NewProject,
    OpenProject,
    ReturnHome,
    Exit,
}

impl PendingAction {
    pub fn description(self) -> &'static str {
        match self {
            Self::NewProject => "create a new project",
            Self::OpenProject => "open another project",
            Self::ReturnHome => "return home",
            Self::Exit => "exit CutCraft",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDialogMode {
    Open,
    SaveAs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProjectDialog {
    mode: NativeDialogMode,
    path: String,
    error: Option<String>,
    pending_after_save: Option<PendingAction>,
}

impl NativeProjectDialog {
    pub fn open() -> Self {
        Self {
            mode: NativeDialogMode::Open,
            path: String::new(),
            error: None,
            pending_after_save: None,
        }
    }

    pub fn save_as(default_path: String, pending_after_save: Option<PendingAction>) -> Self {
        Self {
            mode: NativeDialogMode::SaveAs,
            path: default_path,
            error: None,
            pending_after_save,
        }
    }

    pub fn mode(&self) -> NativeDialogMode {
        self.mode
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn pending_after_save(&self) -> Option<PendingAction> {
        self.pending_after_save
    }

    pub fn clear_pending_after_save(&mut self) {
        self.pending_after_save = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavedPrompt {
    action: PendingAction,
    error: Option<String>,
}

impl UnsavedPrompt {
    pub fn new(action: PendingAction) -> Self {
        Self {
            action,
            error: None,
        }
    }

    pub fn action(&self) -> PendingAction {
        self.action
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }
}

pub fn default_save_path(project_name: &str) -> String {
    let base = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.join("Documents").join("CutCraft"))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.join("CutCraft"))
        })
        .unwrap_or_else(|| PathBuf::from("CutCraft"));
    base.join(format!("{}.cutcraft", safe_file_stem(project_name)))
        .to_string_lossy()
        .into_owned()
}

fn safe_file_stem(project_name: &str) -> String {
    let mut stem: String = project_name
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '-'
            } else {
                character
            }
        })
        .take(80)
        .collect();
    stem = stem
        .trim_matches(|character: char| character.is_whitespace() || character == '.')
        .to_owned();
    if stem.is_empty() {
        "Untitled Project".to_owned()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_paths_have_cutcraft_extension_and_safe_stems() {
        assert!(default_save_path("My Project").ends_with("My Project.cutcraft"));
        assert!(default_save_path("A:B/C?").ends_with("A-B-C-.cutcraft"));
        assert!(default_save_path("...").ends_with("Untitled Project.cutcraft"));
        assert!(!default_save_path("Control\u{0007}Name").contains('\u{0007}'));
    }

    #[test]
    fn pending_actions_and_dialog_modes_are_explicit() {
        let dialog = NativeProjectDialog::save_as(
            "C:\\tmp\\a.cutcraft".to_owned(),
            Some(PendingAction::ReturnHome),
        );
        assert_eq!(dialog.mode(), NativeDialogMode::SaveAs);
        assert_eq!(dialog.pending_after_save(), Some(PendingAction::ReturnHome));
        assert_eq!(dialog.error(), None);
        let mut open = NativeProjectDialog::open();
        open.set_error("bad file");
        assert_eq!(open.error(), Some("bad file"));
        assert_eq!(PendingAction::Exit.description(), "exit CutCraft");
    }
}
