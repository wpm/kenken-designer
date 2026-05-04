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

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
    Given,
}

fn split_operation(op: Operation) -> (OpKind, u32) {
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

        let cages = p
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

        Self { n, cells, cages }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
