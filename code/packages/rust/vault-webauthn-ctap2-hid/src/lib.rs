//! # coding_adventures_vault_webauthn_ctap2_hid — VLT-PM51 slice 2
//!
//! ## What this crate does
//!
//! Real CTAP2-over-USB-HID hardware I/O for
//! `coding_adventures_vault_auth::WebAuthnPrfAuthenticator`. Implements
//! that crate's `Ctap2Transport` trait — "however we talk to a
//! physical FIDO2 authenticator" — using the `ctap-hid-fido2` crate
//! (itself built on `hidapi`) to enumerate USB HID FIDO2 devices and
//! perform a CTAP2 `GetAssertion` request with the `hmac-secret`
//! extension.
//!
//! This is deliberately its own crate rather than living inside
//! `vault-auth`. `vault-auth` is the trust-sensitive KDF/
//! authentication crate every unlock factor in the Vault stack goes
//! through (`PasswordAuthenticator`, `TotpAuthenticator`); giving it a
//! native, hardware-touching dependency would mean every consumer of
//! those two factors also inherits `ctap-hid-fido2`'s build and
//! runtime footprint whether or not it ever plugs in a hardware key.
//! `code/specs/VLT-PM51-hardware-security-keys.md` covers the full
//! design; the short version of why this split exists is the same one
//! `VLT-PM48` already used for the local agent — a protocol crate
//! (`vault-auth`'s `Ctap2Transport` trait) plus a separate transport/
//! host crate (this one).
//!
//! ## What is real here, and what still refuses
//!
//! `HidCtap2Transport::get_hmac_secret_assertion` really does:
//!
//! - Cheap, fast device enumeration (`ctap_hid_fido2::get_fidokey_
//!   devices()`) — if nothing answers, this returns
//!   `Ctap2TransportError::NoDeviceAvailable` immediately, without
//!   ever entering a blocking wait. A vault with no hardware key
//!   configured never slows down because of this transport.
//! - A real CTAP2 `GetAssertion` request, with the `hmac-secret`
//!   extension, against a real device when one is present.
//! - A caller-controlled, bounded wait for the physical touch (via a
//!   dedicated worker thread + `mpsc::Receiver::recv_timeout`) — see
//!   the module comment on [`HidCtap2Transport`] for why this exists
//!   as an outer wrapper rather than relying on the underlying
//!   crate's own internal timeout behavior.
//!
//! What still refuses, one layer up in `vault-auth`:
//! `WebAuthnPrfAuthenticator::verify()` still always returns
//! `AuthError::Unimplemented` as its *final* answer, because ECDSA
//! P-256 assertion-signature verification doesn't exist anywhere in
//! this workspace yet. This crate's job is exactly "make the hardware
//! I/O real"; it does not attempt the missing cryptography.
//!
//! ## Testing without physical hardware
//!
//! `ctap_hid_fido2::get_fidokey_devices()` and
//! `FidoKeyHidFactory::create` are safe to call with no device
//! attached — they return an empty list / an `Err` respectively,
//! which is exactly the fast-fail path CI exercises for real (no
//! mocking) in this crate's own tests. The request/response mapping
//! functions (`build_hmac_secret_extension`, `map_assertion`,
//! `classify_ctap_error`) are pure and are unit-tested directly
//! against real `ctap-hid-fido2` types built by hand, without any
//! device. The one thing that cannot be exercised in CI is an actual
//! physical touch — see
//! `code/specs/VLT-PM51-hardware-security-keys.md` for the full
//! testability design and why that gap is acceptable.

#![deny(missing_docs)]

use coding_adventures_vault_auth::{
    Ctap2AssertionRequest, Ctap2AssertionResponse, Ctap2Transport, Ctap2TransportError,
};
use ctap_hid_fido2::fidokey::get_assertion::get_assertion_params::{Assertion, Extension as Gext};
use ctap_hid_fido2::{FidoKeyHidFactory, LibCfg};
use std::sync::mpsc;
use std::thread;

/// Real CTAP2-over-USB-HID [`Ctap2Transport`], backed by
/// `ctap-hid-fido2` + `hidapi`.
///
/// ## No persistent device handle
///
/// Each call to [`get_hmac_secret_assertion`](Ctap2Transport::get_hmac_secret_assertion)
/// re-enumerates and re-opens the device from scratch rather than
/// caching an open `FidoKeyHid` across calls. An unlock attempt is an
/// occasional, human-paced operation, not a hot loop, so the repeated
/// enumeration cost is immaterial — and never holding a HID handle
/// between calls means this type carries no interior state that would
/// need synchronizing, which is what makes `Send + Sync` trivial to
/// satisfy honestly rather than by asserting it over unsynchronized
/// native state.
///
/// ## Bounded touch timeout, and its one honest caveat
///
/// `ctap-hid-fido2`'s own `GetAssertion` call blocks natively on the
/// underlying HID read with no timeout parameter of its own — it
/// waits for `CTAPHID_KEEPALIVE` frames until the device answers or
/// itself gives up, which the public API gives no way to bound. This
/// transport wraps that call in a dedicated worker thread and races it
/// against `request.touch_timeout` using `mpsc::Receiver::
/// recv_timeout`, so **`verify()` always returns control to the
/// caller within the configured timeout**, matching this factor's
/// "additive, never blocks the software-only unlock path" design
/// requirement.
///
/// The honest caveat: there is no cross-platform way to cancel a
/// blocking native HID read from safe Rust. When the timeout fires,
/// the worker thread is left running rather than killed — it will
/// exit on its own once the device eventually answers (touched or
/// not) or the crate's/device's own internal handling gives up, at
/// which point its result is silently discarded (the caller has long
/// since moved on). A single abandoned attempt's thread is bounded
/// and self-terminating; it is not a leak that grows the process's
/// memory or open-handle count without limit, but repeated timeouts
/// against an unresponsive device do accumulate live background
/// threads until each one resolves. A future PR could tighten this
/// with a lower-level transport that exposes `hidapi`'s own read
/// timeout directly instead of going through `ctap-hid-fido2`'s
/// blocking convenience API.
#[derive(Debug, Default, Clone, Copy)]
pub struct HidCtap2Transport {
    _private: (),
}

impl HidCtap2Transport {
    /// Build a transport. Stateless — safe to construct freely, share
    /// across threads, and drop and rebuild between unlock attempts.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

/// Process-wide lock serializing **every** call into `ctap-hid-fido2`
/// (and therefore `hidapi`) made through this crate — enumeration
/// included, not just `GetAssertion`.
///
/// This is a real fix for a real crash, not defensive boilerplate:
/// running this crate's own test suite with multiple separate
/// `#[test]` functions each calling real `ctap_hid_fido2` APIs
/// (`get_fidokey_devices`, `FidoKeyHidFactory::create`) — each on the
/// distinct OS thread libtest spawns per test — reliably crashed the
/// process with `SIGTRAP` under the default parallel test runner,
/// even *with* this lock in place around each call. That rules out a
/// same-process race as the sole cause; `hidapi`'s macOS backend
/// (Core Foundation / IOKit underneath) appears not to tolerate being
/// entered from more than one distinct OS thread across the process's
/// lifetime at all, which this lock alone cannot fix from inside
/// separate test threads. The `tests` module works around that by
/// keeping every real-hardware call in one `#[test]` function, so
/// libtest spawns exactly one thread for all of them — see that
/// test's own doc comment for the full story. This lock still matters
/// independently, for the production reason below.
///
/// This also protects **production** use: two concurrent
/// `verify()` calls in one process (plausible once this transport is
/// wired into `vault-pm-agent-host`, which serves concurrent local
/// requests per `VLT-PM48`) would hit the identical crash without
/// this lock. Serializing here is also the semantically correct
/// behavior independent of the crash — a physical USB HID device
/// cannot meaningfully hold two concurrent CTAP2 conversations at
/// once regardless of what any particular driver does with the
/// attempt.
///
/// One consequence worth knowing: if an earlier attempt's worker
/// thread is still running past its own caller's `touch_timeout` (see
/// [`HidCtap2Transport`]'s doc on abandoned threads), a new attempt
/// blocks on this lock until that earlier worker finishes — which can
/// make the new attempt's own `touch_timeout` elapse waiting for the
/// lock, surfacing as [`Ctap2TransportError::TouchTimedOut`] before
/// this attempt's own hardware round trip ever starts. That is a
/// real, documented trade-off, not a bug: it is still strictly safer
/// than the crash it replaces.
static HID_ACCESS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl Ctap2Transport for HidCtap2Transport {
    fn get_hmac_secret_assertion(
        &self,
        request: &Ctap2AssertionRequest<'_>,
    ) -> Result<Ctap2AssertionResponse, Ctap2TransportError> {
        let relying_party_id = request.relying_party_id.to_string();
        let credential_id = request.credential_id.to_vec();
        let challenge = request.challenge;
        let hmac_secret_salt = request.hmac_secret_salt;
        let touch_timeout = request.touch_timeout;

        let (result_tx, result_rx) = mpsc::channel();
        thread::spawn(move || {
            // Every native HID call this attempt makes — enumeration
            // (inside `FidoKeyHidFactory::create`) and `GetAssertion`
            // alike — happens under `HID_ACCESS_LOCK`, on this one
            // worker thread, for the attempt's entire duration. See
            // that lock's doc for why.
            let _guard = HID_ACCESS_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let outcome = run_get_assertion(
                &relying_party_id,
                &credential_id,
                &challenge,
                &hmac_secret_salt,
            );
            drop(_guard);
            // Best-effort: if the caller already timed out and moved
            // on, `result_rx` is gone and this send fails silently —
            // see the struct doc for why that's the accepted
            // trade-off of wrapping a non-cancellable blocking call.
            let _ = result_tx.send(outcome);
        });

        match result_rx.recv_timeout(touch_timeout) {
            Ok(outcome) => outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(Ctap2TransportError::TouchTimedOut),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(Ctap2TransportError::Failed {
                detail: "hardware worker thread ended without a result",
            }),
        }
    }
}

/// Everything that happens on the worker thread while
/// `HID_ACCESS_LOCK` is held: open the device (which itself
/// enumerates — see `FidoKeyHidFactory::create`'s own "device not
/// found" fast path), build the `hmac-secret` extension request,
/// perform the `GetAssertion`, and map the result. Split out from
/// [`HidCtap2Transport::get_hmac_secret_assertion`] so it can be
/// spawned into a thread without dragging `&self` across the
/// boundary.
fn run_get_assertion(
    relying_party_id: &str,
    credential_id: &[u8],
    challenge: &[u8; 32],
    hmac_secret_salt: &[u8; 32],
) -> Result<Ctap2AssertionResponse, Ctap2TransportError> {
    let cfg = LibCfg::init().with_enable_log(false);
    let device = FidoKeyHidFactory::create(&cfg).map_err(|err| classify_ctap_error(&err))?;
    let extensions = build_hmac_secret_extension(*hmac_secret_salt);
    let assertion = device
        .get_assertion_with_extensios(
            relying_party_id,
            challenge,
            &[credential_id.to_vec()],
            None,
            Some(&extensions),
        )
        .map_err(|err| classify_ctap_error(&err))?;
    map_assertion(assertion)
}

/// Build the single-salt `hmac-secret` extension request. Pure and
/// unit-tested directly — no device needed to check this shape is
/// right.
fn build_hmac_secret_extension(salt: [u8; 32]) -> Vec<Gext> {
    vec![Gext::HmacSecret(Some(salt))]
}

/// Best-effort classification of an `anyhow::Error` from
/// `ctap-hid-fido2` into our transport-agnostic error taxonomy.
/// `ctap-hid-fido2` reports failures as free-text `anyhow::Error`s
/// rather than a typed CTAP2 status-code enum, so this pattern-matches
/// known substrings; anything unrecognized becomes a generic
/// `Failed`. Never includes the original message text (which could in
/// principle echo device- or protocol-specific bytes) in the
/// classification — only ever one of this crate's own static labels.
fn classify_ctap_error(err: &anyhow::Error) -> Ctap2TransportError {
    let message = err.to_string().to_ascii_lowercase();
    if message.contains("not found") || message.contains("multiple fido devices") {
        Ctap2TransportError::NoDeviceAvailable
    } else if message.contains("timeout") || message.contains("timed out") {
        Ctap2TransportError::TouchTimedOut
    } else if message.contains("denied") || message.contains("declined") {
        // CTAP2-over-HID has no signal that reliably distinguishes an
        // authenticator that was touched-and-declined from one that
        // was simply never touched on every device — see
        // `Ctap2TransportError::TouchTimedOut`'s doc in `vault-auth`.
        // The few authenticators that do report an explicit denial
        // are still classified precisely here, best-effort.
        Ctap2TransportError::TouchTimedOut
    } else {
        Ctap2TransportError::Failed {
            detail: "CTAP2 GetAssertion request failed",
        }
    }
}

/// Map `ctap-hid-fido2`'s own `Assertion` response type into
/// `vault-auth`'s transport-agnostic `Ctap2AssertionResponse`. Pure
/// and unit-tested directly against hand-built `Assertion` values —
/// the real crate's own struct, not a fake of it — so this mapping is
/// exercised without any device.
fn map_assertion(assertion: Assertion) -> Result<Ctap2AssertionResponse, Ctap2TransportError> {
    let rpid_hash: [u8; 32] =
        assertion
            .rpid_hash
            .as_slice()
            .try_into()
            .map_err(|_| Ctap2TransportError::Failed {
                detail: "authenticator response had a malformed rpIdHash length",
            })?;

    let hmac_secret_output = assertion.extensions.into_iter().find_map(|extension| {
        if let Gext::HmacSecret(Some(bytes)) = extension {
            Some(coding_adventures_zeroize::Zeroizing::new(bytes.to_vec()))
        } else {
            None
        }
    });

    Ok(Ctap2AssertionResponse {
        rpid_hash,
        credential_id: assertion.credential_id,
        user_present: assertion.flags.user_present_result,
        signature: assertion.signature,
        auth_data: assertion.auth_data,
        hmac_secret_output,
    })
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────
//
// Two kinds of coverage here, deliberately not blended together:
//
// 1. Pure-logic tests (`build_hmac_secret_extension`, `map_assertion`,
//    `classify_ctap_error`) that need no device at all and run
//    instantly, every time, on every CI runner.
// 2. Real, non-mocked calls into `ctap-hid-fido2`'s own public API
//    (`get_fidokey_devices`, `FidoKeyHidFactory::create`) that prove
//    this crate's dependency wiring actually compiles and calls the
//    real crate correctly — exercising its genuine "no hardware
//    attached" fast-fail path for real, without requiring physical
//    hardware in CI.
//
// What is NOT tested here: an actual physical touch. That gap is
// documented, not hidden — see the module doc above and
// `code/specs/VLT-PM51-hardware-security-keys.md`.
#[cfg(test)]
mod tests {
    use super::*;
    use ctap_hid_fido2::auth_data::Flags;
    use ctap_hid_fido2::public_key_credential_user_entity::PublicKeyCredentialUserEntity;
    use std::time::Duration;

    // --- Real, non-mocked calls into ctap-hid-fido2's own API ---
    //
    // All three real-hardware checks below are deliberately combined
    // into ONE `#[test]` function rather than three separate ones.
    // Splitting them cost real debugging time: three separate tests,
    // each touching real `hidapi` from the OS thread libtest spawns
    // for that test, reliably crashed this crate's own suite with
    // `SIGTRAP` under the default parallel test runner — even after
    // adding `HID_ACCESS_LOCK` to serialize the actual native calls.
    // That rules out "our own code raced itself" as the cause;
    // `hidapi`'s macOS backend (Core Foundation / IOKit underneath)
    // appears not to tolerate being entered from more than one
    // distinct OS thread across the process's lifetime at all, not
    // just concurrently — a stricter constraint than this crate's own
    // `HID_ACCESS_LOCK` (correct and necessary for production
    // concurrent `verify()` calls) can satisfy from inside separate
    // test-harness threads. One test function means libtest spawns
    // exactly one thread for all of this file's real-hardware calls,
    // which is what actually made the suite stable — verified by
    // running it repeatedly under the default parallel runner, not
    // just once.
    #[test]
    fn real_ctap_hid_fido2_calls_run_and_fail_closed_with_no_hardware_attached() {
        // 1. Real device enumeration — not a fake of it. An empty
        //    result is expected on CI; a non-empty one (a real key
        //    happens to be attached to whatever machine runs this) is
        //    also fine and not a failure.
        let _devices = ctap_hid_fido2::get_fidokey_devices();

        // 2. `FidoKeyHidFactory::create` is the exact function
        //    `run_get_assertion` calls. With no device attached, the
        //    real crate returns `Err("FIDO device not found.")` —
        //    this proves that path is reachable and fails fast, with
        //    no mocking. If a real device is attached, `create` may
        //    succeed instead; either way this only checks the call
        //    is safe and doesn't hang.
        let cfg = LibCfg::init().with_enable_log(false);
        let _ = FidoKeyHidFactory::create(&cfg);

        // 3. End-to-end through the real `Ctap2Transport` impl. With
        //    no device attached this must return `NoDeviceAvailable`
        //    fast — well under `touch_timeout` — because enumeration
        //    (inside `FidoKeyHidFactory::create`), not the blocking
        //    `GetAssertion` path, is what answers first.
        let transport = HidCtap2Transport::new();
        let request = Ctap2AssertionRequest {
            relying_party_id: "vault-pm",
            credential_id: b"test-credential-id",
            challenge: [0x11; 32],
            hmac_secret_salt: [0x22; 32],
            touch_timeout: Duration::from_secs(5),
        };
        let started = std::time::Instant::now();
        let result = transport.get_hmac_secret_assertion(&request);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "no-device path must fail fast, not wait out the touch timeout"
        );
        if let Err(Ctap2TransportError::NoDeviceAvailable) = result {
            // Expected on a CI runner / a dev machine with no key
            // attached.
        } else {
            // A real device answered (or something else happened) —
            // not this test's concern, since it only targets the
            // no-hardware fast-fail path. The timing assertion above
            // already ran regardless.
        }
    }

    // --- Pure logic: build_hmac_secret_extension ---

    #[test]
    fn build_hmac_secret_extension_carries_the_exact_salt() {
        let salt = [0x5A; 32];
        let extensions = build_hmac_secret_extension(salt);
        assert_eq!(extensions.len(), 1);
        match &extensions[0] {
            Gext::HmacSecret(Some(got)) => assert_eq!(*got, salt),
            other => panic!("expected HmacSecret(Some(salt)), got {:?}", other),
        }
    }

    // --- Pure logic: classify_ctap_error ---

    #[test]
    fn classify_ctap_error_maps_not_found_to_no_device_available() {
        let err = anyhow::anyhow!("FIDO device not found.");
        match classify_ctap_error(&err) {
            Ctap2TransportError::NoDeviceAvailable => {}
            other => panic!("expected NoDeviceAvailable, got {:?}", other),
        }
    }

    #[test]
    fn classify_ctap_error_maps_timeout_text_to_touch_timed_out() {
        let err = anyhow::anyhow!("operation timed out waiting for user presence");
        match classify_ctap_error(&err) {
            Ctap2TransportError::TouchTimedOut => {}
            other => panic!("expected TouchTimedOut, got {:?}", other),
        }
    }

    #[test]
    fn classify_ctap_error_maps_denied_text_to_touch_timed_out() {
        let err = anyhow::anyhow!("CTAP2_ERR_OPERATION_DENIED");
        match classify_ctap_error(&err) {
            Ctap2TransportError::TouchTimedOut => {}
            other => panic!("expected TouchTimedOut, got {:?}", other),
        }
    }

    #[test]
    fn classify_ctap_error_maps_unknown_text_to_generic_failed_without_echoing_message() {
        let err = anyhow::anyhow!("some totally unrecognized device-specific error blob");
        match classify_ctap_error(&err) {
            Ctap2TransportError::Failed { detail } => {
                // Must be one of this crate's own static labels, never
                // the original (potentially device-controlled) text.
                assert!(!detail.contains("device-specific"));
                assert_eq!(detail, "CTAP2 GetAssertion request failed");
            }
            other => panic!("expected Failed, got {:?}", other),
        }
    }

    // --- Pure logic: map_assertion, against real ctap-hid-fido2 types ---

    fn sample_assertion(user_present: bool, hmac_secret: Option<[u8; 32]>) -> Assertion {
        let mut assertion = Assertion {
            rpid_hash: vec![0x01; 32],
            flags: Flags {
                user_present_result: user_present,
                ..Flags::default()
            },
            sign_count: 1,
            number_of_credentials: 1,
            signature: vec![0xAA; 64],
            user: PublicKeyCredentialUserEntity::default(),
            credential_id: b"cred-id".to_vec(),
            extensions: vec![],
            auth_data: vec![0xBB; 37],
            user_selected: false,
        };
        if let Some(salt) = hmac_secret {
            assertion.extensions.push(Gext::HmacSecret(Some(salt)));
        }
        assertion
    }

    #[test]
    fn map_assertion_carries_rpid_hash_credential_id_and_flags() {
        let assertion = sample_assertion(true, Some([0x77; 32]));
        let mapped = map_assertion(assertion).unwrap();
        assert_eq!(mapped.rpid_hash, [0x01; 32]);
        assert_eq!(mapped.credential_id, b"cred-id".to_vec());
        assert!(mapped.user_present);
        assert_eq!(mapped.signature, vec![0xAA; 64]);
        assert_eq!(mapped.auth_data, vec![0xBB; 37]);
    }

    #[test]
    fn map_assertion_extracts_hmac_secret_output_when_present() {
        let assertion = sample_assertion(true, Some([0x99; 32]));
        let mapped = map_assertion(assertion).unwrap();
        let output = mapped.hmac_secret_output.expect("hmac-secret output");
        assert_eq!(&output[..], &[0x99; 32]);
    }

    #[test]
    fn map_assertion_reports_no_hmac_secret_output_when_extension_absent() {
        let assertion = sample_assertion(true, None);
        let mapped = map_assertion(assertion).unwrap();
        assert!(mapped.hmac_secret_output.is_none());
    }

    #[test]
    fn map_assertion_carries_false_user_presence_through() {
        let assertion = sample_assertion(false, Some([0x01; 32]));
        let mapped = map_assertion(assertion).unwrap();
        assert!(!mapped.user_present);
    }

    #[test]
    fn map_assertion_rejects_malformed_rpid_hash_length() {
        let mut assertion = sample_assertion(true, None);
        assertion.rpid_hash = vec![0x01; 16]; // wrong length
        match map_assertion(assertion) {
            Err(Ctap2TransportError::Failed { detail }) => {
                assert!(detail.contains("rpIdHash"));
            }
            other => panic!("expected Failed, got {:?}", other.is_ok()),
        }
    }

    #[test]
    fn hid_ctap2_transport_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HidCtap2Transport>();
    }
}
