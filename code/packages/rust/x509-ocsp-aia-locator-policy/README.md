# x509-ocsp-aia-locator-policy

`x509-ocsp-aia-locator-policy` is the effect-free policy layer above the
repository-owned `x509-authority-information-access` decoder. It finds every
exact `id-ad-ocsp` description, validates each locator against one strict
absolute-URI profile and caller-supplied scheme/port policy, and succeeds only
when exactly one locator remains.

The accepted profile is deliberately narrower than general RFC 3986. It
requires `scheme://dns-host`, rejects userinfo, IP literals, fragments, empty
or malformed DNS labels, legacy numeric IPv4 resolver spellings, and malformed
ports, and admits only RFC 3986
path/query characters with complete percent escapes. The caller supplies the
allowed lowercase schemes, their default ports, the exact explicit ports, and
all input length ceilings. There is no built-in provider or transport policy.

## Example

```rust
use x509_ocsp_aia_locator_policy::{
    select_unique_x509_ocsp_aia_locator, X509OcspAiaLocatorPolicy,
    X509OcspAiaSchemePolicy,
};

# fn select<'a>(
#     aia: &'a x509_authority_information_access::AuthorityInformationAccess<'a>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let https = X509OcspAiaSchemePolicy::new("https", 443, &[8443]);
let policy = X509OcspAiaLocatorPolicy::new(&[https], 2048, 253, 1024);
let selected = select_unique_x509_ocsp_aia_locator(aia, &policy)?;

assert_eq!(selected.scheme_policy().scheme(), "https");
println!("selected OCSP host: {}", selected.host());
# Ok(())
# }
```

## Security boundary

Policy configuration is validated before locator content. Every exact OCSP
description must pass; malformed or disallowed alternatives cannot be silently
ignored. Multiple acceptable descriptions are ambiguous and fail closed.
Failures and `Debug` output retain no URI, host, certificate, or provider data.

Success is borrowed selection evidence only. It does not authorize DNS or a
network destination, establish public-suffix or hostname identity, resolve an
address, build an HTTP request, follow redirects, fetch or cache OCSP data,
authenticate a response, read a clock, decide revocation, validate a path,
establish trust, perform TLS or HTTP, authorize OAuth transport, open a socket,
or access a credential. Concrete HTTPS remains unavailable.

## Verification

```bash
bash BUILD
cargo clippy -p x509-ocsp-aia-locator-policy --all-targets -- -D warnings
cargo doc -p x509-ocsp-aia-locator-policy --no-deps
```
