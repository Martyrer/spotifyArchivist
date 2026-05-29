use std::sync::Arc;

use tauri::{Emitter, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::commands::AppState;
use crate::notify;
use crate::scheduler;
use crate::sync::SyncOutcome;

pub fn update_tray_badge<R: Runtime>(handle: &tauri::AppHandle<R>, unseen: u32) {
    if let Some(tray) = handle.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(if unseen == 0 {
            "Spotify Archivist".to_string()
        } else {
            format!("Spotify Archivist — {unseen} unseen loss(es)")
        }));
    }
}

/// Apply post-sync side-effects: emit `sync:completed`, coalesce losses into a
/// toast, increment unseen-loss counter, emit `losses:updated`, refresh tray.
/// Single source of truth for both the manual `trigger_sync` command and the
/// scheduler's background runs.
pub async fn dispatch_post_sync<R: Runtime>(
    handle: &tauri::AppHandle<R>,
    state: &AppState,
    outcomes: &[SyncOutcome],
) {
    let _ = handle.emit("sync:completed", outcomes.len());
    let Some(summary) = notify::summarize(outcomes) else {
        return;
    };
    let total_lost = summary.total_lost as u32;
    let new_total = state
        .store
        .add_unseen_losses(total_lost)
        .await
        .unwrap_or(total_lost);
    let (title, body) = notify::toast_text(&summary);
    let _ = handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
    let _ = handle.emit("losses:updated", new_total);
    update_tray_badge(handle, new_total);
}

pub struct ToastSink<R: Runtime> {
    pub handle: tauri::AppHandle<R>,
    pub state: Arc<AppState>,
}

impl<R: Runtime> scheduler::OnSyncDone for ToastSink<R> {
    fn on_start(&self) {
        let _ = self.handle.emit("sync:started", ());
    }

    fn handle(&self, outcomes: Vec<SyncOutcome>) {
        let handle = self.handle.clone();
        let state = self.state.clone();
        tauri::async_runtime::spawn(async move {
            dispatch_post_sync(&handle, &state, &outcomes).await;
        });
    }
}
