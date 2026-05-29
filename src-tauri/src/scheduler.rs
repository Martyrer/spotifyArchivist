use std::sync::Arc;
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tokio::sync::{mpsc, Notify};

use crate::commands::AppState;
use crate::sync::{SyncOutcome, Syncer};

#[derive(Debug)]
pub enum Tick {
    Trigger,
    Reschedule,
    Shutdown,
}

pub struct SchedulerHandle {
    pub tx: mpsc::Sender<Tick>,
    join: Option<JoinHandle<()>>,
}

impl SchedulerHandle {
    pub async fn shutdown(mut self) {
        let _ = self.tx.send(Tick::Shutdown).await;
        if let Some(j) = self.join.take() {
            let _ = j.await;
        }
    }

    /// Detach the join handle for parking on `AppHandle`. Replaces the
    /// previous `mem::forget(sched)` hack — the loop still runs forever, but
    /// the handle is owned cleanly so a future shutdown hook can `.await` it.
    pub fn into_join_handle(mut self) -> Option<JoinHandle<()>> {
        self.join.take()
    }
}

pub trait OnSyncDone: Send + Sync {
    fn handle(&self, outcomes: Vec<SyncOutcome>);
}

pub fn spawn(state: Arc<AppState>, on_done: Arc<dyn OnSyncDone>) -> SchedulerHandle {
    let (tx, mut rx) = mpsc::channel::<Tick>(8);
    let trigger_tx = tx.clone();
    let initial = Arc::new(Notify::new());
    let initial_clone = initial.clone();

    let join = tauri::async_runtime::spawn(async move {
        initial_clone.notify_one();

        let mut interval_secs = read_interval_secs(&state).await;
        let mut next_at = tokio::time::Instant::now() + Duration::from_secs(interval_secs);

        // Kick off an initial run on startup.
        let _ = trigger_tx.try_send(Tick::Trigger);

        loop {
            let sleep_until = tokio::time::sleep_until(next_at);
            tokio::pin!(sleep_until);
            tokio::select! {
                _ = &mut sleep_until => {
                    run_once(&state, on_done.as_ref()).await;
                    next_at = tokio::time::Instant::now() + Duration::from_secs(interval_secs);
                }
                msg = rx.recv() => {
                    match msg {
                        Some(Tick::Trigger) => {
                            run_once(&state, on_done.as_ref()).await;
                            next_at = tokio::time::Instant::now() + Duration::from_secs(interval_secs);
                        }
                        Some(Tick::Reschedule) => {
                            interval_secs = read_interval_secs(&state).await;
                            next_at = tokio::time::Instant::now() + Duration::from_secs(interval_secs);
                        }
                        Some(Tick::Shutdown) | None => break,
                    }
                }
            }
        }
    });

    SchedulerHandle {
        tx,
        join: Some(join),
    }
}

async fn read_interval_secs(state: &AppState) -> u64 {
    let hours = state.store.sync_interval_hours().await.unwrap_or(6) as u64;
    hours.saturating_mul(3600).max(60)
}

async fn run_once(state: &AppState, on_done: &dyn OnSyncDone) {
    let syncer = Syncer::new(
        state.store.clone(),
        state.spotify.clone(),
        state.clock.clone(),
    );
    let sources = match state.store.list_sources().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(?e, "scheduler: list_sources failed");
            return;
        }
    };
    let outcomes = syncer
        .sync_all(&sources)
        .await
        .into_iter()
        .filter_map(|r| {
            r.map_err(|e| tracing::warn!(?e, "scheduler: sync failed"))
                .ok()
        })
        .collect();
    on_done.handle(outcomes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::TokenStore;
    use crate::store::Store;
    use std::sync::Mutex;

    struct CaptureSink(Mutex<Vec<usize>>);

    impl OnSyncDone for CaptureSink {
        fn handle(&self, outcomes: Vec<SyncOutcome>) {
            self.0.lock().unwrap().push(outcomes.len());
        }
    }

    async fn fixture() -> Arc<AppState> {
        let store = Store::open_in_memory().await.unwrap();
        let tokens = TokenStore::memory();
        Arc::new(AppState::new(store, tokens, "CID", std::env::temp_dir()))
    }

    #[tokio::test]
    async fn initial_trigger_runs_once_with_no_sources() {
        let state = fixture().await;
        let sink = Arc::new(CaptureSink(Mutex::new(Vec::new())));
        let handle = spawn(state, sink.clone());
        tokio::time::sleep(Duration::from_millis(80)).await;
        handle.shutdown().await;
        let runs = sink.0.lock().unwrap().clone();
        assert!(!runs.is_empty(), "scheduler should run at least once");
        assert!(runs.iter().all(|n| *n == 0));
    }

    #[tokio::test]
    async fn manual_trigger_runs_immediately() {
        let state = fixture().await;
        let sink = Arc::new(CaptureSink(Mutex::new(Vec::new())));
        let handle = spawn(state, sink.clone());
        tokio::time::sleep(Duration::from_millis(40)).await;
        let initial = sink.0.lock().unwrap().len();
        handle.tx.send(Tick::Trigger).await.unwrap();
        tokio::time::sleep(Duration::from_millis(40)).await;
        let after = sink.0.lock().unwrap().len();
        assert!(after > initial, "manual trigger should add a run");
        handle.shutdown().await;
    }

    #[tokio::test]
    async fn reschedule_message_does_not_panic() {
        let state = fixture().await;
        let sink = Arc::new(CaptureSink(Mutex::new(Vec::new())));
        let handle = spawn(state, sink.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        handle.tx.send(Tick::Reschedule).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        handle.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_stops_the_task() {
        let state = fixture().await;
        let sink = Arc::new(CaptureSink(Mutex::new(Vec::new())));
        let handle = spawn(state, sink.clone());
        handle.shutdown().await;
    }

    /// Tauri's `setup()` callback runs on the main thread before the Tauri
    /// async runtime is started; it is *not* inside a tokio runtime context.
    /// `spawn()` must therefore use `tauri::async_runtime::spawn`, which
    /// owns its own reactor, rather than bare `tokio::spawn`. This test
    /// drives `spawn()` from a thread with no runtime to lock that in.
    #[test]
    fn spawn_runs_outside_a_tokio_runtime() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let state = tauri::async_runtime::block_on(fixture());
            let sink = Arc::new(CaptureSink(Mutex::new(Vec::new())));
            let handle = spawn(state, sink);
            tauri::async_runtime::block_on(handle.shutdown());
            tx.send(()).unwrap();
        })
        .join()
        .unwrap();
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}
