//! Active shortcut profiles, pending edits, conflict detection, and save.

use std::collections::{BTreeMap, BTreeSet};

use super::binding::ShortcutBinding;
use super::persistence::{PersistedShortcuts, ShortcutSaveError, ShortcutStore, FORMAT_VERSION};
use super::profile::ShortcutProfile;
use super::registry::{contexts_overlap, default_bindings, spec, CommandId, COMMAND_SPECS};

#[derive(Debug, Clone)]
pub struct ShortcutManager {
    active_profile: ShortcutProfile,
    custom: BTreeMap<CommandId, Option<ShortcutBinding>>,
    store: Option<ShortcutStore>,
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self {
            active_profile: ShortcutProfile::Default,
            custom: default_bindings(),
            store: None,
        }
    }
}

impl ShortcutManager {
    /// Load the user's configuration, falling back to complete defaults on any
    /// error. The warning is surfaced in the status bar by the application.
    pub fn load(store: Option<ShortcutStore>) -> (Self, Option<String>) {
        let defaults = default_bindings();
        let mut manager = Self {
            active_profile: ShortcutProfile::Default,
            custom: defaults.clone(),
            store: store.clone(),
        };
        let Some(store) = store else {
            return (manager, None);
        };

        let mut warnings = Vec::new();
        match store.load() {
            Ok(None) => {}
            Err(error) => warnings.push(format!(
                "Shortcut configuration could not be loaded: {error}. Using defaults."
            )),
            Ok(Some(file)) => {
                if file.version != FORMAT_VERSION {
                    warnings.push(format!(
                        "Unsupported shortcut configuration version {}. Using defaults.",
                        file.version
                    ));
                } else {
                    manager.active_profile = ShortcutProfile::parse(&file.active_profile)
                        .unwrap_or_else(|| {
                            warnings.push(
                                "Unknown shortcut profile in configuration; using defaults."
                                    .to_owned(),
                            );
                            ShortcutProfile::Default
                        });
                    let mut seen = BTreeSet::new();
                    for (raw_id, raw_binding) in file.custom_bindings {
                        let Some(command) = CommandId::from_stable_id(&raw_id) else {
                            warnings
                                .push(format!("Ignored shortcut for unknown command '{raw_id}'."));
                            continue;
                        };
                        seen.insert(command);
                        match raw_binding {
                            None => {
                                manager.custom.insert(command, None);
                            }
                            Some(raw) => match ShortcutBinding::parse(&raw) {
                                Ok(binding) => {
                                    manager.custom.insert(command, Some(binding));
                                }
                                Err(error) => warnings.push(format!(
                                    "Ignored shortcut '{}' for {}: {error}. Using default.",
                                    raw,
                                    command.label()
                                )),
                            },
                        }
                    }
                    for spec in COMMAND_SPECS {
                        if !seen.contains(&spec.id) {
                            manager.custom.insert(spec.id, spec.default_binding);
                        }
                    }
                    let conflicts = resolve_conflicts(&mut manager.custom);
                    for (command, binding) in conflicts {
                        warnings.push(format!(
                            "Ignored duplicate shortcut {} for {}.",
                            binding.label(),
                            command.label()
                        ));
                    }
                }
            }
        }

        let warning = (!warnings.is_empty()).then(|| warnings.join(" "));
        (manager, warning)
    }

    pub fn active_profile(&self) -> ShortcutProfile {
        self.active_profile
    }

    pub fn custom_bindings(&self) -> &BTreeMap<CommandId, Option<ShortcutBinding>> {
        &self.custom
    }

    pub fn binding(&self, command: CommandId) -> Option<ShortcutBinding> {
        match self.active_profile {
            ShortcutProfile::Default => spec(command).and_then(|spec| spec.default_binding),
            ShortcutProfile::Custom => self.custom.get(&command).copied().flatten(),
        }
    }

    pub fn binding_label(&self, command: CommandId) -> String {
        self.binding(command)
            .map(ShortcutBinding::label)
            .unwrap_or_else(|| "Unassigned".to_owned())
    }

    /// Human-readable shortcut suffix for tooltips and menu labels.
    pub fn shortcut_hint(&self, command: CommandId) -> Option<String> {
        self.binding(command).map(|binding| binding.label())
    }

    pub fn bindings(&self) -> BTreeMap<CommandId, Option<ShortcutBinding>> {
        match self.active_profile {
            ShortcutProfile::Default => default_bindings(),
            ShortcutProfile::Custom => self.custom.clone(),
        }
    }

    pub fn store_path(&self) -> Option<&std::path::Path> {
        self.store.as_ref().map(ShortcutStore::path)
    }

    /// Persist a pending draft, then make it active. The manager is unchanged if
    /// validation or disk replacement fails.
    pub fn save_draft(&mut self, draft: &ShortcutDraft) -> Result<(), ShortcutSaveError> {
        let conflicts = duplicate_conflicts(&draft.custom);
        if !conflicts.is_empty() {
            return Err(ShortcutSaveError::Validation(
                conflicts
                    .iter()
                    .map(|(left, right)| {
                        format!(
                            "{} and {} share an overlapping binding",
                            left.label(),
                            right.label()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }

        if let Some(store) = &self.store {
            let persisted = PersistedShortcuts {
                version: FORMAT_VERSION,
                active_profile: draft.profile.as_str().to_owned(),
                custom_bindings: draft
                    .custom
                    .iter()
                    .map(|(command, binding)| {
                        (
                            command.id().to_owned(),
                            binding.map(ShortcutBinding::serialized),
                        )
                    })
                    .collect(),
            };
            store.save(&persisted)?;
        }

        self.active_profile = draft.profile;
        self.custom = draft.custom.clone();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ShortcutDraft {
    pub profile: ShortcutProfile,
    custom: BTreeMap<CommandId, Option<ShortcutBinding>>,
}

impl ShortcutDraft {
    pub fn new(manager: &ShortcutManager) -> Self {
        Self {
            profile: manager.active_profile,
            custom: manager.custom.clone(),
        }
    }

    pub fn binding(&self, command: CommandId) -> Option<ShortcutBinding> {
        match self.profile {
            ShortcutProfile::Default => spec(command).and_then(|spec| spec.default_binding),
            ShortcutProfile::Custom => self.custom.get(&command).copied().flatten(),
        }
    }

    pub fn bindings(&self) -> BTreeMap<CommandId, Option<ShortcutBinding>> {
        match self.profile {
            ShortcutProfile::Default => default_bindings(),
            ShortcutProfile::Custom => self.custom.clone(),
        }
    }

    /// Editing always targets the Custom profile, immediately switching away
    /// from the read-only default preset when needed.
    pub fn begin_custom(&mut self) {
        if self.profile == ShortcutProfile::Default {
            self.profile = ShortcutProfile::Custom;
        }
    }

    pub fn set_binding(&mut self, command: CommandId, binding: ShortcutBinding) {
        self.begin_custom();
        self.custom.insert(command, Some(binding));
    }

    pub fn clear_binding(&mut self, command: CommandId) {
        self.begin_custom();
        self.custom.insert(command, None);
    }

    pub fn reset_to_defaults(&mut self) {
        self.custom = default_bindings();
    }

    /// Other commands whose active bindings collide in an overlapping context.
    pub fn conflicts_for(&self, command: CommandId, binding: ShortcutBinding) -> Vec<CommandId> {
        let Some(candidate) = spec(command) else {
            return Vec::new();
        };
        let bindings = self.bindings();
        COMMAND_SPECS
            .iter()
            .filter(|other| other.id != command)
            .filter(|other| {
                bindings
                    .get(&other.id)
                    .copied()
                    .flatten()
                    .is_some_and(|current| current == binding)
            })
            .filter(|other| contexts_overlap(candidate.context, other.context))
            .map(|other| other.id)
            .collect()
    }

    /// Explicit reassignment clears every conflicting binding in the pending
    /// custom profile before assigning the requested one.
    pub fn reassign(
        &mut self,
        command: CommandId,
        binding: ShortcutBinding,
        conflicts: &[CommandId],
    ) {
        self.begin_custom();
        for conflicting in conflicts {
            self.custom.insert(*conflicting, None);
        }
        self.custom.insert(command, Some(binding));
    }

    pub fn custom_bindings(&self) -> &BTreeMap<CommandId, Option<ShortcutBinding>> {
        &self.custom
    }

    pub fn is_custom_dirty(&self, manager: &ShortcutManager) -> bool {
        self.profile != manager.active_profile || self.custom != manager.custom
    }
}

/// Clear later duplicates that overlap, returning the commands that lost theirs.
fn resolve_conflicts(
    bindings: &mut BTreeMap<CommandId, Option<ShortcutBinding>>,
) -> Vec<(CommandId, ShortcutBinding)> {
    let mut accepted: Vec<(CommandId, ShortcutBinding, super::registry::CommandContext)> =
        Vec::new();
    let mut cleared = Vec::new();
    for spec in COMMAND_SPECS {
        let Some(binding) = bindings.get(&spec.id).copied().flatten() else {
            continue;
        };
        let conflicts = accepted.iter().any(|(_, other_binding, context)| {
            *other_binding == binding && contexts_overlap(spec.context, *context)
        });
        if conflicts {
            bindings.insert(spec.id, None);
            cleared.push((spec.id, binding));
        } else {
            accepted.push((spec.id, binding, spec.context));
        }
    }
    cleared
}

fn duplicate_conflicts(
    bindings: &BTreeMap<CommandId, Option<ShortcutBinding>>,
) -> Vec<(CommandId, CommandId)> {
    let mut conflicts = Vec::new();
    for (index, left) in COMMAND_SPECS.iter().enumerate() {
        let Some(left_binding) = bindings.get(&left.id).copied().flatten() else {
            continue;
        };
        for right in COMMAND_SPECS.iter().skip(index + 1) {
            let Some(right_binding) = bindings.get(&right.id).copied().flatten() else {
                continue;
            };
            if left_binding == right_binding && contexts_overlap(left.context, right.context) {
                conflicts.push((left.id, right.id));
            }
        }
    }
    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::persistence::ShortcutStore;
    use egui::Key;

    fn binding(key: Key, ctrl: bool) -> ShortcutBinding {
        ShortcutBinding::new(key, ctrl, false, false)
    }

    fn alt_binding(key: Key, ctrl: bool) -> ShortcutBinding {
        ShortcutBinding::new(key, ctrl, false, true)
    }

    #[test]
    fn default_profile_resolves_registered_defaults() {
        let manager = ShortcutManager::default();
        assert_eq!(manager.active_profile(), ShortcutProfile::Default);
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            Some(binding(Key::Equals, true))
        );
        assert_eq!(
            manager.binding(CommandId::NewProject),
            Some(binding(Key::N, true))
        );
        assert_eq!(manager.binding_label(CommandId::ZoomIn), "Ctrl+=");
        assert_eq!(manager.binding(CommandId::AddTrack), None);
    }

    #[test]
    fn custom_assignment_only_becomes_active_after_save() {
        let mut manager = ShortcutManager::default();
        let mut draft = ShortcutDraft::new(&manager);
        draft.set_binding(CommandId::ZoomIn, alt_binding(Key::Z, true));
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            Some(binding(Key::Equals, true))
        );

        manager.save_draft(&draft).expect("save draft");
        assert_eq!(manager.active_profile(), ShortcutProfile::Custom);
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            Some(alt_binding(Key::Z, true))
        );
    }

    #[test]
    fn conflicts_are_detected_only_in_overlapping_contexts() {
        let draft = ShortcutDraft::new(&ShortcutManager::default());
        let conflicts = draft.conflicts_for(CommandId::ZoomIn, binding(Key::E, true));
        assert!(conflicts.contains(&CommandId::Export));

        let nonoverlap = draft.conflicts_for(CommandId::NewProject, binding(Key::E, true));
        assert!(nonoverlap.is_empty(), "Homepage and Editor never co-exist");
    }

    #[test]
    fn reassigning_clears_conflicting_bindings() {
        let mut draft = ShortcutDraft::new(&ShortcutManager::default());
        let conflicts = draft.conflicts_for(CommandId::ZoomIn, binding(Key::E, true));
        draft.reassign(CommandId::ZoomIn, binding(Key::E, true), &conflicts);
        assert_eq!(draft.binding(CommandId::Export), None);
        assert_eq!(
            draft.binding(CommandId::ZoomIn),
            Some(binding(Key::E, true))
        );
        assert_eq!(draft.profile, ShortcutProfile::Custom);
    }

    #[test]
    fn clear_binding_switches_to_custom_and_persists_absence() {
        let mut manager = ShortcutManager::default();
        let mut draft = ShortcutDraft::new(&manager);
        draft.clear_binding(CommandId::SnappingToggle);
        assert_eq!(draft.profile, ShortcutProfile::Custom);
        assert_eq!(draft.binding(CommandId::SnappingToggle), None);
        manager.save_draft(&draft).expect("save");
        assert_eq!(manager.binding(CommandId::SnappingToggle), None);
    }

    #[test]
    fn reset_restores_defaults_in_the_pending_custom_profile() {
        let mut draft = ShortcutDraft::new(&ShortcutManager::default());
        draft.set_binding(CommandId::ZoomIn, binding(Key::Z, true));
        draft.clear_binding(CommandId::LinkedSelection);
        draft.reset_to_defaults();
        assert_eq!(draft.profile, ShortcutProfile::Custom);
        assert_eq!(
            draft.binding(CommandId::ZoomIn),
            Some(binding(Key::Equals, true))
        );
        assert_eq!(
            draft.binding(CommandId::LinkedSelection),
            Some(binding(Key::L, true))
        );
    }

    #[test]
    fn unsaved_draft_edits_leave_the_manager_untouched() {
        let manager = ShortcutManager::default();
        let mut draft = ShortcutDraft::new(&manager);
        draft.set_binding(CommandId::ZoomIn, binding(Key::Z, true));
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            Some(binding(Key::Equals, true))
        );
        assert!(draft.is_custom_dirty(&manager));
    }

    #[test]
    fn nonoverlapping_duplicates_are_valid_and_both_bindings_are_retained() {
        let mut manager = ShortcutManager::default();
        let mut draft = ShortcutDraft::new(&manager);
        draft.set_binding(CommandId::NewProject, binding(Key::E, true));
        assert!(draft
            .conflicts_for(CommandId::NewProject, binding(Key::E, true))
            .is_empty());
        manager
            .save_draft(&draft)
            .expect("nonoverlapping duplicate is valid");
        assert_eq!(
            manager.binding(CommandId::NewProject),
            Some(binding(Key::E, true))
        );
        assert_eq!(
            manager.binding(CommandId::Export),
            Some(binding(Key::E, true))
        );
    }

    #[test]
    fn profile_and_custom_bindings_survive_store_reload() {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-manager-reload-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let store = ShortcutStore::new(&path);
        let (mut manager, warning) = ShortcutManager::load(Some(store.clone()));
        assert!(warning.is_none());

        let mut draft = ShortcutDraft::new(&manager);
        draft.set_binding(CommandId::ZoomIn, alt_binding(Key::Z, true));
        manager.save_draft(&draft).expect("save custom profile");

        let (reloaded, warning) = ShortcutManager::load(Some(store));
        assert!(warning.is_none(), "unexpected warning: {warning:?}");
        assert_eq!(reloaded.active_profile(), ShortcutProfile::Custom);
        assert_eq!(
            reloaded.binding(CommandId::ZoomIn),
            Some(alt_binding(Key::Z, true))
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_store_falls_back_to_defaults_with_a_warning() {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-manager-corrupt-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, b"not-json").expect("write corrupt config");
        let (manager, warning) = ShortcutManager::load(Some(ShortcutStore::new(&path)));
        assert!(warning.is_some());
        assert_eq!(manager.active_profile(), ShortcutProfile::Default);
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            Some(binding(Key::Equals, true))
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loaded_duplicate_bindings_are_sanitized_and_warned() {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-manager-duplicate-{}.json",
            std::process::id()
        ));
        let mut bindings = BTreeMap::new();
        bindings.insert("timeline.zoom_in".to_owned(), Some("Ctrl+E".to_owned()));
        let data = PersistedShortcuts {
            version: FORMAT_VERSION,
            active_profile: "custom".to_owned(),
            custom_bindings: bindings,
        };
        let store = ShortcutStore::new(&path);
        store.save(&data).expect("write duplicate config");
        let (manager, warning) = ShortcutManager::load(Some(store));
        assert!(warning.is_some());
        assert_eq!(
            manager.binding(CommandId::ZoomIn),
            Some(binding(Key::E, true))
        );
        assert_eq!(
            manager.binding(CommandId::Export),
            None,
            "the later duplicate loses its binding"
        );
        let _ = std::fs::remove_file(&path);
    }
}
