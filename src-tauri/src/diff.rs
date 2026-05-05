use kenken::{N, Puzzle};

/// Per-cell change between two puzzle states.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CellDiff {
    pub cell: (usize, usize),
    pub removed: Vec<N>,
    pub added: Vec<N>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Default)]
pub struct PuzzleDiff {
    pub changes: Vec<CellDiff>,
}

impl PuzzleDiff {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Compute the diff from `before` to `after`. Both must have the same `n`.
    ///
    /// `removed` and `added` are sorted ascending. `changes` is sorted in row-major
    /// cell order. Cells whose fill is unchanged produce no entry.
    pub fn between(before: &Puzzle, after: &Puzzle) -> Self {
        debug_assert_eq!(before.n(), after.n(), "puzzles must have the same n");

        let changes: Vec<CellDiff> = before
            .cells()
            .filter_map(|cell| {
                let before_vals = before.candidates(cell).ok()?;
                let after_vals = after.candidates(cell).ok()?;
                if before_vals == after_vals {
                    return None;
                }
                let removed: Vec<N> = before_vals
                    .iter()
                    .filter(|&v| !after_vals.iter().any(|a| a == v))
                    .collect();
                let added: Vec<N> = after_vals
                    .iter()
                    .filter(|&v| !before_vals.iter().any(|b| b == v))
                    .collect();
                Some(CellDiff {
                    cell: (cell.row, cell.column),
                    removed,
                    added,
                })
            })
            .collect();

        Self { changes }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kenken::{Cell, Delta, Values};

    #[test]
    fn between_identical_puzzles_is_empty() {
        let p = Puzzle::new(4).unwrap();
        let q = p.clone();
        assert_eq!(PuzzleDiff::between(&p, &q), PuzzleDiff::default());
        assert!(PuzzleDiff::between(&p, &q).is_empty());
    }

    #[test]
    fn between_narrowed_puzzle_lists_removed_values() {
        // pre has {1,2,3,4} at (0,0); post has {1,2}
        let before = Puzzle::new(4).unwrap();
        let delta = Delta::identity(4)
            .unwrap()
            .set(Cell::new(0, 0), Values::new([1, 2]));
        let after = before.narrow(&delta);

        let diff = PuzzleDiff::between(&before, &after);
        let cd = diff
            .changes
            .iter()
            .find(|cd| cd.cell == (0, 0))
            .expect("(0,0) must appear in diff");
        assert_eq!(cd.removed, vec![3, 4]);
        assert!(cd.added.is_empty());
    }

    #[test]
    fn between_widened_puzzle_lists_added_values() {
        // before has {1,2} at (0,0); after has {1,2,3,4}
        let after = Puzzle::new(4).unwrap();
        let delta = Delta::identity(4)
            .unwrap()
            .set(Cell::new(0, 0), Values::new([1, 2]));
        let before = after.narrow(&delta);

        let diff = PuzzleDiff::between(&before, &after);
        let cd = diff
            .changes
            .iter()
            .find(|cd| cd.cell == (0, 0))
            .expect("(0,0) must appear in diff");
        assert!(cd.removed.is_empty());
        assert_eq!(cd.added, vec![3, 4]);
    }

    #[test]
    fn between_mixed_changes() {
        // (0,0) narrowed in before; (0,1) narrowed in after — one cell narrows, one widens.
        let base = Puzzle::new(4).unwrap();

        let delta_before = Delta::identity(4)
            .unwrap()
            .set(Cell::new(0, 0), Values::new([1, 2]));
        let before = base.narrow(&delta_before);

        let delta_after = Delta::identity(4)
            .unwrap()
            .set(Cell::new(0, 1), Values::new([1, 2]));
        let after = base.narrow(&delta_after);

        let diff = PuzzleDiff::between(&before, &after);

        let idx_00 = diff.changes.iter().position(|cd| cd.cell == (0, 0));
        let idx_01 = diff.changes.iter().position(|cd| cd.cell == (0, 1));

        assert!(idx_00.is_some(), "cell (0,0) should appear in diff");
        assert!(idx_01.is_some(), "cell (0,1) should appear in diff");

        // row-major: (0,0) before (0,1)
        assert!(
            idx_00.unwrap() < idx_01.unwrap(),
            "row-major: (0,0) must precede (0,1)"
        );

        let cd_01 = diff.changes.iter().find(|cd| cd.cell == (0, 1)).unwrap();
        assert!(!cd_01.removed.is_empty(), "(0,1) should have removals");
        assert!(cd_01.added.is_empty(), "(0,1) should have no additions");
    }

    #[test]
    fn between_singleton_collapse() {
        // pre has {1,2,3} at (0,0) on 3x3; post has {2}
        let before = Puzzle::new(3).unwrap();
        let delta = Delta::identity(3)
            .unwrap()
            .set(Cell::new(0, 0), Values::new([2]));
        let after = before.narrow(&delta);

        let diff = PuzzleDiff::between(&before, &after);
        let cd = diff
            .changes
            .iter()
            .find(|cd| cd.cell == (0, 0))
            .expect("(0,0) must appear in diff");
        // removed: [1, 3] (sorted ascending)
        assert_eq!(cd.removed, vec![1, 3]);
        assert!(cd.added.is_empty());
    }

    #[test]
    fn between_no_change_in_one_cell_does_not_emit_entry() {
        // Only (0,0) narrowed; unchanged cells must not appear in diff.
        let before = Puzzle::new(4).unwrap();
        let delta = Delta::identity(4)
            .unwrap()
            .set(Cell::new(0, 0), Values::new([1, 2]));
        let after = before.narrow(&delta);

        let diff = PuzzleDiff::between(&before, &after);

        // (0,0) must appear
        assert!(
            diff.changes.iter().any(|cd| cd.cell == (0, 0)),
            "(0,0) must appear in diff"
        );
        // All entries must represent genuinely changed cells
        for cd in &diff.changes {
            let cell = Cell::new(cd.cell.0, cd.cell.1);
            let before_vals = before.candidates(cell).unwrap();
            let after_vals = after.candidates(cell).unwrap();
            assert_ne!(
                before_vals, after_vals,
                "cell {:?} appears in diff but fill is unchanged",
                cd.cell
            );
        }
    }
}
