use crate::app::OpKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveCage {
    Committed(usize),
    Draft,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorEntry {
    pub cage: ActiveCage,
    pub op: Option<OpKind>,
    pub digits: String,
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

/// Pure state machine step for operator entry mode.
/// `single_cell` is true when the active cage has exactly 1 cell (allows Given).
pub fn step(entry: OperatorEntry, key: &str, single_cell: bool) -> Step {
    match key {
        "+" => Step::Update(OperatorEntry {
            op: Some(OpKind::Add),
            ..entry
        }),
        "-" => Step::Update(OperatorEntry {
            op: Some(OpKind::Sub),
            ..entry
        }),
        "*" | "x" | "X" => Step::Update(OperatorEntry {
            op: Some(OpKind::Mul),
            ..entry
        }),
        "/" => Step::Update(OperatorEntry {
            op: Some(OpKind::Div),
            ..entry
        }),
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
            if entry.digits.len() < 4 {
                let mut digits = entry.digits.clone();
                digits.push_str(key);
                Step::Update(OperatorEntry { digits, ..entry })
            } else {
                Step::Update(entry)
            }
        }
        "Backspace" => {
            let mut digits = entry.digits.clone();
            if digits.pop().is_some() {
                Step::Update(OperatorEntry { digits, ..entry })
            } else {
                Step::Update(OperatorEntry {
                    op: None,
                    digits,
                    ..entry
                })
            }
        }
        "Enter" => {
            if entry.digits.is_empty() {
                return Step::Update(entry);
            }
            let target: u32 = entry.digits.parse().unwrap_or(0);
            let op = match entry.op {
                Some(op) => op,
                None if single_cell => OpKind::Given,
                None => return Step::Update(entry),
            };
            Step::Commit { op, target }
        }
        "Escape" => Step::Cancel,
        _ => Step::Update(entry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(cage: ActiveCage, op: Option<OpKind>, digits: &str) -> OperatorEntry {
        OperatorEntry {
            cage,
            op,
            digits: digits.to_string(),
        }
    }

    fn committed(idx: usize) -> ActiveCage {
        ActiveCage::Committed(idx)
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
    fn plus_sets_op_add() {
        let e = entry(committed(0), None, "");
        let result = step(e, "+", false);
        assert_update(result, entry(committed(0), Some(OpKind::Add), ""));
    }

    #[test]
    fn minus_sets_op_sub() {
        let e = entry(committed(0), None, "");
        let result = step(e, "-", false);
        assert_update(result, entry(committed(0), Some(OpKind::Sub), ""));
    }

    #[test]
    fn star_sets_op_mul() {
        let e = entry(committed(0), None, "");
        let result = step(e, "*", false);
        assert_update(result, entry(committed(0), Some(OpKind::Mul), ""));
    }

    #[test]
    fn x_lowercase_sets_op_mul() {
        let e = entry(committed(0), None, "");
        let result = step(e, "x", false);
        assert_update(result, entry(committed(0), Some(OpKind::Mul), ""));
    }

    #[test]
    fn x_uppercase_sets_op_mul() {
        let e = entry(committed(0), None, "");
        let result = step(e, "X", false);
        assert_update(result, entry(committed(0), Some(OpKind::Mul), ""));
    }

    #[test]
    fn slash_sets_op_div() {
        let e = entry(committed(0), None, "");
        let result = step(e, "/", false);
        assert_update(result, entry(committed(0), Some(OpKind::Div), ""));
    }

    #[test]
    fn digit_appends_to_digits() {
        let e = entry(committed(0), Some(OpKind::Add), "");
        let result = step(e, "5", false);
        assert_update(result, entry(committed(0), Some(OpKind::Add), "5"));
    }

    #[test]
    fn digit_accumulates_multiple() {
        let e = entry(committed(0), Some(OpKind::Add), "12");
        let result = step(e, "3", false);
        assert_update(result, entry(committed(0), Some(OpKind::Add), "123"));
    }

    #[test]
    fn digit_stops_at_four() {
        let e = entry(committed(0), Some(OpKind::Add), "1234");
        let result = step(e, "5", false);
        assert_update(result, entry(committed(0), Some(OpKind::Add), "1234"));
    }

    #[test]
    #[allow(clippy::panic)]
    fn all_digit_chars_work() {
        for d in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            let e = entry(committed(0), None, "");
            let result = step(e, d, false);
            match result {
                Step::Update(u) => assert_eq!(u.digits, d),
                _ => panic!("expected Update for digit {d}"),
            }
        }
    }

    #[test]
    fn backspace_pops_last_digit() {
        let e = entry(committed(0), Some(OpKind::Add), "12");
        let result = step(e, "Backspace", false);
        assert_update(result, entry(committed(0), Some(OpKind::Add), "1"));
    }

    #[test]
    fn backspace_on_empty_digits_clears_op() {
        let e = entry(committed(0), Some(OpKind::Add), "");
        let result = step(e, "Backspace", false);
        assert_update(result, entry(committed(0), None, ""));
    }

    #[test]
    #[allow(clippy::panic)]
    fn backspace_through_digits_then_op() {
        let e = entry(committed(0), Some(OpKind::Mul), "3");
        let step1 = step(e, "Backspace", false);
        let Step::Update(e1) = step1 else {
            panic!("expected Update")
        };
        assert_eq!(e1, entry(committed(0), Some(OpKind::Mul), ""));

        let step2 = step(e1, "Backspace", false);
        assert_update(step2, entry(committed(0), None, ""));
    }

    #[test]
    fn enter_with_empty_digits_stays_in_mode() {
        let e = entry(committed(0), Some(OpKind::Add), "");
        let result = step(e.clone(), "Enter", false);
        assert_update(result, e);
    }

    #[test]
    fn enter_with_digits_and_op_commits() {
        let e = entry(committed(0), Some(OpKind::Add), "15");
        let result = step(e, "Enter", false);
        assert_commit(result, OpKind::Add, 15);
    }

    #[test]
    fn enter_with_digits_no_op_stays_for_multi_cell() {
        let e = entry(committed(0), None, "5");
        let result = step(e.clone(), "Enter", false);
        assert_update(result, e);
    }

    #[test]
    fn enter_with_digits_no_op_single_cell_commits_given() {
        let e = entry(committed(0), None, "3");
        let result = step(e, "Enter", true);
        assert_commit(result, OpKind::Given, 3);
    }

    #[test]
    fn escape_cancels() {
        let e = entry(committed(0), Some(OpKind::Add), "5");
        assert_cancel(step(e, "Escape", false));
    }

    #[test]
    fn unknown_key_is_noop() {
        let e = entry(committed(0), Some(OpKind::Add), "5");
        let result = step(e.clone(), "F1", false);
        assert_update(result, e);
    }

    #[test]
    fn draft_cage_variant_works() {
        let e = entry(ActiveCage::Draft, None, "");
        let result = step(e, "+", false);
        assert_update(result, entry(ActiveCage::Draft, Some(OpKind::Add), ""));
    }

    #[test]
    fn enter_commits_for_draft_cage_with_op_and_digits() {
        let e = entry(ActiveCage::Draft, Some(OpKind::Mul), "42");
        let result = step(e, "Enter", false);
        assert_commit(result, OpKind::Mul, 42);
    }

    #[test]
    #[allow(clippy::panic)]
    fn enter_with_all_op_kinds() {
        for (key, expected_op) in [
            ("+", OpKind::Add),
            ("-", OpKind::Sub),
            ("*", OpKind::Mul),
            ("/", OpKind::Div),
        ] {
            let e = entry(committed(0), None, "");
            let Step::Update(e) = step(e, key, false) else {
                panic!("expected Update")
            };
            let Step::Update(e) = step(e, "7", false) else {
                panic!("expected Update")
            };
            assert_commit(step(e, "Enter", false), expected_op, 7);
        }
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
