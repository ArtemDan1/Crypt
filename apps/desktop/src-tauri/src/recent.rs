use crypt_core::RecordId;
use std::{collections::VecDeque, sync::Mutex};

#[derive(Default)]
pub struct RecentSecrets(Mutex<VecDeque<RecordId>>);
impl RecentSecrets {
    pub fn touch(&self, id: RecordId) {
        let mut ids = self.0.lock().unwrap_or_else(|e| e.into_inner());
        ids.retain(|old| *old != id);
        ids.push_front(id);
        ids.truncate(20);
    }
    pub fn list(&self, limit: usize) -> Vec<RecordId> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .take(limit.min(20))
            .copied()
            .collect()
    }
    pub fn clear(&self) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypt_core::RecordId;
    #[test]
    fn recent_order_dedup_limit_and_clear() {
        let recent = RecentSecrets::default();
        let ids: Vec<_> = (0..25).map(|_| RecordId::new()).collect();
        for id in &ids[..3] {
            recent.touch(*id);
        }
        assert_eq!(recent.list(20), vec![ids[2], ids[1], ids[0]]);
        recent.touch(ids[0]);
        assert_eq!(recent.list(2), vec![ids[0], ids[2]]);
        for id in &ids {
            recent.touch(*id);
        }
        assert_eq!(recent.list(100).len(), 20);
        recent.clear();
        assert!(recent.list(20).is_empty());
    }
}
