//! Проверяет классификацию ошибок отдельной попытки восстановления сервиса.

use std::io;

use pretty_assertions::assert_eq;

use super::RecoveryAttemptError;

/// Только `NotFound` и `PermissionDenied` создания транспорта завершают серию
/// немедленно; те же ошибки рукопожатия остаются повторяемыми.
#[test]
fn stdio_recovery_error_classification_distinguishes_creation_from_initialize_failures() {
    let classify_creation = |kind| {
        RecoveryAttemptError::TransportCreation(io::Error::from(kind).into())
            .is_permanent_transport_creation()
    };
    assert_eq!(
        [
            classify_creation(io::ErrorKind::NotFound),
            classify_creation(io::ErrorKind::PermissionDenied),
            classify_creation(io::ErrorKind::ConnectionRefused),
        ],
        [true, true, false]
    );

    assert!(
        !RecoveryAttemptError::Initialize(io::Error::from(io::ErrorKind::NotFound).into())
            .is_permanent_transport_creation()
    );
}
