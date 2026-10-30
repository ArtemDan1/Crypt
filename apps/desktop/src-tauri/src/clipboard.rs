#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copy_writes_and_clears_after_timeout() {
        let cb = Arc::new(MemoryClipboard::default());
        copy_with_clear(cb.clone(), "secret", Duration::from_millis(20));
        assert_eq!(cb.read(), "secret");
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !cb.read().is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(cb.read(), "");
    }
    #[test]
    fn clear_skips_when_clipboard_changed() {
        let cb = Arc::new(MemoryClipboard::default());
        copy_with_clear(cb.clone(), "secret", Duration::from_millis(20));
        cb.write_concealed("mine");
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(cb.read(), "mine");
    }
    #[test]
    fn newer_copy_owns_its_timeout() {
        let cb = Arc::new(MemoryClipboard::default());
        copy_with_clear(cb.clone(), "first", Duration::from_millis(20));
        copy_with_clear(cb.clone(), "second", Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(cb.read(), "second");
    }
}

use std::sync::{Arc, Mutex};
use std::time::Duration;
pub trait Clipboard: Send + Sync {
    fn write_concealed(&self, text: &str) -> u64;
    fn change_count(&self) -> u64;
    fn clear(&self);
}
#[derive(Default)]
pub struct MemoryClipboard(Mutex<(u64, String)>);
impl MemoryClipboard {
    pub fn read(&self) -> String {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).1.clone()
    }
}
impl Clipboard for MemoryClipboard {
    fn write_concealed(&self, text: &str) -> u64 {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.0 += 1;
        state.1 = text.to_owned();
        state.0
    }
    fn change_count(&self) -> u64 {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).0
    }
    fn clear(&self) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.0 += 1;
        state.1.clear();
    }
}
pub fn copy_with_clear(cb: Arc<dyn Clipboard>, text: &str, after: Duration) {
    let token = cb.write_concealed(text);
    std::thread::spawn(move || {
        std::thread::sleep(after);
        if cb.change_count() == token {
            cb.clear();
        }
    });
}
#[cfg(target_os = "macos")]
pub struct MacClipboard;
#[cfg(target_os = "macos")]
impl Clipboard for MacClipboard {
    fn write_concealed(&self, text: &str) -> u64 {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        use objc2_foundation::NSString;
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let text = NSString::from_str(text);
        unsafe {
            pb.setString_forType(&text, NSPasteboardTypeString);
        }
        pb.setString_forType(
            &NSString::from_str(""),
            &NSString::from_str("org.nspasteboard.ConcealedType"),
        );
        pb.changeCount() as u64
    }
    fn change_count(&self) -> u64 {
        objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as u64
    }
    fn clear(&self) {
        objc2_app_kit::NSPasteboard::generalPasteboard().clearContents();
    }
}
