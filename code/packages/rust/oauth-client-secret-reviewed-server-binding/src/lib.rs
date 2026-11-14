//! Exact identity binding between an audited OAuth request and reviewed server.
//!
//! This crate is an in-memory evidence composition. It performs no request-byte
//! access, audit, parsing, cryptography, revocation, TLS, HTTP, I/O, or external
//! effect and grants no HTTPS, OAuth, network, or live-provider authority.

use core::fmt;

use https_reviewed_server_path_binding::BoundHttpsDnsDestinationToReviewedServerPathAnchorSelfSignature;
use oauth_client_secret_http1_release::ReleasedClientSecretHttp1Request;

mod sealed {
    pub struct DestinationIdentity<'a> {
        pub certificate: *const (),
        pub server_name: &'a [u8],
        pub host: &'a [u8],
        pub port: Option<u16>,
    }

    pub trait RequestEvidence {
        fn destination_identity(&self) -> DestinationIdentity<'_>;
    }

    pub trait ReviewedServerEvidence {
        fn destination_identity(&self) -> DestinationIdentity<'_>;
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
    fn destination_identity(&self) -> sealed::DestinationIdentity<'_> {
        let destination = self.destination();
        sealed::DestinationIdentity {
            certificate: core::ptr::from_ref(destination.certificate()).cast(),
            server_name: destination.server_name().as_str().as_bytes(),
            host: destination.host_authority().host(),
            port: destination.host_authority().port(),
        }
    }
}

/// Stable payload-free request/server binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretReviewedServerBindingError {
    /// The retained certificate, SNI, or Host identity differs.
    DestinationMismatch,
}

impl fmt::Display for ClientSecretReviewedServerBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OAuth client-secret request and reviewed server destination mismatch")
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
    let server_identity = sealed::ReviewedServerEvidence::destination_identity(&reviewed_server);

    if !core::ptr::eq(request_identity.certificate, server_identity.certificate)
        || !request_identity
            .server_name
            .eq_ignore_ascii_case(server_identity.server_name)
        || !request_identity
            .host
            .eq_ignore_ascii_case(server_identity.host)
        || request_identity.port != server_identity.port
    {
        return Err(ClientSecretReviewedServerBindingError::DestinationMismatch);
    }

    Ok(BoundClientSecretRequestToSignedReviewedHttpsServer {
        request,
        reviewed_server,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static CERTIFICATE_A: u8 = 1;
    static CERTIFICATE_B: u8 = 1;

    struct RequestEvidence {
        identity: TestIdentity,
        retained: u8,
    }

    struct ReviewedServerEvidence {
        identity: TestIdentity,
        retained: u8,
    }

    #[derive(Clone, Copy)]
    struct TestIdentity {
        certificate: &'static u8,
        server_name: &'static [u8],
        host: &'static [u8],
        port: Option<u16>,
    }

    impl sealed::RequestEvidence for RequestEvidence {
        fn destination_identity(&self) -> sealed::DestinationIdentity<'_> {
            identity(self.identity)
        }
    }

    impl sealed::ReviewedServerEvidence for ReviewedServerEvidence {
        fn destination_identity(&self) -> sealed::DestinationIdentity<'_> {
            identity(self.identity)
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

    fn matching_identity() -> TestIdentity {
        TestIdentity {
            certificate: &CERTIFICATE_A,
            server_name: b"Token.Example",
            host: b"TOKEN.EXAMPLE",
            port: Some(443),
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
}
