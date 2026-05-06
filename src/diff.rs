use crate::grid::Layout;

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

#[must_use]
pub fn flash_entries(diff: &PuzzleDiff, layout: Layout) -> Vec<FlashEntry> {
    if layout.n == 0 {
        return vec![];
    }
    let mut entries = Vec::new();
    for change in &diff.changes {
        let (row, col) = change.cell;
        for &v in &change.removed {
            let (x, y) = layout.sub_cell_center(row, col, v);
            entries.push(FlashEntry {
                x,
                y,
                value: v,
                removed: true,
            });
        }
        for &v in &change.added {
            let (x, y) = layout.sub_cell_center(row, col, v);
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

    fn test_layout(n: usize, cell: f64) -> Layout {
        Layout { n, cell }
    }

    #[test]
    fn empty_diff_is_inert() {
        let diff = PuzzleDiff::default();
        assert!(diff.is_empty());
        let entries = flash_entries(&diff, test_layout(4, 100.0));
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
        let entries = flash_entries(&diff, test_layout(4, 100.0));
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
        let entries = flash_entries(&diff, test_layout(4, 100.0));
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
        assert!(flash_entries(&diff, test_layout(0, 100.0)).is_empty());
    }

    #[test]
    fn flash_entries_share_layout_positions_with_grid() {
        let layout = test_layout(4, 100.0);
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (1, 2),
                removed: vec![],
                added: vec![3],
            }],
        };
        let entries = flash_entries(&diff, layout);
        assert_eq!(entries.len(), 1);
        let expected = layout.sub_cell_center(1, 2, 3);
        assert!((entries[0].x - expected.0).abs() < f64::EPSILON);
        assert!((entries[0].y - expected.1).abs() < f64::EPSILON);
    }

    #[test]
    fn flash_entries_respect_digit_inset() {
        let layout = test_layout(4, 100.0);
        let inset = layout.digit_inset();
        let diff = PuzzleDiff {
            changes: vec![CellDiff {
                cell: (0, 0),
                removed: vec![1, 2, 3, 4],
                added: vec![],
            }],
        };
        let entries = flash_entries(&diff, layout);
        let (left, top) = layout.origin(0, 0);
        let inner_left = left + inset;
        let inner_top = top + inset;
        let inner_right = left + layout.cell - inset;
        let inner_bottom = top + layout.cell - inset;
        for entry in &entries {
            assert!(
                entry.x >= inner_left,
                "x={} should be >= inner_left={inner_left}",
                entry.x
            );
            assert!(
                entry.y >= inner_top,
                "y={} should be >= inner_top={inner_top}",
                entry.y
            );
            assert!(
                entry.x <= inner_right,
                "x={} should be <= inner_right={inner_right}",
                entry.x
            );
            assert!(
                entry.y <= inner_bottom,
                "y={} should be <= inner_bottom={inner_bottom}",
                entry.y
            );
        }
    }
}
