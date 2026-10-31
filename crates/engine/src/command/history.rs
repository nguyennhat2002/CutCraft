//! Snapshot-based undo/redo history for native edits.
//!
//! Snapshots contain native project metadata, selection and the engine-owned
//! Magnet preference. They never contain decoded frames, GPU resources or
//! CapCut document bytes.

use std::collections::VecDeque;

use crate::project::Project;
use crate::selection::Selection;

pub const MAX_HISTORY_ENTRIES: usize = 100;
pub const MAX_HISTORY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot {
    pub project: Project,
    pub selection: Selection,
    pub magnet_enabled: bool,
}

impl StateSnapshot {
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.project.estimated_bytes()
            + self.selection.estimated_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub before: StateSnapshot,
    pub after: StateSnapshot,
    pub estimated_bytes: usize,
}

impl Transaction {
    pub fn new(before: StateSnapshot, after: StateSnapshot) -> Self {
        let estimated_bytes =
            std::mem::size_of::<Self>() + before.estimated_bytes() + after.estimated_bytes();
        Self {
            before,
            after,
            estimated_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditHistory {
    undo: VecDeque<Transaction>,
    redo: Vec<Transaction>,
    undo_bytes: usize,
    redo_bytes: usize,
}

impl EditHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a transaction, dropping the oldest undo entries until both the
    /// entry and byte budgets fit. Returns false if one transaction alone
    /// exceeds the byte budget; callers must check this before mutating state.
    pub fn record(&mut self, transaction: Transaction, byte_budget: usize) -> bool {
        if transaction.estimated_bytes > byte_budget {
            return false;
        }
        self.redo.clear();
        self.redo_bytes = 0;
        while !self.undo.is_empty()
            && (self.undo.len() >= MAX_HISTORY_ENTRIES
                || self.undo_bytes + transaction.estimated_bytes > byte_budget)
        {
            if let Some(oldest) = self.undo.pop_front() {
                self.undo_bytes = self.undo_bytes.saturating_sub(oldest.estimated_bytes);
            }
        }
        if self.undo_bytes + transaction.estimated_bytes > byte_budget {
            return false;
        }
        self.undo_bytes += transaction.estimated_bytes;
        self.undo.push_back(transaction);
        true
    }

    pub fn pop_undo(&mut self) -> Option<Transaction> {
        let transaction = self.undo.pop_back()?;
        self.undo_bytes = self.undo_bytes.saturating_sub(transaction.estimated_bytes);
        Some(transaction)
    }

    pub fn push_redo(&mut self, transaction: Transaction) {
        self.redo_bytes += transaction.estimated_bytes;
        self.redo.push(transaction);
    }

    pub fn pop_redo(&mut self) -> Option<Transaction> {
        let transaction = self.redo.pop()?;
        self.redo_bytes = self.redo_bytes.saturating_sub(transaction.estimated_bytes);
        Some(transaction)
    }

    pub fn push_undo(&mut self, transaction: Transaction) {
        if self.undo.len() >= MAX_HISTORY_ENTRIES {
            if let Some(oldest) = self.undo.pop_front() {
                self.undo_bytes = self.undo_bytes.saturating_sub(oldest.estimated_bytes);
            }
        }
        self.undo_bytes += transaction.estimated_bytes;
        self.undo.push_back(transaction);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn undo_bytes(&self) -> usize {
        self.undo_bytes
    }

    pub fn redo_bytes(&self) -> usize {
        self.redo_bytes
    }

    pub fn total_bytes(&self) -> usize {
        self.undo_bytes + self.redo_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capcut_core::time::FrameRate;

    fn snapshot(revision: u64) -> StateSnapshot {
        let mut project = Project::empty(FrameRate::new(30, 1).expect("rate")).expect("project");
        project.revision = revision;
        StateSnapshot {
            project,
            selection: Selection::new(),
            magnet_enabled: false,
        }
    }

    fn transaction(revision: u64) -> Transaction {
        Transaction::new(snapshot(revision), snapshot(revision + 1))
    }

    #[test]
    fn history_is_bounded_and_new_edits_clear_redo() {
        let mut history = EditHistory::new();
        for revision in 0..(MAX_HISTORY_ENTRIES as u64 + 5) {
            assert!(history.record(transaction(revision), MAX_HISTORY_BYTES));
        }
        assert_eq!(history.undo_len(), MAX_HISTORY_ENTRIES);
        assert_eq!(history.redo_len(), 0);
        assert!(history.undo_bytes() <= MAX_HISTORY_BYTES);

        let undone = history.pop_undo().expect("undo");
        history.push_redo(undone);
        assert!(history.can_redo());
        assert!(history.record(transaction(999), MAX_HISTORY_BYTES));
        assert!(!history.can_redo());
    }

    #[test]
    fn byte_budget_evicts_oldest_and_rejects_oversized_single_transactions() {
        let mut history = EditHistory::new();
        let one = transaction(1);
        let transaction_bytes = one.estimated_bytes;
        assert!(history.record(one, transaction_bytes));
        assert_eq!(history.undo_len(), 1);

        let two = transaction(2);
        assert!(history.record(two, transaction_bytes));
        assert_eq!(
            history.undo_len(),
            1,
            "oldest transaction should be evicted"
        );
        assert!(history.undo_bytes() <= transaction_bytes);

        let oversized = transaction(3);
        assert!(!history.record(oversized, 1));
        assert_eq!(history.undo_len(), 1);
        assert!(history.undo_bytes() <= transaction_bytes);
    }
}
