mod session;
mod view;

use std::sync::Mutex;

use kenken::{generate, Puzzle};
use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
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

fn build_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let undo = MenuItemBuilder::with_id("undo", "Undo")
        .accelerator("CmdOrCtrl+Z")
        .build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "Redo")
        .accelerator("CmdOrCtrl+Shift+Z")
        .build(app)?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&undo)
        .item(&redo)
        .build()?;
    MenuBuilder::new(app).item(&edit).build()
}

#[allow(clippy::needless_pass_by_value)] // on_menu_event requires by-value MenuEvent
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let Some(state) = app.try_state::<Mutex<Session>>() else {
        return;
    };
    let Ok(mut session) = state.lock() else {
        return;
    };
    let id: &str = event.id().as_ref();
    match id {
        "undo" => {
            session.undo();
        }
        "redo" => {
            session.redo();
        }
        _ => return,
    }
    let view = PuzzleView::from(session.current());
    let _ = app.emit(PUZZLE_UPDATED_EVENT, view);
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
    fn fresh_puzzle_returns_err_for_invalid_size() {
        assert!(fresh_puzzle(0).is_err());
        assert!(fresh_puzzle(99).is_err());
    }
}
