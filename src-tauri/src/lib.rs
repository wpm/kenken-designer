mod cage_edit;
pub mod diff;
mod edit;
mod session;
mod view;

use std::sync::Mutex;

use kenken::constraints::cover::Cover;
use kenken::{Delta, Fill, Puzzle};
use tauri::image::Image;
use tauri::menu::{
    AboutMetadata, IsMenuItem, Menu, MenuEvent, MenuItemBuilder, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use session::Session;
use view::{CageOption, DraftCage, EditResult, OpKind, PuzzleView, RankedTupleView};

const PUZZLE_UPDATED_EVENT: &str = "puzzle-updated";
const CLEAR_ALL_CAGES_EVENT: &str = "clear-all-cages";

const ERR_VALUE_OUT_OF_RANGE: &str = "value out of range";

fn commit_new_puzzle(state: &Mutex<Session>, n: usize) -> Result<PuzzleView, String> {
    let puzzle = Puzzle::new(n).map_err(|e| format!("{e:?}"))?;
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.load(puzzle);
    Ok(PuzzleView::from(session.current()))
}

fn current_state(state: &Mutex<Session>) -> Result<PuzzleView, String> {
    let session = state.lock().map_err(|e| format!("{e:?}"))?;
    Ok(PuzzleView::from(session.current()))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn new_puzzle(n: usize, state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    commit_new_puzzle(&state, n)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn get_state(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    current_state(&state)
}

fn apply_undo(state: &Mutex<Session>) -> Result<PuzzleView, String> {
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    let pre = session.current().clone();
    session.undo();
    let post = session.current().clone();
    drop(session);
    let d = same_size_diff(&pre, &post);
    Ok(PuzzleView::from(&post).with_diff(d))
}

fn apply_redo(state: &Mutex<Session>) -> Result<PuzzleView, String> {
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    let pre = session.current().clone();
    session.redo();
    let post = session.current().clone();
    drop(session);
    let d = same_size_diff(&pre, &post);
    Ok(PuzzleView::from(&post).with_diff(d))
}

fn same_size_diff(pre: &Puzzle, post: &Puzzle) -> diff::PuzzleDiff {
    if pre.n() == post.n() {
        diff::PuzzleDiff::between(pre, post)
    } else {
        diff::PuzzleDiff::default()
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn undo(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    apply_undo(&state)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn redo(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    apply_redo(&state)
}

fn commit_view(session: &mut Session, next: Puzzle) -> PuzzleView {
    let pre = session.current().clone();
    let propagated = next.propagate_fully();
    session.commit(propagated);
    let post = session.current();
    let d = diff::PuzzleDiff::between(&pre, post);
    PuzzleView::from(post).with_diff(d)
}

fn commit_edit(session: &mut Session, next: Puzzle, drafts: Vec<DraftCage>) -> EditResult {
    let view = commit_view(session, next);
    EditResult { view, drafts }
}

fn do_command<T>(
    state: &Mutex<Session>,
    f: impl FnOnce(&mut Session) -> Result<T, String>,
) -> Result<T, String> {
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    f(&mut session)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn insert_cage(
    cells: Vec<(usize, usize)>,
    op: OpKind,
    target: u32,
    state: State<Mutex<Session>>,
) -> Result<PuzzleView, String> {
    do_command(&state, |session| {
        let next = cage_edit::do_insert_cage(session.current(), &cells, op, target)?;
        Ok(commit_view(session, next))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn remove_cage(anchor: (usize, usize), state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    do_command(&state, |session| {
        let next = cage_edit::do_remove_cage(session.current(), anchor)?;
        Ok(commit_view(session, next))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn extend_cage(
    anchor: (usize, usize),
    cell: (usize, usize),
    state: State<Mutex<Session>>,
) -> Result<EditResult, String> {
    do_command(&state, |session| {
        let (next, draft) = cage_edit::do_extend_cage(session.current(), anchor, cell)?;
        Ok(commit_edit(session, next, draft.into_iter().collect()))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn shrink_cage(cell: (usize, usize), state: State<Mutex<Session>>) -> Result<EditResult, String> {
    do_command(&state, |session| {
        let (next, draft) = cage_edit::do_shrink_cage(session.current(), cell)?;
        Ok(commit_edit(session, next, draft.into_iter().collect()))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn set_cage_operation(
    anchor: (usize, usize),
    op: OpKind,
    target: u32,
    state: State<Mutex<Session>>,
) -> Result<PuzzleView, String> {
    do_command(&state, |session| {
        let next = cage_edit::do_set_cage_operation(session.current(), anchor, op, target)?;
        Ok(commit_view(session, next))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn merge_cages(
    a_anchor: (usize, usize),
    b_anchor: (usize, usize),
    state: State<Mutex<Session>>,
) -> Result<EditResult, String> {
    do_command(&state, |session| {
        let (next, draft) = cage_edit::do_merge_cages(session.current(), a_anchor, b_anchor)?;
        Ok(commit_edit(session, next, draft.into_iter().collect()))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn clear_all_cages(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    do_command(&state, |session| {
        let next = edit::apply_edit(session.current(), edit::EditKind::Widening, |p| {
            Ok(cage_edit::do_clear_all_cages(p))
        })?;
        Ok(commit_view(session, next))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn flip_cell(
    cell: (usize, usize),
    target_anchor: (usize, usize),
    state: State<Mutex<Session>>,
) -> Result<EditResult, String> {
    do_command(&state, |session| {
        let (next, drafts) = cage_edit::do_flip_cell(session.current(), cell, target_anchor)?;
        Ok(commit_edit(session, next, drafts))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn move_cell(
    cell: (usize, usize),
    target_anchor: (usize, usize),
    state: State<Mutex<Session>>,
) -> Result<EditResult, String> {
    do_command(&state, |session| {
        let (next, drafts) = cage_edit::do_move_cell(session.current(), cell, target_anchor)?;
        Ok(commit_edit(session, next, drafts))
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn legal_move_targets(
    cell: (usize, usize),
    state: State<Mutex<Session>>,
) -> Result<Vec<(usize, usize)>, String> {
    let session = state.lock().map_err(|e| format!("{e:?}"))?;
    Ok(cage_edit::legal_move_targets(session.current(), cell))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn cage_options(
    cells: Vec<(usize, usize)>,
    state: State<Mutex<Session>>,
) -> Result<Vec<CageOption>, String> {
    let session = state.lock().map_err(|e| format!("{e:?}"))?;
    Ok(cage_edit::cage_options(&cells, session.current().n()))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn rank_active_cage(
    anchor: (usize, usize),
    state: State<Mutex<Session>>,
) -> Result<Vec<RankedTupleView>, String> {
    let puzzle = state
        .lock()
        .map_err(|e| format!("{e:?}"))?
        .current()
        .clone();
    let cage = cage_edit::cage_at_or_err(&puzzle, anchor)?.clone();
    let ranked = puzzle
        .rank_tuples_for_cage(&cage)
        .map_err(|e| format!("{e:?}"))?;
    Ok(ranked
        .into_iter()
        .map(|(tuple, narrowed, score)| RankedTupleView {
            tuple: tuple.into_iter().map(u32::from).collect(),
            view: PuzzleView::from(&narrowed),
            total_reduction: score.total_reduction,
            newly_singleton: score.newly_singleton,
        })
        .collect())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn apply_narrowing(
    anchor: (usize, usize),
    tuple: Vec<u32>,
    state: State<Mutex<Session>>,
) -> Result<PuzzleView, String> {
    let puzzle = state
        .lock()
        .map_err(|e| format!("{e:?}"))?
        .current()
        .clone();
    let cells = cage_edit::cage_at_or_err(&puzzle, anchor)?.cells();
    if cells.len() != tuple.len() {
        return Err(format!(
            "tuple length {} does not match cage size {}",
            tuple.len(),
            cells.len()
        ));
    }
    let mut delta = Delta::identity(puzzle.n()).map_err(|e| format!("{e:?}"))?;
    for (cell, v) in cells.iter().zip(tuple.iter()) {
        let val = u8::try_from(*v).map_err(|e| format!("{ERR_VALUE_OUT_OF_RANGE}: {e}"))?;
        delta = delta.set(*cell, Fill::new([val]));
    }
    Ok(PuzzleView::from(
        &puzzle.narrow(&delta).map_err(|e| format!("{e:?}"))?,
    ))
}

#[allow(clippy::too_many_lines)] // Per-OS submenu construction is the long part
fn build_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let pkg_info = app.package_info();
    let config = app.config();
    let icon = Image::from_bytes(include_bytes!("../icons/128x128.png"))?;
    let about_metadata = AboutMetadata {
        name: None,
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        icon: Some(icon),
        ..Default::default()
    };

    // Accelerators live in the WASM keydown handler (see src/app.rs), not on
    // the menu items, so Cmd/Ctrl+Z fires exactly once regardless of platform.
    let undo = MenuItemBuilder::with_id("undo", "Undo").build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "Redo").build(app)?;
    let clear_all = MenuItemBuilder::with_id("clear_all_cages", "Clear all cages").build(app)?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &PredefinedMenuItem::separator(app)?,
            &clear_all,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    #[cfg(target_os = "macos")]
    let help_menu = Submenu::with_items(app, "Help", true, &[])?;
    #[cfg(not(target_os = "macos"))]
    let help_menu = Submenu::with_items(
        app,
        "Help",
        true,
        &[&PredefinedMenuItem::about(
            app,
            None,
            Some(about_metadata.clone()),
        )?],
    )?;

    #[cfg(target_os = "macos")]
    let app_menu = Submenu::with_items(
        app,
        pkg_info.name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(
                app,
                Some("About KenKen Designer"),
                Some(about_metadata.clone()),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    let file_menu: Option<Submenu<R>> = None;
    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    let file_menu = {
        let close = PredefinedMenuItem::close_window(app, None)?;
        #[cfg(not(target_os = "macos"))]
        let quit = PredefinedMenuItem::quit(app, None)?;
        #[cfg(target_os = "macos")]
        let items: Vec<&dyn IsMenuItem<R>> = vec![&close];
        #[cfg(not(target_os = "macos"))]
        let items: Vec<&dyn IsMenuItem<R>> = vec![&close, &quit];
        Some(Submenu::with_items(app, "File", true, &items)?)
    };

    #[cfg(target_os = "macos")]
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;

    let mut items: Vec<&dyn IsMenuItem<R>> = Vec::new();
    #[cfg(target_os = "macos")]
    items.push(&app_menu);
    if let Some(ref file_menu) = file_menu {
        items.push(file_menu);
    }
    items.push(&edit);
    #[cfg(target_os = "macos")]
    items.push(&view_menu);
    items.push(&window_menu);
    items.push(&help_menu);

    Menu::with_items(app, &items)
}

fn apply_menu_action(session: &mut Session, id: &str) -> bool {
    match id {
        "undo" => {
            session.undo();
            true
        }
        "redo" => {
            session.redo();
            true
        }
        _ => false,
    }
}

fn dispatch_menu_action(state: &Mutex<Session>, id: &str) -> Option<PuzzleView> {
    let mut session = state.lock().ok()?;
    if apply_menu_action(&mut session, id) {
        Some(PuzzleView::from(session.current()))
    } else {
        None
    }
}

#[allow(clippy::needless_pass_by_value)] // on_menu_event requires by-value MenuEvent
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id().as_ref();
    if id == "clear_all_cages" {
        let _ = app.emit(CLEAR_ALL_CAGES_EVENT, ());
        return;
    }
    let Some(state) = app.try_state::<Mutex<Session>>() else {
        return;
    };
    if let Some(view) = dispatch_menu_action(&state, id) {
        let _ = app.emit(PUZZLE_UPDATED_EVENT, view);
    }
}

/// # Panics
///
/// Panics if the Tauri application fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    let initial = Puzzle::new(4).expect("4 is a valid grid size");
    let session = Session::new(initial);
    tauri::Builder::default()
        .manage(Mutex::new(session))
        .menu(build_app_menu)
        .on_menu_event(handle_menu_event)
        .invoke_handler(tauri::generate_handler![
            new_puzzle,
            get_state,
            undo,
            redo,
            insert_cage,
            remove_cage,
            extend_cage,
            shrink_cage,
            merge_cages,
            set_cage_operation,
            flip_cell,
            move_cell,
            legal_move_targets,
            cage_options,
            clear_all_cages,
            rank_active_cage,
            apply_narrowing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic, // tests intentionally panic to poison a Mutex
    clippy::significant_drop_tightening // tests don't have lock-contention concerns
)]
mod tests {
    use super::*;

    fn poison_lock<T>(m: &Mutex<T>) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = m.lock().unwrap();
            panic!("intentional");
        }));
    }

    #[test]
    fn apply_menu_action_undo_pops_undo_stack() {
        let mut s = Session::new(Puzzle::new(3).unwrap());
        s.commit(Puzzle::new(4).unwrap());
        assert_eq!(s.current().n(), 4);

        assert!(apply_menu_action(&mut s, "undo"));
        assert_eq!(s.current().n(), 3);
    }

    #[test]
    fn apply_menu_action_redo_pops_redo_stack() {
        let mut s = Session::new(Puzzle::new(3).unwrap());
        s.commit(Puzzle::new(4).unwrap());
        s.undo();
        assert_eq!(s.current().n(), 3);

        assert!(apply_menu_action(&mut s, "redo"));
        assert_eq!(s.current().n(), 4);
    }

    #[test]
    fn apply_menu_action_returns_false_for_unknown_id() {
        let mut s = Session::new(Puzzle::new(3).unwrap());
        assert!(!apply_menu_action(&mut s, "quit"));
        assert!(!apply_menu_action(&mut s, ""));
        assert_eq!(s.current().n(), 3);
    }

    // muda enforces "main thread only" for menu construction on macOS, but cargo's test
    // harness runs each test on a worker thread, so this test can only be exercised on
    // Linux and Windows. The macOS branch of build_app_menu is still compile-checked there
    // and exercised at runtime when the app launches.
    #[cfg(not(target_os = "macos"))]
    fn find_submenu_by_text<'a>(
        items: &'a [tauri::menu::MenuItemKind<tauri::test::MockRuntime>],
        text: &str,
    ) -> Option<&'a tauri::menu::Submenu<tauri::test::MockRuntime>> {
        items.iter().find_map(|i| match i {
            tauri::menu::MenuItemKind::Submenu(s) if s.text().ok().as_deref() == Some(text) => {
                Some(s)
            }
            _ => None,
        })
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn build_app_menu_includes_edit_submenu_with_custom_undo() {
        let app = tauri::test::mock_app();
        let menu = build_app_menu(app.handle()).unwrap();
        let items = menu.items().unwrap();
        assert!(!items.is_empty());

        let edit = find_submenu_by_text(&items, "Edit").unwrap();
        let edit_items = edit.items().unwrap();
        let has_custom_undo = edit_items.iter().any(
            |i| matches!(i, tauri::menu::MenuItemKind::MenuItem(m) if m.id().as_ref() == "undo"),
        );
        assert!(has_custom_undo, "Edit submenu should contain custom Undo");
    }

    // Window submenu is not the first item, so the helper's `find_map` closure
    // visits the Edit submenu first and falls through the `_ => None` arm
    // before matching Window. This exercises the non-matching branch.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn build_app_menu_includes_window_submenu() {
        let app = tauri::test::mock_app();
        let menu = build_app_menu(app.handle()).unwrap();
        let items = menu.items().unwrap();

        assert!(
            find_submenu_by_text(&items, "Window").is_some(),
            "menu should contain a Window submenu"
        );
    }

    fn current_n(app: &tauri::App<tauri::test::MockRuntime>) -> usize {
        let state = app.state::<Mutex<Session>>();
        let n = state.lock().unwrap().current().n();
        n
    }

    #[test]
    fn handle_menu_event_undoes_when_id_is_undo() {
        let app = tauri::test::mock_app();
        let mut session = Session::new(Puzzle::new(3).unwrap());
        session.commit(Puzzle::new(4).unwrap());
        app.manage(Mutex::new(session));

        handle_menu_event(
            app.handle(),
            MenuEvent {
                id: tauri::menu::MenuId::new("undo"),
            },
        );

        assert_eq!(current_n(&app), 3);
    }

    #[test]
    fn handle_menu_event_ignores_unknown_id() {
        let app = tauri::test::mock_app();
        let session = Session::new(Puzzle::new(3).unwrap());
        app.manage(Mutex::new(session));

        handle_menu_event(
            app.handle(),
            MenuEvent {
                id: tauri::menu::MenuId::new("quit"),
            },
        );

        assert_eq!(current_n(&app), 3);
    }

    #[test]
    fn commit_new_puzzle_returns_empty_puzzle() {
        let state = Mutex::new(Session::new(Puzzle::new(2).unwrap()));
        let view = commit_new_puzzle(&state, 5).unwrap();
        assert_eq!(view.n, 5);
        assert_eq!(view.cells.len(), 5);
        assert_eq!(view.cells.iter().flat_map(|r| r.iter()).count(), 25);
        assert!(view.cages.is_empty(), "startup puzzle should have no cages");
        let session = state.lock().unwrap();
        assert_eq!(session.current().n(), 5);
        assert_eq!(session.current().cages().count(), 0);
    }

    #[test]
    fn commit_new_puzzle_propagates_invalid_size_error() {
        let state = Mutex::new(Session::new(Puzzle::new(3).unwrap()));
        assert!(commit_new_puzzle(&state, 0).is_err());
        let session = state.lock().unwrap();
        assert_eq!(session.current().n(), 3, "session unchanged on error");
    }

    #[test]
    fn current_state_returns_a_view_of_the_current_puzzle() {
        let state = Mutex::new(Session::new(Puzzle::new(5).unwrap()));
        let view = current_state(&state).unwrap();
        assert_eq!(view.n, 5);
    }

    #[test]
    fn current_state_returns_err_when_lock_poisoned() {
        let state = Mutex::new(Session::new(Puzzle::new(3).unwrap()));
        poison_lock(&state);
        assert!(state.is_poisoned());
        assert!(current_state(&state).is_err());
    }

    #[test]
    fn dispatch_menu_action_returns_view_for_undo() {
        let mut s = Session::new(Puzzle::new(3).unwrap());
        s.commit(Puzzle::new(4).unwrap());
        let state = Mutex::new(s);

        let view = dispatch_menu_action(&state, "undo").unwrap();
        assert_eq!(view.n, 3);
    }

    #[test]
    fn dispatch_menu_action_returns_none_for_unknown_id() {
        let state = Mutex::new(Session::new(Puzzle::new(3).unwrap()));
        assert!(dispatch_menu_action(&state, "quit").is_none());
    }

    #[test]
    fn dispatch_menu_action_returns_none_when_lock_poisoned() {
        let state = Mutex::new(Session::new(Puzzle::new(3).unwrap()));
        poison_lock(&state);
        assert!(dispatch_menu_action(&state, "undo").is_none());
    }

    #[test]
    fn apply_undo_pops_undo_stack_and_returns_view() {
        let mut s = Session::new(Puzzle::new(3).unwrap());
        s.commit(Puzzle::new(4).unwrap());
        let state = Mutex::new(s);

        let view = apply_undo(&state).unwrap();
        assert_eq!(view.n, 3);
        assert_eq!(state.lock().unwrap().current().n(), 3);
    }

    #[test]
    fn apply_undo_is_noop_when_undo_stack_empty() {
        let state = Mutex::new(Session::new(Puzzle::new(5).unwrap()));
        let view = apply_undo(&state).unwrap();
        assert_eq!(view.n, 5);
    }

    #[test]
    fn apply_undo_returns_err_when_lock_poisoned() {
        let state = Mutex::new(Session::new(Puzzle::new(3).unwrap()));
        poison_lock(&state);
        assert!(state.is_poisoned());
        assert!(apply_undo(&state).is_err());
    }

    #[test]
    fn apply_redo_pops_redo_stack_and_returns_view() {
        let mut s = Session::new(Puzzle::new(3).unwrap());
        s.commit(Puzzle::new(4).unwrap());
        s.undo();
        let state = Mutex::new(s);

        let view = apply_redo(&state).unwrap();
        assert_eq!(view.n, 4);
        assert_eq!(state.lock().unwrap().current().n(), 4);
    }

    #[test]
    fn apply_redo_is_noop_when_redo_stack_empty() {
        let state = Mutex::new(Session::new(Puzzle::new(5).unwrap()));
        let view = apply_redo(&state).unwrap();
        assert_eq!(view.n, 5);
    }

    #[test]
    fn apply_redo_returns_err_when_lock_poisoned() {
        let state = Mutex::new(Session::new(Puzzle::new(3).unwrap()));
        poison_lock(&state);
        assert!(state.is_poisoned());
        assert!(apply_redo(&state).is_err());
    }

    #[test]
    fn undo_command_returns_view_after_session_change() {
        let app = tauri::test::mock_app();
        let mut session = Session::new(Puzzle::new(3).unwrap());
        session.commit(Puzzle::new(4).unwrap());
        app.manage(Mutex::new(session));

        let view = undo(app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.n, 3);
        assert_eq!(current_n(&app), 3);
    }

    #[test]
    fn redo_command_returns_view_after_undo() {
        let app = tauri::test::mock_app();
        let mut session = Session::new(Puzzle::new(3).unwrap());
        session.commit(Puzzle::new(4).unwrap());
        session.undo();
        app.manage(Mutex::new(session));

        let view = redo(app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.n, 4);
        assert_eq!(current_n(&app), 4);
    }

    #[test]
    fn new_puzzle_command_replaces_session_and_returns_view() {
        let app = tauri::test::mock_app();
        app.manage(Mutex::new(Session::new(Puzzle::new(2).unwrap())));

        let view = new_puzzle(5, app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.n, 5);
        assert_eq!(current_n(&app), 5);
    }

    #[test]
    fn get_state_command_returns_current_view() {
        let app = tauri::test::mock_app();
        app.manage(Mutex::new(Session::new(Puzzle::new(6).unwrap())));

        let view = get_state(app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.n, 6);
    }

    #[test]
    fn handle_menu_event_no_op_when_state_missing() {
        let app = tauri::test::mock_app();
        let event = MenuEvent {
            id: tauri::menu::MenuId::new("undo"),
        };
        handle_menu_event(app.handle(), event);
    }

    fn empty_session_app(n: usize) -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(Mutex::new(Session::new(Puzzle::new(n).unwrap())));
        app
    }

    #[test]
    fn insert_cage_command_commits_cage_and_pushes_undo() {
        let app = empty_session_app(4);
        let view = insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert_eq!(view.cages.len(), 1);

        let view_after_undo = undo(app.state::<Mutex<Session>>()).unwrap();
        assert!(view_after_undo.cages.is_empty());
    }

    #[test]
    fn insert_cage_command_returns_err_for_invalid_cells() {
        let app = empty_session_app(4);
        assert!(
            insert_cage(vec![], OpKind::Add, 3, app.state::<Mutex<Session>>()).is_err(),
            "empty cells should error"
        );
    }

    #[test]
    fn remove_cage_command_drops_cage_and_undo_restores() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let view = remove_cage((0, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(view.cages.is_empty());

        let view = undo(app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.cages.len(), 1);
    }

    #[test]
    fn remove_cage_command_returns_err_when_no_cage_at_anchor() {
        let app = empty_session_app(4);
        assert!(remove_cage((0, 0), app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn extend_cage_command_grows_cage_in_place_for_add() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let result = extend_cage((0, 0), (0, 2), app.state::<Mutex<Session>>()).unwrap();
        assert!(result.drafts.is_empty());
        assert_eq!(result.view.cages.len(), 1);
        assert_eq!(result.view.cages[0].cells.len(), 3);
    }

    #[test]
    fn extend_cage_command_returns_draft_when_op_invalid_for_new_size() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Sub,
            1,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let result = extend_cage((0, 0), (0, 2), app.state::<Mutex<Session>>()).unwrap();
        assert!(result.view.cages.is_empty());
        assert_eq!(result.drafts.len(), 1);
        assert_eq!(result.drafts[0].cells.len(), 3);
    }

    #[test]
    fn shrink_cage_command_removes_singleton() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0)],
            OpKind::Given,
            1,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let result = shrink_cage((0, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(result.view.cages.is_empty());
        assert!(result.drafts.is_empty());
    }

    #[test]
    fn shrink_cage_command_returns_err_for_uncovered_cell() {
        let app = empty_session_app(4);
        assert!(shrink_cage((0, 0), app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn set_cage_operation_command_replaces_op_and_target() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let view =
            set_cage_operation((0, 0), OpKind::Mul, 12, app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.cages.len(), 1);
        assert_eq!(view.cages[0].op, OpKind::Mul);
        assert_eq!(view.cages[0].target, 12);
    }

    #[test]
    fn set_cage_operation_command_returns_err_when_no_cage_at_anchor() {
        let app = empty_session_app(4);
        assert!(set_cage_operation((0, 0), OpKind::Add, 3, app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn merge_cages_command_combines_two_add_cages() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let result = merge_cages((0, 0), (1, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(result.drafts.is_empty());
        assert_eq!(result.view.cages.len(), 1);
        assert_eq!(result.view.cages[0].cells.len(), 4);
    }

    #[test]
    fn flip_cell_command_happy_path() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let result = flip_cell((0, 0), (1, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(result.drafts.is_empty());
        assert_eq!(result.view.cages.len(), 2);
    }

    #[test]
    fn clear_all_cages_returns_empty_puzzle() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(2, 0), (2, 1)],
            OpKind::Add,
            7,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let view = clear_all_cages(app.state::<Mutex<Session>>()).unwrap();
        assert!(view.cages.is_empty(), "all cages should be removed");
        let n = view.n;
        for row in &view.cells {
            for cell in row {
                assert_eq!(
                    cell.len(),
                    n,
                    "each cell should have full candidates after clear"
                );
            }
        }
    }

    #[test]
    fn clear_all_cages_undoes_to_prior_state() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let view_before_clear = app
            .state::<Mutex<Session>>()
            .lock()
            .unwrap()
            .current()
            .cages()
            .count();
        clear_all_cages(app.state::<Mutex<Session>>()).unwrap();

        let view_after_undo = undo(app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(
            view_after_undo.cages.len(),
            view_before_clear,
            "undo should restore all cages"
        );
    }

    #[test]
    fn clear_all_cages_on_empty_puzzle_is_noop() {
        let app = empty_session_app(4);
        let view = clear_all_cages(app.state::<Mutex<Session>>()).unwrap();
        assert!(view.cages.is_empty(), "no cages to clear");
        assert!(view.diff.is_empty(), "no change means empty diff");
    }

    #[test]
    fn clear_all_cages_creates_single_undo_entry() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(2, 0), (2, 1)],
            OpKind::Add,
            7,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(3, 0), (3, 1)],
            OpKind::Add,
            4,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(0, 2), (0, 3)],
            OpKind::Add,
            6,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let undo_before = app.state::<Mutex<Session>>().lock().unwrap().undo_count();
        clear_all_cages(app.state::<Mutex<Session>>()).unwrap();
        let undo_after = app.state::<Mutex<Session>>().lock().unwrap().undo_count();

        assert_eq!(
            undo_after,
            undo_before + 1,
            "clear_all_cages should push exactly one undo entry"
        );
    }

    #[test]
    fn move_cell_command_happy_path() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let result = move_cell((0, 0), (1, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(result.drafts.is_empty());
        assert_eq!(result.view.cages.len(), 2);
    }

    #[test]
    fn move_cell_command_returns_err_when_cell_not_caged() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert!(move_cell((0, 0), (1, 0), app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn flip_cell_command_returns_err_when_cell_not_caged() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert!(flip_cell((0, 0), (1, 0), app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn insert_cage_command_returns_diff_when_propagation_changes_fills() {
        let app = empty_session_app(3);
        // Insert a Given(1) at (0,0) — propagation removes 1 from row 0 peers.
        let view = insert_cage(
            vec![(0, 0)],
            OpKind::Given,
            1,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert!(
            !view.diff.is_empty(),
            "propagation should produce a non-empty diff"
        );
    }

    #[test]
    fn insert_cage_command_returns_empty_diff_when_idempotent() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        // Re-inserting the same cage shape with the same op is idempotent; diff should be empty.
        let view = insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert!(
            view.diff.is_empty(),
            "idempotent edit should produce an empty diff"
        );
    }

    #[test]
    fn undo_returns_diff_to_undone_state() {
        let app = empty_session_app(3);
        // Insert a Given(1) at (0,0) so propagation removes 1 from row peers.
        insert_cage(
            vec![(0, 0)],
            OpKind::Given,
            1,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        // Undo: the removed candidates come back as "added" in the diff.
        let view = undo(app.state::<Mutex<Session>>()).unwrap();
        assert!(
            !view.diff.is_empty(),
            "undo of a propagating edit should produce a non-empty diff"
        );
        let has_added = view.diff.changes.iter().any(|cd| !cd.added.is_empty());
        assert!(
            has_added,
            "undo diff should contain added candidates (values restored)"
        );
    }

    #[test]
    fn redo_returns_diff_to_redone_state() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0)],
            OpKind::Given,
            1,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        undo(app.state::<Mutex<Session>>()).unwrap();
        // Redo: propagation re-removes candidates; diff should contain removals.
        let view = redo(app.state::<Mutex<Session>>()).unwrap();
        assert!(
            !view.diff.is_empty(),
            "redo of a propagating edit should produce a non-empty diff"
        );
        let has_removed = view.diff.changes.iter().any(|cd| !cd.removed.is_empty());
        assert!(has_removed, "redo diff should contain removed candidates");
    }

    #[test]
    fn rank_active_cage_returns_ranked_tuples_for_caged_cell() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let ranked = rank_active_cage((0, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(!ranked.is_empty(), "should return at least one tuple");
        for rt in &ranked {
            assert_eq!(rt.tuple.len(), 2, "each tuple should match cage size of 2");
            assert_eq!(rt.view.n, 3, "narrowed view should have correct grid size");
        }
    }

    #[test]
    fn rank_active_cage_returns_err_when_no_cage_at_anchor() {
        let app = empty_session_app(3);
        assert!(rank_active_cage((0, 0), app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn rank_active_cage_tuples_are_sorted_by_reduction_descending() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let ranked = rank_active_cage((0, 0), app.state::<Mutex<Session>>()).unwrap();
        let reductions: Vec<usize> = ranked.iter().map(|rt| rt.total_reduction).collect();
        let mut sorted = reductions.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(
            reductions, sorted,
            "tuples should be sorted by reduction descending"
        );
    }

    #[test]
    fn apply_narrowing_pins_cage_cells_and_narrows() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let ranked = rank_active_cage((0, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(!ranked.is_empty());
        let first_tuple = ranked[0].tuple.clone();

        let view =
            apply_narrowing((0, 0), first_tuple.clone(), app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(view.n, 3);
        assert_eq!(
            view.cells[0][0],
            vec![u8::try_from(first_tuple[0]).unwrap()],
            "first cage cell should be pinned to tuple value"
        );
        assert_eq!(
            view.cells[0][1],
            vec![u8::try_from(first_tuple[1]).unwrap()],
            "second cage cell should be pinned to tuple value"
        );
    }

    #[test]
    fn apply_narrowing_returns_err_when_no_cage_at_anchor() {
        let app = empty_session_app(3);
        assert!(apply_narrowing((0, 0), vec![1, 2], app.state::<Mutex<Session>>()).is_err());
    }

    /// Issue #58: an L-shaped 6+ cage's corner cell shares both a row and a
    /// column with another cage cell, so the surviving tuples include
    /// (4,1,1) at the corner — the row peer and column peer don't share a
    /// row or column, so they're free to repeat the same value.
    #[test]
    fn main_grid_candidates_reflect_cage_all_different_for_l_shape() {
        let app = empty_session_app(4);
        let view = insert_cage(
            vec![(0, 0), (0, 1), (1, 0)],
            OpKind::Add,
            6,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert_eq!(view.cells[0][0], vec![1, 2, 3, 4]);
        assert_eq!(view.cells[0][1], vec![1, 2, 3]);
        assert_eq!(view.cells[1][0], vec![1, 2, 3]);
    }

    /// Issue #58: a straight 3-cell cage forces all three cells into the
    /// same row group, so every surviving 6+ tuple is a permutation of
    /// (1,2,3) and 4 is not a candidate anywhere in the cage.
    #[test]
    fn main_grid_candidates_reflect_cage_all_different_for_horizontal_triple() {
        let app = empty_session_app(4);
        let view = insert_cage(
            vec![(0, 0), (0, 1), (0, 2)],
            OpKind::Add,
            6,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert_eq!(view.cells[0][0], vec![1, 2, 3]);
        assert_eq!(view.cells[0][1], vec![1, 2, 3]);
        assert_eq!(view.cells[0][2], vec![1, 2, 3]);
    }

    #[test]
    fn apply_narrowing_returns_err_when_tuple_length_mismatches_cage() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        assert!(apply_narrowing((0, 0), vec![1], app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn apply_narrowing_returns_err_when_value_exceeds_u8() {
        let app = empty_session_app(3);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        let err = apply_narrowing((0, 0), vec![300, 1], app.state::<Mutex<Session>>()).unwrap_err();
        assert!(
            err.starts_with(ERR_VALUE_OUT_OF_RANGE),
            "expected error to start with {ERR_VALUE_OUT_OF_RANGE:?}, got {err:?}"
        );
    }

    #[test]
    fn legal_move_targets_command_returns_neighbor_anchors() {
        let app = empty_session_app(4);
        insert_cage(
            vec![(0, 0), (0, 1)],
            OpKind::Add,
            3,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();
        insert_cage(
            vec![(1, 0), (1, 1)],
            OpKind::Add,
            5,
            app.state::<Mutex<Session>>(),
        )
        .unwrap();

        let targets = legal_move_targets((0, 0), app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(targets, vec![(1, 0)]);
    }

    #[test]
    fn legal_move_targets_command_returns_empty_for_uncaged_cell() {
        let app = empty_session_app(4);
        let targets = legal_move_targets((0, 0), app.state::<Mutex<Session>>()).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn legal_move_targets_command_returns_err_when_lock_poisoned() {
        let app = empty_session_app(3);
        poison_lock(&app.state::<Mutex<Session>>());
        assert!(legal_move_targets((0, 0), app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn cage_options_command_returns_valid_operators_for_singleton() {
        let app = empty_session_app(4);
        let options = cage_options(vec![(0, 0)], app.state::<Mutex<Session>>()).unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].op, OpKind::Given);
        assert_eq!(options[0].targets, vec![1, 2, 3, 4]);
    }

    #[test]
    fn cage_options_command_returns_binary_operators_for_two_cells() {
        let app = empty_session_app(4);
        let options = cage_options(vec![(0, 0), (0, 1)], app.state::<Mutex<Session>>()).unwrap();
        let ops: Vec<OpKind> = options.iter().map(|o| o.op).collect();
        assert_eq!(
            ops,
            vec![OpKind::Add, OpKind::Sub, OpKind::Mul, OpKind::Div]
        );
    }

    #[test]
    fn cage_options_command_returns_err_when_lock_poisoned() {
        let app = empty_session_app(3);
        poison_lock(&app.state::<Mutex<Session>>());
        assert!(cage_options(vec![(0, 0)], app.state::<Mutex<Session>>()).is_err());
    }

    #[test]
    fn handle_menu_event_emits_clear_all_cages_without_dispatching() {
        // The clear_all_cages id branch returns before touching session state,
        // so the session committed below must remain at n=4 after the event.
        let app = empty_session_app(3);
        app.state::<Mutex<Session>>()
            .lock()
            .unwrap()
            .commit(Puzzle::new(4).unwrap());

        handle_menu_event(
            app.handle(),
            MenuEvent {
                id: tauri::menu::MenuId::new("clear_all_cages"),
            },
        );

        assert_eq!(current_n(&app), 4);
    }
}
