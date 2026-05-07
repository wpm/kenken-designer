use kenken::{Cage, Cell, Operation, Operator, Polyomino, Puzzle};

use crate::view::{CageOption, DraftCage, OpKind};

const fn op_kind_of(op: Operator) -> OpKind {
    match op {
        Operator::Add => OpKind::Add,
        Operator::Subtract => OpKind::Sub,
        Operator::Multiply => OpKind::Mul,
        Operator::Divide => OpKind::Div,
        Operator::Given => OpKind::Given,
    }
}

const fn target_of(op: Operation) -> u32 {
    match op {
        Operation::Add(t)
        | Operation::Subtract(t)
        | Operation::Multiply(t)
        | Operation::Divide(t)
        | Operation::Given(t) => t as u32,
    }
}

/// Returns valid (operator, targets) pairs for a cage of shape `cells` on an `n`x`n` grid.
///
/// Wraps `Cage::valid_operators` and `Cage::valid_targets` from the kenken crate. Used by
/// the picker UI: step 1 lists `op` values; step 2 lists `targets` for the chosen op.
pub fn cage_options(cells: &[(usize, usize)], n: usize) -> Vec<CageOption> {
    let Ok(n_u8) = u8::try_from(n) else {
        return Vec::new();
    };
    if n_u8 == 0 {
        return Vec::new();
    }
    let cells_vec: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
    Cage::valid_operators(&cells_vec)
        .into_iter()
        .map(|op| CageOption {
            op: op_kind_of(op),
            targets: Cage::valid_targets(&cells_vec, op, n_u8)
                .map(target_of)
                .collect(),
        })
        .collect()
}

const fn op_legal_for_size(op: Operation, size: usize) -> bool {
    match op {
        Operation::Add(_) | Operation::Multiply(_) => true,
        Operation::Subtract(_) | Operation::Divide(_) => size == 2,
        Operation::Given(_) => size == 1,
    }
}

#[allow(clippy::cast_possible_truncation)]
pub const fn build_operation(op: OpKind, target: u32) -> Operation {
    match op {
        OpKind::Add => Operation::Add(target as u16),
        OpKind::Sub => Operation::Subtract(target as u16),
        OpKind::Mul => Operation::Multiply(target as u16),
        OpKind::Div => Operation::Divide(target as u16),
        OpKind::Given => Operation::Given(target as u16),
    }
}

fn puzzle_size_u8(puzzle: &Puzzle) -> Result<u8, String> {
    u8::try_from(puzzle.n()).map_err(|e| format!("invalid puzzle size: {e}"))
}

fn cells_to_vec(poly: &Polyomino) -> Vec<(usize, usize)> {
    poly.as_slice().iter().map(|c| (c.row, c.column)).collect()
}

pub fn cage_at_or_err(puzzle: &Puzzle, anchor: (usize, usize)) -> Result<&Cage, String> {
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

pub fn do_clear_all_cages(puzzle: Puzzle) -> Puzzle {
    let polys: Vec<_> = puzzle.cages().map(|c| c.polyomino().clone()).collect();
    polys.iter().fold(puzzle, Puzzle::remove_cage)
}

/// Returns anchor cells (row-major sorted) of cages that are legal targets for moving `cell`.
///
/// A cage is a legal target if:
/// - It is 4-adjacent to `cell`
/// - It is a different cage from `cell`'s current cage
///
/// Returns empty if: cell is in no cage, or removing cell from its cage would disconnect it
/// (unless the cage is a singleton).
pub fn legal_move_targets(puzzle: &Puzzle, cell: (usize, usize)) -> Vec<(usize, usize)> {
    let cell_obj = Cell::new(cell.0, cell.1);
    let Some(src_cage) = puzzle.cage_at(cell_obj) else {
        return Vec::new();
    };
    let src_poly = src_cage.polyomino().clone();

    // If src has more than one cell, check that removing `cell` keeps it connected.
    if src_poly.len() > 1 && src_poly.without(cell_obj).is_err() {
        return Vec::new();
    }

    let n = puzzle.n();
    let mut targets: Vec<(usize, usize)> = Vec::new();

    for neighbor in cell_obj.neighbors_4() {
        if neighbor.row >= n || neighbor.column >= n {
            continue;
        }
        if let Some(tgt_cage) = puzzle.cage_at(neighbor) {
            if tgt_cage.polyomino() != &src_poly {
                let anchor = tgt_cage
                    .polyomino()
                    .as_slice()
                    .first()
                    .map_or((0, 0), |c| (c.row, c.column));
                if !targets.contains(&anchor) {
                    targets.push(anchor);
                }
            }
        }
    }

    targets.sort_unstable();
    targets
}

/// Shared mutation logic for `do_move_cell` and `do_flip_cell`: removes `cell_obj` from
/// `src_poly`, adds it to `tgt_poly`, and reinserts both (or drafts them) as appropriate.
fn apply_cell_transfer(
    puzzle: &Puzzle,
    n: u8,
    cell_obj: Cell,
    src_poly: &Polyomino,
    src_op: Operation,
    tgt_poly: &Polyomino,
    tgt_op: Operation,
) -> Result<(Puzzle, Vec<DraftCage>), String> {
    let new_tgt_poly = tgt_poly.extend(cell_obj).map_err(|e| format!("{e:?}"))?;
    let intermediate = puzzle.clone().remove_cage(src_poly).remove_cage(tgt_poly);
    let mut drafts = Vec::new();

    let next = if src_poly.len() == 1 {
        intermediate
    } else {
        let new_src_poly = src_poly.without(cell_obj).map_err(|e| format!("{e:?}"))?;
        if op_legal_for_size(src_op, new_src_poly.len()) {
            intermediate
                .insert_cage(Cage::new(n, new_src_poly, src_op))
                .map_err(|e| format!("{e:?}"))?
        } else {
            drafts.push(DraftCage {
                cells: cells_to_vec(&new_src_poly),
            });
            intermediate
        }
    };

    if op_legal_for_size(tgt_op, new_tgt_poly.len()) {
        let next = next
            .insert_cage(Cage::new(n, new_tgt_poly, tgt_op))
            .map_err(|e| format!("{e:?}"))?;
        Ok((next, drafts))
    } else {
        drafts.push(DraftCage {
            cells: cells_to_vec(&new_tgt_poly),
        });
        Ok((next, drafts))
    }
}

pub fn do_move_cell(
    puzzle: &Puzzle,
    cell: (usize, usize),
    target_anchor: (usize, usize),
) -> Result<(Puzzle, Vec<DraftCage>), String> {
    let n = puzzle_size_u8(puzzle)?;
    let cell_obj = Cell::new(cell.0, cell.1);

    let src_cage = puzzle
        .cage_at(cell_obj)
        .ok_or_else(|| format!("no cage at ({}, {})", cell.0, cell.1))?;
    let src_op = src_cage.operation();
    let src_poly = src_cage.polyomino().clone();

    let tgt_cage = cage_at_or_err(puzzle, target_anchor)?;
    if tgt_cage.polyomino() == &src_poly {
        return Err("target cage is the same as source cage".into());
    }
    let tgt_op = tgt_cage.operation();
    let tgt_poly = tgt_cage.polyomino().clone();

    if !cell_obj.neighbors_4().any(|nb| tgt_poly.contains_cell(nb)) {
        return Err("cell is not adjacent to target cage".into());
    }
    if src_poly.len() > 1 && src_poly.without(cell_obj).is_err() {
        return Err("removing cell would disconnect source cage".into());
    }

    apply_cell_transfer(puzzle, n, cell_obj, &src_poly, src_op, &tgt_poly, tgt_op)
}

pub fn do_flip_cell(
    puzzle: &Puzzle,
    cell: (usize, usize),
    target_anchor: (usize, usize),
) -> Result<(Puzzle, Vec<DraftCage>), String> {
    let n = puzzle_size_u8(puzzle)?;
    let cell_obj = Cell::new(cell.0, cell.1);

    let src_cage = puzzle
        .cage_at(cell_obj)
        .ok_or_else(|| format!("no cage at ({}, {})", cell.0, cell.1))?;
    let src_op = src_cage.operation();
    let src_poly = src_cage.polyomino().clone();

    let tgt_cage = cage_at_or_err(puzzle, target_anchor)?;
    if tgt_cage.polyomino() == &src_poly {
        return Err("cell is already in target cage".into());
    }
    let tgt_op = tgt_cage.operation();
    let tgt_poly = tgt_cage.polyomino().clone();

    apply_cell_transfer(puzzle, n, cell_obj, &src_poly, src_op, &tgt_poly, tgt_op)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kenken::{Cage, Operation, Polyomino};

    fn add_cage(cells: &[(usize, usize)], target: u16, n: u8) -> Cage {
        let cells: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
        Cage::new(n, Polyomino::new(&cells), Operation::Add(target))
    }

    fn sub_cage(cells: &[(usize, usize)], target: u16, n: u8) -> Cage {
        let cells: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
        Cage::new(n, Polyomino::new(&cells), Operation::Subtract(target))
    }

    fn given_cage(cell: (usize, usize), value: u16, n: u8) -> Cage {
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
    fn cage_options_singleton_returns_given_with_full_value_range() {
        let options = cage_options(&[(0, 0)], 4);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].op, OpKind::Given);
        assert_eq!(options[0].targets, vec![1, 2, 3, 4]);
    }

    #[test]
    fn cage_options_two_cells_yields_all_binary_operators() {
        let options = cage_options(&[(0, 0), (1, 1)], 4);
        let ops: Vec<OpKind> = options.iter().map(|o| o.op).collect();
        assert_eq!(
            ops,
            vec![OpKind::Add, OpKind::Sub, OpKind::Mul, OpKind::Div]
        );

        let sub = options.iter().find(|o| o.op == OpKind::Sub).unwrap();
        assert_eq!(sub.targets, vec![1, 2, 3]);
        let div = options.iter().find(|o| o.op == OpKind::Div).unwrap();
        assert_eq!(div.targets, vec![2, 3, 4]);
    }

    #[test]
    fn cage_options_three_cells_yields_only_commutative_operators() {
        let options = cage_options(&[(0, 0), (0, 1), (1, 0)], 4);
        let ops: Vec<OpKind> = options.iter().map(|o| o.op).collect();
        assert_eq!(ops, vec![OpKind::Add, OpKind::Mul]);
    }

    #[test]
    fn cage_options_empty_cells_returns_empty() {
        assert!(cage_options(&[], 4).is_empty());
    }

    #[test]
    fn cage_options_zero_n_returns_empty() {
        assert!(cage_options(&[(0, 0)], 0).is_empty());
    }

    #[test]
    fn cage_options_targets_are_ascending() {
        let options = cage_options(&[(0, 0), (0, 1), (1, 0)], 5);
        for opt in &options {
            assert!(opt.targets.windows(2).all(|w| w[0] < w[1]));
        }
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

    #[test]
    fn flip_cell_moves_cell_between_cages() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap()
            .insert_cage(add_cage(&[(1, 0), (1, 1)], 5, 4))
            .unwrap();
        let (next, drafts) = do_flip_cell(&p, (0, 0), (1, 0)).unwrap();
        assert!(drafts.is_empty());
        assert_eq!(next.cages().count(), 2);
        let src = next.cage_at(Cell::new(0, 1)).unwrap();
        assert_eq!(src.cells().len(), 1);
        let tgt = next.cage_at(Cell::new(0, 0)).unwrap();
        assert_eq!(tgt.cells().len(), 3);
    }

    #[test]
    fn flip_cell_returns_draft_when_src_op_invalid() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(sub_cage(&[(0, 0), (0, 1)], 1, 4))
            .unwrap()
            .insert_cage(add_cage(&[(1, 0), (1, 1)], 5, 4))
            .unwrap();
        let (next, drafts) = do_flip_cell(&p, (0, 0), (1, 0)).unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].cells, vec![(0, 1)]);
        assert_eq!(next.cages().count(), 1);
    }

    #[test]
    fn flip_cell_returns_draft_when_tgt_op_invalid() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap()
            .insert_cage(sub_cage(&[(1, 0), (1, 1)], 1, 4))
            .unwrap();
        let (next, drafts) = do_flip_cell(&p, (0, 0), (1, 0)).unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(next.cages().count(), 1);
        let src = next.cage_at(Cell::new(0, 1)).unwrap();
        assert_eq!(src.cells().len(), 1);
    }

    #[test]
    fn flip_cell_removes_singleton_src_without_draft() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(given_cage((0, 0), 1, 4))
            .unwrap()
            .insert_cage(add_cage(&[(1, 0), (1, 1)], 5, 4))
            .unwrap();
        let (next, drafts) = do_flip_cell(&p, (0, 0), (1, 0)).unwrap();
        assert!(drafts.is_empty());
        assert_eq!(next.cages().count(), 1);
        let tgt = next.cage_at(Cell::new(0, 0)).unwrap();
        assert_eq!(tgt.cells().len(), 3);
    }

    #[test]
    fn flip_cell_errors_when_cell_not_in_cage() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(1, 0), (1, 1)], 5, 4))
            .unwrap();
        assert!(do_flip_cell(&p, (0, 0), (1, 0)).is_err());
    }

    #[test]
    fn flip_cell_errors_when_target_not_in_cage() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        assert!(do_flip_cell(&p, (0, 0), (1, 0)).is_err());
    }

    #[test]
    fn flip_cell_errors_when_cell_already_in_target_cage() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap();
        assert!(do_flip_cell(&p, (0, 0), (0, 1)).is_err());
    }

    #[test]
    fn flip_cell_errors_when_target_cage_not_adjacent() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap()
            .insert_cage(add_cage(&[(2, 0), (2, 1)], 5, 4))
            .unwrap();
        assert!(do_flip_cell(&p, (0, 0), (2, 0)).is_err());
    }

    // --- legal_move_targets tests ---

    /// 5-cell I-pentomino in row 0: inner cells (columns 1, 2, 3) cannot be removed
    /// without disconnecting the source cage, so they get no legal targets.
    #[test]
    fn legal_move_targets_excludes_disconnecting_cells() {
        // Row 0: cells (0,0)..(0,4) form a single cage.
        // A 6x6 puzzle gives room for adjacent cages in row 1.
        let p = Puzzle::new(6)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)], 15, 6))
            .unwrap()
            .insert_cage(add_cage(&[(1, 2)], 2, 6))
            .unwrap(); // adjacent to inner cell (0,2)
                       // Inner cell (0,2) — removing it disconnects the I-pentomino
        assert!(legal_move_targets(&p, (0, 2)).is_empty());
    }

    /// A corner cell in a 3-cell L-cage is adjacent to two different cages.
    /// Both should appear in sorted row-major order.
    #[test]
    fn legal_move_targets_includes_corner_neighbors() {
        // Cage A: (0,0), (0,1)  anchor (0,0)
        // Cage B: (1,0), (1,1)  anchor (1,0)
        // Moving (0,0) from Cage A: it is adjacent to Cage B via (1,0).
        // (0,0) is an end cell of the 2-cell cage so removal keeps it connected (only 1 remains).
        // Actually "removing from 2-cell cage" produces singleton — still connected.
        // Cage B anchor should appear.
        // Let's also add Cage C adjacent to (0,0) from the left — not possible at column 0.
        // Instead use a 3x3 grid: cage at (0,1) adjacent to (0,0) too.
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (1, 0)], 3, 4))
            .unwrap()
            .insert_cage(add_cage(&[(0, 1), (0, 2)], 3, 4))
            .unwrap()
            .insert_cage(add_cage(&[(1, 1), (1, 2)], 5, 4))
            .unwrap();
        // Moving (0,0) from cage (0,0)-(1,0): it is adjacent to cage anchored at (0,1).
        // After removal source becomes singleton (1,0) — still connected.
        let targets = legal_move_targets(&p, (0, 0));
        assert!(
            targets.contains(&(0, 1)),
            "should include cage anchored at (0,1): got {targets:?}"
        );
        // Not adjacent to (1,1) cage directly from (0,0) — (1,1) is diagonal not 4-adjacent
        // Actually (0,0)'s 4-neighbors are (1,0) [same cage] and (0,1) [target cage].
        assert_eq!(targets, vec![(0, 1)], "expected exactly one target");
    }

    /// A 1-cell singleton cage adjacent to exactly one other cage → that cage is the sole target.
    #[test]
    fn legal_move_targets_includes_only_target_for_singleton_source() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0)], 1, 4))
            .unwrap()
            .insert_cage(add_cage(&[(0, 1), (0, 2)], 3, 4))
            .unwrap();
        let targets = legal_move_targets(&p, (0, 0));
        assert_eq!(targets, vec![(0, 1)]);
    }

    // --- do_move_cell tests ---

    /// 2-cell Add(3) cage adjacent to 3-cell Mul(24) cage; move corner cell.
    /// Both cages should stay valid (Add and Mul are size-agnostic).
    #[test]
    fn move_cell_command_happy_path() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1)], 3, 4))
            .unwrap()
            .insert_cage(mul_cage(&[(1, 0), (1, 1), (1, 2)], 24, 4))
            .unwrap();
        // Move (0,0) from the Add cage to the Mul cage — (0,0) is adjacent to (1,0).
        let (next, drafts) = do_move_cell(&p, (0, 0), (1, 0)).unwrap();
        assert!(drafts.is_empty(), "no draft expected: {drafts:?}");
        // Source cage now has 1 cell: (0,1)
        assert!(next.is_covered(Cell::new(0, 1)));
        let src = next.cage_at(Cell::new(0, 1)).unwrap();
        assert_eq!(src.cells().len(), 1);
        // Target cage now has 4 cells
        let tgt = next.cage_at(Cell::new(1, 0)).unwrap();
        assert_eq!(tgt.cells().len(), 4);
    }

    /// Moving the only cell of a singleton cage causes the source cage to be removed.
    #[test]
    fn move_cell_command_deletes_source_when_last_cell_moves() {
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0)], 1, 4))
            .unwrap()
            .insert_cage(add_cage(&[(0, 1), (0, 2)], 3, 4))
            .unwrap();
        let (next, drafts) = do_move_cell(&p, (0, 0), (0, 1)).unwrap();
        assert!(drafts.is_empty());
        // Source cage (0,0) should no longer exist
        assert!(
            !next.is_covered(Cell::new(0, 0)) || {
                // (0,0) might now be part of the target — check it's in the target
                next.cage_at(Cell::new(0, 0))
                    .is_some_and(|c| c.cells().len() == 3)
            }
        );
        // Target should have 3 cells now
        let tgt = next.cage_at(Cell::new(0, 0)).unwrap();
        assert_eq!(tgt.cells().len(), 3);
        // Only 1 cage total
        assert_eq!(next.cages().count(), 1);
    }

    /// Trying to move an inner cell of an I-pentomino should fail.
    #[test]
    fn move_cell_command_rejects_disconnecting_move() {
        let p = Puzzle::new(6)
            .unwrap()
            .insert_cage(add_cage(&[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)], 15, 6))
            .unwrap()
            .insert_cage(add_cage(&[(1, 2)], 2, 6))
            .unwrap();
        // Inner cell (0,2): removing it disconnects source
        assert!(do_move_cell(&p, (0, 2), (1, 2)).is_err());
    }

    fn mul_cage(cells: &[(usize, usize)], target: u16, n: u8) -> Cage {
        let cells: Vec<Cell> = cells.iter().map(|&(r, c)| Cell::new(r, c)).collect();
        Cage::new(n, Polyomino::new(&cells), Operation::Multiply(target))
    }
}
