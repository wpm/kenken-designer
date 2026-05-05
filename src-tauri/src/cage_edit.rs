use kenken::{Cage, Cell, Operation, Polyomino, Puzzle};

use crate::view::{DraftCage, OpKind};

const fn op_legal_for_size(op: Operation, size: usize) -> bool {
    match op {
        Operation::Add(_) | Operation::Multiply(_) => true,
        Operation::Subtract(_) | Operation::Divide(_) => size == 2,
        Operation::Given(_) => size == 1,
    }
}

#[allow(clippy::cast_possible_truncation)]
const fn build_operation(op: OpKind, target: u32) -> Operation {
    match op {
        OpKind::Add => Operation::Add(target as u8),
        OpKind::Sub => Operation::Subtract(target as u8),
        OpKind::Mul => Operation::Multiply(target as u16),
        OpKind::Div => Operation::Divide(target as u8),
        OpKind::Given => Operation::Given(target as u8),
    }
}

fn puzzle_size_u8(puzzle: &Puzzle) -> Result<u8, String> {
    u8::try_from(puzzle.n()).map_err(|e| format!("invalid puzzle size: {e}"))
}

fn cells_to_vec(poly: &Polyomino) -> Vec<(usize, usize)> {
    poly.as_slice().iter().map(|c| (c.row, c.column)).collect()
}

fn cage_at_or_err(puzzle: &Puzzle, anchor: (usize, usize)) -> Result<&Cage, String> {
    puzzle
        .cage_at(Cell::new(anchor.0, anchor.1))
        .ok_or_else(|| format!("no cage at ({}, {})", anchor.0, anchor.1))
}

fn rebuild_with_shape(
    puzzle: &Puzzle,
    old_poly: &Polyomino,
    new_poly: Polyomino,
    op: Operation,
    n: u8,
) -> Result<(Puzzle, Option<DraftCage>), String> {
    let next = puzzle.clone().remove_cage(old_poly);
    reinsert_or_draft(next, new_poly, op, n)
}

fn reinsert_or_draft(
    puzzle: Puzzle,
    poly: Polyomino,
    op: Operation,
    n: u8,
) -> Result<(Puzzle, Option<DraftCage>), String> {
    if op_legal_for_size(op, poly.len()) {
        let next = puzzle
            .insert_cage(Cage::new(n, poly, op))
            .map_err(|e| format!("{e:?}"))?;
        Ok((next, None))
    } else {
        let draft = Some(DraftCage {
            cells: cells_to_vec(&poly),
        });
        Ok((puzzle, draft))
    }
}

pub fn do_insert_cage(
    puzzle: &Puzzle,
    cells: &[(usize, usize)],
    op: OpKind,
    target: u32,
) -> Result<Puzzle, String> {
    let n = puzzle_size_u8(puzzle)?;
    let cells_vec: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
    let poly = Polyomino::new(&cells_vec);
    if poly.is_empty() {
        return Err("cage must have at least one cell".into());
    }
    let cage = Cage::new(n, poly, build_operation(op, target));
    puzzle
        .clone()
        .insert_cage(cage)
        .map_err(|e| format!("{e:?}"))
}

pub fn do_remove_cage(puzzle: &Puzzle, anchor: (usize, usize)) -> Result<Puzzle, String> {
    let poly = cage_at_or_err(puzzle, anchor)?.polyomino().clone();
    Ok(puzzle.clone().remove_cage(&poly))
}

pub fn do_set_cage_operation(
    puzzle: &Puzzle,
    anchor: (usize, usize),
    op: OpKind,
    target: u32,
) -> Result<Puzzle, String> {
    let cage = cage_at_or_err(puzzle, anchor)?;
    let cells = cells_to_vec(cage.polyomino());
    let poly = cage.polyomino().clone();
    let puzzle = puzzle.clone().remove_cage(&poly);
    do_insert_cage(&puzzle, &cells, op, target)
}

pub fn do_extend_cage(
    puzzle: &Puzzle,
    anchor: (usize, usize),
    cell: (usize, usize),
) -> Result<(Puzzle, Option<DraftCage>), String> {
    let n = puzzle_size_u8(puzzle)?;
    let target = Cell::new(cell.0, cell.1);
    if puzzle.is_covered(target) {
        return Err(format!("cell ({}, {}) already covered", cell.0, cell.1));
    }
    let cage = cage_at_or_err(puzzle, anchor)?;
    let op = cage.operation();
    let old_poly = cage.polyomino().clone();
    let new_poly = old_poly.extend(target).map_err(|e| format!("{e:?}"))?;
    rebuild_with_shape(puzzle, &old_poly, new_poly, op, n)
}

pub fn do_shrink_cage(
    puzzle: &Puzzle,
    cell: (usize, usize),
) -> Result<(Puzzle, Option<DraftCage>), String> {
    let n = puzzle_size_u8(puzzle)?;
    let target = Cell::new(cell.0, cell.1);
    let cage = cage_at_or_err(puzzle, cell)?;
    let op = cage.operation();
    let old_poly = cage.polyomino().clone();
    if old_poly.len() == 1 {
        return Ok((puzzle.clone().remove_cage(&old_poly), None));
    }
    let new_poly = old_poly.without(target).map_err(|e| format!("{e:?}"))?;
    rebuild_with_shape(puzzle, &old_poly, new_poly, op, n)
}

pub fn do_merge_cages(
    puzzle: &Puzzle,
    a_anchor: (usize, usize),
    b_anchor: (usize, usize),
) -> Result<(Puzzle, Option<DraftCage>), String> {
    let n = puzzle_size_u8(puzzle)?;
    let cage_a = cage_at_or_err(puzzle, a_anchor)?;
    let cage_b = cage_at_or_err(puzzle, b_anchor)?;
    if cage_a.polyomino() == cage_b.polyomino() {
        return Err("cages are the same".into());
    }
    let op = cage_a.operation();
    let poly_a = cage_a.polyomino().clone();
    let poly_b = cage_b.polyomino().clone();

    let mut all_cells: Vec<Cell> = poly_a.as_slice().to_vec();
    all_cells.extend_from_slice(poly_b.as_slice());
    let merged = Polyomino::new(&all_cells);

    let intermediate = puzzle.clone().remove_cage(&poly_a).remove_cage(&poly_b);
    reinsert_or_draft(intermediate, merged, op, n)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kenken::{Cage, Operation, Polyomino};

    fn add_cage(cells: &[(usize, usize)], target: u8, n: u8) -> Cage {
        let cells: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
        Cage::new(n, Polyomino::new(&cells), Operation::Add(target))
    }

    fn sub_cage(cells: &[(usize, usize)], target: u8, n: u8) -> Cage {
        let cells: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
        Cage::new(n, Polyomino::new(&cells), Operation::Subtract(target))
    }

    fn given_cage(cell: (usize, usize), value: u8, n: u8) -> Cage {
        let cells = vec![Cell::new(cell.0, cell.1)];
        Cage::new(n, Polyomino::new(&cells), Operation::Given(value))
    }

    #[test]
    fn op_legal_for_size_add_and_mul_are_size_agnostic() {
        assert!(op_legal_for_size(Operation::Add(3), 1));
        assert!(op_legal_for_size(Operation::Add(3), 5));
        assert!(op_legal_for_size(Operation::Multiply(12), 1));
        assert!(op_legal_for_size(Operation::Multiply(12), 5));
    }

    #[test]
    fn op_legal_for_size_sub_and_div_require_two() {
        assert!(!op_legal_for_size(Operation::Subtract(1), 1));
        assert!(op_legal_for_size(Operation::Subtract(1), 2));
        assert!(!op_legal_for_size(Operation::Subtract(1), 3));
        assert!(!op_legal_for_size(Operation::Divide(2), 1));
        assert!(op_legal_for_size(Operation::Divide(2), 2));
        assert!(!op_legal_for_size(Operation::Divide(2), 3));
    }

    #[test]
    fn op_legal_for_size_given_requires_one() {
        assert!(op_legal_for_size(Operation::Given(3), 1));
        assert!(!op_legal_for_size(Operation::Given(3), 2));
    }

    #[test]
    fn insert_cage_adds_cage() {
        let p = Puzzle::new(4).unwrap();
        let next = do_insert_cage(&p, &[(0, 0), (0, 1)], OpKind::Add, 3).unwrap();
        assert_eq!(next.cages().count(), 1);
        assert!(next.is_covered(Cell::new(0, 0)));
        assert!(next.is_covered(Cell::new(0, 1)));
    }

    #[test]
    fn insert_cage_rejects_empty_cells() {
        let p = Puzzle::new(4).unwrap();
        assert!(do_insert_cage(&p, &[], OpKind::Add, 3).is_err());
    }

    #[test]
    fn insert_cage_rejects_conflict() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        assert!(do_insert_cage(&p, &[(0, 1), (0, 2)], OpKind::Add, 5).is_err());
    }

    #[test]
    fn remove_cage_removes_cage_at_anchor() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        let next = do_remove_cage(&p, (0, 1)).unwrap();
        assert_eq!(next.cages().count(), 0);
    }

    #[test]
    fn remove_cage_errors_when_no_cage_at_anchor() {
        let p = Puzzle::new(4).unwrap();
        assert!(do_remove_cage(&p, (0, 0)).is_err());
    }

    #[test]
    fn extend_cage_keeps_cage_when_op_size_agnostic() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        let (next, draft) = do_extend_cage(&p, (0, 0), (0, 2)).unwrap();
        assert!(draft.is_none());
        let cage = next.cage_at(Cell::new(0, 2)).unwrap();
        assert_eq!(cage.cells().len(), 3);
        assert!(matches!(cage.operation(), Operation::Add(3)));
    }

    #[test]
    fn extend_cage_returns_draft_when_op_no_longer_legal() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(sub_cage(&[(0, 0), (0, 1)], 1, 4))
            .unwrap();
        let (next, draft) = do_extend_cage(&p, (0, 0), (0, 2)).unwrap();
        assert_eq!(next.cages().count(), 0);
        let draft = draft.unwrap();
        assert_eq!(draft.cells.len(), 3);
        assert!(draft.cells.contains(&(0, 0)));
        assert!(draft.cells.contains(&(0, 1)));
        assert!(draft.cells.contains(&(0, 2)));
    }

    #[test]
    fn extend_cage_returns_draft_when_given_grows_past_one() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(given_cage((0, 0), 2, 4))
            .unwrap();
        let (_next, draft) = do_extend_cage(&p, (0, 0), (0, 1)).unwrap();
        assert!(draft.is_some());
    }

    #[test]
    fn extend_cage_errors_when_target_already_covered() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap()
            .insert_cage(add_cage(&[(0, 2), (0, 3)], 5, 4))
            .unwrap();
        assert!(do_extend_cage(&p, (0, 0), (0, 2)).is_err());
    }

    #[test]
    fn extend_cage_errors_when_anchor_not_in_cage() {
        let p = Puzzle::new(4).unwrap();
        assert!(do_extend_cage(&p, (0, 0), (0, 1)).is_err());
    }

    #[test]
    fn extend_cage_errors_when_target_not_adjacent() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(given_cage((0, 0), 1, 4))
            .unwrap();
        assert!(do_extend_cage(&p, (0, 0), (2, 2)).is_err());
    }

    #[test]
    fn shrink_cage_removes_singleton() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(given_cage((0, 0), 1, 4))
            .unwrap();
        let (next, draft) = do_shrink_cage(&p, (0, 0)).unwrap();
        assert!(draft.is_none());
        assert!(!next.is_covered(Cell::new(0, 0)));
        assert_eq!(next.cages().count(), 0);
    }

    #[test]
    fn shrink_cage_keeps_cage_when_op_size_agnostic() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1), (0, 2)], 6, 4))
            .unwrap();
        let (next, draft) = do_shrink_cage(&p, (0, 2)).unwrap();
        assert!(draft.is_none());
        assert!(!next.is_covered(Cell::new(0, 2)));
        let cage = next.cage_at(Cell::new(0, 0)).unwrap();
        assert_eq!(cage.cells().len(), 2);
    }

    #[test]
    fn shrink_cage_returns_draft_when_op_no_longer_legal() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(sub_cage(&[(0, 0), (0, 1)], 1, 4))
            .unwrap();
        let (next, draft) = do_shrink_cage(&p, (0, 1)).unwrap();
        assert_eq!(next.cages().count(), 0);
        let draft = draft.unwrap();
        assert_eq!(draft.cells, vec![(0, 0)]);
    }

    #[test]
    fn shrink_cage_errors_when_cell_not_in_cage() {
        let p = Puzzle::new(4).unwrap();
        assert!(do_shrink_cage(&p, (0, 0)).is_err());
    }

    #[test]
    fn shrink_cage_errors_when_removal_would_disconnect() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1), (0, 2)], 6, 4))
            .unwrap();
        assert!(do_shrink_cage(&p, (0, 1)).is_err());
    }

    #[test]
    fn merge_cages_keeps_cage_when_op_size_agnostic() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap()
            .insert_cage(add_cage(&[(1, 0), (1, 1)], 5, 4))
            .unwrap();
        let (next, draft) = do_merge_cages(&p, (0, 0), (1, 0)).unwrap();
        assert!(draft.is_none());
        assert_eq!(next.cages().count(), 1);
        let cage = next.cage_at(Cell::new(0, 0)).unwrap();
        assert_eq!(cage.cells().len(), 4);
        assert!(matches!(cage.operation(), Operation::Add(3)));
    }

    #[test]
    fn merge_cages_returns_draft_when_op_invalid_for_size() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(sub_cage(&[(0, 0), (0, 1)], 1, 4))
            .unwrap()
            .insert_cage(given_cage((1, 0), 2, 4))
            .unwrap();
        let (next, draft) = do_merge_cages(&p, (0, 0), (1, 0)).unwrap();
        assert_eq!(next.cages().count(), 0);
        let draft = draft.unwrap();
        assert_eq!(draft.cells.len(), 3);
    }

    #[test]
    fn merge_cages_errors_when_anchors_in_same_cage() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        assert!(do_merge_cages(&p, (0, 0), (0, 1)).is_err());
    }

    #[test]
    fn do_set_cage_operation_replaces_op_and_target() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        let next = do_set_cage_operation(&p, (0, 0), OpKind::Mul, 12).unwrap();
        assert_eq!(next.cages().count(), 1);
        let cage = next.cage_at(kenken::Cell::new(0, 0)).unwrap();
        assert!(matches!(cage.operation(), Operation::Multiply(12)));
    }

    #[test]
    fn do_set_cage_operation_errors_when_no_cage_at_anchor() {
        let p = Puzzle::new(4).unwrap();
        assert!(do_set_cage_operation(&p, (0, 0), OpKind::Add, 3).is_err());
    }

    #[test]
    fn merge_cages_errors_when_anchor_not_in_a_cage() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        assert!(do_merge_cages(&p, (0, 0), (3, 3)).is_err());
    }
}
