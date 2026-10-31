//! Native timeline selection state.

use std::collections::BTreeSet;

use crate::project::{ClipId, Project};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    clips: BTreeSet<ClipId>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_ids(ids: impl IntoIterator<Item = ClipId>) -> Self {
        Self {
            clips: ids.into_iter().collect(),
        }
    }

    pub fn ids(&self) -> &BTreeSet<ClipId> {
        &self.clips
    }

    pub fn len(&self) -> usize {
        self.clips.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    pub fn contains(&self, id: ClipId) -> bool {
        self.clips.contains(&id)
    }

    pub fn select(&mut self, id: ClipId) {
        self.clips.insert(id);
    }

    pub fn deselect(&mut self, id: ClipId) {
        self.clips.remove(&id);
    }

    pub fn toggle(&mut self, id: ClipId) {
        if !self.clips.remove(&id) {
            self.clips.insert(id);
        }
    }

    pub fn clear(&mut self) {
        self.clips.clear();
    }

    /// Approximate metadata-only bytes for history budgeting.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.clips.len() * (std::mem::size_of::<ClipId>() + 48)
    }

    /// Remove IDs that no longer resolve in the project.
    pub fn retain_existing(&mut self, project: &Project) {
        self.clips.retain(|id| {
            project
                .tracks()
                .iter()
                .flat_map(|track| track.clips())
                .any(|clip| clip.id() == *id)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_is_a_set_of_stable_ids() {
        let a = ClipId::new(1);
        let b = ClipId::new(2);
        let mut selection = Selection::new();
        selection.select(a);
        selection.toggle(a);
        assert!(!selection.contains(a));
        selection.select(b);
        assert_eq!(selection.len(), 1);
        selection.clear();
        assert!(selection.is_empty());
    }
}
