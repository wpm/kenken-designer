use crate::app::{CageView, PuzzleView};

#[must_use]
pub fn cage_at(view: &PuzzleView, r: usize, c: usize) -> Option<usize> {
    view.cages
        .iter()
        .position(|cage| cage.cells.iter().any(|&(cr, cc)| cr == r && cc == c))
}

#[must_use]
pub fn cage_anchor(cage: &CageView) -> (usize, usize) {
    cells_anchor(&cage.cells)
}

#[must_use]
pub fn cells_anchor(cells: &[(usize, usize)]) -> (usize, usize) {
    cells
        .iter()
        .min_by_key(|&&(r, c)| (r, c))
        .copied()
        .unwrap_or((0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpKind;

    fn cage(cells: &[(usize, usize)]) -> CageView {
        CageView {
            cells: cells.to_vec(),
            op: OpKind::Given,
            target: 0,
        }
    }

    fn view_with(n: usize, cages: Vec<CageView>) -> PuzzleView {
        PuzzleView {
            n,
            cells: vec![vec![Vec::new(); n]; n],
            cages,
        }
    }

    #[test]
    fn cage_at_returns_index_when_cell_in_cage() {
        let view = view_with(2, vec![cage(&[(0, 0), (1, 1)]), cage(&[(0, 1), (1, 0)])]);
        assert_eq!(cage_at(&view, 0, 0), Some(0));
        assert_eq!(cage_at(&view, 1, 1), Some(0));
        assert_eq!(cage_at(&view, 0, 1), Some(1));
        assert_eq!(cage_at(&view, 1, 0), Some(1));
    }

    #[test]
    fn cage_at_returns_none_when_cell_uncaged() {
        let view = view_with(3, vec![cage(&[(0, 0)])]);
        assert_eq!(cage_at(&view, 1, 1), None);
        assert_eq!(cage_at(&view, 2, 2), None);
    }

    #[test]
    fn cage_at_with_no_cages_returns_none() {
        let view = view_with(3, vec![]);
        assert_eq!(cage_at(&view, 0, 0), None);
    }

    #[test]
    fn cage_anchor_picks_row_major_first_cell() {
        let c = cage(&[(2, 1), (0, 3), (1, 0), (0, 1)]);
        assert_eq!(cage_anchor(&c), (0, 1));
    }

    #[test]
    fn cage_anchor_for_empty_cage_is_origin() {
        let c = cage(&[]);
        assert_eq!(cage_anchor(&c), (0, 0));
    }
}
