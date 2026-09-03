//! Выполняет best-effort предварительное обновление MCP runtime.
//!
//! Ограниченный канал объединяет запросы refresh, а worker всегда готовит
//! последнее состояние thread. Согласование не ждёт pending startup; если
//! разделённый startup завершился ошибкой, worker запускает одну замену.

use super::*;

impl Session {
    pub(crate) fn request_mcp_runtime_refresh(&self) {
        self.mark_mcp_runtime_dirty();
        self.schedule_mcp_prewarm();
    }

    /// Запускает worker объединённых refresh и одной замены после общего `Failed`.
    pub(super) fn start_mcp_prewarm_worker(
        self: &Arc<Self>,
        requests: async_channel::Receiver<()>,
        mut auth_changes: tokio::sync::watch::Receiver<u64>,
    ) {
        let session = Arc::downgrade(self);
        let shutdown = self.mcp_prewarm_shutdown.clone();
        let worker = self.services.runtime_handle.spawn(async move {
            'worker: loop {
                let auth_changed = tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    request = requests.recv() => {
                        if request.is_err() {
                            break;
                        }
                        false
                    },
                    auth_change = auth_changes.changed() => {
                        if auth_change.is_err() {
                            break;
                        }
                        true
                    },
                };
                let Some(session) = session.upgrade() else {
                    break;
                };
                if auth_changed {
                    session.mark_mcp_runtime_dirty();
                }

                loop {
                    tokio::select! {
                        biased;
                        _ = shutdown.cancelled() => break 'worker,
                        _ = session.refresh_mcp_if_dirty() => {},
                    }
                    let reused_pending_startup_failed = tokio::select! {
                        biased;
                        _ = shutdown.cancelled() => break 'worker,
                        request = requests.recv() => {
                            if request.is_err() {
                                break 'worker;
                            }
                            continue;
                        },
                        auth_change = auth_changes.changed() => {
                            if auth_change.is_err() {
                                break 'worker;
                            }
                            session.mark_mcp_runtime_dirty();
                            continue;
                        },
                        failed = session
                            .services
                            .mcp_runtime
                            .wait_for_current_reused_pending_startup_failure() => failed,
                    };
                    if !reused_pending_startup_failed {
                        break;
                    }
                    session.mark_mcp_runtime_dirty();
                }
            }
        });
        *self
            .mcp_prewarm_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(worker);
    }

    pub(super) fn schedule_mcp_prewarm(&self) {
        let _ = self.mcp_prewarm_tx.try_send(());
    }

    pub(super) async fn stop_mcp_prewarm_worker(&self) {
        self.mcp_prewarm_shutdown.cancel();
        let worker = self
            .mcp_prewarm_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(worker) = worker
            && let Err(error) = worker.await
        {
            warn!(%error, "MCP prewarm worker stopped unexpectedly");
        }
    }
}
