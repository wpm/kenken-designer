mod session;
mod view;

use std::sync::Mutex;

use kenken::Puzzle;
use tauri::State;

use session::Session;
use view::PuzzleView;

#[tauri::command]
fn new_puzzle(n: usize, state: State<Mutex<Session>>) -> Result<PuzzleView, String> {
    let next = Puzzle::new(n).map_err(|e| format!("{e:?}"))?;
    let mut session = state.lock().map_err(|e| format!("{e:?}"))?;
    session.commit(next);
    Ok(PuzzleView::from(session.current()))
}

#[tauri::command]
fn get_state(state: State<Mutex<Session>>) -> PuzzleView {
    let session = state.lock().expect("session mutex poisoned");
    PuzzleView::from(session.current())
}

#[tauri::command]
fn undo(state: State<Mutex<Session>>) -> PuzzleView {
    let mut session = state.lock().expect("session mutex poisoned");
    session.undo();
    PuzzleView::from(session.current())
}

#[tauri::command]
fn redo(state: State<Mutex<Session>>) -> PuzzleView {
    let mut session = state.lock().expect("session mutex poisoned");
    session.redo();
    PuzzleView::from(session.current())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let session = Session::new(4).expect("4 is a valid grid size");
    tauri::Builder::default()
        .manage(Mutex::new(session))
        .invoke_handler(tauri::generate_handler![new_puzzle, get_state, undo, redo])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
