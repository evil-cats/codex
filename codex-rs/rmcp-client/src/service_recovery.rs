//! Восстанавливает уже инициализированный MCP transport после отказа сервиса.
//!
//! Для stdio восстановление выполняется ограниченной последовательной серией под
//! общим `session_recovery_lock`. HTTP сохраняет собственную политику повторов
//! инициализации, а замена `RunningService` происходит только после успешного
//! рукопожатия.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use rmcp::service::RoleClient;
use rmcp::service::RunningService;
use tokio::time;
use tracing::warn;

use super::ClientState;
use super::ElicitationClientService;
use super::InitializeContext;
use super::OAuthPersistor;
use super::RmcpClient;
use super::StdioServerProcessHandle;
use super::TransportRecipe;

const STDIO_RECOVERY_RETRY_DELAYS_MS: [u64; 4] = [250, 500, 1_000, 2_000];

/// Полностью инициализированный сервис и связанные с ним runtime-ресурсы.
struct RecoveredService {
    service: Arc<RunningService<RoleClient, ElicitationClientService>>,
    oauth_persistor: Option<OAuthPersistor>,
    stdio_process: Option<StdioServerProcessHandle>,
}

/// Разделяет ошибки создания транспорта и рукопожатия для fail-fast политики.
enum RecoveryAttemptError {
    TransportCreation(Error),
    Initialize(Error),
}

impl RecoveryAttemptError {
    /// Постоянные ошибки создания процесса не должны задерживать внешний вызов.
    fn is_permanent_transport_creation(&self) -> bool {
        let Self::TransportCreation(error) = self else {
            return false;
        };
        error.downcast_ref::<io::Error>().is_some_and(|error| {
            matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            )
        })
    }

    /// Возвращает исходную ошибку без потери её цепочки причин.
    fn into_inner(self) -> Error {
        match self {
            Self::TransportCreation(error) | Self::Initialize(error) => error,
        }
    }
}

impl RmcpClient {
    /// Сериализованно заменяет отказавший сервис, если другой вызов ещё не успел
    /// восстановить тот же `RunningService`.
    pub(super) async fn reinitialize_after_service_failure(
        &self,
        failed_service: &Arc<RunningService<RoleClient, ElicitationClientService>>,
    ) -> Result<()> {
        let _recovery_guard = self
            .session_recovery_lock
            .acquire()
            .await
            .map_err(|_| anyhow!("MCP client recovery semaphore closed"))?;

        {
            let guard = self.state.lock().await;
            match &*guard {
                ClientState::Ready { service, .. } if !Arc::ptr_eq(service, failed_service) => {
                    return Ok(());
                }
                ClientState::Ready { .. } => {}
                ClientState::Connecting { .. } => {
                    return Err(anyhow!("MCP client not initialized"));
                }
                ClientState::Closed => {
                    return Err(anyhow!("MCP client is shut down"));
                }
            }
        }

        let initialize_context = self
            .initialize_context
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow!("MCP client cannot recover before initialize succeeds"))?;
        let recovered = if matches!(&self.transport_recipe, TransportRecipe::Stdio { .. }) {
            self.recover_stdio_service(&initialize_context).await?
        } else {
            self.connect_recovery_attempt(&initialize_context)
                .await
                .map_err(RecoveryAttemptError::into_inner)?
        };
        let RecoveredService {
            service,
            oauth_persistor,
            stdio_process,
        } = recovered;

        {
            // Оба `MutexGuard` получаются одной точкой ожидания: читатель не
            // должен увидеть новый сервис со старым `StdioServerProcessHandle`
            // или наоборот.
            let (mut state, mut current_stdio_process) =
                tokio::join!(self.state.lock(), self.stdio_process.lock());
            if matches!(*state, ClientState::Closed) {
                return Err(anyhow!("MCP client is shut down"));
            }
            *state = ClientState::Ready {
                service,
                oauth: oauth_persistor.clone(),
            };
            *current_stdio_process = stdio_process;
        }

        if let Some(runtime) = oauth_persistor
            && let Err(error) = runtime.persist_if_needed().await
        {
            warn!("failed to persist OAuth tokens after session recovery: {error}");
        }

        Ok(())
    }

    /// Выполняет до пяти последовательных stdio-запусков; после исчерпания
    /// возвращает ошибку последней попытки.
    async fn recover_stdio_service(
        &self,
        initialize_context: &InitializeContext,
    ) -> Result<RecoveredService> {
        for retry_delay_ms in STDIO_RECOVERY_RETRY_DELAYS_MS
            .iter()
            .copied()
            .map(Some)
            .chain(std::iter::once(None))
        {
            match self.connect_recovery_attempt(initialize_context).await {
                Ok(recovered) => return Ok(recovered),
                Err(error) if error.is_permanent_transport_creation() => {
                    return Err(error.into_inner());
                }
                Err(error) => {
                    let Some(retry_delay_ms) = retry_delay_ms else {
                        return Err(error.into_inner());
                    };
                    time::sleep(Duration::from_millis(retry_delay_ms)).await;
                }
            }
        }

        unreachable!("stdio recovery loop should return on success or final error")
    }

    /// Создаёт один transport, выполняет рукопожатие и возвращает все ресурсы,
    /// необходимые для атомарной замены текущего сервиса.
    async fn connect_recovery_attempt(
        &self,
        initialize_context: &InitializeContext,
    ) -> std::result::Result<RecoveredService, RecoveryAttemptError> {
        let pending_transport = Self::create_pending_transport(&self.transport_recipe)
            .await
            .map_err(RecoveryAttemptError::TransportCreation)?;
        let stdio_process = Self::stdio_process_from_pending_transport(&pending_transport);
        let connected = self
            .connect_pending_transport_with_initialize_retries(
                pending_transport,
                initialize_context.client_service.clone(),
                initialize_context.timeout,
            )
            .await;
        let (service, oauth_persistor) = match connected {
            Ok(connected) => connected,
            Err(error) => {
                let error = Self::cleanup_failed_stdio_process(stdio_process.as_ref(), error).await;
                return Err(RecoveryAttemptError::Initialize(error));
            }
        };
        if service.peer().peer_info().is_none() {
            let error = anyhow!("recovered handshake succeeded but server info was missing");
            let error = Self::cleanup_failed_stdio_process(stdio_process.as_ref(), error).await;
            return Err(RecoveryAttemptError::Initialize(error));
        }

        Ok(RecoveredService {
            service,
            oauth_persistor,
            stdio_process,
        })
    }

    /// Завершает процесс неудачной stdio-попытки до следующего запуска и
    /// сохраняет исходную ошибку основной причиной отказа.
    async fn cleanup_failed_stdio_process(
        stdio_process: Option<&StdioServerProcessHandle>,
        error: Error,
    ) -> Error {
        let Some(stdio_process) = stdio_process else {
            return error;
        };
        match stdio_process.terminate().await {
            Ok(()) => error,
            Err(cleanup_error) => error.context(format!(
                "failed to terminate stdio MCP process after recovery error: {cleanup_error}"
            )),
        }
    }
}

#[cfg(test)]
#[path = "service_recovery_tests.rs"]
mod tests;
