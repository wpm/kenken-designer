use kenken::Puzzle;

pub struct Session {
    current: Puzzle,
    undo: Vec<Puzzle>,
    redo: Vec<Puzzle>,
}

impl Session {
    pub fn new(current: Puzzle) -> Self {
        Self {
            current,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub const fn current(&self) -> &Puzzle {
        &self.current
    }

    pub fn commit(&mut self, next: Puzzle) {
        let prev = std::mem::replace(&mut self.current, next);
        self.undo.push(prev);
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        let next = std::mem::replace(&mut self.current, prev);
        self.redo.push(next);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let prev = std::mem::replace(&mut self.current, next);
        self.undo.push(prev);
        true
    }

    pub fn replace(&mut self, p: Puzzle) {
        self.current = p;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn fresh(n: usize) -> Puzzle {
        Puzzle::new(n).unwrap()
    }

    #[test]
    fn new_session_has_empty_stacks() {
        let s = Session::new(fresh(4));
        assert_eq!(s.current().n(), 4);
        assert!(s.undo.is_empty());
        assert!(s.redo.is_empty());
    }

    #[test]
    fn commit_pushes_previous_onto_undo_and_clears_redo() {
        let mut s = Session::new(fresh(3));
        s.commit(fresh(4));
        assert_eq!(s.current().n(), 4);
        assert_eq!(s.undo.len(), 1);
        assert_eq!(s.undo[0].n(), 3);
        assert!(s.redo.is_empty());

        s.undo();
        assert_eq!(s.redo.len(), 1);
        s.commit(fresh(5));
        assert!(s.redo.is_empty());
    }

    #[test]
    fn undo_returns_false_when_empty() {
        let mut s = Session::new(fresh(4));
        assert!(!s.undo());
        assert_eq!(s.current().n(), 4);
    }

    #[test]
    fn redo_returns_false_when_empty() {
        let mut s = Session::new(fresh(4));
        assert!(!s.redo());
        assert_eq!(s.current().n(), 4);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut s = Session::new(fresh(3));
        s.commit(fresh(4));
        s.commit(fresh(5));
        assert_eq!(s.current().n(), 5);

        assert!(s.undo());
        assert_eq!(s.current().n(), 4);
        assert!(s.undo());
        assert_eq!(s.current().n(), 3);
        assert!(!s.undo());

        assert!(s.redo());
        assert_eq!(s.current().n(), 4);
        assert!(s.redo());
        assert_eq!(s.current().n(), 5);
        assert!(!s.redo());
    }

    #[test]
    fn replace_does_not_touch_stacks() {
        let mut s = Session::new(fresh(3));
        s.commit(fresh(4));
        assert_eq!(s.undo.len(), 1);

        s.replace(fresh(7));
        assert_eq!(s.current().n(), 7);
        assert_eq!(s.undo.len(), 1);
        assert!(s.redo.is_empty());
    }
}
