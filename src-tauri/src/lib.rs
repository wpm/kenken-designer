mod session;
mod view;

use std::sync::Mutex;

use kenken::Puzzle;
use tauri::State;

use session::Session;
use view::PuzzleView;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn new_puzzle(n: usize, state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    let next = Puzzle::new(n).map_err(|e| format!("{e:?}"))?;
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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn undo(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.undo();
    Ok(PuzzleView::from(session.current()))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri requires State to be passed by value
fn redo(state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.redo();
    Ok(PuzzleView::from(session.current()))
}

/// # Panics
///
/// Panics if the Tauri application fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    let session = Session::new(4).expect("4 is a valid grid size");
    tauri::Builder::default()
        .manage(Mutex::new(session))
        .invoke_handler(tauri::generate_handler![new_puzzle, get_state, undo, redo])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
