use kenken::{Error, Index, Puzzle};

pub struct Session {
    current: Puzzle,
    undo: Vec<Puzzle>,
    redo: Vec<Puzzle>,
}

impl Session {
    pub fn new(n: Index) -> Result<Self, Error> {
        Ok(Self {
            current: Puzzle::new(n)?,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    pub fn current(&self) -> &Puzzle {
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

    #[allow(dead_code)] // exposed for future "abandon" semantics
    pub fn replace(&mut self, p: Puzzle) {
        self.current = p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_has_empty_stacks() {
        let s = Session::new(4).unwrap();
        assert_eq!(s.current().n(), 4);
        assert!(s.undo.is_empty());
        assert!(s.redo.is_empty());
    }

    #[test]
    fn commit_pushes_previous_onto_undo_and_clears_redo() {
        let mut s = Session::new(3).unwrap();
        let p4 = Puzzle::new(4).unwrap();
        s.commit(p4);
        assert_eq!(s.current().n(), 4);
        assert_eq!(s.undo.len(), 1);
        assert_eq!(s.undo[0].n(), 3);
        assert!(s.redo.is_empty());

        s.undo();
        assert_eq!(s.redo.len(), 1);
        let p5 = Puzzle::new(5).unwrap();
        s.commit(p5);
        assert!(s.redo.is_empty());
    }

    #[test]
    fn undo_returns_false_when_empty() {
        let mut s = Session::new(4).unwrap();
        assert!(!s.undo());
        assert_eq!(s.current().n(), 4);
    }

    #[test]
    fn redo_returns_false_when_empty() {
        let mut s = Session::new(4).unwrap();
        assert!(!s.redo());
        assert_eq!(s.current().n(), 4);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut s = Session::new(3).unwrap();
        s.commit(Puzzle::new(4).unwrap());
        s.commit(Puzzle::new(5).unwrap());
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
        let mut s = Session::new(3).unwrap();
        s.commit(Puzzle::new(4).unwrap());
        assert_eq!(s.undo.len(), 1);

        s.replace(Puzzle::new(7).unwrap());
        assert_eq!(s.current().n(), 7);
        assert_eq!(s.undo.len(), 1);
        assert!(s.redo.is_empty());
    }
}
