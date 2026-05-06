use crate::grid::{ceil_sqrt, usize_to_f64};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct CellDiff {
    pub cell: (usize, usize),
    pub removed: Vec<u8>,
    pub added: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct PuzzleDiff {
    pub changes: Vec<CellDiff>,
}

impl PuzzleDiff {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlashEntry {
    pub x: f64,
    pub y: f64,
    pub value: u8,
    pub removed: bool,
}

struct SubGrid {
    cell_size: f64,
    digit_inset: f64,
    cols: usize,
    sub_w: f64,
    sub_h: f64,
}

impl SubGrid {
    fn new(cell_size: f64, digit_inset: f64, n: usize) -> Self {
        let cols = ceil_sqrt(n).max(1);
        let rows = n.div_ceil(cols).max(1);
        let inner = 2.0_f64.mul_add(-digit_inset, cell_size).max(0.0);
        Self {
            cell_size,
            digit_inset,
            cols,
            sub_w: inner / usize_to_f64(cols),
            sub_h: inner / usize_to_f64(rows),
        }
    }

    fn digit_center(&self, v: u8, cell_x: f64, cell_y: f64) -> (f64, f64) {
        let idx = usize::from(v.saturating_sub(1));
        let sub_r = idx / self.cols;
        let sub_c = idx % self.cols;
        let x = usize_to_f64(sub_c).mul_add(self.sub_w, cell_x + self.digit_inset) + self.sub_w / 2.0;
        let y = usize_to_f64(sub_r).mul_add(self.sub_h, cell_y + self.digit_inset) + self.sub_h / 2.0;
        (
            x.min(cell_x + self.cell_size - self.digit_inset),
            y.min(cell_y + self.cell_size - self.digit_inset),
        )
    }
}

#[must_use]
pub fn flash_entries(
    diff: &PuzzleDiff,
    cell_size: f64,
    margin: f64,
    n: usize,
    digit_inset: f64,
) -> Vec<FlashEntry> {
    if n == 0 {
        return vec![];
    }
    let sub = SubGrid::new(cell_size, digit_inset, n);

    let mut entries = Vec::new();
    for change in &diff.changes {
        let (row, col) = change.cell;
        let cell_x = usize_to_f64(col).mul_add(cell_size, margin);
        let cell_y = usize_to_f64(row).mul_add(cell_size, margin);

        for &v in &change.removed {
            let (x, y) = sub.digit_center(v, cell_x, cell_y);
            entries.push(FlashEntry {
                x,
                y,
                value: v,
                removed: true,
            });
        }
        for &v in &change.added {
            let (x, y) = sub.digit_center(v, cell_x, cell_y);
            entries.push(FlashEntry {
                x,
                y,
                value: v,
                removed: false,
            });
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_diff_is_inert() {
        let diff = PuzzleDiff::default();
        assert!(diff.is_empty());
        let entries = flash_entries(&diff, 100.0, 14.0, 4, 0.0);
        assert!(
            entries.is_empty(),
            "empty diff should produce no flash entries"
        );
    }

    #[test]
    fn removed_digits_appear_in_flash_state() {
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (0, 0),
                removed: vec![3, 4],
                added: vec![],
            }],
        };
        let entries = flash_entries(&diff, 100.0, 14.0, 4, 0.0);
        let removed: Vec<_> = entries.iter().filter(|e| e.removed).collect();
        assert_eq!(removed.len(), 2, "should have 2 removed flash entries");
        assert!(
            removed.iter().any(|e| e.value == 3),
            "digit 3 should appear"
        );
        assert!(
            removed.iter().any(|e| e.value == 4),
            "digit 4 should appear"
        );
        assert!(
            entries.iter().all(|e| e.removed),
            "no added entries expected"
        );
    }

    #[test]
    fn added_digits_appear_in_flash_state() {
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (1, 2),
                removed: vec![],
                added: vec![1, 2],
            }],
        };
        let entries = flash_entries(&diff, 100.0, 14.0, 4, 0.0);
        let added: Vec<_> = entries.iter().filter(|e| !e.removed).collect();
        assert_eq!(added.len(), 2, "should have 2 added flash entries");
        assert!(added.iter().any(|e| e.value == 1), "digit 1 should appear");
        assert!(added.iter().any(|e| e.value == 2), "digit 2 should appear");
        assert!(
            entries.iter().all(|e| !e.removed),
            "no removed entries expected"
        );
    }

    #[test]
    fn zero_n_returns_empty() {
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (0, 0),
                removed: vec![1],
                added: vec![],
            }],
        };
        assert!(flash_entries(&diff, 100.0, 14.0, 0, 0.0).is_empty());
    }

    #[test]
    fn flash_entries_positions_are_within_cell_bounds() {
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (0, 0),
                removed: vec![1, 2, 3, 4],
                added: vec![],
            }],
        };
        let cell_size = 100.0_f64;
        let margin = 14.0_f64;
        let entries = flash_entries(&diff, cell_size, margin, 4, 0.0);
        for entry in &entries {
            assert!(
                entry.x >= margin,
                "x={} should be >= margin={margin}",
                entry.x
            );
            assert!(
                entry.y >= margin,
                "y={} should be >= margin={margin}",
                entry.y
            );
            assert!(
                entry.x <= margin + cell_size,
                "x={} should be <= {}",
                entry.x,
                margin + cell_size
            );
            assert!(
                entry.y <= margin + cell_size,
                "y={} should be <= {}",
                entry.y,
                margin + cell_size
            );
        }
    }

    #[test]
    fn flash_entries_respect_digit_inset() {
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (0, 0),
                removed: vec![1, 2, 3, 4],
                added: vec![],
            }],
        };
        let cell_size = 100.0_f64;
        let margin = 14.0_f64;
        let digit_inset = 20.0_f64;
        let entries = flash_entries(&diff, cell_size, margin, 4, digit_inset);
        for entry in &entries {
            assert!(
                entry.x >= margin + digit_inset,
                "x={} should be >= margin+inset={}",
                entry.x,
                margin + digit_inset
            );
            assert!(
                entry.y >= margin + digit_inset,
                "y={} should be >= margin+inset={}",
                entry.y,
                margin + digit_inset
            );
            assert!(
                entry.x <= margin + cell_size - digit_inset,
                "x={} should be <= {}",
                entry.x,
                margin + cell_size - digit_inset
            );
            assert!(
                entry.y <= margin + cell_size - digit_inset,
                "y={} should be <= {}",
                entry.y,
                margin + cell_size - digit_inset
            );
        }
    }
}
