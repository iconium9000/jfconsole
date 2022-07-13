use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

type Inner = Arc<AtomicBool>;

pub struct SyncFlagVictim {
    inner: Inner,
}
pub struct SyncFlagAssassin {
    inner: Inner,
}

pub fn new_sync_flag() -> (SyncFlagVictim, SyncFlagAssassin) {
    let inner = Arc::new(AtomicBool::new(true));
    (
        SyncFlagVictim {
            inner: Arc::clone(&inner),
        },
        SyncFlagAssassin { inner },
    )
}

impl Clone for SyncFlagAssassin {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl SyncFlagVictim {
    pub fn is_alive(&self) -> bool {
        self.inner.load(Ordering::Relaxed)
    }
}

impl SyncFlagAssassin {
    pub fn kill_victim(self) {
        self.inner.store(false, Ordering::Relaxed);
    }
}

impl Drop for SyncFlagAssassin {
    fn drop(&mut self) {
        self.inner.store(false, Ordering::Relaxed);
    }
}
