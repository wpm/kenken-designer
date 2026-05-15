use crate::theme::{BG, INK, INK2, LINE, SANS_FONT};
use leptos::prelude::*;

/// A single toast entry: unique id + message text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub message: String,
}

/// Shared signal holding the list of active toasts.
pub type Toasts = RwSignal<Vec<Toast>>;

/// Append an error message to the toast list, ignoring duplicates.
pub fn push_error(toasts: Toasts, message: String) {
    toasts.update(|v| {
        if v.iter().any(|t| t.message == message) {
            return;
        }
        let id = next_id(v);
        v.push(Toast { id, message });
    });
}

fn next_id(v: &[Toast]) -> u64 {
    v.iter().map(|t| t.id).max().map_or(0, |m| m + 1)
}

/// Dismiss a toast by id.
pub fn dismiss(toasts: Toasts, id: u64) {
    toasts.update(|v| v.retain(|t| t.id != id));
}

#[component]
pub fn ToastStack(toasts: Toasts) -> impl IntoView {
    let container_style = "position:fixed;bottom:1.5rem;right:1.5rem;z-index:9000;\
         display:flex;flex-direction:column;align-items:flex-end;gap:0.6rem;\
         pointer-events:none;";

    view! {
        <div style=container_style>
            {move || {
                toasts.get().into_iter().map(|toast| {
                    let id = toast.id;
                    view! { <ToastItem message=toast.message on_dismiss=move || dismiss(toasts, id) /> }
                }).collect_view()
            }}
        </div>
    }
}

#[component]
fn ToastItem(message: String, on_dismiss: impl Fn() + 'static) -> impl IntoView {
    let item_style = format!(
        "pointer-events:auto;\
         background:{BG};border:1px solid {LINE};border-left:4px solid #c0392b;\
         border-radius:6px;box-shadow:0 2px 12px rgba(0,0,0,0.18);\
         padding:0.6rem 0.75rem;max-width:360px;\
         display:flex;align-items:flex-start;gap:0.6rem;\
         font-family:{SANS_FONT};font-size:13.5px;color:{INK};"
    );

    let msg_style = format!("flex:1;color:{INK2};line-height:1.4;");

    let btn_style = format!(
        "flex-shrink:0;background:none;border:none;padding:0;\
         cursor:pointer;font-size:1rem;line-height:1;color:{INK2};\
         opacity:0.6;font-family:{SANS_FONT};"
    );

    view! {
        <div style=item_style>
            <span style=msg_style>{message}</span>
            <button
                style=btn_style
                aria-label="Dismiss"
                on:click=move |_| on_dismiss()
            >
                "\u{00D7}"
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_toasts(messages: &[&str]) -> Vec<Toast> {
        messages
            .iter()
            .enumerate()
            .map(|(i, m)| Toast {
                id: i as u64,
                message: m.to_string(),
            })
            .collect()
    }

    #[test]
    fn next_id_empty() {
        assert_eq!(next_id(&[]), 0);
    }

    #[test]
    fn next_id_increments_past_max() {
        let v = make_toasts(&["a", "b", "c"]);
        assert_eq!(next_id(&v), 3);
    }

    #[test]
    fn next_id_gap_in_ids() {
        let v = vec![
            Toast {
                id: 5,
                message: "x".into(),
            },
            Toast {
                id: 2,
                message: "y".into(),
            },
        ];
        assert_eq!(next_id(&v), 6);
    }

    #[test]
    fn dismiss_removes_by_id() {
        let mut v = make_toasts(&["a", "b", "c"]);
        v.retain(|t| t.id != 1);
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|t| t.id != 1));
    }

    #[test]
    fn dismiss_unknown_id_is_noop() {
        let mut v = make_toasts(&["a", "b"]);
        v.retain(|t| t.id != 99);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn dismiss_last_item_empties_list() {
        let mut v = make_toasts(&["only"]);
        v.retain(|t| t.id != 0);
        assert!(v.is_empty());
    }
}
