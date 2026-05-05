use crate::cage_edit::{delete_at, escape_at, shift_arrow, splinter_at, CageEdit};
use crate::cage_index::{cage_anchor, cage_at, cells_anchor};
use crate::grid::Grid;
use crate::navigation::{move_cursor, next_state, NavKey};
use crate::operator_entry::{step as operator_step, ActiveCage, OperatorEntry, Step};
use leptos::ev::{Event, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

const PUZZLE_UPDATED_EVENT: &str = "puzzle-updated";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}

#[derive(Serialize)]
struct NewPuzzleArgs {
    n: usize,
}

#[derive(Serialize)]
struct NoArgs {}

#[derive(Serialize)]
struct AnchorArgs {
    anchor: (usize, usize),
}

#[derive(Serialize)]
struct CellArgs {
    cell: (usize, usize),
}

#[derive(Serialize)]
struct ExtendCageArgs {
    anchor: (usize, usize),
    cell: (usize, usize),
}

#[derive(Serialize)]
struct InsertCageArgs {
    cells: Vec<(usize, usize)>,
    op: OpKind,
    target: u32,
}

#[derive(Serialize)]
struct SetCageOperationArgs {
    anchor: (usize, usize),
    op: OpKind,
    target: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MergeCagesArgs {
    a_anchor: (usize, usize),
    b_anchor: (usize, usize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PuzzleView {
    pub n: usize,
    pub cells: Vec<Vec<Vec<u8>>>,
    pub cages: Vec<CageView>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CageView {
    pub cells: Vec<(usize, usize)>,
    pub op: OpKind,
    pub target: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
    Given,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DraftCage {
    pub cells: Vec<(usize, usize)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditResult {
    pub view: PuzzleView,
    pub draft: Option<DraftCage>,
}

#[allow(clippy::future_not_send)] // WASM single-threaded runtime; Send is meaningless here
async fn call<A: Serialize>(cmd: &str, args: A) -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&args).ok()?;
    let value = invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(value).ok()
}

#[allow(clippy::future_not_send)]
async fn call_edit<A: Serialize>(cmd: &str, args: A) -> Option<EditResult> {
    let args = serde_wasm_bindgen::to_value(&args).ok()?;
    let value = invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(value).ok()
}

fn listen_for_puzzle_updates(set_puzzle: WriteSignal<Option<PuzzleView>>) {
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
            if let Ok(view) = serde_wasm_bindgen::from_value::<PuzzleView>(payload) {
                set_puzzle.set(Some(view));
            }
        }
    });
    let _ = listen(PUZZLE_UPDATED_EVENT, &cb);
    cb.forget();
}

fn is_text_input_focused() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(doc) = window.document() else {
        return false;
    };
    let Some(element) = doc.active_element() else {
        return false;
    };
    is_text_input_tag(&element.tag_name())
}

const fn is_text_input_tag(tag: &str) -> bool {
    tag.as_bytes().eq_ignore_ascii_case(b"INPUT")
        || tag.as_bytes().eq_ignore_ascii_case(b"TEXTAREA")
        || tag.as_bytes().eq_ignore_ascii_case(b"SELECT")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Undo,
    Redo,
    Navigate(NavKey),
    ShiftArrow(NavKey),
    Escape,
    Delete,
    Splinter,
    Ignore,
}

fn dispatch_key(key: &str, shift: bool, modifier: bool, in_text_input: bool) -> KeyAction {
    if in_text_input {
        return KeyAction::Ignore;
    }
    if modifier && key.eq_ignore_ascii_case("z") {
        return if shift {
            KeyAction::Redo
        } else {
            KeyAction::Undo
        };
    }
    if modifier {
        return KeyAction::Ignore;
    }
    if let Some(nav_key) = NavKey::from_key(key, shift) {
        return if shift
            && matches!(
                nav_key,
                NavKey::ArrowUp | NavKey::ArrowDown | NavKey::ArrowLeft | NavKey::ArrowRight
            ) {
            KeyAction::ShiftArrow(nav_key)
        } else {
            KeyAction::Navigate(nav_key)
        };
    }
    match key {
        "Escape" => KeyAction::Escape,
        "x" | "X" => KeyAction::Delete,
        " " | "Spacebar" | "c" | "C" => KeyAction::Splinter,
        _ => KeyAction::Ignore,
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (puzzle, set_puzzle) = signal::<Option<PuzzleView>>(None);
    let cursor = RwSignal::new((0_usize, 0_usize));
    let active_cage = RwSignal::new(None::<usize>);
    let draft = RwSignal::new(None::<DraftCage>);
    let entry = RwSignal::new(None::<OperatorEntry>);

    refresh_from(set_puzzle, Box::pin(call("get_state", NoArgs {})));
    listen_for_puzzle_updates(set_puzzle);

    let on_size_change = move |ev: Event| {
        let Ok(n) = event_target_value(&ev).parse::<usize>() else {
            return;
        };
        cursor.set((0, 0));
        active_cage.set(None);
        set_draft_if_changed(draft, None);
        refresh_from(
            set_puzzle,
            Box::pin(call("new_puzzle", NewPuzzleArgs { n })),
        );
    };

    let current_n = move || {
        puzzle
            .get()
            .map_or_else(|| "4".to_string(), |v| v.n.to_string())
    };

    let on_cell_click = Callback::new(move |(r, c): (usize, usize)| {
        if cursor.get_untracked() != (r, c) {
            cursor.set((r, c));
        }
        let next_active = puzzle.with_untracked(|opt| opt.as_ref().and_then(|v| cage_at(v, r, c)));
        if active_cage.get_untracked() != next_active {
            active_cage.set(next_active);
        }
    });

    install_keydown_handler(puzzle, set_puzzle, cursor, active_cage, draft, entry);

    view! {
        <main class="app-main">
            {move || {
                let view = puzzle.get()?;
                let draft_value = draft.get();
                Some(view! {
                    <Grid
                        view=view
                        draft=draft_value
                        size=560
                        cursor=cursor.into()
                        active_cage=active_cage.into()
                        on_cell_click=on_cell_click
                        entry=entry.into()
                    />
                })
            }}
            <div class="size-control">
                <label>
                    "Size: "
                    <select on:change=on_size_change prop:value=current_n>
                        <option value="2">"2"</option>
                        <option value="3">"3"</option>
                        <option value="4">"4"</option>
                        <option value="5">"5"</option>
                        <option value="6">"6"</option>
                        <option value="7">"7"</option>
                        <option value="8">"8"</option>
                        <option value="9">"9"</option>
                    </select>
                </label>
            </div>
        </main>
    }
}

fn install_keydown_handler(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    draft: RwSignal<Option<DraftCage>>,
    entry: RwSignal<Option<OperatorEntry>>,
) {
    window_event_listener(leptos::ev::keydown, move |ev: KeyboardEvent| {
        if let Some(current_entry) = entry.get_untracked() {
            ev.prevent_default();
            handle_entry_key(puzzle, set_puzzle, draft, entry, current_entry, &ev.key());
            return;
        }

        let modifier = ev.meta_key() || ev.ctrl_key();
        let key = ev.key();

        if key == "Enter" && !ev.shift_key() && !modifier && !is_text_input_focused() {
            handle_enter_key(puzzle, cursor, draft, entry, &ev);
            return;
        }

        let action = dispatch_key(&key, ev.shift_key(), modifier, is_text_input_focused());
        match action {
            KeyAction::Ignore => {}
            KeyAction::Undo => {
                ev.prevent_default();
                refresh_from_then(set_puzzle, Box::pin(call("undo", NoArgs {})), move || {
                    set_draft_if_changed(draft, None);
                });
            }
            KeyAction::Redo => {
                ev.prevent_default();
                refresh_from_then(set_puzzle, Box::pin(call("redo", NoArgs {})), move || {
                    set_draft_if_changed(draft, None);
                });
            }
            KeyAction::Navigate(nav_key) => {
                ev.prevent_default();
                handle_navigate(puzzle, cursor, active_cage, nav_key);
            }
            KeyAction::ShiftArrow(nav_key) => {
                ev.prevent_default();
                handle_shift_arrow(puzzle, set_puzzle, cursor, active_cage, draft, nav_key);
            }
            KeyAction::Escape => {
                ev.prevent_default();
                handle_cell_action(puzzle, set_puzzle, cursor, active_cage, draft, escape_at);
            }
            KeyAction::Delete => {
                ev.prevent_default();
                handle_cell_action(puzzle, set_puzzle, cursor, active_cage, draft, delete_at);
            }
            KeyAction::Splinter => {
                ev.prevent_default();
                handle_cell_action(puzzle, set_puzzle, cursor, active_cage, draft, splinter_at);
            }
        }
    });
}

fn handle_entry_key(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    draft: RwSignal<Option<DraftCage>>,
    entry: RwSignal<Option<OperatorEntry>>,
    current_entry: OperatorEntry,
    key: &str,
) {
    let single_cell = puzzle.with_untracked(|opt| {
        opt.as_ref().is_some_and(|v| match &current_entry.cage {
            ActiveCage::Committed(idx) => v.cages.get(*idx).is_some_and(|c| c.cells.len() == 1),
            ActiveCage::Draft => {
                draft.with_untracked(|d| d.as_ref().is_some_and(|d| d.cells.len() == 1))
            }
        })
    });
    let cage = current_entry.cage.clone();
    match operator_step(current_entry, key, single_cell) {
        Step::Update(new_entry) => entry.set(Some(new_entry)),
        Step::Cancel => entry.set(None),
        Step::Commit { op, target } => match cage {
            ActiveCage::Draft => {
                let cells = draft
                    .with_untracked(|d| d.as_ref().map(|d| d.cells.clone()).unwrap_or_default());
                refresh_from_then(
                    set_puzzle,
                    Box::pin(call("insert_cage", InsertCageArgs { cells, op, target })),
                    move || {
                        set_draft_if_changed(draft, None);
                        entry.set(None);
                    },
                );
            }
            ActiveCage::Committed(idx) => {
                let anchor = puzzle.with_untracked(|opt| {
                    opt.as_ref().and_then(|v| v.cages.get(idx)).map(cage_anchor)
                });
                let Some(anchor) = anchor else { return };
                refresh_from_then(
                    set_puzzle,
                    Box::pin(call(
                        "set_cage_operation",
                        SetCageOperationArgs { anchor, op, target },
                    )),
                    move || entry.set(None),
                );
            }
        },
    }
}

fn handle_enter_key(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    draft: RwSignal<Option<DraftCage>>,
    entry: RwSignal<Option<OperatorEntry>>,
    ev: &KeyboardEvent,
) {
    let active = puzzle.with_untracked(|opt| {
        opt.as_ref().and_then(|v| {
            let (r, c) = cursor.get_untracked();
            cage_at(v, r, c).map(|idx| {
                let anchor = cage_anchor(&v.cages[idx]);
                let cage_op = v.cages[idx].op;
                let cage_target = v.cages[idx].target;
                (ActiveCage::Committed(idx), anchor, cage_op, cage_target)
            })
        })
    });
    if let Some((active_cage_val, anchor, cage_op, cage_target)) = active {
        ev.prevent_default();
        cursor.set(anchor);
        entry.set(Some(OperatorEntry {
            cage: active_cage_val,
            op: Some(cage_op),
            digits: if cage_target > 0 {
                cage_target.to_string()
            } else {
                String::new()
            },
        }));
        return;
    }
    let draft_anchor = draft.with_untracked(|d| d.as_ref().map(|d| cells_anchor(&d.cells)));
    if let Some(anchor) = draft_anchor {
        ev.prevent_default();
        cursor.set(anchor);
        entry.set(Some(OperatorEntry {
            cage: ActiveCage::Draft,
            op: None,
            digits: String::new(),
        }));
    }
}

fn handle_navigate(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    nav_key: NavKey,
) {
    let next = puzzle.with_untracked(|opt| {
        opt.as_ref().map(|v| {
            next_state(
                cursor.get_untracked(),
                active_cage.get_untracked(),
                v.n,
                &v.cages,
                nav_key,
            )
        })
    });
    let Some((next_cursor, next_active)) = next else {
        return;
    };
    if cursor.get_untracked() != next_cursor {
        cursor.set(next_cursor);
    }
    if active_cage.get_untracked() != next_active {
        active_cage.set(next_active);
    }
}

fn handle_shift_arrow(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    draft: RwSignal<Option<DraftCage>>,
    nav_key: NavKey,
) {
    let at = cursor.get_untracked();
    let action = puzzle.with_untracked(|opt| {
        let v = opt.as_ref()?;
        let neighbor = move_cursor(at, v.n, nav_key);
        if neighbor == at {
            return None;
        }
        let action = draft.with_untracked(|d| shift_arrow(at, neighbor, v, d.as_ref()));
        Some((neighbor, action))
    });
    let Some((neighbor, action)) = action else {
        return;
    };
    apply_edit(set_puzzle, draft, active_cage, action);
    if cursor.get_untracked() != neighbor {
        cursor.set(neighbor);
    }
    sync_active_cage(puzzle, cursor, active_cage);
}

fn handle_cell_action(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    draft: RwSignal<Option<DraftCage>>,
    branch: fn((usize, usize), &PuzzleView, Option<&DraftCage>) -> CageEdit,
) {
    let at = cursor.get_untracked();
    let action = puzzle.with_untracked(|opt| {
        opt.as_ref()
            .map(|v| draft.with_untracked(|d| branch(at, v, d.as_ref())))
    });
    let Some(action) = action else { return };
    apply_edit(set_puzzle, draft, active_cage, action);
    sync_active_cage(puzzle, cursor, active_cage);
}

fn apply_edit(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    draft: RwSignal<Option<DraftCage>>,
    active_cage: RwSignal<Option<usize>>,
    action: CageEdit,
) {
    match action {
        CageEdit::Noop => {}
        CageEdit::SetDraft(d) => set_draft_if_changed(draft, d),
        CageEdit::ExtendCage { anchor, cell } => {
            dispatch_edit(
                set_puzzle,
                draft,
                Box::pin(call_edit("extend_cage", ExtendCageArgs { anchor, cell })),
                None,
            );
        }
        CageEdit::MergeCages { a_anchor, b_anchor } => {
            dispatch_edit(
                set_puzzle,
                draft,
                Box::pin(call_edit(
                    "merge_cages",
                    MergeCagesArgs { a_anchor, b_anchor },
                )),
                None,
            );
        }
        CageEdit::ShrinkCage(cell) => {
            dispatch_edit(
                set_puzzle,
                draft,
                Box::pin(call_edit("shrink_cage", CellArgs { cell })),
                None,
            );
        }
        CageEdit::SplinterFromCommitted(cell) => {
            dispatch_edit(
                set_puzzle,
                draft,
                Box::pin(call_edit("shrink_cage", CellArgs { cell })),
                Some(DraftCage { cells: vec![cell] }),
            );
        }
        CageEdit::RemoveCage(anchor) => {
            refresh_from_then(
                set_puzzle,
                Box::pin(call("remove_cage", AnchorArgs { anchor })),
                move || {
                    if active_cage.get_untracked().is_some() {
                        active_cage.set(None);
                    }
                    set_draft_if_changed(draft, None);
                },
            );
        }
    }
}

fn set_draft_if_changed(draft: RwSignal<Option<DraftCage>>, next: Option<DraftCage>) {
    if draft.with_untracked(|d| d != &next) {
        draft.set(next);
    }
}

fn dispatch_edit(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    draft: RwSignal<Option<DraftCage>>,
    fut: Pin<Box<dyn Future<Output = Option<EditResult>>>>,
    override_draft: Option<DraftCage>,
) {
    spawn_local(async move {
        if let Some(result) = fut.await {
            set_puzzle.set(Some(result.view));
            let next_draft = override_draft.or(result.draft);
            set_draft_if_changed(draft, next_draft);
        }
    });
}

fn sync_active_cage(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
) {
    let (r, c) = cursor.get_untracked();
    let next_active = puzzle.with_untracked(|opt| opt.as_ref().and_then(|v| cage_at(v, r, c)));
    if active_cage.get_untracked() != next_active {
        active_cage.set(next_active);
    }
}

fn refresh_from(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    fut: Pin<Box<dyn Future<Output = Option<PuzzleView>>>>,
) {
    refresh_from_then(set_puzzle, fut, || {});
}

fn refresh_from_then(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    fut: Pin<Box<dyn Future<Output = Option<PuzzleView>>>>,
    on_success: impl FnOnce() + 'static,
) {
    spawn_local(async move {
        if let Some(view) = fut.await {
            set_puzzle.set(Some(view));
            on_success();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_text_input_tag_matches_form_controls() {
        assert!(is_text_input_tag("INPUT"));
        assert!(is_text_input_tag("input"));
        assert!(is_text_input_tag("Textarea"));
        assert!(is_text_input_tag("SELECT"));
    }

    #[test]
    fn is_text_input_tag_rejects_other_elements() {
        assert!(!is_text_input_tag("DIV"));
        assert!(!is_text_input_tag("BUTTON"));
        assert!(!is_text_input_tag("svg"));
        assert!(!is_text_input_tag(""));
    }

    #[test]
    fn dispatch_key_returns_undo_for_modifier_z() {
        assert_eq!(dispatch_key("z", false, true, false), KeyAction::Undo);
        assert_eq!(dispatch_key("Z", false, true, false), KeyAction::Undo);
    }

    #[test]
    fn dispatch_key_returns_redo_for_modifier_shift_z() {
        assert_eq!(dispatch_key("z", true, true, false), KeyAction::Redo);
        assert_eq!(dispatch_key("Z", true, true, false), KeyAction::Redo);
    }

    #[test]
    fn dispatch_key_ignores_z_without_modifier() {
        assert_eq!(dispatch_key("z", false, false, false), KeyAction::Ignore);
        assert_eq!(dispatch_key("z", true, false, false), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_returns_navigate_for_arrow_keys() {
        assert_eq!(
            dispatch_key("ArrowUp", false, false, false),
            KeyAction::Navigate(NavKey::ArrowUp)
        );
        assert_eq!(
            dispatch_key("ArrowDown", false, false, false),
            KeyAction::Navigate(NavKey::ArrowDown)
        );
        assert_eq!(
            dispatch_key("ArrowLeft", false, false, false),
            KeyAction::Navigate(NavKey::ArrowLeft)
        );
        assert_eq!(
            dispatch_key("ArrowRight", false, false, false),
            KeyAction::Navigate(NavKey::ArrowRight)
        );
    }

    #[test]
    fn dispatch_key_returns_navigate_for_tab_and_shift_tab() {
        assert_eq!(
            dispatch_key("Tab", false, false, false),
            KeyAction::Navigate(NavKey::Tab)
        );
        assert_eq!(
            dispatch_key("Tab", true, false, false),
            KeyAction::Navigate(NavKey::ShiftTab)
        );
    }

    #[test]
    fn dispatch_key_ignores_when_text_input_focused() {
        assert_eq!(dispatch_key("z", false, true, true), KeyAction::Ignore);
        assert_eq!(
            dispatch_key("ArrowUp", false, false, true),
            KeyAction::Ignore
        );
        assert_eq!(dispatch_key("Tab", false, false, true), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_ignores_unrelated_keys() {
        assert_eq!(dispatch_key("a", false, false, false), KeyAction::Ignore);
        assert_eq!(
            dispatch_key("Enter", false, false, false),
            KeyAction::Ignore
        );
        assert_eq!(dispatch_key("", false, false, false), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_returns_shift_arrow_for_shifted_arrows() {
        assert_eq!(
            dispatch_key("ArrowUp", true, false, false),
            KeyAction::ShiftArrow(NavKey::ArrowUp)
        );
        assert_eq!(
            dispatch_key("ArrowDown", true, false, false),
            KeyAction::ShiftArrow(NavKey::ArrowDown)
        );
        assert_eq!(
            dispatch_key("ArrowLeft", true, false, false),
            KeyAction::ShiftArrow(NavKey::ArrowLeft)
        );
        assert_eq!(
            dispatch_key("ArrowRight", true, false, false),
            KeyAction::ShiftArrow(NavKey::ArrowRight)
        );
    }

    #[test]
    fn dispatch_key_returns_escape_for_escape_key() {
        assert_eq!(
            dispatch_key("Escape", false, false, false),
            KeyAction::Escape
        );
    }

    #[test]
    fn dispatch_key_returns_delete_for_x() {
        assert_eq!(dispatch_key("x", false, false, false), KeyAction::Delete);
        assert_eq!(dispatch_key("X", false, false, false), KeyAction::Delete);
    }

    #[test]
    fn dispatch_key_returns_splinter_for_space_or_c() {
        assert_eq!(dispatch_key(" ", false, false, false), KeyAction::Splinter);
        assert_eq!(dispatch_key("c", false, false, false), KeyAction::Splinter);
        assert_eq!(dispatch_key("C", false, false, false), KeyAction::Splinter);
    }

    #[test]
    fn dispatch_key_ignores_modifier_plus_other_keys() {
        // Cmd+x, Cmd+c, Cmd+Escape, Cmd+Space should not trigger cage edits.
        assert_eq!(dispatch_key("x", false, true, false), KeyAction::Ignore);
        assert_eq!(dispatch_key("c", false, true, false), KeyAction::Ignore);
        assert_eq!(dispatch_key(" ", false, true, false), KeyAction::Ignore);
        assert_eq!(
            dispatch_key("Escape", false, true, false),
            KeyAction::Ignore
        );
    }
}
