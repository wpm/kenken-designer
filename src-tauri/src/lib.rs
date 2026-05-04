mod session;
mod view;

use std::sync::Mutex;

use kenken::{generate, Puzzle};
use tauri::menu::{
    AboutMetadata, IsMenuItem, Menu, MenuEvent, MenuItemBuilder, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use session::Session;
use view::PuzzleView;

const PUZZLE_UPDATED_EVENT: &str = "puzzle-updated";

fn fresh_puzzle(n: usize) -> Result<Puzzle, String> {
    let mut rng = rand::rng();
    generate(n, &mut rng).map_err(|e| format!("{e:?}"))
}

fn commit_new_puzzle(state: &Mutex<Session>, n: usize) -> Result<PuzzleView, String> {
    let next = fresh_puzzle(n)?;
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.commit(next);
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
    session.undo();
    Ok(PuzzleView::from(session.current()))
}

fn apply_redo(state: &Mutex<Session>) -> Result<PuzzleView, String> {
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.redo();
    Ok(PuzzleView::from(session.current()))
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
    let initial = fresh_puzzle(4).expect("4 is a valid grid size");
    let session = Session::new(initial);
    tauri::Builder::default()
        .manage(Mutex::new(session))
        .menu(build_app_menu)
        .on_menu_event(handle_menu_event)
        .invoke_handler(tauri::generate_handler![new_puzzle, get_state, undo, redo])
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
    fn fresh_puzzle_returns_ok_for_valid_size() {
        let p = fresh_puzzle(4).unwrap();
        assert_eq!(p.n(), 4);
    }

    #[test]
    fn fresh_puzzle_supports_minimum_size() {
        let p = fresh_puzzle(2).unwrap();
        assert_eq!(p.n(), 2);
    }

    #[test]
    fn fresh_puzzle_returns_err_for_invalid_size() {
        assert!(fresh_puzzle(0).is_err());
        assert!(fresh_puzzle(99).is_err());
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
    fn commit_new_puzzle_replaces_session_current() {
        let state = Mutex::new(Session::new(Puzzle::new(2).unwrap()));
        let view = commit_new_puzzle(&state, 4).unwrap();
        assert_eq!(view.n, 4);
        let session = state.lock().unwrap();
        assert_eq!(session.current().n(), 4);
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
}
