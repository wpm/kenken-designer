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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn new_puzzle(n: usize, state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    let next = fresh_puzzle(n)?;
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.commit(next);
    Ok(PuzzleView::from(session.current()))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn get_state(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    let session = state.lock().map_err(|e| format!("{e:?}"))?;
    Ok(PuzzleView::from(session.current()))
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

    let undo = MenuItemBuilder::with_id("undo", "Undo")
        .accelerator("CmdOrCtrl+Z")
        .build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "Redo")
        .accelerator("CmdOrCtrl+Shift+Z")
        .build(app)?;
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
        #[cfg(target_os = "macos")]
        let items: Vec<&dyn IsMenuItem<R>> = vec![&PredefinedMenuItem::close_window(app, None)?];
        #[cfg(not(target_os = "macos"))]
        let close = PredefinedMenuItem::close_window(app, None)?;
        #[cfg(not(target_os = "macos"))]
        let quit = PredefinedMenuItem::quit(app, None)?;
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

#[allow(clippy::needless_pass_by_value)] // on_menu_event requires by-value MenuEvent
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let Some(state) = app.try_state::<Mutex<Session>>() else {
        return;
    };
    let Ok(mut session) = state.lock() else {
        return;
    };
    if apply_menu_action(&mut session, event.id().as_ref()) {
        let view = PuzzleView::from(session.current());
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
        .invoke_handler(tauri::generate_handler![new_puzzle, get_state])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
    fn handle_menu_event_no_op_when_state_missing() {
        let app = tauri::test::mock_app();
        let event = MenuEvent {
            id: tauri::menu::MenuId::new("undo"),
        };
        handle_menu_event(app.handle(), event);
    }
}
