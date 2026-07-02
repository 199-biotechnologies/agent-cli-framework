//! Duplicate guard: prevent expensive or irreversible operations from
//! running twice concurrently (agent retries, two agents hitting the same
//! CLI). Lock file with PID + timestamp in the state directory; locks from
//! dead processes or older than one hour are treated as stale and overwritten.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Serialize, Deserialize)]
struct LockFile {
    pid: u32,
    started_at: String,
    operation: String,
}

const STALE_THRESHOLD_SECS: i64 = 3600; // 1 hour

pub struct DuplicateGuard {
    lock_path: PathBuf,
}

impl DuplicateGuard {
    pub fn new(data_dir: &std::path::Path, operation: &str) -> Self {
        let lock_dir = data_dir.join("locks");
        let _ = std::fs::create_dir_all(&lock_dir);
        Self {
            lock_path: lock_dir.join(format!("{operation}.lock")),
        }
    }

    /// Check whether the operation is already running. Returns Ok(()) when it
    /// is safe to proceed and writes a fresh lock.
    pub fn acquire(&self, force: bool) -> Result<(), AppError> {
        if let Ok(contents) = std::fs::read_to_string(&self.lock_path) {
            if let Ok(lock) = serde_json::from_str::<LockFile>(&contents) {
                let pid_alive = unsafe { libc::kill(lock.pid as i32, 0) == 0 };
                // Unparseable timestamps count as stale.
                let is_stale = chrono::DateTime::parse_from_rfc3339(&lock.started_at)
                    .map(|t| {
                        chrono::Utc::now().signed_duration_since(t).num_seconds()
                            > STALE_THRESHOLD_SECS
                    })
                    .unwrap_or(true);

                if pid_alive && !is_stale && !force {
                    return Err(AppError::InvalidInput(format!(
                        "Operation '{}' already running (pid {}). Use --force to override.",
                        lock.operation, lock.pid
                    )));
                }
            }
        }

        let lock = LockFile {
            pid: std::process::id(),
            started_at: chrono::Utc::now().to_rfc3339(),
            operation: self
                .lock_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        };
        let contents =
            serde_json::to_string(&lock).map_err(|e| AppError::Transient(e.to_string()))?;
        std::fs::write(&self.lock_path, contents)?;
        Ok(())
    }

    /// Release the lock. Also called automatically on Drop, so early returns
    /// and panics still clean up.
    pub fn release(&self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for DuplicateGuard {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_conflicts_and_force_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let first = DuplicateGuard::new(tmp.path(), "op");
        first.acquire(false).unwrap();

        let second = DuplicateGuard::new(tmp.path(), "op");
        let err = second.acquire(false).unwrap_err();
        assert_eq!(err.exit_code(), 3);

        second.acquire(true).unwrap();
    }

    #[test]
    fn lock_released_on_drop() {
        let tmp = tempfile::tempdir().unwrap();
        let lock_path = {
            let guard = DuplicateGuard::new(tmp.path(), "op");
            guard.acquire(false).unwrap();
            guard.lock_path.clone()
        };
        assert!(!lock_path.exists());
    }

    #[test]
    fn stale_lock_is_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let lock_dir = tmp.path().join("locks");
        std::fs::create_dir_all(&lock_dir).unwrap();
        let two_hours_ago = chrono::Utc::now() - chrono::Duration::hours(2);
        let stale = serde_json::json!({
            "pid": std::process::id(),
            "started_at": two_hours_ago.to_rfc3339(),
            "operation": "op",
        });
        std::fs::write(lock_dir.join("op.lock"), stale.to_string()).unwrap();

        let guard = DuplicateGuard::new(tmp.path(), "op");
        guard.acquire(false).unwrap();
    }
}
