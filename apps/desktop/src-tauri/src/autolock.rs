use crate::commands::lifecycle::{VaultStatus, lock, status};
use crate::state::AppState;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const FLUSH_TIMEOUT: Duration = Duration::from_millis(500);

/// Ask the UI to save pending edits, wait for it (bounded), then drop the key.
pub fn graceful_lock(st: &AppState, notify: impl FnOnce(), wait: &Receiver<()>, timeout: Duration) {
    if !matches!(status(st), VaultStatus::Unlocked { .. }) {
        return;
    }
    notify();
    let _ = wait.recv_timeout(timeout);
    lock(st);
}

/// All UI/system locks ask the main editor to save, then drop the shared key.
pub fn lock_with_flush(app: &AppHandle) -> VaultStatus {
    let st = app.state::<std::sync::Arc<AppState>>();
    let _serial = st.lock_requests.lock().unwrap_or_else(|e| e.into_inner());
    let (tx, rx) = mpsc::channel();
    let token = format!("{:032x}", rand::random::<u128>());
    *st.flush_waiter.lock().unwrap_or_else(|e| e.into_inner()) = Some((token.clone(), tx));
    graceful_lock(
        &st,
        || {
            let _ = app.emit_to("main", "vault-will-lock", token);
        },
        &rx,
        FLUSH_TIMEOUT,
    );
    st.flush_waiter
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    status(&st)
}
pub fn lock_from_system(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        lock_with_flush(&app);
    });
}
pub fn acknowledge_flush(st: &AppState, label: &str, token: &str) {
    if label != "main" {
        return;
    }
    let mut waiter = st.flush_waiter.lock().unwrap_or_else(|e| e.into_inner());
    if waiter
        .as_ref()
        .is_some_and(|(expected, _)| expected == token)
    {
        if let Some((_, tx)) = waiter.take() {
            let _ = tx.send(());
        }
    }
}
#[tauri::command]
pub fn flush_done(
    window: tauri::WebviewWindow,
    st: tauri::State<'_, std::sync::Arc<AppState>>,
    token: String,
) {
    acknowledge_flush(&st, window.label(), &token);
}

#[cfg(target_os = "macos")]
pub fn observe_system(app: &AppHandle) {
    use block2::RcBlock;
    use objc2_app_kit::{NSWorkspace, NSWorkspaceWillSleepNotification};
    use objc2_foundation::{NSDistributedNotificationCenter, NSNotification, NSString};
    use std::ptr::NonNull;

    let a = app.clone();
    let on_event = RcBlock::new(move |_n: NonNull<NSNotification>| lock_from_system(&a));
    unsafe {
        let ws = NSWorkspace::sharedWorkspace().notificationCenter();
        let t1 = ws.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceWillSleepNotification),
            None,
            None,
            &on_event,
        );
        let dist = NSDistributedNotificationCenter::defaultCenter();
        let name = NSString::from_str("com.apple.screenIsLocked");
        let t2 =
            dist.addObserverForName_object_queue_usingBlock(Some(&name), None, None, &on_event);
        // Observers live for the whole app run.
        std::mem::forget(t1);
        std::mem::forget(t2);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn observe_system(_app: &AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::lifecycle::{VaultStatus, create_vault, generate_seed, status};
    use crate::state::AppState;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(
            dir.path().join("vault"),
            dir.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        create_vault(&st, generate_seed()).unwrap();
        (dir, st)
    }

    #[test]
    fn lock_sequence_waits_for_flush_with_timeout() {
        // Frontend answers: lock right after the answer, well before the timeout.
        let (_d, st) = state();
        let (tx, rx) = mpsc::channel();
        let t0 = Instant::now();
        graceful_lock(&st, || tx.send(()).unwrap(), &rx, Duration::from_secs(5));
        assert!(t0.elapsed() < Duration::from_secs(1));
        assert!(matches!(status(&st), VaultStatus::Locked { .. }));

        // Frontend silent: lock anyway after the timeout.
        let (_d2, st2) = state();
        let (_tx2, rx2) = mpsc::channel::<()>();
        let t0 = Instant::now();
        graceful_lock(&st2, || {}, &rx2, Duration::from_millis(200));
        assert!(t0.elapsed() >= Duration::from_millis(200));
        assert!(matches!(status(&st2), VaultStatus::Locked { .. }));
    }

    #[test]
    fn graceful_lock_is_noop_when_already_locked() {
        let (_d, st) = state();
        crate::commands::lifecycle::lock(&st);
        let (_tx, rx) = mpsc::channel::<()>();
        let mut notified = false;
        graceful_lock(&st, || notified = true, &rx, Duration::from_millis(50));
        assert!(!notified);
    }
}

#[cfg(test)]
mod acknowledgement_tests {
    use super::*;
    #[test]
    fn tray_cannot_acknowledge_main_editor_flush() {
        let d = tempfile::tempdir().unwrap();
        let st = AppState::new(
            d.path().join("vault"),
            d.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        let (tx, rx) = mpsc::channel();
        *st.flush_waiter.lock().unwrap() = Some(("current".into(), tx));
        acknowledge_flush(&st, "tray", "current");
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(st.flush_waiter.lock().unwrap().is_some());
        acknowledge_flush(&st, "main", "old");
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        acknowledge_flush(&st, "main", "current");
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
    }
}
