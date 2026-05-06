mod cage_edit;
pub mod diff;
mod session;
mod view;

use std::sync::Mutex;

use kenken::Puzzle;
use tauri::menu::{
    AboutMetadata, IsMenuItem, Menu, MenuEvent, MenuItemBuilder, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use session::Session;
use view::{DraftCage, EditResult, OpKind, PuzzleView};

const PUZZLE_UPDATED_EVENT: &str = "puzzle-updated";

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

#[allow(clippy::too_many_lines)] // Per-OS submenu construction is the long part
fn build_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let pkg_info = app.package_info();
    let config = app.config();
    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };

    // Accelerators live in the WASM keydown handler (see src/app.rs), not on
    // the menu items, so Cmd/Ctrl+Z fires exactly once regardless of platform.
    let undo = MenuItemBuilder::with_id("undo", "Undo").build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "Redo").build(app)?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
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
            &PredefinedMenuItem::about(app, None, Some(about_metadata.clone()))?,
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
    let Some(state) = app.try_state::<Mutex<Session>>() else {
        return;
    };
    if let Some(view) = dispatch_menu_action(&state, event.id().as_ref()) {
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
    #[test]
    fn build_app_menu_includes_edit_submenu_with_custom_undo() {
        let app = tauri::test::mock_app();
        let menu = build_app_menu(app.handle()).unwrap();
        let items = menu.items().unwrap();
        assert!(!items.is_empty());

        let edit = items.iter().find_map(|i| match i {
            tauri::menu::MenuItemKind::Submenu(s) if s.text().ok().as_deref() == Some("Edit") => {
                Some(s.clone())
            }
            _ => None,
        });
        let edit_items = edit.unwrap().items().unwrap();
        let has_custom_undo = edit_items.iter().any(
            |i| matches!(i, tauri::menu::MenuItemKind::MenuItem(m) if m.id().as_ref() == "undo"),
        );
        assert!(has_custom_undo, "Edit submenu should contain custom Undo");
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
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.lock().unwrap();
            panic!("intentional");
        }));
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
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.lock().unwrap();
            panic!("intentional");
        }));
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
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.lock().unwrap();
            panic!("intentional");
        }));
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
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.lock().unwrap();
            panic!("intentional");
        }));
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
        insert_cage(vec![(0, 0)], OpKind::Add, 1, app.state::<Mutex<Session>>()).unwrap();
        // Re-inserting the same cage shape with the same op is idempotent; diff should be empty.
        let view =
            insert_cage(vec![(0, 0)], OpKind::Add, 1, app.state::<Mutex<Session>>()).unwrap();
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
}
