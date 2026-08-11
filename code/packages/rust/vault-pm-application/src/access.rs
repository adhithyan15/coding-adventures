use crate::{ActiveStateV1, ApplicationError};
use core::fmt::{self, Debug, Formatter};

/// One authenticated access result whose audit event and next owner state are
/// already durable.
///
/// The operation result may still be a closed application failure. In that
/// case the returned active state proves the failed post-authentication attempt
/// was recorded before the caller observes the failure. Publication failures
/// return directly from the access method and never construct this wrapper.
#[must_use = "the durable next owner state and audited operation result must be handled"]
pub struct AuditedAccessResultV1<T> {
    active: ActiveStateV1,
    operation: Result<T, ApplicationError>,
}

impl<T> AuditedAccessResultV1<T> {
    pub(crate) const fn new(active: ActiveStateV1, operation: Result<T, ApplicationError>) -> Self {
        Self { active, operation }
    }

    /// Borrow the durable next owner state installed with the audit event.
    pub const fn active_state(&self) -> &ActiveStateV1 {
        &self.active
    }

    /// Return whether the authenticated operation itself succeeded.
    pub const fn operation_succeeded(&self) -> bool {
        self.operation.is_ok()
    }

    /// Consume the wrapper and return the original operation result.
    pub fn into_operation(self) -> Result<T, ApplicationError> {
        self.operation
    }

    /// Consume the wrapper into the durable next state and operation result.
    pub fn into_parts(self) -> (ActiveStateV1, Result<T, ApplicationError>) {
        (self.active, self.operation)
    }
}

impl<T> Debug for AuditedAccessResultV1<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditedAccessResultV1")
            .field("operation_succeeded", &self.operation_succeeded())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthorityFingerprint, BootstrapLocator};
    use coding_adventures_vault_pm_format::{
        AeadEnvelopeV1, BootstrapId, DeviceId, ObjectFrameV1, ObjectId, VaultId, CRYPTO_SUITE_V1,
    };
    use coding_adventures_vault_pm_repository::PinnedHeads;

    fn active() -> ActiveStateV1 {
        let certificate = ObjectFrameV1 {
            suite: CRYPTO_SUITE_V1,
            wrap_nonce: [2; 24],
            wrapped_dek: [3; 32],
            wrap_tag: [4; 16],
            payload_nonce: [5; 24],
            ciphertext: vec![6],
            payload_tag: [7; 16],
        };
        let certificate_id = certificate.id().unwrap();
        ActiveStateV1::new(
            BootstrapLocator::new([6; 32]),
            VaultId::new([1; 16]),
            BootstrapId::new([7; 32]),
            AuthorityFingerprint::new([8; 32]),
            DeviceId::new([9; 16]),
            certificate_id,
            certificate,
            AeadEnvelopeV1 {
                suite: CRYPTO_SUITE_V1,
                nonce: [10; 24],
                ciphertext: vec![11],
                tag: [12; 16],
            },
            PinnedHeads::new([ObjectId::new([13; 32])]).unwrap(),
            1,
            ObjectId::new([14; 32]),
        )
        .unwrap()
    }

    #[test]
    fn wrapper_exposes_only_status_until_explicit_consumption() {
        let success = AuditedAccessResultV1::new(active(), Ok::<_, ApplicationError>(17));
        assert!(success.operation_succeeded());
        assert_eq!(
            format!("{success:?}"),
            "AuditedAccessResultV1 { operation_succeeded: true, .. }"
        );
        assert_eq!(success.into_operation(), Ok(17));

        let failure =
            AuditedAccessResultV1::<()>::new(active(), Err(ApplicationError::ConflictRequired));
        assert!(!failure.operation_succeeded());
        let (active, operation) = failure.into_parts();
        assert_eq!(active.last_device_counter(), 1);
        assert_eq!(operation, Err(ApplicationError::ConflictRequired));
    }
}
