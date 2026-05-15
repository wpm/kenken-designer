use crate::cage_edit::build_operation;
use crate::view::{split_operation, CageView};
use kenken::{Cage, Cell, Cover, Polyomino, Puzzle};

/// Serializable puzzle representation (version 1).
///
/// Reuses `CageView` (which already carries cells, op, target), so there is
/// one canonical cage-wire format shared by both the live view and the file format.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PuzzleData {
    pub n: usize,
    pub cages: Vec<CageView>,
}

pub type PuzzleV1 = PuzzleData;

/// Versioned save envelope.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SaveEnvelope {
    pub version: u32,
    pub puzzle: PuzzleV1,
}

/// Errors that can occur when loading a puzzle.
#[derive(Debug)]
pub enum LoadError {
    UnsupportedVersion(u32),
    Parse(String),
    Io(String),
    KenKen(kenken::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            Self::Parse(s) => write!(f, "parse error: {s}"),
            Self::Io(s) => write!(f, "I/O error: {s}"),
            Self::KenKen(e) => write!(f, "puzzle error: {e}"),
        }
    }
}

impl From<kenken::Error> for LoadError {
    fn from(e: kenken::Error) -> Self {
        Self::KenKen(e)
    }
}

pub fn load(envelope: SaveEnvelope) -> Result<Puzzle, LoadError> {
    match envelope.version {
        1 => reconstruct(envelope.puzzle),
        n => Err(LoadError::UnsupportedVersion(n)),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn reconstruct(data: PuzzleData) -> Result<Puzzle, LoadError> {
    let mut puzzle = Puzzle::new(data.n)?;
    let n = data.n as u8;
    for cage_view in data.cages {
        let cells: Vec<Cell> = cage_view
            .cells
            .iter()
            .map(|&(r, c)| Cell::new(r, c))
            .collect();
        let polyomino = Polyomino::new(&cells)?;
        let op = build_operation(cage_view.op, cage_view.target);
        puzzle = puzzle.insert_cage(Cage::new(n, polyomino, op))?;
    }
    Ok(puzzle)
}

fn puzzle_to_data(puzzle: &Puzzle) -> PuzzleData {
    let n = puzzle.n();
    let cages = puzzle
        .cages()
        .map(|cage| {
            let cells = cage.cells().iter().map(|c| (c.row, c.column)).collect();
            let (op, target) = split_operation(cage.operation());
            CageView { cells, op, target }
        })
        .collect();
    PuzzleData { n, cages }
}

pub fn save(puzzle: &Puzzle, path: &str) -> Result<(), LoadError> {
    let envelope = SaveEnvelope {
        version: 1,
        puzzle: puzzle_to_data(puzzle),
    };
    let json =
        serde_json::to_string_pretty(&envelope).map_err(|e| LoadError::Parse(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| LoadError::Io(e.to_string()))
}

pub fn load_from_path(path: &str) -> Result<Puzzle, LoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| LoadError::Io(e.to_string()))?;
    let envelope: SaveEnvelope =
        serde_json::from_str(&content).map_err(|e| LoadError::Parse(e.to_string()))?;
    load(envelope)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kenken::{Cage, Cell, Cover, Operation, Polyomino};

    fn make_5x5_puzzle() -> Puzzle {
        let p = Puzzle::new(5).unwrap();
        let cage_a = Cage::new(
            5,
            Polyomino::new(&[Cell::new(0, 0), Cell::new(0, 1)]).unwrap(),
            Operation::Add(5),
        );
        let cage_b = Cage::new(
            5,
            Polyomino::new(&[Cell::new(1, 0)]).unwrap(),
            Operation::Given(3),
        );
        p.insert_cage(cage_a).unwrap().insert_cage(cage_b).unwrap()
    }

    #[test]
    fn save_and_load_round_trips_a_5x5_puzzle() {
        let puzzle = make_5x5_puzzle();
        let path = std::env::temp_dir()
            .join("kenken_test_roundtrip.kenken")
            .to_str()
            .unwrap()
            .to_string();

        save(&puzzle, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();

        assert_eq!(puzzle.n(), loaded.n());
        assert_eq!(puzzle.cages().count(), loaded.cages().count());

        // Verify cage contents survive the round-trip: look up the two-cell
        // Add cage (anchor at (0,0)) in the loaded puzzle and confirm its
        // operation and cells match the original.
        let original_add_cage = puzzle
            .cages()
            .find(|c| {
                c.cells()
                    .iter()
                    .any(|cell| cell.row == 0 && cell.column == 0)
            })
            .unwrap();
        let loaded_add_cage = loaded
            .cages()
            .find(|c| {
                c.cells()
                    .iter()
                    .any(|cell| cell.row == 0 && cell.column == 0)
            })
            .unwrap();
        assert_eq!(
            original_add_cage.operation(),
            loaded_add_cage.operation(),
            "Add cage operation should survive round-trip"
        );
        let original_cells: std::collections::BTreeSet<(usize, usize)> = original_add_cage
            .cells()
            .iter()
            .map(|c| (c.row, c.column))
            .collect();
        let loaded_cells: std::collections::BTreeSet<(usize, usize)> = loaded_add_cage
            .cells()
            .iter()
            .map(|c| (c.row, c.column))
            .collect();
        assert_eq!(
            original_cells, loaded_cells,
            "Add cage cells should survive round-trip"
        );
    }

    #[test]
    fn load_rejects_unknown_version() {
        let path = std::env::temp_dir()
            .join("kenken_test_unknown_version.kenken")
            .to_str()
            .unwrap()
            .to_string();
        std::fs::write(
            &path,
            r#"{"version": 999, "puzzle": {"n": 4, "cages": []}}"#,
        )
        .unwrap();

        let err = load_from_path(&path).unwrap_err();
        assert!(
            matches!(err, LoadError::UnsupportedVersion(999)),
            "Expected UnsupportedVersion(999), got {err:?}"
        );
    }

    #[test]
    fn load_rejects_malformed_json() {
        let path = std::env::temp_dir()
            .join("kenken_test_malformed.kenken")
            .to_str()
            .unwrap()
            .to_string();
        std::fs::write(&path, "").unwrap();

        let err = load_from_path(&path).unwrap_err();
        assert!(
            matches!(err, LoadError::Parse(_)),
            "Expected Parse error, got {err:?}"
        );
    }

    #[test]
    fn save_writes_pretty_printed_json() {
        let puzzle = Puzzle::new(3).unwrap();
        let path = std::env::temp_dir()
            .join("kenken_test_pretty.kenken")
            .to_str()
            .unwrap()
            .to_string();

        save(&puzzle, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains('\n'),
            "File should contain newlines (pretty-printed)"
        );
    }
}
