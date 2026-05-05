use kenken::Puzzle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    /// Edit adds a constraint or restricts a domain. Examples: `insert_cage`; `extend_cage`.
    Narrowing,
    /// Edit removes a constraint or relaxes a domain. Examples: `remove_cage`; `shrink_cage`; `clear_all_cages`.
    Widening,
}

/// Apply a structural edit to a puzzle, then propagate constraints to fixpoint.
///
/// For [`EditKind::Widening`] edits (cage removal / size shrink), the grid is reset to the
/// full candidate-value domain before propagation so that previously narrowed cells can
/// regain values that are no longer excluded by the updated constraint set.
pub fn apply_edit<F>(pre: &Puzzle, kind: EditKind, edit: F) -> Result<Puzzle, String>
where
    F: FnOnce(Puzzle) -> Result<Puzzle, String>,
{
    let after_edit = edit(pre.clone())?;
    let propagated = match kind {
        EditKind::Narrowing => after_edit.propagate_fully(),
        EditKind::Widening => rebuild_from_constraints(&after_edit)?,
    };
    Ok(propagated)
}

/// Reconstruct a puzzle with a full-domain grid and the same cages as `p`, then propagate.
///
/// This is used after Widening edits to ensure that cells whose domains were narrowed by a
/// now-removed constraint can regain candidate values.
fn rebuild_from_constraints(p: &Puzzle) -> Result<Puzzle, String> {
    let n = p.n();
    let fresh = Puzzle::new(n).map_err(|e| format!("{e:?}"))?;
    let rebuilt = p.cages().cloned().try_fold(fresh, |acc, cage| {
        acc.insert_cage(cage).map_err(|e| format!("{e:?}"))
    })?;
    Ok(rebuilt.propagate_fully())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kenken::{Cage, Cell, Operation, Polyomino, Puzzle};

    fn given_cage_at(cell: (usize, usize), value: u8, n: u8) -> Cage {
        let cells = vec![Cell::new(cell.0, cell.1)];
        Cage::new(n, Polyomino::new(&cells), Operation::Given(value))
    }

    /// After inserting Given(1) at (0,0), propagation must eliminate 1 from (0,1)'s fill
    /// (row constraint: each value appears exactly once per row).
    #[test]
    fn apply_edit_calls_propagate() {
        let pre = Puzzle::new(3).unwrap();
        let cage = given_cage_at((0, 0), 1, 3);

        let result = apply_edit(&pre, EditKind::Narrowing, |p| {
            p.insert_cage(cage).map_err(|e| format!("{e:?}"))
        })
        .unwrap();

        let fill_00: Vec<u8> = result.candidates(Cell::new(0, 0)).unwrap().iter().collect();
        assert_eq!(fill_00, vec![1], "(0,0) should be fixed to 1");

        let fill_01: Vec<u8> = result.candidates(Cell::new(0, 1)).unwrap().iter().collect();
        assert!(
            !fill_01.contains(&1),
            "1 should be eliminated from (0,1) via row constraint; got {fill_01:?}"
        );
    }

    /// After removing a Given cage that forced row 0 values, widening via `apply_edit` must
    /// restore the full domain to cells that were constrained by it.
    #[test]
    fn apply_edit_widening_grows_domains() {
        let base = Puzzle::new(3).unwrap();
        let cage = given_cage_at((0, 0), 1, 3);
        let propagated = base.insert_cage(cage).unwrap().propagate_fully();

        assert!(
            !propagated
                .candidates(Cell::new(0, 1))
                .unwrap()
                .iter()
                .any(|x| x == 1),
            "sanity: 1 should be absent from (0,1) before removal"
        );

        let poly = propagated
            .cage_at(Cell::new(0, 0))
            .unwrap()
            .polyomino()
            .clone();
        let result = apply_edit(
            &propagated,
            EditKind::Widening,
            |p| Ok(p.remove_cage(&poly)),
        )
        .unwrap();

        assert!(
            result
                .candidates(Cell::new(0, 1))
                .unwrap()
                .iter()
                .any(|x| x == 1),
            "1 should return to (0,1) after removing the Given cage"
        );
    }
}
