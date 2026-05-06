use crate::app::{CageView, DraftCage, PuzzleView};
use crate::cage_index::{cage_anchor, cage_at};

/// What the cursor cell belongs to, if anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellOwner {
    Uncovered,
    Draft,
    Committed(usize),
}

/// Outcome of a cage-editing key press.
///
/// `SetDraft` is applied locally; the other variants dispatch a Tauri command
/// and update the draft from the response (`SplinterFromCommitted` overrides
/// the response with a singleton draft).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CageEdit {
    Noop,
    SetDraft(Option<DraftCage>),
    ExtendCage {
        anchor: (usize, usize),
        cell: (usize, usize),
    },
    MergeCages {
        a_anchor: (usize, usize),
        b_anchor: (usize, usize),
    },
    ShrinkCage((usize, usize)),
    SplinterFromCommitted((usize, usize)),
    RemoveCage((usize, usize)),
}

#[must_use]
pub fn owner_of(view: &PuzzleView, draft: Option<&DraftCage>, cell: (usize, usize)) -> CellOwner {
    if let Some(d) = draft {
        if d.cells.contains(&cell) {
            return CellOwner::Draft;
        }
    }
    cage_at(view, cell.0, cell.1).map_or(CellOwner::Uncovered, CellOwner::Committed)
}

/// Pure branch table for `Shift+Arrow`.
///
/// Returns the action to dispatch. For server-call variants, the new draft is
/// determined by the server's response. For local actions, the new draft is
/// embedded in the action.
#[must_use]
pub fn shift_arrow(
    at: (usize, usize),
    neighbor: (usize, usize),
    view: &PuzzleView,
    draft: Option<&DraftCage>,
) -> CageEdit {
    let at_owner = owner_of(view, draft, at);
    let neighbor_owner = owner_of(view, draft, neighbor);
    match (at_owner, neighbor_owner) {
        (CellOwner::Uncovered, CellOwner::Uncovered) => CageEdit::SetDraft(Some(DraftCage {
            cells: vec![at, neighbor],
        })),
        (CellOwner::Draft, CellOwner::Uncovered) => {
            let mut cells = draft.map(|d| d.cells.clone()).unwrap_or_default();
            cells.push(neighbor);
            CageEdit::SetDraft(Some(DraftCage { cells }))
        }
        (CellOwner::Committed(_), CellOwner::Uncovered) => CageEdit::ExtendCage {
            anchor: at,
            cell: neighbor,
        },
        (CellOwner::Committed(a), CellOwner::Committed(b)) if a != b => CageEdit::MergeCages {
            a_anchor: at,
            b_anchor: neighbor,
        },
        _ => CageEdit::Noop,
    }
}

/// Branch table for `Escape`.
#[must_use]
pub fn escape_at(cell: (usize, usize), view: &PuzzleView, draft: Option<&DraftCage>) -> CageEdit {
    match owner_of(view, draft, cell) {
        CellOwner::Committed(_) => CageEdit::ShrinkCage(cell),
        CellOwner::Draft => CageEdit::SetDraft(remove_from_draft(draft, cell)),
        CellOwner::Uncovered => CageEdit::Noop,
    }
}

/// Branch table for `x`.
#[must_use]
pub fn delete_at(cell: (usize, usize), view: &PuzzleView, draft: Option<&DraftCage>) -> CageEdit {
    match owner_of(view, draft, cell) {
        CellOwner::Committed(idx) => {
            let anchor = view.cages.get(idx).map_or(cell, cage_anchor);
            CageEdit::RemoveCage(anchor)
        }
        CellOwner::Draft => CageEdit::SetDraft(None),
        CellOwner::Uncovered => CageEdit::Noop,
    }
}

/// Branch table for `Space` / `c`: split the cursor cell off as a singleton draft.
#[must_use]
pub fn splinter_at(cell: (usize, usize), view: &PuzzleView, draft: Option<&DraftCage>) -> CageEdit {
    match owner_of(view, draft, cell) {
        CellOwner::Committed(idx) => {
            if view.cages.get(idx).is_some_and(|c| c.cells.len() == 1) {
                CageEdit::Noop
            } else {
                CageEdit::SplinterFromCommitted(cell)
            }
        }
        CellOwner::Draft | CellOwner::Uncovered => {
            CageEdit::SetDraft(Some(DraftCage { cells: vec![cell] }))
        }
    }
}

fn remove_from_draft(draft: Option<&DraftCage>, cell: (usize, usize)) -> Option<DraftCage> {
    let cells: Vec<_> = draft?
        .cells
        .iter()
        .copied()
        .filter(|c| *c != cell)
        .collect();
    (!cells.is_empty()).then_some(DraftCage { cells })
}

/// Check whether a set of cells is 4-connected.
fn is_connected(cells: &[(usize, usize)]) -> bool {
    if cells.len() <= 1 {
        return true;
    }
    let mut visited = vec![false; cells.len()];
    let mut stack = vec![0usize];
    visited[0] = true;
    while let Some(idx) = stack.pop() {
        let (r, c) = cells[idx];
        for j in 0..cells.len() {
            if visited[j] {
                continue;
            }
            let (r2, c2) = cells[j];
            let dr = r.abs_diff(r2);
            let dc = c.abs_diff(c2);
            if (dr == 1 && dc == 0) || (dr == 0 && dc == 1) {
                visited[j] = true;
                stack.push(j);
            }
        }
    }
    visited.iter().all(|&v| v)
}

/// Returns anchor cells (row-major sorted) of cages that are legal targets for moving `cell`.
///
/// Returns empty if: cell is in no cage, or removing cell from its cage would disconnect it
/// (unless the cage is a singleton).
#[must_use]
pub fn legal_move_targets(view: &PuzzleView, cell: (usize, usize)) -> Vec<(usize, usize)> {
    let (r, c) = cell;
    let Some(src_idx) = cage_at(view, r, c) else {
        return Vec::new();
    };
    let src_cells = &view.cages[src_idx].cells;

    // If the source has more than one cell, check connectivity after removal.
    if src_cells.len() > 1 {
        let remaining: Vec<(usize, usize)> =
            src_cells.iter().copied().filter(|&p| p != cell).collect();
        if !is_connected(&remaining) {
            return Vec::new();
        }
    }

    let n = view.n;
    let neighbors: &[(isize, isize)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
    let mut targets: Vec<(usize, usize)> = Vec::new();

    for &(dr, dc) in neighbors {
        let Some(nr) = r.checked_add_signed(dr) else {
            continue;
        };
        let Some(nc) = c.checked_add_signed(dc) else {
            continue;
        };
        if nr >= n || nc >= n {
            continue;
        }
        if let Some(tgt_idx) = cage_at(view, nr, nc) {
            if tgt_idx != src_idx {
                let anchor = cage_anchor(&view.cages[tgt_idx]);
                if !targets.contains(&anchor) {
                    targets.push(anchor);
                }
            }
        }
    }

    targets.sort_unstable();
    targets
}

/// Augmented cage list for rendering: committed cages plus any drafts appended at the end.
/// The returned `first_draft_idx` points to the first draft's position when present.
#[must_use]
pub fn effective_cages(view: &PuzzleView, drafts: &[DraftCage]) -> (Vec<CageView>, Option<usize>) {
    let mut cages = view.cages.clone();
    let first_draft_idx = if drafts.is_empty() {
        None
    } else {
        let idx = cages.len();
        for d in drafts {
            cages.push(CageView {
                cells: d.cells.clone(),
                op: crate::app::OpKind::Add,
                target: 0,
            });
        }
        Some(idx)
    };
    (cages, first_draft_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpKind;

    fn cage(cells: &[(usize, usize)]) -> CageView {
        CageView {
            cells: cells.to_vec(),
            op: OpKind::Add,
            target: 3,
        }
    }

    fn view(n: usize, cages: Vec<CageView>) -> PuzzleView {
        PuzzleView {
            n,
            cells: vec![vec![Vec::new(); n]; n],
            cages,
            diff: crate::diff::PuzzleDiff::default(),
        }
    }

    fn draft(cells: &[(usize, usize)]) -> DraftCage {
        DraftCage {
            cells: cells.to_vec(),
        }
    }

    #[test]
    fn shift_arrow_uncovered_to_uncovered_starts_draft() {
        let v = view(3, vec![]);
        let action = shift_arrow((0, 0), (0, 1), &v, None);
        assert_eq!(action, CageEdit::SetDraft(Some(draft(&[(0, 0), (0, 1)]))));
    }

    #[test]
    fn shift_arrow_draft_to_uncovered_appends_to_draft() {
        let v = view(3, vec![]);
        let d = draft(&[(0, 0), (0, 1)]);
        let action = shift_arrow((0, 1), (0, 2), &v, Some(&d));
        assert_eq!(
            action,
            CageEdit::SetDraft(Some(draft(&[(0, 0), (0, 1), (0, 2)])))
        );
    }

    #[test]
    fn shift_arrow_within_same_draft_is_noop() {
        let v = view(3, vec![]);
        let d = draft(&[(0, 0), (0, 1)]);
        assert_eq!(shift_arrow((0, 0), (0, 1), &v, Some(&d)), CageEdit::Noop);
    }

    #[test]
    fn shift_arrow_committed_to_uncovered_extends() {
        let v = view(3, vec![cage(&[(0, 0)])]);
        let action = shift_arrow((0, 0), (0, 1), &v, None);
        assert_eq!(
            action,
            CageEdit::ExtendCage {
                anchor: (0, 0),
                cell: (0, 1),
            }
        );
    }

    #[test]
    fn shift_arrow_committed_to_committed_merges_when_distinct() {
        let v = view(3, vec![cage(&[(0, 0)]), cage(&[(0, 1)])]);
        let action = shift_arrow((0, 0), (0, 1), &v, None);
        assert_eq!(
            action,
            CageEdit::MergeCages {
                a_anchor: (0, 0),
                b_anchor: (0, 1),
            }
        );
    }

    #[test]
    fn shift_arrow_within_same_committed_cage_is_noop() {
        let v = view(3, vec![cage(&[(0, 0), (0, 1)])]);
        assert_eq!(shift_arrow((0, 0), (0, 1), &v, None), CageEdit::Noop);
    }

    #[test]
    fn shift_arrow_uncovered_to_committed_is_noop() {
        let v = view(3, vec![cage(&[(0, 1)])]);
        assert_eq!(shift_arrow((0, 0), (0, 1), &v, None), CageEdit::Noop);
    }

    #[test]
    fn escape_on_committed_cell_calls_shrink() {
        let v = view(3, vec![cage(&[(0, 0), (0, 1)])]);
        assert_eq!(escape_at((0, 0), &v, None), CageEdit::ShrinkCage((0, 0)));
    }

    #[test]
    fn escape_on_draft_cell_removes_from_draft() {
        let v = view(3, vec![]);
        let d = draft(&[(0, 0), (0, 1)]);
        assert_eq!(
            escape_at((0, 0), &v, Some(&d)),
            CageEdit::SetDraft(Some(draft(&[(0, 1)])))
        );
    }

    #[test]
    fn escape_on_last_draft_cell_clears_draft() {
        let v = view(3, vec![]);
        let d = draft(&[(0, 0)]);
        assert_eq!(escape_at((0, 0), &v, Some(&d)), CageEdit::SetDraft(None));
    }

    #[test]
    fn escape_on_uncovered_is_noop() {
        let v = view(3, vec![]);
        assert_eq!(escape_at((0, 0), &v, None), CageEdit::Noop);
    }

    #[test]
    fn delete_on_committed_calls_remove_with_anchor() {
        let v = view(3, vec![cage(&[(0, 1), (0, 0)])]);
        // anchor is the smallest cell row-major: (0,0)
        assert_eq!(delete_at((0, 1), &v, None), CageEdit::RemoveCage((0, 0)));
    }

    #[test]
    fn delete_on_draft_clears_draft() {
        let v = view(3, vec![]);
        let d = draft(&[(0, 0), (0, 1)]);
        assert_eq!(delete_at((0, 0), &v, Some(&d)), CageEdit::SetDraft(None));
    }

    #[test]
    fn delete_on_uncovered_is_noop() {
        let v = view(3, vec![]);
        assert_eq!(delete_at((0, 0), &v, None), CageEdit::Noop);
    }

    #[test]
    fn splinter_on_singleton_committed_is_noop() {
        let v = view(3, vec![cage(&[(0, 0)])]);
        assert_eq!(splinter_at((0, 0), &v, None), CageEdit::Noop);
    }

    #[test]
    fn splinter_on_multi_cell_committed_calls_splinter() {
        let v = view(3, vec![cage(&[(0, 0), (0, 1)])]);
        assert_eq!(
            splinter_at((0, 0), &v, None),
            CageEdit::SplinterFromCommitted((0, 0))
        );
    }

    #[test]
    fn splinter_on_uncovered_starts_singleton_draft() {
        let v = view(3, vec![]);
        assert_eq!(
            splinter_at((1, 1), &v, None),
            CageEdit::SetDraft(Some(draft(&[(1, 1)])))
        );
    }

    #[test]
    fn splinter_on_draft_isolates_to_singleton() {
        let v = view(3, vec![]);
        let d = draft(&[(0, 0), (0, 1)]);
        assert_eq!(
            splinter_at((0, 0), &v, Some(&d)),
            CageEdit::SetDraft(Some(draft(&[(0, 0)])))
        );
    }

    #[test]
    fn effective_cages_appends_draft_at_end_with_index() {
        let v = view(3, vec![cage(&[(0, 0)]), cage(&[(2, 2)])]);
        let d = draft(&[(1, 1)]);
        let (eff, draft_idx) = effective_cages(&v, &[d]);
        assert_eq!(eff.len(), 3);
        assert_eq!(eff[2].cells, vec![(1, 1)]);
        assert_eq!(draft_idx, Some(2));
    }

    #[test]
    fn effective_cages_appends_multiple_drafts() {
        let v = view(3, vec![cage(&[(0, 0)])]);
        let d1 = draft(&[(1, 1)]);
        let d2 = draft(&[(2, 2)]);
        let (eff, draft_idx) = effective_cages(&v, &[d1, d2]);
        assert_eq!(eff.len(), 3);
        assert_eq!(eff[1].cells, vec![(1, 1)]);
        assert_eq!(eff[2].cells, vec![(2, 2)]);
        assert_eq!(draft_idx, Some(1));
    }

    #[test]
    fn effective_cages_returns_none_index_when_no_draft() {
        let v = view(3, vec![cage(&[(0, 0)])]);
        let (eff, draft_idx) = effective_cages(&v, &[]);
        assert_eq!(eff.len(), 1);
        assert_eq!(draft_idx, None);
    }
}
