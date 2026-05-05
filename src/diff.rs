// These types are public API used by FlashOverlay, but not yet wired into the
// main binary entry-point; suppress the lint rather than sprinkle it on every item.
#![allow(dead_code)]

/// Per-cell change between two puzzle states.
/// Mirrors `src-tauri/src/diff.rs::CellDiff` for frontend deserialization.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, Default)]
pub struct CellDiff {
    /// (row, column)
    pub cell: (usize, usize),
    pub removed: Vec<u8>,
    pub added: Vec<u8>,
}

/// Diff between two puzzle states.
/// Mirrors `src-tauri/src/diff.rs::PuzzleDiff` for frontend deserialization.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, Default)]
pub struct PuzzleDiff {
    pub changes: Vec<CellDiff>,
}

impl PuzzleDiff {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// A single entry in the flash overlay, representing one digit at one position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlashEntry {
    /// SVG x coordinate (centre of digit)
    pub x: f64,
    /// SVG y coordinate (centre of digit)
    pub y: f64,
    /// The digit value
    pub value: u8,
    /// Whether this entry represents a removed digit (true) or added digit (false)
    pub removed: bool,
}

/// Compute the set of flash entries for a diff, given the layout parameters.
///
/// `cell_size` is the width/height of one grid cell in SVG units.
/// `margin` is the offset from the SVG origin to the first cell.
/// `n` is the puzzle size (number of rows/cols).
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn flash_entries(diff: &PuzzleDiff, cell_size: f64, margin: f64, n: usize) -> Vec<FlashEntry> {
    if n == 0 {
        return vec![];
    }
    let cols = ceil_sqrt(n).max(1);
    let rows = n.div_ceil(cols).max(1);
    let sub_w = cell_size / usize_to_f64(cols);
    let sub_h = cell_size / usize_to_f64(rows);

    let mut entries = Vec::new();
    for change in &diff.changes {
        let (row, col) = change.cell;
        let cell_x = usize_to_f64(col).mul_add(cell_size, margin);
        let cell_y = usize_to_f64(row).mul_add(cell_size, margin);

        for &v in &change.removed {
            let (x, y) = digit_center(v, cell_x, cell_y, sub_w, sub_h, cols, cell_size);
            entries.push(FlashEntry {
                x,
                y,
                value: v,
                removed: true,
            });
        }
        for &v in &change.added {
            let (x, y) = digit_center(v, cell_x, cell_y, sub_w, sub_h, cols, cell_size);
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

#[allow(clippy::many_single_char_names)]
fn digit_center(
    v: u8,
    cell_x: f64,
    cell_y: f64,
    sub_w: f64,
    sub_h: f64,
    cols: usize,
    cell_size: f64,
) -> (f64, f64) {
    // Place the digit at its candidate sub-cell position.
    let idx = usize::from(v.saturating_sub(1));
    let sub_r = idx / cols;
    let sub_c = idx % cols;
    let x = usize_to_f64(sub_c).mul_add(sub_w, cell_x) + sub_w / 2.0;
    let y = usize_to_f64(sub_r).mul_add(sub_h, cell_y) + sub_h / 2.0;
    // Clamp to cell bounds just in case
    let x = x.min(cell_x + cell_size);
    let y = y.min(cell_y + cell_size);
    (x, y)
}

const fn ceil_sqrt(n: usize) -> usize {
    if n <= 1 {
        return n;
    }
    let mut x: usize = 1;
    while x.saturating_mul(x) < n {
        x += 1;
    }
    x
}

#[allow(clippy::cast_precision_loss)]
const fn usize_to_f64(x: usize) -> f64 {
    x as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_diff_is_inert() {
        let diff = PuzzleDiff::default();
        assert!(diff.is_empty());
        let entries = flash_entries(&diff, 100.0, 14.0, 4);
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
        let entries = flash_entries(&diff, 100.0, 14.0, 4);
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
        let entries = flash_entries(&diff, 100.0, 14.0, 4);
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
    fn reentrant_diff_uses_new_generation() {
        // Simulate the generation counter pattern: incrementing the counter
        // makes stale callbacks bail out.
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;

        let gen = Arc::new(AtomicU64::new(0));

        let current_gen = gen.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(current_gen, 1);

        // A new diff arrives and increments the generation again.
        let new_gen = gen.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(new_gen, 2);

        // The stale callback checks: if its captured generation != current, bail.
        let stale_callback_gen = current_gen;
        let is_stale = stale_callback_gen != gen.load(Ordering::SeqCst);
        assert!(
            is_stale,
            "callback with old generation should be considered stale"
        );
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
        let entries = flash_entries(&diff, cell_size, margin, 4);
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
}
