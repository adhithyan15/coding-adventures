//! Exact identity binding between an audited OAuth request and reviewed server.
//!
//! This crate is an in-memory evidence composition. It performs no request-byte
//! access, audit, parsing, cryptography, revocation, TLS, HTTP, I/O, or external
//! effect and grants no HTTPS, OAuth, network, or live-provider authority.

use core::fmt;

use https_reviewed_server_path_binding::{
    BoundHttpsDnsDestinationToReviewedDirectNoCheckCompleteFixedPathRevocation,
    BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature,
};
use oauth_client_secret_http1_release::ReleasedClientSecretHttp1Request;
use x509_ocsp_authenticated_target_revocation::AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation;
use x509_ocsp_reviewed_server_revocation_binding::BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation;
use x509_rsa_reviewed_server_path_anchor_self_signature_binding::BoundX509RsaReviewedServerPathAnchorSelfSignature;

mod sealed {
    pub struct DestinationIdentity<'a> {
        pub certificate: *const (),
        pub server_name: &'a [u8],
        pub host: &'a [u8],
        pub port: Option<u16>,
    }

    pub struct ReviewedPathIdentity<'a> {
        pub destination: DestinationIdentity<'a>,
        pub target_nearest_issuer: *const (),
        pub reviewed_anchor: *const (),
        pub processed_intermediate_count: usize,
    }

    pub trait RequestEvidence {
        fn destination_identity(&self) -> DestinationIdentity<'_>;
    }

    pub trait ReviewedServerEvidence {
        fn path_identity(&self) -> ReviewedPathIdentity<'_>;
    }

    pub trait DirectRevocationEvidence {
        fn path_identity(&self) -> ReviewedPathIdentity<'_>;
    }

    pub trait SignedReviewedServerRevocationEvidence {
        fn target_nearest_issuer(&self) -> *const ();
        fn reviewed_anchor(&self) -> *const ();
        fn processed_intermediate_count(&self) -> usize;
    }
}

/// Sealed complete client-secret HTTP/1 request evidence admitted by this crate.
///
/// Only the repository-owned audited release type implements this trait.
pub trait CompleteClientSecretHttp1RequestEvidence: sealed::RequestEvidence {}

impl<T: sealed::RequestEvidence> CompleteClientSecretHttp1RequestEvidence for T {}

/// Sealed signed reviewed HTTPS server-path evidence admitted by this crate.
///
/// Only the repository-owned destination/path/anchor-self-signature type
/// implements this trait.
pub trait SignedReviewedHttpsServerEvidence: sealed::ReviewedServerEvidence {}

impl<T: sealed::ReviewedServerEvidence> SignedReviewedHttpsServerEvidence for T {}

/// Sealed direct no-check complete fixed-path revocation evidence.
///
/// Only the repository-owned destination-bound direct completion whose
/// retained server-status proof includes the exact signed reviewed path and
/// anchor-self-signature shape implements this trait. It remains distinct from
/// signed reviewed server-path evidence.
pub trait DirectNoCheckCompleteFixedPathRevocationEvidence:
    sealed::DirectRevocationEvidence
{
}

impl<T: sealed::DirectRevocationEvidence> DirectNoCheckCompleteFixedPathRevocationEvidence for T {}

impl<
        'policy,
        'request_line,
        'destination,
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'request,
        'length,
        'content_type,
    > sealed::RequestEvidence
    for ReleasedClientSecretHttp1Request<
        'policy,
        'request_line,
        'destination,
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'request,
        'length,
        'content_type,
    >
{
    fn destination_identity(&self) -> sealed::DestinationIdentity<'_> {
        let destination = self.destination().destination();
        sealed::DestinationIdentity {
            certificate: core::ptr::from_ref(destination.certificate()).cast(),
            server_name: destination.server_name().as_str().as_bytes(),
            host: destination.host_authority().host(),
            port: destination.host_authority().port(),
        }
    }
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'revocation_target,
        'revocation_target_encoded,
        'revocation_issuer,
        'revocation_issuer_encoded,
        'evidence,
        'initialized,
        'path,
        'path_target,
        'path_target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    > sealed::SignedReviewedServerRevocationEvidence
    for BoundX509OcspRoutedIssuerAccumulatedNamePolicyReviewedServerRevocation<
        AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
            'binding,
            'basic_response,
            'basic_encoded,
            'single_response,
            'single_encoded,
            'responder,
            'responder_encoded,
            'revocation_target,
            'revocation_target_encoded,
            'revocation_issuer,
            'revocation_issuer_encoded,
        >,
        BoundX509RsaReviewedServerPathAnchorSelfSignature<
            'evidence,
            'initialized,
            'path,
            'path_target,
            'path_target_encoded,
            'candidates,
            'candidate_encoded,
            'policy,
            'anchor_certificate,
            'anchor_encoded,
            'role_certificate,
            'role_encoded,
            'signature_certificate,
            'signature_encoded,
            'signature_issuer_certificate,
            'signature_issuer_encoded,
            'signature_anchor_certificate,
            'signature_anchor_encoded,
        >,
    >
{
    fn target_nearest_issuer(&self) -> *const () {
        let target_closure = self
            .strong_name_policy_path()
            .reviewed_server_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure();
        core::ptr::from_ref(
            target_closure
                .target_signature()
                .signature()
                .issuer_rsa_algorithm_policy()
                .certificate(),
        )
        .cast()
    }

    fn reviewed_anchor(&self) -> *const () {
        let target_closure = self
            .strong_name_policy_path()
            .reviewed_server_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure();
        core::ptr::from_ref(
            target_closure
                .accumulated_intermediates()
                .reviewed_terminal()
                .reviewed_anchor()
                .candidate_certificate(),
        )
        .cast()
    }

    fn processed_intermediate_count(&self) -> usize {
        self.strong_name_policy_path()
            .reviewed_server_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure()
            .accumulated_intermediates()
            .processed_intermediate_count()
    }
}

impl<'server_name, 'host, 'target, 'target_encoded, Proof, IntermediateRevocation>
    sealed::DirectRevocationEvidence
    for BoundHttpsDnsDestinationToReviewedDirectNoCheckCompleteFixedPathRevocation<
        'server_name,
        'host,
        'target,
        'target_encoded,
        Proof,
        IntermediateRevocation,
    >
where
    Proof: sealed::SignedReviewedServerRevocationEvidence,
{
    fn path_identity(&self) -> sealed::ReviewedPathIdentity<'_> {
        let destination = self.destination();
        let reviewed_server_revocation = self
            .complete_revocation()
            .intermediate_status_responder_no_check_policy()
            .anchor_dns_policy()
            .responder_path()
            .no_check_intermediate_revocation()
            .responder_no_check_policy()
            .intermediate_dns_policy()
            .anchor_policy()
            .responder_path()
            .reviewed_server_revocation();
        sealed::ReviewedPathIdentity {
            destination: sealed::DestinationIdentity {
                certificate: core::ptr::from_ref(destination.certificate()).cast(),
                server_name: destination.server_name().as_str().as_bytes(),
                host: destination.host_authority().host(),
                port: destination.host_authority().port(),
            },
            target_nearest_issuer: reviewed_server_revocation.target_nearest_issuer(),
            reviewed_anchor: reviewed_server_revocation.reviewed_anchor(),
            processed_intermediate_count: reviewed_server_revocation.processed_intermediate_count(),
        }
    }
}

impl<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    > sealed::ReviewedServerEvidence
    for BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature<
        'server_name,
        'host,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'signature_certificate,
        'signature_encoded,
        'signature_issuer_certificate,
        'signature_issuer_encoded,
        'signature_anchor_certificate,
        'signature_anchor_encoded,
    >
{
    fn path_identity(&self) -> sealed::ReviewedPathIdentity<'_> {
        let destination = self.destination();
        let target_closure = self
            .reviewed_server_path_anchor_self_signature()
            .reviewed_server_path()
            .subordinate_policy()
            .target_policy()
            .anchor_policy()
            .target_signature_closure();
        sealed::ReviewedPathIdentity {
            destination: sealed::DestinationIdentity {
                certificate: core::ptr::from_ref(destination.certificate()).cast(),
                server_name: destination.server_name().as_str().as_bytes(),
                host: destination.host_authority().host(),
                port: destination.host_authority().port(),
            },
            target_nearest_issuer: core::ptr::from_ref(
                target_closure
                    .target_signature()
                    .signature()
                    .issuer_rsa_algorithm_policy()
                    .certificate(),
            )
            .cast(),
            reviewed_anchor: core::ptr::from_ref(
                target_closure
                    .accumulated_intermediates()
                    .reviewed_terminal()
                    .reviewed_anchor()
                    .candidate_certificate(),
            )
            .cast(),
            processed_intermediate_count: target_closure
                .accumulated_intermediates()
                .processed_intermediate_count(),
        }
    }
}

/// Stable payload-free request/server binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretReviewedServerBindingError {
    /// The retained certificate, SNI, or Host identity differs.
    DestinationMismatch,
    /// The retained target-nearest issuer or reviewed anchor differs.
    PathMismatch,
    /// Either retained path does not contain exactly one intermediate.
    UnsupportedPathShape,
}

impl fmt::Display for ClientSecretReviewedServerBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DestinationMismatch => formatter
                .write_str("OAuth client-secret request and reviewed server destination mismatch"),
            Self::PathMismatch => formatter.write_str(
                "OAuth signed reviewed server and direct revocation path identity mismatch",
            ),
            Self::UnsupportedPathShape => formatter
                .write_str("OAuth direct revocation composition requires exactly one intermediate"),
        }
    }
}

impl std::error::Error for ClientSecretReviewedServerBindingError {}

/// One complete audited request bound to one signed reviewed HTTPS server path.
///
/// This non-copyable value retains both complete inputs. It grants no access to
/// request bytes and no TLS, HTTP, I/O, or external-effect authority.
pub struct BoundClientSecretRequestToSignedReviewedHttpsServer<Request, ReviewedServer> {
    request: Request,
    reviewed_server: ReviewedServer,
}

impl<Request, ReviewedServer>
    BoundClientSecretRequestToSignedReviewedHttpsServer<Request, ReviewedServer>
{
    /// Return the exact complete request retained by this proof.
    #[must_use]
    pub const fn request(&self) -> &Request {
        &self.request
    }

    /// Return the exact signed reviewed server proof retained by this result.
    #[must_use]
    pub const fn reviewed_server(&self) -> &ReviewedServer {
        &self.reviewed_server
    }
}

impl<Request, ReviewedServer> fmt::Debug
    for BoundClientSecretRequestToSignedReviewedHttpsServer<Request, ReviewedServer>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundClientSecretRequestToSignedReviewedHttpsServer")
            .field("request", &"[REDACTED]")
            .field("reviewed_server", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Bind one audited complete request to one signed reviewed HTTPS server path.
///
/// Both complete inputs are moved into the result only when their exact
/// decoded certificate object, SNI DNS name, Host DNS name, and optional Host
/// port agree. DNS comparisons are ASCII case-insensitive. This function never
/// accesses the request's credential-bearing bytes and emits no audit event.
pub fn bind_client_secret_request_to_signed_reviewed_https_server<Request, ReviewedServer>(
    request: Request,
    reviewed_server: ReviewedServer,
) -> Result<
    BoundClientSecretRequestToSignedReviewedHttpsServer<Request, ReviewedServer>,
    ClientSecretReviewedServerBindingError,
>
where
    Request: CompleteClientSecretHttp1RequestEvidence,
    ReviewedServer: SignedReviewedHttpsServerEvidence,
{
    let request_identity = sealed::RequestEvidence::destination_identity(&request);
    let server_identity = sealed::ReviewedServerEvidence::path_identity(&reviewed_server);

    if !same_destination(request_identity, server_identity.destination) {
        return Err(ClientSecretReviewedServerBindingError::DestinationMismatch);
    }

    Ok(BoundClientSecretRequestToSignedReviewedHttpsServer {
        request,
        reviewed_server,
    })
}

fn same_destination(
    left: sealed::DestinationIdentity<'_>,
    right: sealed::DestinationIdentity<'_>,
) -> bool {
    core::ptr::eq(left.certificate, right.certificate)
        && left.server_name.eq_ignore_ascii_case(right.server_name)
        && left.host.eq_ignore_ascii_case(right.host)
        && left.port == right.port
}

/// Audited request and signed server evidence bound to direct revocation.
///
/// This non-copyable value retains the existing complete request/server
/// binding and the exact destination-bound direct no-check complete fixed-path
/// revocation proof. It grants no request-byte, TLS, HTTP, I/O, or
/// external-effect authority.
pub struct BoundClientSecretRequestToSignedReviewedHttpsServerAndDirectRevocation<
    Request,
    ReviewedServer,
    DirectRevocation,
> {
    request_and_reviewed_server:
        BoundClientSecretRequestToSignedReviewedHttpsServer<Request, ReviewedServer>,
    direct_revocation: DirectRevocation,
}

impl<Request, ReviewedServer, DirectRevocation>
    BoundClientSecretRequestToSignedReviewedHttpsServerAndDirectRevocation<
        Request,
        ReviewedServer,
        DirectRevocation,
    >
{
    /// Return the retained complete request/signed-server binding.
    #[must_use]
    pub const fn request_and_reviewed_server(
        &self,
    ) -> &BoundClientSecretRequestToSignedReviewedHttpsServer<Request, ReviewedServer> {
        &self.request_and_reviewed_server
    }

    /// Return the exact retained direct complete-revocation proof.
    #[must_use]
    pub const fn direct_revocation(&self) -> &DirectRevocation {
        &self.direct_revocation
    }
}

impl<Request, ReviewedServer, DirectRevocation> fmt::Debug
    for BoundClientSecretRequestToSignedReviewedHttpsServerAndDirectRevocation<
        Request,
        ReviewedServer,
        DirectRevocation,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundClientSecretRequestToSignedReviewedHttpsServerAndDirectRevocation")
            .field("request_and_reviewed_server", &"[REDACTED]")
            .field("direct_revocation", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Bind the audited request/signed-server proof to complete direct revocation.
///
/// Both complete inputs are moved into the result only when their exact
/// decoded leaf, target-nearest issuer, and reviewed-anchor objects, SNI DNS
/// name, Host DNS name, and optional Host port agree. DNS comparisons are
/// ASCII case-insensitive. Each retained path must contain exactly one
/// processed intermediate, matching the target-plus-intermediate coverage of
/// this direct completion. This function never accesses credential-bearing
/// request bytes and emits no audit event.
pub fn bind_client_secret_request_to_signed_reviewed_https_server_and_direct_no_check_complete_fixed_path_revocation<
    Request,
    ReviewedServer,
    DirectRevocation,
>(
    request_and_reviewed_server: BoundClientSecretRequestToSignedReviewedHttpsServer<
        Request,
        ReviewedServer,
    >,
    direct_revocation: DirectRevocation,
) -> Result<
    BoundClientSecretRequestToSignedReviewedHttpsServerAndDirectRevocation<
        Request,
        ReviewedServer,
        DirectRevocation,
    >,
    ClientSecretReviewedServerBindingError,
>
where
    ReviewedServer: SignedReviewedHttpsServerEvidence,
    DirectRevocation: DirectNoCheckCompleteFixedPathRevocationEvidence,
{
    let server_identity = sealed::ReviewedServerEvidence::path_identity(
        request_and_reviewed_server.reviewed_server(),
    );
    let revocation_identity = sealed::DirectRevocationEvidence::path_identity(&direct_revocation);

    if !same_destination(server_identity.destination, revocation_identity.destination) {
        return Err(ClientSecretReviewedServerBindingError::DestinationMismatch);
    }
    if server_identity.processed_intermediate_count != 1
        || revocation_identity.processed_intermediate_count != 1
    {
        return Err(ClientSecretReviewedServerBindingError::UnsupportedPathShape);
    }
    if !core::ptr::eq(
        server_identity.target_nearest_issuer,
        revocation_identity.target_nearest_issuer,
    ) || !core::ptr::eq(
        server_identity.reviewed_anchor,
        revocation_identity.reviewed_anchor,
    ) {
        return Err(ClientSecretReviewedServerBindingError::PathMismatch);
    }

    Ok(
        BoundClientSecretRequestToSignedReviewedHttpsServerAndDirectRevocation {
            request_and_reviewed_server,
            direct_revocation,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    static CERTIFICATE_A: u8 = 1;
    static CERTIFICATE_B: u8 = 2;
    static ISSUER_A: u8 = 3;
    static ISSUER_B: u8 = 4;
    static ANCHOR_A: u8 = 5;
    static ANCHOR_B: u8 = 6;

    struct RequestEvidence {
        identity: TestIdentity,
        retained: u8,
    }

    struct ReviewedServerEvidence {
        identity: TestIdentity,
        retained: u8,
    }

    struct DirectRevocationEvidence {
        identity: TestIdentity,
        retained: u8,
    }

    #[derive(Clone, Copy)]
    struct TestIdentity {
        certificate: &'static u8,
        server_name: &'static [u8],
        host: &'static [u8],
        port: Option<u16>,
        target_nearest_issuer: &'static u8,
        reviewed_anchor: &'static u8,
        processed_intermediate_count: usize,
    }

    impl sealed::RequestEvidence for RequestEvidence {
        fn destination_identity(&self) -> sealed::DestinationIdentity<'_> {
            identity(self.identity)
        }
    }

    impl sealed::ReviewedServerEvidence for ReviewedServerEvidence {
        fn path_identity(&self) -> sealed::ReviewedPathIdentity<'_> {
            path_identity(self.identity)
        }
    }

    impl sealed::DirectRevocationEvidence for DirectRevocationEvidence {
        fn path_identity(&self) -> sealed::ReviewedPathIdentity<'_> {
            path_identity(self.identity)
        }
    }

    fn identity(identity: TestIdentity) -> sealed::DestinationIdentity<'static> {
        sealed::DestinationIdentity {
            certificate: core::ptr::from_ref(identity.certificate).cast(),
            server_name: identity.server_name,
            host: identity.host,
            port: identity.port,
        }
    }

    fn path_identity(value: TestIdentity) -> sealed::ReviewedPathIdentity<'static> {
        sealed::ReviewedPathIdentity {
            destination: identity(value),
            target_nearest_issuer: core::ptr::from_ref(value.target_nearest_issuer).cast(),
            reviewed_anchor: core::ptr::from_ref(value.reviewed_anchor).cast(),
            processed_intermediate_count: value.processed_intermediate_count,
        }
    }

    fn request(identity: TestIdentity) -> RequestEvidence {
        RequestEvidence {
            identity,
            retained: 7,
        }
    }

    fn reviewed_server(identity: TestIdentity) -> ReviewedServerEvidence {
        ReviewedServerEvidence {
            identity,
            retained: 9,
        }
    }

    fn direct_revocation(identity: TestIdentity) -> DirectRevocationEvidence {
        DirectRevocationEvidence {
            identity,
            retained: 11,
        }
    }

    fn bound_request_and_server(
    ) -> BoundClientSecretRequestToSignedReviewedHttpsServer<RequestEvidence, ReviewedServerEvidence>
    {
        bind_client_secret_request_to_signed_reviewed_https_server(
            request(matching_identity()),
            reviewed_server(matching_identity()),
        )
        .unwrap()
    }

    fn matching_identity() -> TestIdentity {
        TestIdentity {
            certificate: &CERTIFICATE_A,
            server_name: b"Token.Example",
            host: b"TOKEN.EXAMPLE",
            port: Some(443),
            target_nearest_issuer: &ISSUER_A,
            reviewed_anchor: &ANCHOR_A,
            processed_intermediate_count: 1,
        }
    }

    #[test]
    fn matching_identity_retains_both_complete_inputs() {
        let mut server_identity = matching_identity();
        server_identity.server_name = b"token.example";
        server_identity.host = b"token.example";
        let bound = bind_client_secret_request_to_signed_reviewed_https_server(
            request(matching_identity()),
            reviewed_server(server_identity),
        )
        .unwrap();

        assert_eq!(bound.request().retained, 7);
        assert_eq!(bound.reviewed_server().retained, 9);
        let debug = format!("{bound:?}");
        assert!(!debug.contains("token.example"));
        assert!(!debug.contains('7'));
        assert!(!debug.contains('9'));
    }

    #[test]
    fn every_destination_substitution_fails_closed_without_payloads() {
        let mismatches = [
            TestIdentity {
                certificate: &CERTIFICATE_B,
                ..matching_identity()
            },
            TestIdentity {
                server_name: b"other.example",
                ..matching_identity()
            },
            TestIdentity {
                host: b"other.example",
                ..matching_identity()
            },
            TestIdentity {
                port: None,
                ..matching_identity()
            },
        ];

        for mismatch in mismatches {
            let error = bind_client_secret_request_to_signed_reviewed_https_server(
                request(matching_identity()),
                reviewed_server(mismatch),
            )
            .unwrap_err();
            assert_eq!(
                error,
                ClientSecretReviewedServerBindingError::DestinationMismatch
            );
            assert_eq!(format!("{error:?}"), "DestinationMismatch");
            for payload in ["token.example", "other.example", "443"] {
                assert!(!error.to_string().contains(payload));
            }
        }
    }

    #[test]
    fn matching_direct_revocation_retains_every_complete_input() {
        let mut revocation_identity = matching_identity();
        revocation_identity.server_name = b"token.example";
        revocation_identity.host = b"token.example";
        let bound = bind_client_secret_request_to_signed_reviewed_https_server_and_direct_no_check_complete_fixed_path_revocation(
            bound_request_and_server(),
            direct_revocation(revocation_identity),
        )
        .unwrap();

        assert_eq!(bound.request_and_reviewed_server().request().retained, 7);
        assert_eq!(
            bound
                .request_and_reviewed_server()
                .reviewed_server()
                .retained,
            9
        );
        assert_eq!(bound.direct_revocation().retained, 11);
        let debug = format!("{bound:?}");
        for payload in ["token.example", "7", "9", "11"] {
            assert!(!debug.contains(payload));
        }
    }

    #[test]
    fn every_direct_revocation_destination_substitution_fails_closed() {
        let mismatches = [
            TestIdentity {
                certificate: &CERTIFICATE_B,
                ..matching_identity()
            },
            TestIdentity {
                server_name: b"other.example",
                ..matching_identity()
            },
            TestIdentity {
                host: b"other.example",
                ..matching_identity()
            },
            TestIdentity {
                port: None,
                ..matching_identity()
            },
        ];

        for mismatch in mismatches {
            let error = bind_client_secret_request_to_signed_reviewed_https_server_and_direct_no_check_complete_fixed_path_revocation(
                bound_request_and_server(),
                direct_revocation(mismatch),
            )
            .unwrap_err();
            assert_eq!(
                error,
                ClientSecretReviewedServerBindingError::DestinationMismatch
            );
            assert_eq!(format!("{error:?}"), "DestinationMismatch");
            for payload in ["token.example", "other.example", "443"] {
                assert!(!error.to_string().contains(payload));
            }
        }
    }

    #[test]
    fn alternate_direct_revocation_path_substitution_fails_closed() {
        let mismatches = [
            TestIdentity {
                target_nearest_issuer: &ISSUER_B,
                ..matching_identity()
            },
            TestIdentity {
                reviewed_anchor: &ANCHOR_B,
                ..matching_identity()
            },
        ];

        for mismatch in mismatches {
            let error = bind_client_secret_request_to_signed_reviewed_https_server_and_direct_no_check_complete_fixed_path_revocation(
                bound_request_and_server(),
                direct_revocation(mismatch),
            )
            .unwrap_err();
            assert_eq!(error, ClientSecretReviewedServerBindingError::PathMismatch);
            assert_eq!(format!("{error:?}"), "PathMismatch");
            for payload in ["token.example", "issuer", "anchor"] {
                assert!(!error.to_string().contains(payload));
            }
        }
    }

    #[test]
    fn unsupported_direct_revocation_path_depth_fails_closed() {
        for processed_intermediate_count in [0, 2] {
            let mut server_identity = matching_identity();
            server_identity.processed_intermediate_count = processed_intermediate_count;
            let error = bind_client_secret_request_to_signed_reviewed_https_server_and_direct_no_check_complete_fixed_path_revocation(
                bind_client_secret_request_to_signed_reviewed_https_server(
                    request(matching_identity()),
                    reviewed_server(server_identity),
                )
                .unwrap(),
                direct_revocation(matching_identity()),
            )
            .unwrap_err();
            assert_eq!(
                error,
                ClientSecretReviewedServerBindingError::UnsupportedPathShape
            );
            assert_eq!(format!("{error:?}"), "UnsupportedPathShape");
            assert!(!error
                .to_string()
                .contains(&processed_intermediate_count.to_string()));

            let mut revocation_identity = matching_identity();
            revocation_identity.processed_intermediate_count = processed_intermediate_count;
            let error = bind_client_secret_request_to_signed_reviewed_https_server_and_direct_no_check_complete_fixed_path_revocation(
                bound_request_and_server(),
                direct_revocation(revocation_identity),
            )
            .unwrap_err();
            assert_eq!(
                error,
                ClientSecretReviewedServerBindingError::UnsupportedPathShape
            );
            assert_eq!(format!("{error:?}"), "UnsupportedPathShape");
            assert!(!error
                .to_string()
                .contains(&processed_intermediate_count.to_string()));
        }
    }
}
