use crate::{
    decode_device_certificate, decode_signed_commit, open_object, ApplicationError, ObjectKind,
    V1Keys,
};
use coding_adventures_ed25519::{is_valid_public_key, verify};
use coding_adventures_vault_pm_format::{
    AnnouncementV1, CommitV1, DeviceId, ObjectFrameV1, ObjectId, PublicKey, VaultId,
};
use coding_adventures_vault_pm_repository::{RepositoryVerifier, VerificationError};
use core::fmt::{self, Debug, Formatter};

/// Authority-anchored VLT-PM04 verifier for the single authorized Phase 1A
/// device.
///
/// Construction authenticates and decrypts the authority-signed device
/// certificate. The verifier then accepts only commits and announcements from
/// that exact vault, device, and certificate object. Multi-device policy and
/// revocation are deliberately left to the later enrollment slice.
pub struct V1SingleDeviceVerifier {
    keys: V1Keys,
    vault_id: VaultId,
    device_id: DeviceId,
    certificate_id: ObjectId,
    signing_public_key: PublicKey,
}

impl V1SingleDeviceVerifier {
    /// Match one pinned encrypted certificate object, authenticate it against
    /// the pinned vault authority, and construct the mandatory repository
    /// verification boundary.
    pub fn authorize(
        keys: V1Keys,
        authority_public_key: PublicKey,
        expected_certificate_id: ObjectId,
        certificate_frame: &ObjectFrameV1,
    ) -> Result<Self, ApplicationError> {
        if !is_valid_public_key(authority_public_key.as_bytes()) {
            return Err(ApplicationError::IntegrityFailure);
        }
        let certificate_id = certificate_frame
            .id()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if certificate_id != expected_certificate_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        let plaintext = open_object(&keys, ObjectKind::DeviceCertificate, certificate_frame)?;
        let certificate = decode_device_certificate(&plaintext)?;
        if certificate.vault_id != keys.vault_id()
            || !is_valid_public_key(certificate.signing_public_key.as_bytes())
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        let preimage = certificate
            .signing_preimage()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if !verify(
            &preimage,
            certificate.signature.as_bytes(),
            authority_public_key.as_bytes(),
        ) {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(Self {
            keys,
            vault_id: certificate.vault_id,
            device_id: certificate.device_id,
            certificate_id,
            signing_public_key: certificate.signing_public_key,
        })
    }

    /// Return the authorized vault identity.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// Return the authorized writer device identity.
    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Return the exact encrypted certificate object ID authorized at
    /// construction.
    pub const fn certificate_id(&self) -> ObjectId {
        self.certificate_id
    }

    fn commit_is_authorized(&self, commit: &CommitV1) -> bool {
        commit.vault_id == self.vault_id
            && commit.device_id == self.device_id
            && commit.device_certificate == self.certificate_id
            && commit.signing_preimage().is_ok_and(|preimage| {
                verify(
                    &preimage,
                    commit.signature.as_bytes(),
                    self.signing_public_key.as_bytes(),
                )
            })
    }

    fn announcement_is_authorized(&self, announcement: &AnnouncementV1) -> bool {
        announcement.vault_id == self.vault_id
            && announcement.device_id == self.device_id
            && announcement.device_certificate == self.certificate_id
            && announcement.signing_preimage().is_ok_and(|preimage| {
                verify(
                    &preimage,
                    announcement.signature.as_bytes(),
                    self.signing_public_key.as_bytes(),
                )
            })
    }
}

impl RepositoryVerifier for V1SingleDeviceVerifier {
    fn verify_commit(
        &self,
        expected: &ObjectId,
        frame: &ObjectFrameV1,
    ) -> Result<CommitV1, VerificationError> {
        if frame.id().map_err(|_| VerificationError)? != *expected {
            return Err(VerificationError);
        }
        let plaintext =
            open_object(&self.keys, ObjectKind::Commit, frame).map_err(|_| VerificationError)?;
        let commit = decode_signed_commit(&plaintext).map_err(|_| VerificationError)?;
        if !self.commit_is_authorized(&commit) {
            return Err(VerificationError);
        }
        Ok(commit)
    }

    fn verify_announcement(&self, bytes: &[u8]) -> Result<AnnouncementV1, VerificationError> {
        let announcement = AnnouncementV1::decode(bytes).map_err(|_| VerificationError)?;
        if !self.announcement_is_authorized(&announcement) {
            return Err(VerificationError);
        }
        Ok(announcement)
    }
}

impl Debug for V1SingleDeviceVerifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("V1SingleDeviceVerifier(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_device_certificate, encode_signed_commit, seal_object, ObjectRandomness};
    use coding_adventures_ed25519::{generate_keypair, sign};
    use coding_adventures_vault_pm_format::{DeviceCertificateV1, Signature, CRYPTO_SUITE_V1};

    const VAULT_ID: VaultId = VaultId::new([0x11; 16]);
    const DEVICE_ID: DeviceId = DeviceId::new([0x22; 16]);
    const ROOT_KEY: [u8; 32] = [0x33; 32];

    struct Fixture {
        verifier: V1SingleDeviceVerifier,
        device_secret: [u8; 64],
        certificate_id: ObjectId,
    }

    fn randomness(value: u8) -> ObjectRandomness {
        ObjectRandomness::new(
            [value; 32],
            [value.wrapping_add(1); 24],
            [value.wrapping_add(2); 24],
        )
    }

    fn signed_certificate(
        vault_id: VaultId,
        authority_secret: &[u8; 64],
        signing_public_key: PublicKey,
    ) -> DeviceCertificateV1 {
        let certificate = DeviceCertificateV1 {
            vault_id,
            device_id: DEVICE_ID,
            signing_public_key,
            wrapping_public_key: PublicKey::new([0x44; 32]),
            created_at_ms: 100,
            capabilities: vec![1],
            signature: Signature::new([0; 64]),
        };
        let signature = sign(&certificate.signing_preimage().unwrap(), authority_secret);
        certificate.with_signature(Signature::new(signature))
    }

    fn fixture() -> Fixture {
        let (authority_public, authority_secret) = generate_keypair(&[0x51; 32]);
        let (device_public, device_secret) = generate_keypair(&[0x52; 32]);
        let certificate =
            signed_certificate(VAULT_ID, &authority_secret, PublicKey::new(device_public));
        let plaintext = encode_device_certificate(&certificate).unwrap();
        let certificate_frame = seal_object(
            &V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            ObjectKind::DeviceCertificate,
            &plaintext,
            &randomness(0x61),
        )
        .unwrap();
        let certificate_id = certificate_frame.id().unwrap();
        let verifier = V1SingleDeviceVerifier::authorize(
            V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            PublicKey::new(authority_public),
            certificate_id,
            &certificate_frame,
        )
        .unwrap();
        Fixture {
            verifier,
            device_secret,
            certificate_id,
        }
    }

    fn signed_commit(fixture: &Fixture, vault_id: VaultId) -> CommitV1 {
        let commit = CommitV1 {
            vault_id,
            device_id: DEVICE_ID,
            device_counter: 1,
            parents: Vec::new(),
            catalog_root: ObjectId::new([0x71; 32]),
            added_objects: vec![ObjectId::new([0x71; 32])],
            tombstone_root: None,
            wall_time_ms: 200,
            device_certificate: fixture.certificate_id,
            signature: Signature::new([0; 64]),
        };
        let signature = sign(&commit.signing_preimage().unwrap(), &fixture.device_secret);
        commit.with_signature(Signature::new(signature))
    }

    fn sealed_commit(commit: &CommitV1) -> ObjectFrameV1 {
        seal_object(
            &V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            ObjectKind::Commit,
            &encode_signed_commit(commit).unwrap(),
            &randomness(0x62),
        )
        .unwrap()
    }

    fn signed_announcement(fixture: &Fixture, commit_id: ObjectId) -> AnnouncementV1 {
        let announcement = AnnouncementV1 {
            vault_id: VAULT_ID,
            device_id: DEVICE_ID,
            device_counter: 1,
            commit_id,
            device_certificate: fixture.certificate_id,
            signature: Signature::new([0; 64]),
        };
        let signature = sign(
            &announcement.signing_preimage().unwrap(),
            &fixture.device_secret,
        );
        announcement.with_signature(Signature::new(signature))
    }

    #[test]
    fn authority_anchors_certificate_commit_and_announcement() {
        let fixture = fixture();
        assert_eq!(fixture.verifier.vault_id(), VAULT_ID);
        assert_eq!(fixture.verifier.device_id(), DEVICE_ID);
        assert_eq!(fixture.verifier.certificate_id(), fixture.certificate_id);
        assert_eq!(
            format!("{:?}", fixture.verifier),
            "V1SingleDeviceVerifier(<redacted>)"
        );

        let commit = signed_commit(&fixture, VAULT_ID);
        let frame = sealed_commit(&commit);
        let commit_id = frame.id().unwrap();
        assert_eq!(
            fixture.verifier.verify_commit(&commit_id, &frame).unwrap(),
            commit
        );
        let announcement = signed_announcement(&fixture, commit_id);
        assert_eq!(
            fixture
                .verifier
                .verify_announcement(&announcement.encode().unwrap())
                .unwrap(),
            announcement
        );
    }

    #[test]
    fn commit_verification_rejects_wrong_id_aead_identity_and_signature() {
        let fixture = fixture();
        let commit = signed_commit(&fixture, VAULT_ID);
        let frame = sealed_commit(&commit);
        assert!(fixture
            .verifier
            .verify_commit(&ObjectId::new([0; 32]), &frame)
            .is_err());

        let mut tampered = frame.clone();
        tampered.payload_tag[0] ^= 1;
        let tampered_id = tampered.id().unwrap();
        assert!(fixture
            .verifier
            .verify_commit(&tampered_id, &tampered)
            .is_err());

        for invalid in [
            signed_commit(&fixture, VaultId::new([9; 16])),
            CommitV1 {
                device_id: DeviceId::new([9; 16]),
                ..commit.clone()
            },
            CommitV1 {
                device_certificate: ObjectId::new([9; 32]),
                ..commit.clone()
            },
            CommitV1 {
                signature: Signature::new([9; 64]),
                ..commit.clone()
            },
        ] {
            let invalid_frame = sealed_commit(&invalid);
            assert!(fixture
                .verifier
                .verify_commit(&invalid_frame.id().unwrap(), &invalid_frame)
                .is_err());
        }
    }

    #[test]
    fn announcement_verification_rejects_malformed_identity_and_signature() {
        let fixture = fixture();
        let announcement = signed_announcement(&fixture, ObjectId::new([7; 32]));
        assert!(fixture.verifier.verify_announcement(&[]).is_err());
        for invalid in [
            AnnouncementV1 {
                vault_id: VaultId::new([9; 16]),
                ..announcement.clone()
            },
            AnnouncementV1 {
                device_id: DeviceId::new([9; 16]),
                ..announcement.clone()
            },
            AnnouncementV1 {
                device_certificate: ObjectId::new([9; 32]),
                ..announcement.clone()
            },
            AnnouncementV1 {
                signature: Signature::new([9; 64]),
                ..announcement.clone()
            },
        ] {
            assert!(fixture
                .verifier
                .verify_announcement(&invalid.encode().unwrap())
                .is_err());
        }
    }

    #[test]
    fn authorization_rejects_bad_authority_certificate_and_vault_binding() {
        let (authority_public, authority_secret) = generate_keypair(&[0x51; 32]);
        let (device_public, _) = generate_keypair(&[0x52; 32]);
        for (certificate, authority) in [
            (
                signed_certificate(
                    VaultId::new([9; 16]),
                    &authority_secret,
                    PublicKey::new(device_public),
                ),
                PublicKey::new(authority_public),
            ),
            (
                DeviceCertificateV1 {
                    signature: Signature::new([9; 64]),
                    ..signed_certificate(VAULT_ID, &authority_secret, PublicKey::new(device_public))
                },
                PublicKey::new(authority_public),
            ),
        ] {
            let frame = seal_object(
                &V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
                ObjectKind::DeviceCertificate,
                &encode_device_certificate(&certificate).unwrap(),
                &randomness(0x63),
            )
            .unwrap();
            assert!(V1SingleDeviceVerifier::authorize(
                V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
                authority,
                frame.id().unwrap(),
                &frame,
            )
            .is_err());
        }

        let invalid_device_certificate =
            signed_certificate(VAULT_ID, &authority_secret, PublicKey::new([0; 32]));
        let invalid_device_frame = seal_object(
            &V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            ObjectKind::DeviceCertificate,
            &encode_device_certificate(&invalid_device_certificate).unwrap(),
            &randomness(0x64),
        )
        .unwrap();
        assert!(V1SingleDeviceVerifier::authorize(
            V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            PublicKey::new(authority_public),
            invalid_device_frame.id().unwrap(),
            &invalid_device_frame,
        )
        .is_err());

        let valid_certificate =
            signed_certificate(VAULT_ID, &authority_secret, PublicKey::new(device_public));
        let valid_certificate_frame = seal_object(
            &V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            ObjectKind::DeviceCertificate,
            &encode_device_certificate(&valid_certificate).unwrap(),
            &randomness(0x65),
        )
        .unwrap();
        assert!(V1SingleDeviceVerifier::authorize(
            V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            PublicKey::new([0; 32]),
            valid_certificate_frame.id().unwrap(),
            &valid_certificate_frame,
        )
        .is_err());

        assert!(V1SingleDeviceVerifier::authorize(
            V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            PublicKey::new(authority_public),
            ObjectId::new([0; 32]),
            &valid_certificate_frame,
        )
        .is_err());

        let fixture = fixture();
        let wrong_kind_frame = sealed_commit(&signed_commit(&fixture, VAULT_ID));
        assert!(V1SingleDeviceVerifier::authorize(
            V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            PublicKey::new(authority_public),
            wrong_kind_frame.id().unwrap(),
            &wrong_kind_frame,
        )
        .is_err());

        let mut unsupported = wrong_kind_frame;
        unsupported.suite = CRYPTO_SUITE_V1 + 1;
        assert!(V1SingleDeviceVerifier::authorize(
            V1Keys::derive(VAULT_ID, &ROOT_KEY).unwrap(),
            PublicKey::new(authority_public),
            ObjectId::new([0; 32]),
            &unsupported,
        )
        .is_err());
    }
}
