use crate::app::OpKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveCage {
    Committed(usize),
    Draft,
}

/// One operator and its valid targets for a cage shape, fetched from the backend
/// `cage_options` command and used to drive both the operator picker and the target
/// dropdown.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CageOption {
    pub op: OpKind,
    pub targets: Vec<u32>,
}

/// Editing state for the active cage: either picking an operator (step 1) or picking
/// a target value for an already-chosen operator (step 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryMode {
    OpPicker,
    TargetPicker {
        op: OpKind,
        selected: usize,
        digits: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorEntry {
    pub cage: ActiveCage,
    pub options: Vec<CageOption>,
    pub mode: EntryMode,
}

pub enum Step {
    Update(OperatorEntry),
    Commit { op: OpKind, target: u32 },
    Cancel,
}

pub fn is_entry_trigger_key(key: &str) -> bool {
    matches!(
        key,
        "+" | "-"
            | "*"
            | "x"
            | "X"
            | "/"
            | "0"
            | "1"
            | "2"
            | "3"
            | "4"
            | "5"
            | "6"
            | "7"
            | "8"
            | "9"
    )
}

/// Maps a keyboard key to the operator it represents, or `None` for non-operator keys.
pub fn op_key_to_kind(key: &str) -> Option<OpKind> {
    match key {
        "+" => Some(OpKind::Add),
        "-" => Some(OpKind::Sub),
        "*" | "x" | "X" => Some(OpKind::Mul),
        "/" => Some(OpKind::Div),
        _ => None,
    }
}

pub fn targets_for_op(options: &[CageOption], op: OpKind) -> &[u32] {
    options
        .iter()
        .find(|o| o.op == op)
        .map_or(&[][..], |o| &o.targets)
}

/// Builds an entry pointing at `cage` with options pre-fetched from the backend.
///
/// Singleton cages (whose only valid operator is `Given`) skip step 1 and start in
/// `TargetPicker` with the singleton's existing target (if any) selected. Multi-cell
/// cages start in `OpPicker`. Pass `current_target` to pre-select it in `TargetPicker`
/// when re-editing a singleton.
pub fn enter_picker(
    cage: ActiveCage,
    options: Vec<CageOption>,
    current_target: Option<u32>,
) -> OperatorEntry {
    let singleton_only = matches!(options.as_slice(), [only] if only.op == OpKind::Given);
    let mode = if singleton_only {
        let targets = &options[0].targets;
        let selected = current_target
            .and_then(|t| targets.iter().position(|&x| x == t))
            .unwrap_or(0);
        EntryMode::TargetPicker {
            op: OpKind::Given,
            selected,
            digits: String::new(),
        }
    } else {
        EntryMode::OpPicker
    };
    OperatorEntry {
        cage,
        options,
        mode,
    }
}

/// Like `enter_picker`, but the user has just typed an operator key — skip
/// `OpPicker` and start in `TargetPicker` for that op. Returns `None` if the key is
/// not a valid op for this cage shape.
pub fn enter_picker_with_op(
    cage: ActiveCage,
    options: Vec<CageOption>,
    op: OpKind,
    current_target: Option<u32>,
) -> Option<OperatorEntry> {
    let targets = targets_for_op(&options, op);
    if targets.is_empty() {
        return None;
    }
    let selected = current_target
        .and_then(|t| targets.iter().position(|&x| x == t))
        .unwrap_or(0);
    Some(OperatorEntry {
        cage,
        mode: EntryMode::TargetPicker {
            op,
            selected,
            digits: String::new(),
        },
        options,
    })
}

/// Pure state-machine step for the operator/target picker.
pub fn step(entry: OperatorEntry, key: &str) -> Step {
    if key == "Escape" {
        return Step::Cancel;
    }
    match entry.mode {
        EntryMode::OpPicker => step_op_picker(entry, key),
        EntryMode::TargetPicker { .. } => step_target_picker(entry, key),
    }
}

fn step_op_picker(entry: OperatorEntry, key: &str) -> Step {
    let Some(op) = op_key_to_kind(key) else {
        return Step::Update(entry);
    };
    let targets = targets_for_op(&entry.options, op);
    if targets.is_empty() {
        return Step::Update(entry);
    }
    Step::Update(OperatorEntry {
        mode: EntryMode::TargetPicker {
            op,
            selected: 0,
            digits: String::new(),
        },
        ..entry
    })
}

fn step_target_picker(entry: OperatorEntry, key: &str) -> Step {
    let EntryMode::TargetPicker {
        op,
        selected,
        ref digits,
    } = entry.mode
    else {
        return Step::Update(entry);
    };
    let targets = targets_for_op(&entry.options, op);
    let len = targets.len();
    let selected_target = targets.get(selected).copied();
    if len == 0 {
        return Step::Update(entry);
    }
    let prior_digits = digits.clone();
    match key {
        "Enter" => {
            selected_target.map_or(Step::Update(entry), |target| Step::Commit { op, target })
        }
        "ArrowDown" | "ArrowRight" => Step::Update(replace_target_picker(
            entry,
            op,
            (selected + 1) % len,
            String::new(),
        )),
        "ArrowUp" | "ArrowLeft" => {
            let next = if selected == 0 { len - 1 } else { selected - 1 };
            Step::Update(replace_target_picker(entry, op, next, String::new()))
        }
        "Backspace" => step_buffer_change(entry, op, selected, &prior_digits, |d| {
            d.pop();
        }),
        d if is_digit(d) => {
            let new_digits = format!("{prior_digits}{d}");
            let Some(new_selected) = jump_to_match(targets, &new_digits) else {
                return Step::Update(entry);
            };
            Step::Update(replace_target_picker(entry, op, new_selected, new_digits))
        }
        _ => Step::Update(entry),
    }
}

/// Updates the digit buffer via `mutate`, then moves the selection to the first
/// target whose decimal representation starts with the new buffer (or keeps the
/// old selection if nothing matches). When the buffer empties under Backspace,
/// walks back to `OpPicker` for non-singleton cage shapes.
fn step_buffer_change(
    entry: OperatorEntry,
    op: OpKind,
    selected: usize,
    digits: &str,
    mutate: impl FnOnce(&mut String),
) -> Step {
    let mut new_digits = digits.to_string();
    mutate(&mut new_digits);
    if new_digits.is_empty() && entry.options.iter().any(|o| o.op != OpKind::Given) {
        return Step::Update(OperatorEntry {
            mode: EntryMode::OpPicker,
            ..entry
        });
    }
    let targets = targets_for_op(&entry.options, op);
    let new_selected = jump_to_match(targets, &new_digits).unwrap_or(selected);
    Step::Update(replace_target_picker(entry, op, new_selected, new_digits))
}

fn jump_to_match(targets: &[u32], digits: &str) -> Option<usize> {
    if digits.is_empty() {
        return None;
    }
    targets
        .iter()
        .position(|t| t.to_string().starts_with(digits))
}

fn replace_target_picker(
    entry: OperatorEntry,
    op: OpKind,
    selected: usize,
    digits: String,
) -> OperatorEntry {
    OperatorEntry {
        mode: EntryMode::TargetPicker {
            op,
            selected,
            digits,
        },
        ..entry
    }
}

fn is_digit(s: &str) -> bool {
    matches!(s.as_bytes(), [b'0'..=b'9'])
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    clippy::manual_let_else,
    clippy::too_many_lines
)]
mod tests {
    use super::*;

    fn opt(op: OpKind, targets: Vec<u32>) -> CageOption {
        CageOption { op, targets }
    }

    fn binary_options() -> Vec<CageOption> {
        vec![
            opt(OpKind::Add, vec![3, 4, 5, 6, 7]),
            opt(OpKind::Sub, vec![1, 2, 3]),
            opt(OpKind::Mul, vec![2, 3, 4, 6, 8, 12]),
            opt(OpKind::Div, vec![2, 3, 4]),
        ]
    }

    fn singleton_options(n: u32) -> Vec<CageOption> {
        vec![opt(OpKind::Given, (1..=n).collect())]
    }

    fn op_picker(options: Vec<CageOption>) -> OperatorEntry {
        OperatorEntry {
            cage: ActiveCage::Draft,
            options,
            mode: EntryMode::OpPicker,
        }
    }

    fn target_picker(
        options: Vec<CageOption>,
        op: OpKind,
        selected: usize,
        digits: &str,
    ) -> OperatorEntry {
        OperatorEntry {
            cage: ActiveCage::Draft,
            options,
            mode: EntryMode::TargetPicker {
                op,
                selected,
                digits: digits.to_string(),
            },
        }
    }

    #[allow(clippy::panic, clippy::needless_pass_by_value)]
    fn assert_update(result: Step, expected: OperatorEntry) {
        match result {
            Step::Update(e) => assert_eq!(e, expected),
            Step::Commit { .. } => panic!("expected Update, got Commit"),
            Step::Cancel => panic!("expected Update, got Cancel"),
        }
    }

    #[allow(clippy::panic, clippy::needless_pass_by_value)]
    fn assert_commit(result: Step, expected_op: OpKind, expected_target: u32) {
        match result {
            Step::Commit { op, target } => {
                assert_eq!(op, expected_op);
                assert_eq!(target, expected_target);
            }
            Step::Update(_) => panic!("expected Commit, got Update"),
            Step::Cancel => panic!("expected Commit, got Cancel"),
        }
    }

    #[allow(clippy::panic, clippy::needless_pass_by_value)]
    fn assert_cancel(result: Step) {
        match result {
            Step::Cancel => {}
            Step::Update(_) => panic!("expected Cancel, got Update"),
            Step::Commit { .. } => panic!("expected Cancel, got Commit"),
        }
    }

    #[test]
    fn enter_picker_multi_cell_starts_in_op_picker() {
        let e = enter_picker(ActiveCage::Draft, binary_options(), None);
        assert_eq!(e.mode, EntryMode::OpPicker);
    }

    #[test]
    fn enter_picker_singleton_skips_to_target_picker() {
        let e = enter_picker(ActiveCage::Draft, singleton_options(4), None);
        let EntryMode::TargetPicker {
            op,
            selected,
            digits,
        } = e.mode
        else {
            panic!("expected TargetPicker")
        };
        assert_eq!(op, OpKind::Given);
        assert_eq!(selected, 0);
        assert_eq!(digits, "");
    }

    #[test]
    fn enter_picker_singleton_preselects_current_target() {
        let e = enter_picker(ActiveCage::Draft, singleton_options(5), Some(3));
        let EntryMode::TargetPicker { selected, .. } = e.mode else {
            panic!("expected TargetPicker")
        };
        assert_eq!(selected, 2); // index of value 3 in [1,2,3,4,5]
    }

    #[test]
    fn enter_picker_with_op_jumps_to_target_picker() {
        let e = enter_picker_with_op(ActiveCage::Draft, binary_options(), OpKind::Mul, None)
            .expect("Mul is valid");
        let EntryMode::TargetPicker { op, .. } = e.mode else {
            panic!("expected TargetPicker")
        };
        assert_eq!(op, OpKind::Mul);
    }

    #[test]
    fn enter_picker_with_op_returns_none_for_invalid_op() {
        let three_cell = vec![
            opt(OpKind::Add, vec![6, 7, 8]),
            opt(OpKind::Mul, vec![6, 8, 12]),
        ];
        // Subtract is not valid for a 3-cell cage.
        assert!(enter_picker_with_op(ActiveCage::Draft, three_cell, OpKind::Sub, None).is_none());
    }

    #[test]
    fn op_picker_plus_transitions_to_target_picker_for_add() {
        let result = step(op_picker(binary_options()), "+");
        assert_update(result, target_picker(binary_options(), OpKind::Add, 0, ""));
    }

    #[test]
    fn op_picker_minus_transitions_for_sub() {
        let result = step(op_picker(binary_options()), "-");
        assert_update(result, target_picker(binary_options(), OpKind::Sub, 0, ""));
    }

    #[test]
    fn op_picker_x_transitions_for_mul() {
        let r1 = step(op_picker(binary_options()), "x");
        assert_update(r1, target_picker(binary_options(), OpKind::Mul, 0, ""));
        let r2 = step(op_picker(binary_options()), "X");
        assert_update(r2, target_picker(binary_options(), OpKind::Mul, 0, ""));
        let r3 = step(op_picker(binary_options()), "*");
        assert_update(r3, target_picker(binary_options(), OpKind::Mul, 0, ""));
    }

    #[test]
    fn op_picker_slash_transitions_for_div() {
        let result = step(op_picker(binary_options()), "/");
        assert_update(result, target_picker(binary_options(), OpKind::Div, 0, ""));
    }

    #[test]
    fn op_picker_ignores_invalid_op_for_three_cell() {
        let three_cell = vec![
            opt(OpKind::Add, vec![6, 7, 8]),
            opt(OpKind::Mul, vec![6, 8, 12]),
        ];
        let initial = op_picker(three_cell.clone());
        // Subtract is not valid for 3 cells; the picker stays in OpPicker.
        let result = step(initial.clone(), "-");
        assert_update(result, initial.clone());
        let result = step(initial, "/");
        assert_update(result, op_picker(three_cell));
    }

    #[test]
    fn op_picker_ignores_digits() {
        let initial = op_picker(binary_options());
        let result = step(initial.clone(), "5");
        assert_update(result, initial);
    }

    #[test]
    fn op_picker_ignores_enter() {
        let initial = op_picker(binary_options());
        let result = step(initial.clone(), "Enter");
        assert_update(result, initial);
    }

    #[test]
    fn op_picker_escape_cancels() {
        assert_cancel(step(op_picker(binary_options()), "Escape"));
    }

    #[test]
    fn target_picker_arrow_down_advances_selection() {
        let result = step(
            target_picker(binary_options(), OpKind::Sub, 0, ""),
            "ArrowDown",
        );
        assert_update(result, target_picker(binary_options(), OpKind::Sub, 1, ""));
    }

    #[test]
    fn target_picker_arrow_down_wraps() {
        // Sub targets [1, 2, 3], wrap from 2 -> 0.
        let result = step(
            target_picker(binary_options(), OpKind::Sub, 2, ""),
            "ArrowDown",
        );
        assert_update(result, target_picker(binary_options(), OpKind::Sub, 0, ""));
    }

    #[test]
    fn target_picker_arrow_up_wraps() {
        let result = step(
            target_picker(binary_options(), OpKind::Sub, 0, ""),
            "ArrowUp",
        );
        assert_update(result, target_picker(binary_options(), OpKind::Sub, 2, ""));
    }

    #[test]
    fn target_picker_digit_jumps_to_first_match() {
        // Mul targets [2, 3, 4, 6, 8, 12]. Typing "1" jumps to 12 at index 5.
        let result = step(target_picker(binary_options(), OpKind::Mul, 0, ""), "1");
        assert_update(result, target_picker(binary_options(), OpKind::Mul, 5, "1"));
    }

    #[test]
    fn target_picker_invalid_digit_is_rejected() {
        // Sub targets [1, 2, 3]. Typing "9" is not a prefix of any valid target,
        // so the keystroke is ignored entirely (buffer unchanged, selection unchanged).
        let initial = target_picker(binary_options(), OpKind::Sub, 1, "");
        assert_update(step(initial.clone(), "9"), initial);
    }

    #[test]
    fn target_picker_invalid_extension_digit_is_rejected() {
        // Mul targets [2, 3, 4, 6, 8, 12]. Buffer "1" matches "12"; typing "5" would
        // make the buffer "15" which is not a prefix of any target, so it's ignored.
        let initial = target_picker(binary_options(), OpKind::Mul, 5, "1");
        assert_update(step(initial.clone(), "5"), initial);
    }

    #[test]
    fn target_picker_two_digits_select_two_digit_target() {
        // Mul targets [2, 3, 4, 6, 8, 12]. Type "1" then "2" → selects 12.
        let after_one = match step(target_picker(binary_options(), OpKind::Mul, 0, ""), "1") {
            Step::Update(e) => e,
            _ => panic!("expected Update"),
        };
        let result = step(after_one, "2");
        assert_update(
            result,
            target_picker(binary_options(), OpKind::Mul, 5, "12"),
        );
    }

    #[test]
    fn target_picker_backspace_pops_digit() {
        let result = step(
            target_picker(binary_options(), OpKind::Mul, 5, "12"),
            "Backspace",
        );
        // Buffer becomes "1", which still jumps to 12 (index 5).
        assert_update(result, target_picker(binary_options(), OpKind::Mul, 5, "1"));
    }

    #[test]
    fn target_picker_backspace_with_empty_buffer_returns_to_op_picker() {
        let result = step(
            target_picker(binary_options(), OpKind::Mul, 3, ""),
            "Backspace",
        );
        assert_update(result, op_picker(binary_options()));
    }

    #[test]
    fn target_picker_backspace_singleton_stays_in_target_picker() {
        let result = step(
            target_picker(singleton_options(4), OpKind::Given, 2, ""),
            "Backspace",
        );
        // Singletons skip OpPicker, so backspace at empty buffer is a no-op.
        assert_update(
            result,
            target_picker(singleton_options(4), OpKind::Given, 2, ""),
        );
    }

    #[test]
    fn target_picker_enter_commits_selected_target() {
        let result = step(target_picker(binary_options(), OpKind::Add, 2, ""), "Enter");
        // Add targets [3, 4, 5, 6, 7]; selected=2 → target=5.
        assert_commit(result, OpKind::Add, 5);
    }

    #[test]
    fn target_picker_enter_after_digit_typing_commits_matched_target() {
        // Mul targets [2, 3, 4, 6, 8, 12]; type "8" → selects 8 at index 4 → Enter commits 8.
        let after_eight = match step(target_picker(binary_options(), OpKind::Mul, 0, ""), "8") {
            Step::Update(e) => e,
            _ => panic!("expected Update"),
        };
        assert_commit(step(after_eight, "Enter"), OpKind::Mul, 8);
    }

    #[test]
    fn target_picker_escape_cancels() {
        assert_cancel(step(
            target_picker(binary_options(), OpKind::Add, 2, "5"),
            "Escape",
        ));
    }

    #[test]
    fn target_picker_unknown_key_is_noop() {
        let initial = target_picker(binary_options(), OpKind::Add, 2, "");
        assert_update(step(initial.clone(), "F1"), initial);
    }

    #[test]
    fn singleton_invalid_digit_is_rejected() {
        // Given targets [1, 2, 3, 4]; "9" is not a prefix of any target, so it's ignored.
        let initial = target_picker(singleton_options(4), OpKind::Given, 0, "");
        assert_update(step(initial.clone(), "9"), initial);
    }

    #[test]
    fn singleton_typing_digit_selects_matching_value() {
        // Given targets [1, 2, 3, 4]. Typing "3" jumps to index 2.
        let result = step(
            target_picker(singleton_options(4), OpKind::Given, 0, ""),
            "3",
        );
        assert_update(
            result,
            target_picker(singleton_options(4), OpKind::Given, 2, "3"),
        );
        let after = target_picker(singleton_options(4), OpKind::Given, 2, "3");
        assert_commit(step(after, "Enter"), OpKind::Given, 3);
    }

    #[test]
    fn is_entry_trigger_key_accepts_operator_symbols() {
        for key in ["+", "-", "*", "x", "X", "/"] {
            assert!(is_entry_trigger_key(key), "expected {key} to be a trigger");
        }
    }

    #[test]
    fn is_entry_trigger_key_accepts_all_digits() {
        for key in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            assert!(is_entry_trigger_key(key), "expected {key} to be a trigger");
        }
    }

    #[test]
    fn is_entry_trigger_key_rejects_non_trigger_keys() {
        for key in [
            "Enter",
            "Escape",
            "ArrowUp",
            "a",
            "z",
            "Backspace",
            "Tab",
            "",
        ] {
            assert!(
                !is_entry_trigger_key(key),
                "expected {key} NOT to be a trigger"
            );
        }
    }
}
