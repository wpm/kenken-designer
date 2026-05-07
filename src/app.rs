use crate::cage_band::{self, CageBand};
use crate::cage_edit::{
    delete_at, escape_at, legal_move_targets, shift_arrow, splinter_at, CageEdit,
};
use crate::cage_index::{cage_anchor, cage_at, cells_anchor};
use crate::context_menu::{menu_items_for, ContextMenuItems, MenuContext};
use crate::grid::Grid;
use crate::navigation::{move_cursor, next_state, NavKey};
use crate::operator_entry::{
    is_entry_trigger_key, step as operator_step, ActiveCage, OperatorEntry, Step,
};
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
const FILE_ACTION_EVENT: &str = "file-action";
const CLEAR_ALL_CAGES_EVENT: &str = "clear-all-cages";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;

    /// Open a file-save dialog from the Tauri dialog plugin.
    /// Returns a JS Promise resolving to the path string or null if cancelled.
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    async fn save(options: JsValue) -> JsValue;

    /// Open a file-open dialog from the Tauri dialog plugin.
    /// Returns a JS Promise resolving to the path string or null if cancelled.
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    async fn open(options: JsValue) -> JsValue;
}

#[derive(Serialize)]
struct PathArgs {
    path: String,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveCellArgs {
    pub cell: (usize, usize),
    pub target_anchor: (usize, usize),
}

/// State for the keyboard-driven move-cell mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveState {
    /// The cell being moved.
    pub cell: (usize, usize),
    /// Anchor cells of legal target cages, sorted row-major.
    pub targets: Vec<(usize, usize)>,
    /// Index into `targets`; `None` until first Tab.
    pub selected: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PuzzleView {
    pub n: usize,
    pub cells: Vec<Vec<Vec<u8>>>,
    pub cages: Vec<CageView>,
    pub diff: crate::diff::PuzzleDiff,
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
    pub drafts: Vec<DraftCage>,
}

#[allow(clippy::future_not_send)] // WASM single-threaded runtime; Send is meaningless here
async fn call<A: Serialize>(cmd: &str, args: A) -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&args).ok()?;
    let value = invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(value).ok()
}

#[allow(clippy::future_not_send)]
pub async fn call_edit<A: Serialize>(cmd: &str, args: A) -> Option<EditResult> {
    let args = serde_wasm_bindgen::to_value(&args).ok()?;
    let value = invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(value).ok()
}

fn listen_for_puzzle_updates(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
) {
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
            if let Ok(view) = serde_wasm_bindgen::from_value::<PuzzleView>(payload) {
                set_view(set_puzzle, set_flash_diff, view);
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

/// Keys the cage band's local keydown handler owns when a thumb has focus —
/// the global dispatcher must defer to it so the same press doesn't also
/// move the grid cursor or open operator entry.
const fn is_band_owned_key(key: &str) -> bool {
    matches!(
        key.as_bytes(),
        b"ArrowUp" | b"ArrowDown" | b"Enter" | b"Escape"
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Undo,
    Redo,
    Save,
    SaveAs,
    Open,
    Navigate(NavKey),
    ShiftArrow(NavKey),
    Escape,
    Delete,
    Splinter,
    MoveCell,
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
    if modifier && key.eq_ignore_ascii_case("s") {
        return if shift {
            KeyAction::SaveAs
        } else {
            KeyAction::Save
        };
    }
    if modifier && key.eq_ignore_ascii_case("o") {
        return KeyAction::Open;
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
        "Delete" => KeyAction::Delete,
        " " | "Spacebar" | "c" | "C" => KeyAction::Splinter,
        "m" | "M" => KeyAction::MoveCell,
        _ => KeyAction::Ignore,
    }
}

#[derive(Clone, Debug)]
pub struct ContextMenuState {
    pub x: f64,
    pub y: f64,
    pub cell: (usize, usize),
    pub items: ContextMenuItems,
}

pub const GRID_SIZE: u32 = 560;

#[component]
#[allow(clippy::too_many_lines)]
pub fn App() -> impl IntoView {
    let (puzzle, set_puzzle) = signal::<Option<PuzzleView>>(None);
    let (flash_diff, set_flash_diff) =
        signal::<crate::diff::PuzzleDiff>(crate::diff::PuzzleDiff::default());
    let cursor = RwSignal::new((0_usize, 0_usize));
    let active_cage = RwSignal::new(None::<usize>);
    let drafts = RwSignal::new(Vec::<DraftCage>::new());
    let entry = RwSignal::new(None::<OperatorEntry>);
    let context_menu = RwSignal::new(None::<ContextMenuState>);
    let current_path = RwSignal::new(None::<String>);
    let move_mode = RwSignal::new(None::<MoveState>);
    let show_clear_modal = RwSignal::new(false);

    refresh_from(
        set_puzzle,
        set_flash_diff,
        Box::pin(call("get_state", NoArgs {})),
    );
    listen_for_puzzle_updates(set_puzzle, set_flash_diff);
    listen_for_file_actions(set_puzzle, set_flash_diff, current_path);
    listen_for_clear_all_cages(show_clear_modal);

    let on_size_change = move |ev: Event| {
        let Ok(n) = event_target_value(&ev).parse::<usize>() else {
            return;
        };
        cursor.set((0, 0));
        active_cage.set(None);
        set_drafts_if_changed(drafts, vec![]);
        context_menu.set(None);
        refresh_from(
            set_puzzle,
            set_flash_diff,
            Box::pin(call("new_puzzle", NewPuzzleArgs { n })),
        );
    };

    let current_n = move || {
        puzzle
            .get()
            .map_or_else(|| "4".to_string(), |v| v.n.to_string())
    };

    let on_cell_click = Callback::new(move |(r, c): (usize, usize)| {
        context_menu.set(None);
        if cursor.get_untracked() != (r, c) {
            cursor.set((r, c));
        }
        set_active_cage_for_cell(puzzle, active_cage, r, c);
        let swapped = drafts.with_untracked(|ds| {
            let i = ds.iter().position(|d| d.cells.contains(&(r, c)))?;
            if i == 0 {
                return None;
            }
            let mut v = ds.clone();
            v.swap(0, i);
            Some(v)
        });
        if let Some(v) = swapped {
            drafts.set(v);
        }
    });

    let on_cell_right_click = Callback::new(move |(r, c, x, y): (usize, usize, f64, f64)| {
        cursor.set((r, c));
        set_active_cage_for_cell(puzzle, active_cage, r, c);
        let items = puzzle.with_untracked(|opt| {
            opt.as_ref().map(|v| {
                let ds = drafts.with_untracked(Clone::clone);
                menu_items_for(&MenuContext {
                    cell: (r, c),
                    view: v.clone(),
                    drafts: ds,
                })
            })
        });
        if let Some(items) = items {
            context_menu.set(Some(ContextMenuState {
                x,
                y,
                cell: (r, c),
                items,
            }));
        }
    });

    install_keydown_handler(
        puzzle,
        set_puzzle,
        set_flash_diff,
        cursor,
        active_cage,
        drafts,
        entry,
        context_menu,
        current_path,
        move_mode,
        show_clear_modal,
    );

    // Single derivation that reads puzzle + active_cage once and produces both
    // the anchor and the cell list needed by the cage band.
    let active_cage_info = Signal::derive(move || {
        puzzle.with(|opt| {
            opt.as_ref().and_then(|v| {
                active_cage
                    .get()
                    .and_then(|idx| v.cages.get(idx))
                    .map(|c| (cage_anchor(c), c.cells.clone()))
            })
        })
    });
    let active_cage_anchor =
        Signal::derive(move || active_cage_info.get().map(|(anchor, _)| anchor));
    let active_cage_cells = Signal::derive(move || {
        active_cage_info
            .get()
            .map_or_else(Vec::new, |(_, cells)| cells)
    });

    let on_band_commit = Callback::new(move |view: PuzzleView| {
        set_view(set_puzzle, set_flash_diff, view);
    });

    view! {
        <main class="app-main">
            <div class="grid-and-band">
                {move || {
                    let view = puzzle.get()?;
                    let drafts_value = drafts.get();
                    Some(view! {
                        <Grid
                            view=view
                            drafts=drafts_value
                            size=GRID_SIZE
                            cursor=cursor.into()
                            active_cage=active_cage.into()
                            on_cell_click=on_cell_click
                            on_cell_right_click=on_cell_right_click
                            entry=entry.into()
                            flash_diff=flash_diff
                            move_mode=move_mode.into()
                        />
                    })
                }}
                <CageBand
                    active_cage_anchor=active_cage_anchor
                    active_cage_cells=active_cage_cells
                    on_commit=on_band_commit
                />
            </div>
            {move || context_menu.get().map(|state| {
                view! {
                    <crate::context_menu_view::ContextMenu
                        state=state
                        puzzle=puzzle
                        set_puzzle=set_puzzle
                        set_flash_diff=set_flash_diff
                        drafts=drafts
                        active_cage=active_cage
                        cursor=cursor
                        entry=entry
                        move_mode=move_mode
                        on_close=Callback::new(move |()| context_menu.set(None))
                    />
                }
            })}
            {move || {
                if !show_clear_modal.get() {
                    return None;
                }
                let cage_count = puzzle.with(|opt| {
                    opt.as_ref().map_or(0, |v| v.cages.len())
                });
                Some(view! {
                    <crate::clear_all_cages_modal::ClearAllCagesModal
                        cage_count=cage_count
                        on_confirm=Callback::new(move |()| {
                            show_clear_modal.set(false);
                            refresh_from(
                                set_puzzle,
                                set_flash_diff,
                                Box::pin(call("clear_all_cages", NoArgs {})),
                            );
                        })
                        on_cancel=Callback::new(move |()| {
                            show_clear_modal.set(false);
                        })
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn install_keydown_handler(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    drafts: RwSignal<Vec<DraftCage>>,
    entry: RwSignal<Option<OperatorEntry>>,
    context_menu: RwSignal<Option<ContextMenuState>>,
    current_path: RwSignal<Option<String>>,
    move_mode: RwSignal<Option<MoveState>>,
    show_clear_modal: RwSignal<bool>,
) {
    window_event_listener(leptos::ev::keydown, move |ev: KeyboardEvent| {
        if let Some(current_entry) = entry.get_untracked() {
            ev.prevent_default();
            handle_entry_key(
                puzzle,
                set_puzzle,
                set_flash_diff,
                drafts,
                active_cage,
                entry,
                current_entry,
                &ev.key(),
            );
            return;
        }

        let modifier = ev.meta_key() || ev.ctrl_key();
        let key = ev.key();

        if !modifier && is_band_owned_key(&key) && cage_band::focused_thumb_idx().is_some() {
            return;
        }

        if move_mode.with_untracked(Option::is_some) {
            ev.prevent_default();
            handle_move_mode_key(
                set_puzzle,
                set_flash_diff,
                drafts,
                move_mode,
                &key,
                ev.shift_key(),
            );
            return;
        }

        if key == "Escape" && show_clear_modal.get_untracked() {
            ev.prevent_default();
            show_clear_modal.set(false);
            return;
        }

        if key == "Escape" && context_menu.with_untracked(Option::is_some) {
            ev.prevent_default();
            context_menu.set(None);
            return;
        }

        if key == "Enter" && !ev.shift_key() && !modifier && !is_text_input_focused() {
            handle_enter_key(puzzle, cursor, drafts, entry, &ev);
            return;
        }

        if !modifier
            && !ev.shift_key()
            && !is_text_input_focused()
            && try_enter_entry_with_key(puzzle, cursor, drafts, entry, &key)
        {
            ev.prevent_default();
            return;
        }

        let action = dispatch_key(&key, ev.shift_key(), modifier, is_text_input_focused());
        match action {
            KeyAction::Ignore => {}
            KeyAction::Undo => {
                ev.prevent_default();
                refresh_from_then(
                    set_puzzle,
                    set_flash_diff,
                    Box::pin(call("undo", NoArgs {})),
                    move || {
                        set_drafts_if_changed(drafts, vec![]);
                    },
                );
            }
            KeyAction::Redo => {
                ev.prevent_default();
                refresh_from_then(
                    set_puzzle,
                    set_flash_diff,
                    Box::pin(call("redo", NoArgs {})),
                    move || {
                        set_drafts_if_changed(drafts, vec![]);
                    },
                );
            }
            KeyAction::Save => {
                ev.prevent_default();
                handle_save(current_path, false);
            }
            KeyAction::SaveAs => {
                ev.prevent_default();
                handle_save(current_path, true);
            }
            KeyAction::Open => {
                ev.prevent_default();
                handle_open(set_puzzle, set_flash_diff, current_path);
            }
            KeyAction::Navigate(nav_key) => {
                ev.prevent_default();
                handle_navigate(puzzle, cursor, active_cage, nav_key);
            }
            KeyAction::ShiftArrow(nav_key) => {
                ev.prevent_default();
                handle_shift_arrow(
                    puzzle,
                    set_puzzle,
                    set_flash_diff,
                    cursor,
                    active_cage,
                    drafts,
                    nav_key,
                );
            }
            KeyAction::Escape => {
                ev.prevent_default();
                handle_cell_action(
                    puzzle,
                    set_puzzle,
                    set_flash_diff,
                    cursor,
                    active_cage,
                    drafts,
                    escape_at,
                );
            }
            KeyAction::Delete => {
                ev.prevent_default();
                handle_cell_action(
                    puzzle,
                    set_puzzle,
                    set_flash_diff,
                    cursor,
                    active_cage,
                    drafts,
                    delete_at,
                );
            }
            KeyAction::Splinter => {
                ev.prevent_default();
                handle_cell_action(
                    puzzle,
                    set_puzzle,
                    set_flash_diff,
                    cursor,
                    active_cage,
                    drafts,
                    splinter_at,
                );
            }
            KeyAction::MoveCell => {
                ev.prevent_default();
                enter_move_mode(puzzle, cursor, move_mode);
            }
        }
    });
}

fn enter_move_mode(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    move_mode: RwSignal<Option<MoveState>>,
) {
    let cell = cursor.get_untracked();
    let targets = puzzle.with_untracked(|opt| {
        opt.as_ref()
            .map_or_else(Vec::new, |v| legal_move_targets(v, cell))
    });
    if targets.is_empty() {
        return;
    }
    move_mode.set(Some(MoveState {
        cell,
        targets,
        selected: None,
    }));
}

fn handle_move_mode_key(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    drafts: RwSignal<Vec<DraftCage>>,
    move_mode: RwSignal<Option<MoveState>>,
    key: &str,
    shift: bool,
) {
    let Some(state) = move_mode.get_untracked() else {
        return;
    };
    match key {
        "Escape" => {
            move_mode.set(None);
        }
        "Tab" => {
            let len = state.targets.len();
            let next_selected = if shift {
                // Shift+Tab: decrement with wrap
                Some(state.selected.map_or_else(
                    || len.saturating_sub(1),
                    |i| if i == 0 { len - 1 } else { i - 1 },
                ))
            } else {
                Some((state.selected.unwrap_or(len - 1) + 1) % len)
            };
            move_mode.set(Some(MoveState {
                cell: state.cell,
                targets: state.targets,
                selected: next_selected,
            }));
        }
        "Enter" => {
            if let Some(sel_idx) = state.selected {
                let target_anchor = state.targets[sel_idx];
                let cell = state.cell;
                move_mode.set(None);
                dispatch_edit(
                    set_puzzle,
                    set_flash_diff,
                    drafts,
                    Box::pin(call_edit(
                        "move_cell",
                        MoveCellArgs {
                            cell,
                            target_anchor,
                        },
                    )),
                    None,
                );
            }
        }
        _ => {
            // Any other key cancels move mode
            move_mode.set(None);
        }
    }
}

/// Ask the user to pick a save path if needed, then call `save_puzzle`.
/// `force_prompt` = true means always show the dialog (Save As).
#[allow(clippy::future_not_send)]
fn handle_save(current_path: RwSignal<Option<String>>, force_prompt: bool) {
    spawn_local(async move {
        let path = if force_prompt {
            prompt_save_path().await
        } else {
            match current_path.get_untracked() {
                Some(p) => Some(p),
                None => prompt_save_path().await,
            }
        };
        let Some(path) = path else { return };
        let args = serde_wasm_bindgen::to_value(&PathArgs { path: path.clone() }).ok();
        if let Some(args) = args {
            let result = invoke("save_puzzle", args).await;
            if serde_wasm_bindgen::from_value::<()>(result).is_ok() {
                current_path.set(Some(path));
            }
        }
    });
}

/// Ask the user to pick an existing file, then call `load_puzzle` and update the view.
#[allow(clippy::future_not_send)]
fn handle_open(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    current_path: RwSignal<Option<String>>,
) {
    spawn_local(async move {
        let path = prompt_open_path().await;
        let Some(path) = path else { return };
        let args = serde_wasm_bindgen::to_value(&PathArgs { path: path.clone() }).ok();
        if let Some(args) = args {
            let value = invoke("load_puzzle", args).await;
            if let Ok(view) = serde_wasm_bindgen::from_value::<PuzzleView>(value) {
                set_view(set_puzzle, set_flash_diff, view);
                current_path.set(Some(path));
            }
        }
    });
}

#[derive(Serialize)]
struct DialogFilter {
    name: &'static str,
    extensions: Vec<&'static str>,
}

fn kenken_filter() -> DialogFilter {
    DialogFilter {
        name: "KenKen",
        extensions: vec!["kenken"],
    }
}

#[allow(clippy::future_not_send)]
async fn prompt_save_path() -> Option<String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SaveOptions {
        default_path: &'static str,
        filters: Vec<DialogFilter>,
    }
    let opts = SaveOptions {
        default_path: "puzzle.kenken",
        filters: vec![kenken_filter()],
    };
    let opts_js = serde_wasm_bindgen::to_value(&opts).ok()?;
    save(opts_js).await.as_string()
}

#[allow(clippy::future_not_send)]
async fn prompt_open_path() -> Option<String> {
    #[derive(Serialize)]
    struct OpenOptions {
        multiple: bool,
        filters: Vec<DialogFilter>,
    }
    let opts = OpenOptions {
        multiple: false,
        filters: vec![kenken_filter()],
    };
    let opts_js = serde_wasm_bindgen::to_value(&opts).ok()?;
    open(opts_js).await.as_string()
}

fn listen_for_file_actions(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    current_path: RwSignal<Option<String>>,
) {
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
            if let Some(action) = payload.as_string() {
                match action.as_str() {
                    "save" => handle_save(current_path, false),
                    "save_as" => handle_save(current_path, true),
                    "open" => handle_open(set_puzzle, set_flash_diff, current_path),
                    _ => {}
                }
            }
        }
    });
    let _ = listen(FILE_ACTION_EVENT, &cb);
    cb.forget();
}

fn listen_for_clear_all_cages(show_clear_modal: RwSignal<bool>) {
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |_event: JsValue| {
        show_clear_modal.set(true);
    });
    let _ = listen(CLEAR_ALL_CAGES_EVENT, &cb);
    cb.forget();
}

#[allow(clippy::too_many_arguments)]
fn handle_entry_key(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    drafts: RwSignal<Vec<DraftCage>>,
    active_cage: RwSignal<Option<usize>>,
    entry: RwSignal<Option<OperatorEntry>>,
    current_entry: OperatorEntry,
    key: &str,
) {
    let single_cell = puzzle.with_untracked(|opt| {
        opt.as_ref().is_some_and(|v| match &current_entry.cage {
            ActiveCage::Committed(idx) => v.cages.get(*idx).is_some_and(|c| c.cells.len() == 1),
            ActiveCage::Draft => {
                drafts.with_untracked(|ds| ds.first().is_some_and(|d| d.cells.len() == 1))
            }
        })
    });
    let cage = current_entry.cage.clone();
    match operator_step(current_entry, key, single_cell) {
        Step::Update(new_entry) => entry.set(Some(new_entry)),
        Step::Cancel => entry.set(None),
        Step::Commit { op, target } => match cage {
            ActiveCage::Draft => {
                let cells = drafts
                    .with_untracked(|ds| ds.first().map(|d| d.cells.clone()).unwrap_or_default());
                let remaining_drafts = drafts.with_untracked(|ds| ds[1..].to_vec());
                let anchor = cells_anchor(&cells);
                refresh_from_then(
                    set_puzzle,
                    set_flash_diff,
                    Box::pin(call("insert_cage", InsertCageArgs { cells, op, target })),
                    move || {
                        set_drafts_if_changed(drafts, remaining_drafts);
                        entry.set(None);
                        set_active_cage_for_cell(puzzle, active_cage, anchor.0, anchor.1);
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
                    set_flash_diff,
                    Box::pin(call(
                        "set_cage_operation",
                        SetCageOperationArgs { anchor, op, target },
                    )),
                    move || {
                        entry.set(None);
                        set_active_cage_for_cell(puzzle, active_cage, anchor.0, anchor.1);
                    },
                );
            }
        },
    }
}

struct EntryTarget {
    initial: OperatorEntry,
    anchor: (usize, usize),
    single_cell: bool,
}

fn find_entry_target(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    drafts: RwSignal<Vec<DraftCage>>,
) -> Option<EntryTarget> {
    let committed = puzzle.with_untracked(|opt| {
        opt.as_ref().and_then(|v| {
            let (r, c) = cursor.get_untracked();
            cage_at(v, r, c).map(|idx| {
                let anchor = cage_anchor(&v.cages[idx]);
                let cage_op = v.cages[idx].op;
                let cage_target = v.cages[idx].target;
                let single_cell = v.cages[idx].cells.len() == 1;
                let initial = OperatorEntry {
                    cage: ActiveCage::Committed(idx),
                    op: Some(cage_op),
                    digits: if cage_target > 0 {
                        cage_target.to_string()
                    } else {
                        String::new()
                    },
                };
                EntryTarget {
                    initial,
                    anchor,
                    single_cell,
                }
            })
        })
    });
    if committed.is_some() {
        return committed;
    }
    let (anchor, single_cell) = drafts.with_untracked(|ds| {
        ds.first()
            .map(|d| (cells_anchor(&d.cells), d.cells.len() == 1))
    })?;
    Some(EntryTarget {
        initial: OperatorEntry {
            cage: ActiveCage::Draft,
            op: None,
            digits: String::new(),
        },
        anchor,
        single_cell,
    })
}

fn handle_enter_key(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    drafts: RwSignal<Vec<DraftCage>>,
    entry: RwSignal<Option<OperatorEntry>>,
    ev: &KeyboardEvent,
) {
    if let Some(t) = find_entry_target(puzzle, cursor, drafts) {
        ev.prevent_default();
        cursor.set(t.anchor);
        entry.set(Some(t.initial));
    }
}

fn try_enter_entry_with_key(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    drafts: RwSignal<Vec<DraftCage>>,
    entry: RwSignal<Option<OperatorEntry>>,
    key: &str,
) -> bool {
    if !is_entry_trigger_key(key) {
        return false;
    }
    let Some(t) = find_entry_target(puzzle, cursor, drafts) else {
        return false;
    };
    match operator_step(t.initial, key, t.single_cell) {
        Step::Update(new_entry) => {
            cursor.set(t.anchor);
            entry.set(Some(new_entry));
            true
        }
        // Unreachable: trigger keys cannot produce Commit (requires Enter) or Cancel (requires Escape).
        Step::Commit { .. } | Step::Cancel => false,
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
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    drafts: RwSignal<Vec<DraftCage>>,
    nav_key: NavKey,
) {
    let at = cursor.get_untracked();
    let action = puzzle.with_untracked(|opt| {
        let v = opt.as_ref()?;
        let neighbor = move_cursor(at, v.n, nav_key);
        if neighbor == at {
            return None;
        }
        let action = drafts.with_untracked(|ds| shift_arrow(at, neighbor, v, ds.first()));
        Some((neighbor, action))
    });
    let Some((neighbor, action)) = action else {
        return;
    };
    apply_edit(set_puzzle, set_flash_diff, drafts, active_cage, action);
    if cursor.get_untracked() != neighbor {
        cursor.set(neighbor);
    }
    sync_active_cage(puzzle, cursor, active_cage);
}

fn handle_cell_action(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
    drafts: RwSignal<Vec<DraftCage>>,
    branch: fn((usize, usize), &PuzzleView, Option<&DraftCage>) -> CageEdit,
) {
    let at = cursor.get_untracked();
    let action = puzzle.with_untracked(|opt| {
        opt.as_ref()
            .map(|v| drafts.with_untracked(|ds| branch(at, v, ds.first())))
    });
    let Some(action) = action else { return };
    apply_edit(set_puzzle, set_flash_diff, drafts, active_cage, action);
    sync_active_cage(puzzle, cursor, active_cage);
}

pub fn apply_edit(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    drafts: RwSignal<Vec<DraftCage>>,
    active_cage: RwSignal<Option<usize>>,
    action: CageEdit,
) {
    match action {
        CageEdit::Noop => {}
        CageEdit::SetDraft(d) => {
            set_drafts_if_changed(drafts, d.into_iter().collect());
        }
        CageEdit::ExtendCage { anchor, cell } => {
            dispatch_edit(
                set_puzzle,
                set_flash_diff,
                drafts,
                Box::pin(call_edit("extend_cage", ExtendCageArgs { anchor, cell })),
                None,
            );
        }
        CageEdit::MergeCages { a_anchor, b_anchor } => {
            dispatch_edit(
                set_puzzle,
                set_flash_diff,
                drafts,
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
                set_flash_diff,
                drafts,
                Box::pin(call_edit("shrink_cage", CellArgs { cell })),
                None,
            );
        }
        CageEdit::SplinterFromCommitted(cell) => {
            dispatch_edit(
                set_puzzle,
                set_flash_diff,
                drafts,
                Box::pin(call_edit("shrink_cage", CellArgs { cell })),
                Some(DraftCage { cells: vec![cell] }),
            );
        }
        CageEdit::RemoveCage(anchor) => {
            refresh_from_then(
                set_puzzle,
                set_flash_diff,
                Box::pin(call("remove_cage", AnchorArgs { anchor })),
                move || {
                    if active_cage.get_untracked().is_some() {
                        active_cage.set(None);
                    }
                    set_drafts_if_changed(drafts, vec![]);
                },
            );
        }
    }
}

fn set_drafts_if_changed(drafts: RwSignal<Vec<DraftCage>>, next: Vec<DraftCage>) {
    if drafts.with_untracked(|d| d != &next) {
        drafts.set(next);
    }
}

pub fn dispatch_edit(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    drafts: RwSignal<Vec<DraftCage>>,
    fut: Pin<Box<dyn Future<Output = Option<EditResult>>>>,
    override_draft: Option<DraftCage>,
) {
    spawn_local(async move {
        if let Some(result) = fut.await {
            set_view(set_puzzle, set_flash_diff, result.view);
            let next_drafts = override_draft.map_or(result.drafts, |d| vec![d]);
            set_drafts_if_changed(drafts, next_drafts);
        }
    });
}

// Preserves the prior cage when the cell is uncaged so the tuple strip stays put.
fn set_active_cage_for_cell(
    puzzle: ReadSignal<Option<PuzzleView>>,
    active_cage: RwSignal<Option<usize>>,
    r: usize,
    c: usize,
) {
    let target = puzzle.with_untracked(|opt| opt.as_ref().and_then(|v| cage_at(v, r, c)));
    let next_active = target.or_else(|| active_cage.get_untracked());
    if active_cage.get_untracked() != next_active {
        active_cage.set(next_active);
    }
}

pub fn sync_active_cage(
    puzzle: ReadSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
) {
    let (r, c) = cursor.get_untracked();
    set_active_cage_for_cell(puzzle, active_cage, r, c);
}

fn set_view(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    view: PuzzleView,
) {
    set_flash_diff.set(view.diff.clone());
    set_puzzle.set(Some(view));
}

fn refresh_from(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    fut: Pin<Box<dyn Future<Output = Option<PuzzleView>>>>,
) {
    refresh_from_then(set_puzzle, set_flash_diff, fut, || {});
}

fn refresh_from_then(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    fut: Pin<Box<dyn Future<Output = Option<PuzzleView>>>>,
    on_success: impl FnOnce() + 'static,
) {
    spawn_local(async move {
        if let Some(view) = fut.await {
            set_view(set_puzzle, set_flash_diff, view);
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
    fn dispatch_key_returns_delete_for_delete_key() {
        assert_eq!(
            dispatch_key("Delete", false, false, false),
            KeyAction::Delete
        );
    }

    #[test]
    fn dispatch_key_ignores_x_and_x() {
        assert_eq!(dispatch_key("x", false, false, false), KeyAction::Ignore);
        assert_eq!(dispatch_key("X", false, false, false), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_returns_splinter_for_space_or_c() {
        assert_eq!(dispatch_key(" ", false, false, false), KeyAction::Splinter);
        assert_eq!(dispatch_key("c", false, false, false), KeyAction::Splinter);
        assert_eq!(dispatch_key("C", false, false, false), KeyAction::Splinter);
    }

    #[test]
    fn dispatch_key_ignores_modifier_plus_other_keys() {
        // Cmd+c, Cmd+Escape, Cmd+Space should not trigger cage edits.
        assert_eq!(dispatch_key("x", false, true, false), KeyAction::Ignore);
        assert_eq!(dispatch_key("c", false, true, false), KeyAction::Ignore);
        assert_eq!(dispatch_key(" ", false, true, false), KeyAction::Ignore);
        assert_eq!(
            dispatch_key("Escape", false, true, false),
            KeyAction::Ignore
        );
    }

    #[test]
    fn dispatch_key_returns_save_for_modifier_s() {
        assert_eq!(dispatch_key("s", false, true, false), KeyAction::Save);
        assert_eq!(dispatch_key("S", false, true, false), KeyAction::Save);
    }

    #[test]
    fn dispatch_key_returns_save_as_for_modifier_shift_s() {
        assert_eq!(dispatch_key("s", true, true, false), KeyAction::SaveAs);
        assert_eq!(dispatch_key("S", true, true, false), KeyAction::SaveAs);
    }

    #[test]
    fn dispatch_key_returns_open_for_modifier_o() {
        assert_eq!(dispatch_key("o", false, true, false), KeyAction::Open);
        assert_eq!(dispatch_key("O", false, true, false), KeyAction::Open);
    }

    #[test]
    fn dispatch_key_ignores_file_keys_without_modifier() {
        assert_eq!(dispatch_key("s", false, false, false), KeyAction::Ignore);
        assert_eq!(dispatch_key("o", false, false, false), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_ignores_file_keys_when_text_input_focused() {
        assert_eq!(dispatch_key("s", false, true, true), KeyAction::Ignore);
        assert_eq!(dispatch_key("o", false, true, true), KeyAction::Ignore);
    }

    #[test]
    fn is_band_owned_key_matches_arrow_enter_escape() {
        assert!(is_band_owned_key("ArrowUp"));
        assert!(is_band_owned_key("ArrowDown"));
        assert!(is_band_owned_key("Enter"));
        assert!(is_band_owned_key("Escape"));
    }

    #[test]
    fn is_band_owned_key_rejects_other_keys() {
        assert!(!is_band_owned_key("ArrowLeft"));
        assert!(!is_band_owned_key("ArrowRight"));
        assert!(!is_band_owned_key("Tab"));
        assert!(!is_band_owned_key(" "));
        assert!(!is_band_owned_key("a"));
        assert!(!is_band_owned_key(""));
    }
}
