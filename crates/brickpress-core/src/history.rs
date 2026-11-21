//! Bounded undo/redo of whole-document snapshots.

#[derive(Clone, Debug)]
pub struct History<T> {
    past: Vec<T>,
    future: Vec<T>,
    limit: usize,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

impl<T> History<T> {
    pub fn new(limit: usize) -> Self {
        Self { past: Vec::new(), future: Vec::new(), limit }
    }
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
    pub fn push(&mut self, current: T) {
        self.past.push(current);
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        self.future.clear();
    }
    pub fn undo(&mut self, current: T) -> Option<T> {
        let value = self.past.pop()?;
        self.future.push(current);
        Some(value)
    }
    pub fn redo(&mut self, current: T) -> Option<T> {
        let value = self.future.pop()?;
        self.past.push(current);
        Some(value)
    }
    /// Drops the most recent undo entry, merging the last two steps.
    pub fn discard_last(&mut self) -> Option<T> {
        self.past.pop()
    }
    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undoes_and_redoes() {
        let mut h = History::default();
        h.push("before");
        assert_eq!(h.undo("after"), Some("before"));
        assert!(h.can_redo());
        assert_eq!(h.redo("before"), Some("after"));
    }

    #[test]
    fn clears_redo_on_branch_and_caps() {
        let mut h = History::new(2);
        h.push(0);
        h.push(1);
        h.push(2);
        assert_eq!(h.undo(3), Some(2));
        assert_eq!(h.undo(2), Some(1));
        assert_eq!(h.undo(1), None);
        h.push(9);
        assert!(!h.can_redo());
    }
}
