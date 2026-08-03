//! Fail-closed local authentication and wiring policy for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_api::{Operation, SessionAuthorizer};
use chief_of_staff_orchestrator_core::{ChannelWiringAuthorizer, ChannelWiringRequest};
use coding_adventures_csprng::random_array;
use coding_adventures_ct_compare::ct_eq_fixed;
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};

const SECRET_BYTES: usize = 32;
const ENCODED_BYTES: usize = SECRET_BYTES * 2;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Stable payload-blind local authentication failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalAuthError {
    /// The OS CSPRNG could not generate a fresh credential.
    RandomnessUnavailable,
    /// A retained credential was not exactly 64 lowercase hexadecimal bytes.
    InvalidCredentialEncoding,
    /// A presented credential did not authenticate.
    AuthenticationFailed,
}

impl Display for LocalAuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RandomnessUnavailable => "chief daemon policy: randomness unavailable",
            Self::InvalidCredentialEncoding => "chief daemon policy: invalid credential encoding",
            Self::AuthenticationFailed => "chief daemon policy: authentication failed",
        })
    }
}

impl std::error::Error for LocalAuthError {}

/// Generate one fresh lowercase-hex 256-bit bearer credential.
///
/// The returned string is wiped on drop. Outer composition is responsible for
/// persisting it with OS-appropriate owner-only permissions and delivering it
/// to an already protected CLI boundary.
pub fn generate_local_credential() -> Result<Zeroizing<String>, LocalAuthError> {
    let secret = Zeroizing::new(
        random_array::<SECRET_BYTES>().map_err(|_| LocalAuthError::RandomnessUnavailable)?,
    );
    Ok(Zeroizing::new(encode_secret(&secret)))
}

fn encode_secret(secret: &[u8; SECRET_BYTES]) -> String {
    let mut encoded = String::with_capacity(ENCODED_BYTES);
    for byte in secret {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

/// Opaque authority attached to one successfully authenticated connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalOperatorSession(());

/// Constant-time bearer policy for one loopback Chief daemon instance.
///
/// The retained credential is wiped on drop and intentionally has no `Debug`,
/// `Display`, or cloning implementation.
pub struct LocalBearerAuthorizer {
    expected: Zeroizing<[u8; ENCODED_BYTES]>,
}

impl LocalBearerAuthorizer {
    /// Retain one canonical lowercase-hex credential for authentication.
    pub fn new(encoded: &str) -> Result<Self, LocalAuthError> {
        if encoded.len() != ENCODED_BYTES
            || !encoded
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(LocalAuthError::InvalidCredentialEncoding);
        }
        let mut expected = [0u8; ENCODED_BYTES];
        expected.copy_from_slice(encoded.as_bytes());
        Ok(Self {
            expected: Zeroizing::new(expected),
        })
    }
}

impl SessionAuthorizer for LocalBearerAuthorizer {
    type Session = LocalOperatorSession;
    type Error = LocalAuthError;

    fn authenticate(&self, credential: &str) -> Result<Self::Session, Self::Error> {
        let candidate: &[u8; ENCODED_BYTES] = credential
            .as_bytes()
            .try_into()
            .map_err(|_| LocalAuthError::AuthenticationFailed)?;
        if ct_eq_fixed(&self.expected, candidate) {
            Ok(LocalOperatorSession(()))
        } else {
            Err(LocalAuthError::AuthenticationFailed)
        }
    }

    fn authorize(
        &self,
        _session: &Self::Session,
        _operation: Operation,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Stable refusal from the placeholder channel-wiring trust boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelWiringDenied;

impl Display for ChannelWiringDenied {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("chief daemon policy: channel wiring denied")
    }
}

impl std::error::Error for ChannelWiringDenied {}

/// Deny every channel topology mutation until Trust Checker approval exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DenyChannelWiring;

impl ChannelWiringAuthorizer for DenyChannelWiring {
    type Error = ChannelWiringDenied;

    fn authorize(&mut self, _request: ChannelWiringRequest<'_>) -> Result<(), Self::Error> {
        Err(ChannelWiringDenied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::{ChannelId, KeyEpoch};
    use chief_of_staff_channel_endpoints::{
        AgentId, ChannelDefinition, OriginatorIdentity, ReceiverIdentity,
    };

    const CREDENTIAL: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn deterministic_encoder_produces_canonical_lowercase_hex() {
        let mut secret = [0u8; SECRET_BYTES];
        for (index, byte) in secret.iter_mut().enumerate() {
            *byte = u8::try_from(index).unwrap();
        }
        assert_eq!(encode_secret(&secret), CREDENTIAL);
    }

    #[test]
    fn exact_credential_authenticates_and_authorizes_every_current_operation() {
        let policy = LocalBearerAuthorizer::new(CREDENTIAL).unwrap();
        let session = policy.authenticate(CREDENTIAL).unwrap();
        for operation in [
            Operation::RegisterHost,
            Operation::ListHosts,
            Operation::SetDesiredState,
            Operation::ReconcileOnce,
            Operation::HealthCheck,
            Operation::DeregisterHost,
        ] {
            assert!(policy.authorize(&session, operation).unwrap());
        }
    }

    #[test]
    fn equal_length_mismatches_and_invalid_lengths_fail_without_authority() {
        let policy = LocalBearerAuthorizer::new(CREDENTIAL).unwrap();
        for candidate in [
            format!("f{}", &CREDENTIAL[1..]),
            format!("{}z{}", &CREDENTIAL[..31], &CREDENTIAL[32..]),
            format!("{}0", &CREDENTIAL[..63]),
            "short".to_string(),
            format!("{CREDENTIAL}0"),
        ] {
            assert_eq!(
                policy.authenticate(&candidate),
                Err(LocalAuthError::AuthenticationFailed)
            );
        }
    }

    #[test]
    fn retained_credentials_require_one_canonical_encoding() {
        for invalid in [
            "",
            "abc",
            "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F",
            "g00102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        ] {
            assert!(matches!(
                LocalBearerAuthorizer::new(invalid),
                Err(LocalAuthError::InvalidCredentialEncoding)
            ));
        }
    }

    #[test]
    fn generated_credentials_are_canonical_fresh_and_authenticatable() {
        let first = generate_local_credential().unwrap();
        let second = generate_local_credential().unwrap();
        assert_eq!(first.len(), ENCODED_BYTES);
        assert!(first
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert_ne!(&*first, &*second);
        assert!(LocalBearerAuthorizer::new(&first)
            .unwrap()
            .authenticate(&first)
            .is_ok());
    }

    #[test]
    fn channel_topology_is_denied_until_a_trust_checker_approves_it() {
        let definition = channel_definition();
        let mut policy = DenyChannelWiring;
        assert_eq!(
            policy.authorize(ChannelWiringRequest::Create(&definition)),
            Err(ChannelWiringDenied)
        );
        assert_eq!(
            policy.authorize(ChannelWiringRequest::Destroy(&definition)),
            Err(ChannelWiringDenied)
        );
    }

    #[test]
    fn errors_are_stable_and_payload_blind() {
        assert_eq!(
            LocalAuthError::RandomnessUnavailable.to_string(),
            "chief daemon policy: randomness unavailable"
        );
        assert_eq!(
            LocalAuthError::InvalidCredentialEncoding.to_string(),
            "chief daemon policy: invalid credential encoding"
        );
        assert_eq!(
            LocalAuthError::AuthenticationFailed.to_string(),
            "chief daemon policy: authentication failed"
        );
        assert_eq!(
            ChannelWiringDenied.to_string(),
            "chief daemon policy: channel wiring denied"
        );
    }

    fn channel_definition() -> ChannelDefinition {
        let mut channel_id = [0u8; 16];
        channel_id[6] = 0x70;
        channel_id[8] = 0x80;
        ChannelDefinition::new(
            ChannelId(channel_id),
            OriginatorIdentity {
                agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
                public_key: [1; 32],
            },
            vec![ReceiverIdentity {
                agent_id: AgentId::new(b"receiver".to_vec()).unwrap(),
                public_key: [2; 32],
            }],
            1,
            KeyEpoch(1),
        )
        .unwrap()
    }
}
