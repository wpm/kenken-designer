mod session;
mod view;

use std::sync::Mutex;

use kenken::{generate, Puzzle};
use tauri::State;

use session::Session;
use view::PuzzleView;

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
    let initial = fresh_puzzle(4).expect("4 is a valid grid size");
    let session = Session::new(initial);
    tauri::Builder::default()
        .manage(Mutex::new(session))
        .invoke_handler(tauri::generate_handler![new_puzzle, get_state, undo, redo])
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
