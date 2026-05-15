#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_suffix() -> String {
    let nonce = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("{pid}_{nonce}")
}

/// Owns a unique path under `temp_dir()` for tests; removes the file (if any)
/// on drop so cleanup is panic-safe.
pub struct TmpPath(PathBuf);

impl TmpPath {
    /// A unique writable path. No file is created.
    pub fn new(stem: &str) -> Self {
        Self(std::env::temp_dir().join(format!("kenken_test_{stem}_{}.kenken", unique_suffix())))
    }

    /// A unique path nested inside a directory that does not exist, for
    /// exercising IO failure due to a missing parent.
    pub fn unwritable(stem: &str) -> Self {
        Self(
            std::env::temp_dir()
                .join(format!("kenken_test_missing_{stem}_{}", unique_suffix()))
                .join("nested")
                .join("missing.kenken"),
        )
    }

    pub fn as_str(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for TmpPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
