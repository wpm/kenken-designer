use kenken::{Operation, Puzzle};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PuzzleView {
    pub n: usize,
    pub cells: Vec<Vec<Vec<u8>>>,
    pub cages: Vec<CageView>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CageView {
    pub cells: Vec<(usize, usize)>,
    pub op: OpKind,
    pub target: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
    Given,
}

/// A cage whose shape is built but whose operation is not yet set.
///
/// Returned from shape-editing commands when the previous cage's operation is no longer
/// legal for the new size; the caller surfaces it as a transient draft until the user
/// picks an operation.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DraftCage {
    pub cells: Vec<(usize, usize)>,
}

/// Result of a shape-editing command: the puzzle view plus zero or more draft cages.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EditResult {
    pub view: PuzzleView,
    pub drafts: Vec<DraftCage>,
}

pub(crate) fn split_operation(op: Operation) -> (OpKind, u32) {
    match op {
        Operation::Add(n) => (OpKind::Add, u32::from(n)),
        Operation::Subtract(n) => (OpKind::Sub, u32::from(n)),
        Operation::Multiply(m) => (OpKind::Mul, u32::from(m)),
        Operation::Divide(n) => (OpKind::Div, u32::from(n)),
        Operation::Given(n) => (OpKind::Given, u32::from(n)),
    }
}

impl From<&Puzzle> for PuzzleView {
    fn from(p: &Puzzle) -> Self {
        let n = p.n();
        let mut cells = vec![vec![Vec::new(); n]; n];
        for (cell, values) in p.grid().iter_with_values() {
            cells[cell.row][cell.column] = values.iter().collect();
        }

        let mut cages: Vec<CageView> = p
            .cages()
            .map(|cage| {
                let (op, target) = split_operation(cage.operation());
                CageView {
                    cells: cage.cells().iter().map(|c| (c.row, c.column)).collect(),
                    op,
                    target,
                }
            })
            .collect();
        cages.sort_unstable_by_key(|c| c.cells.first().copied());

        Self { n, cells, cages }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kenken::{Cage, Cell, Polyomino};

    #[test]
    fn split_operation_maps_each_op_kind() {
        assert!(matches!(
            split_operation(Operation::Add(7)),
            (OpKind::Add, 7)
        ));
        assert!(matches!(
            split_operation(Operation::Subtract(3)),
            (OpKind::Sub, 3)
        ));
        assert!(matches!(
            split_operation(Operation::Multiply(12)),
            (OpKind::Mul, 12)
        ));
        assert!(matches!(
            split_operation(Operation::Divide(2)),
            (OpKind::Div, 2)
        ));
        assert!(matches!(
            split_operation(Operation::Given(4)),
            (OpKind::Given, 4)
        ));
    }

    #[test]
    fn puzzle_with_cages_round_trips_into_view() {
        let p = Puzzle::new(3).unwrap();
        let cage_a = Cage::new(
            3,
            Polyomino::new(&[Cell::new(0, 0), Cell::new(0, 1)]),
            Operation::Add(3),
        );
        let cage_b = Cage::new(3, Polyomino::new(&[Cell::new(2, 2)]), Operation::Given(2));
        let p = p.insert_cage(cage_a).unwrap().insert_cage(cage_b).unwrap();

        let v = PuzzleView::from(&p);
        assert_eq!(v.n, 3);
        assert_eq!(v.cages.len(), 2);

        let add_cage = v
            .cages
            .iter()
            .find(|c| matches!(c.op, OpKind::Add))
            .unwrap();
        assert_eq!(add_cage.target, 3);
        assert!(add_cage.cells.contains(&(0, 0)));
        assert!(add_cage.cells.contains(&(0, 1)));

        let given_cage = v
            .cages
            .iter()
            .find(|c| matches!(c.op, OpKind::Given))
            .unwrap();
        assert_eq!(given_cage.target, 2);
        assert_eq!(given_cage.cells, vec![(2, 2)]);
    }

    #[test]
    fn cages_are_ordered_by_row_major_anchor() {
        let p = Puzzle::new(3).unwrap();
        // Cages is backed by a HashMap so iteration order is unspecified; inserting
        // in reverse row-major order ensures the sort in PuzzleView::from is exercised.
        let cage_c = Cage::new(3, Polyomino::new(&[Cell::new(2, 0)]), Operation::Given(3));
        let cage_b = Cage::new(3, Polyomino::new(&[Cell::new(1, 0)]), Operation::Given(2));
        let cage_a = Cage::new(3, Polyomino::new(&[Cell::new(0, 0)]), Operation::Given(1));
        let p = p
            .insert_cage(cage_c)
            .unwrap()
            .insert_cage(cage_b)
            .unwrap()
            .insert_cage(cage_a)
            .unwrap();

        let v = PuzzleView::from(&p);
        assert_eq!(v.cages[0].cells, vec![(0, 0)]);
        assert_eq!(v.cages[1].cells, vec![(1, 0)]);
        assert_eq!(v.cages[2].cells, vec![(2, 0)]);
    }

    #[test]
    fn empty_puzzle_has_no_cages_and_full_candidates() {
        let p = Puzzle::new(4).unwrap();
        let v = PuzzleView::from(&p);
        assert_eq!(v.n, 4);
        assert_eq!(v.cells.len(), 4);
        assert!(v.cages.is_empty());
        for row in &v.cells {
            assert_eq!(row.len(), 4);
            for cell in row {
                assert_eq!(cell, &vec![1, 2, 3, 4]);
            }
        }
    }
}
