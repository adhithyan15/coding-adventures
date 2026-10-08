//! `net.fetch` tests, grouped by D18V rule. No test touches DNS or a socket:
//! a fake resolver answers lookups, and a fake transport records the exact
//! bytes that would have gone on the wire and plays back a canned response.

use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;

use chief_of_staff_agent_manifest::Capability;
use chief_of_staff_net_fetch::{
    is_public, parse_request, read_limited, tool_definition, CredentialSource, FetchError,
    FetchRequest, Limits, NetAllowlist, NetFetch, Resolver, Transport, ALLOWED_REQUEST_HEADERS,
    MAX_RESPONSE_BODY_BYTES, NET_FETCH_TOOL_ID,
};
use coding_adventures_json_value::{parse, JsonValue};
use coding_adventures_zeroize::Zeroizing;

// ── Fakes ────────────────────────────────────────────────────────────────────

struct FakeResolver(Vec<IpAddr>);

impl Resolver for FakeResolver {
    fn resolve(&self, _host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        Ok(self.0.iter().map(|ip| SocketAddr::new(*ip, port)).collect())
    }
}

struct FailingResolver;

impl Resolver for FailingResolver {
    fn resolve(&self, _: &str, _: u16) -> std::io::Result<Vec<SocketAddr>> {
        Err(std::io::Error::other("nxdomain"))
    }
}

/// Records what was sent and returns `response`.
struct FakeTransport {
    response: Vec<u8>,
    sent: Mutex<Option<(String, SocketAddr, Vec<u8>)>>,
}

impl FakeTransport {
    fn returning(response: &[u8]) -> Self {
        Self {
            response: response.to_vec(),
            sent: Mutex::new(None),
        }
    }
    fn sent(&self) -> (String, SocketAddr, String) {
        let (name, address, bytes) = self.sent.lock().unwrap().clone().expect("nothing sent");
        (name, address, String::from_utf8(bytes).unwrap())
    }
    fn nothing_sent(&self) -> bool {
        self.sent.lock().unwrap().is_none()
    }
}

impl Transport for FakeTransport {
    fn exchange(
        &self,
        server_name: &str,
        address: SocketAddr,
        request: &[u8],
        limits: &Limits,
    ) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        *self.sent.lock().unwrap() = Some((server_name.to_string(), address, request.to_vec()));
        let mut reader = self.response.as_slice();
        read_limited(&mut reader, limits.max_wire_bytes)
    }
}

/// A lease store of one: hands the secret out once, and counts redemptions
/// so tests can prove a refusal did not consume it.
struct OneLease {
    vault_ref: &'static str,
    secret: &'static [u8],
    redeemed: RefCell<u32>,
}

impl OneLease {
    fn new(secret: &'static [u8]) -> Self {
        Self {
            vault_ref: "vault-lease:abc",
            secret,
            redeemed: RefCell::new(0),
        }
    }
}

impl CredentialSource for OneLease {
    fn redeem(&self, vault_ref: &str) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        if vault_ref != self.vault_ref || *self.redeemed.borrow() > 0 {
            return Err(FetchError::CredentialRefused);
        }
        *self.redeemed.borrow_mut() += 1;
        Ok(Zeroizing::new(self.secret.to_vec()))
    }
}

fn capability(action: &str, target: &str) -> Capability {
    Capability {
        category: "net".into(),
        action: action.into(),
        target: target.into(),
        justification: "test".into(),
    }
}

fn weather_allowlist() -> NetAllowlist {
    NetAllowlist::from_capabilities(&[
        capability("dns", "api.weather.gov"),
        capability("connect", "api.weather.gov:443"),
    ])
    .unwrap()
    .unwrap()
}

fn public() -> FakeResolver {
    FakeResolver(vec!["203.0.113.10".parse().unwrap()])
}

// 203.0.113.0/24 is documentation space, which `is_public` refuses; tests
// that should reach the transport use a genuinely public-looking address.
fn routable() -> FakeResolver {
    FakeResolver(vec!["93.184.215.14".parse().unwrap()])
}

fn request(json: &str) -> FetchRequest {
    parse_request(&parse(json).unwrap()).unwrap()
}

fn get(url: &str) -> FetchRequest {
    request(&format!(r#"{{"url":"{url}","method":"GET"}}"#))
}

const OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\nSet-Cookie: session=1\r\n\r\n{\"rain\":true}";

// ── Parsing (V-R4 and the input contract) ────────────────────────────────────

#[test]
fn parses_a_full_request() {
    let parsed = request(
        r#"{"url":"https://api.weather.gov/points/1,2","method":"POST",
            "headers":{"Accept":"application/json","user-agent":"me"},
            "body":"{}",
            "credential":{"vault_ref":"vault-lease:abc","header":"x-api-key","scheme":"Token"}}"#,
    );
    assert_eq!(
        parsed.headers[0],
        ("accept".to_string(), "application/json".to_string())
    );
    assert_eq!(parsed.body.as_deref(), Some("{}"));
    let credential = parsed.credential.as_ref().unwrap();
    assert!(
        !format!("{credential:?}").contains("abc"),
        "vault_ref is redacted"
    );
}

#[test]
fn refuses_every_malformed_argument() {
    let long_url = format!(
        r#"{{"url":"https://a.b/{}","method":"GET"}}"#,
        "x".repeat(2050)
    );
    let too_many: String = (0..17)
        .map(|i| format!(r#""accept{i}":"x""#))
        .collect::<Vec<_>>()
        .join(",");
    let cases = [
        r#"[]"#.to_string(),
        r#"{"url":"https://a.b","method":"GET","extra":1}"#.to_string(),
        r#"{"method":"GET"}"#.to_string(),
        r#"{"url":7,"method":"GET"}"#.to_string(),
        r#"{"url":"https://a.b","method":"DELETE"}"#.to_string(),
        r#"{"url":"https://a.b","url":"https://c.d","method":"GET"}"#.to_string(),
        long_url,
        r#"{"url":"https://a.b","method":"GET","headers":[]}"#.to_string(),
        format!(r#"{{"url":"https://a.b","method":"GET","headers":{{{too_many}}}}}"#),
        r#"{"url":"https://a.b","method":"GET","headers":{"cookie":"x"}}"#.to_string(),
        r#"{"url":"https://a.b","method":"GET","headers":{"authorization":"x"}}"#.to_string(),
        r#"{"url":"https://a.b","method":"GET","headers":{"accept":"a","Accept":"b"}}"#.to_string(),
        r#"{"url":"https://a.b","method":"GET","headers":{"accept":1}}"#.to_string(),
        r#"{"url":"https://a.b","method":"GET","headers":{"accept":"a\r\nx-evil: 1"}}"#.to_string(),
        format!(
            r#"{{"url":"https://a.b","method":"GET","headers":{{"accept":"{}"}}}}"#,
            "a".repeat(1025)
        ),
        r#"{"url":"https://a.b","method":"GET","body":"x"}"#.to_string(),
        r#"{"url":"https://a.b","method":"POST","body":5}"#.to_string(),
        format!(
            r#"{{"url":"https://a.b","method":"POST","body":"{}"}}"#,
            "a".repeat(64 * 1024 + 1)
        ),
        r#"{"url":"https://a.b","method":"GET","credential":"x"}"#.to_string(),
        r#"{"url":"https://a.b","method":"GET","credential":{"vault_ref":"r","header":"cookie"}}"#
            .to_string(),
        r#"{"url":"https://a.b","method":"GET","credential":{"vault_ref":"","header":"x-api-key"}}"#
            .to_string(),
        r#"{"url":"https://a.b","method":"GET","credential":{"vault_ref":"r","header":"x-api-key","scheme":"Digest"}}"#
            .to_string(),
        r#"{"url":"https://a.b","method":"GET","credential":{"vault_ref":"r","header":"x-api-key","scheme":1}}"#
            .to_string(),
        r#"{"url":"https://a.b","method":"GET","credential":{"vault_ref":"r","header":"x-api-key","value":"s3cret"}}"#
            .to_string(),
    ];
    for case in cases {
        let error = parse_request(&parse(&case).unwrap()).unwrap_err();
        assert!(
            matches!(error, FetchError::InvalidRequest(_)),
            "{case}: {error:?}"
        );
        assert_eq!(error.kind(), "invalid_request");
        assert!(error.to_string().starts_with("net.fetch invalid_request:"));
    }
}

#[test]
fn null_optional_fields_are_absent() {
    let parsed =
        request(r#"{"url":"https://a.b","method":"GET","headers":null,"credential":null}"#);
    assert!(parsed.headers.is_empty() && parsed.credential.is_none());
}

// ── Authorization (V-A1, V-A3) ───────────────────────────────────────────────

#[test]
fn a_manifest_without_net_connect_yields_no_allowlist() {
    assert!(NetAllowlist::from_capabilities(&[]).unwrap().is_none());
    assert!(NetAllowlist::from_capabilities(&[capability("dns", "a.b")])
        .unwrap()
        .is_none());
    let mut fs = capability("connect", "a.b:443");
    fs.category = "fs".into();
    assert!(NetAllowlist::from_capabilities(&[fs]).unwrap().is_none());
}

#[test]
fn a_malformed_signed_target_is_an_error_not_a_skip() {
    let error =
        NetAllowlist::from_capabilities(&[capability("connect", "*.weather.gov:443")]).unwrap_err();
    assert!(matches!(error, FetchError::InvalidRequest(_)));
}

#[test]
fn only_exact_declared_https_targets_are_authorized() {
    let allowlist = weather_allowlist();
    let target = allowlist
        .authorize("https://API.weather.gov./points/1?x=2")
        .unwrap();
    assert_eq!(target.host, "api.weather.gov");
    assert_eq!(target.port, 443);
    assert_eq!(target.path_and_query, "/points/1?x=2");
    assert_eq!(
        allowlist
            .authorize("https://api.weather.gov")
            .unwrap()
            .path_and_query,
        "/"
    );

    for refused in [
        "http://api.weather.gov/",
        "https://evil.example/",
        "https://sub.api.weather.gov/",
        "https://api.weather.gov:8443/",
        "https://user@api.weather.gov/",
        "https://api.weather.gov/#frag",
        "not a url",
    ] {
        assert_eq!(
            allowlist.authorize(refused),
            Err(FetchError::Unauthorized),
            "{refused}"
        );
    }
}

#[test]
fn ip_literal_hosts_are_refused_even_when_declared() {
    let allowlist = NetAllowlist::from_capabilities(&[
        capability("dns", "127.0.0.1"),
        capability("connect", "127.0.0.1:443"),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        allowlist.authorize("https://127.0.0.1/"),
        Err(FetchError::Unauthorized)
    );
}

// ── Addresses (V-R2) ─────────────────────────────────────────────────────────

#[test]
fn is_public_refuses_every_internal_range() {
    for internal in [
        "127.0.0.1",
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "0.0.0.0",
        "0.1.2.3",
        "255.255.255.255",
        "224.0.0.1",
        "192.0.2.1",
        "198.51.100.1",
        "203.0.113.1",
        "192.0.0.8",
        "198.18.0.1",
        "240.0.0.1",
        "::1",
        "::",
        "fc00::1",
        "fd12::1",
        "fe80::1",
        "fec0::1",
        "ff02::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::7f00:1",
    ] {
        assert!(
            !is_public(internal.parse().unwrap()),
            "{internal} must be refused"
        );
    }
    for external in [
        "93.184.215.14",
        "1.1.1.1",
        "2606:4700::1111",
        "::ffff:8.8.8.8",
    ] {
        assert!(is_public(external.parse().unwrap()), "{external} is public");
    }
}

#[test]
fn a_name_resolving_to_any_internal_address_is_refused_before_sending() {
    let transport = FakeTransport::returning(OK);
    let mixed = FakeResolver(vec![
        "93.184.215.14".parse().unwrap(),
        "127.0.0.1".parse().unwrap(),
    ]);
    let fetch = NetFetch::new(mixed, &transport);
    let error = fetch
        .execute(&weather_allowlist(), &get("https://api.weather.gov/"), None)
        .unwrap_err();
    assert_eq!(error, FetchError::AddressRefused);
    assert!(transport.nothing_sent());

    let documentation = NetFetch::new(public(), FakeTransport::returning(OK));
    assert_eq!(
        documentation
            .execute(&weather_allowlist(), &get("https://api.weather.gov/"), None)
            .unwrap_err(),
        FetchError::AddressRefused
    );
    let empty = NetFetch::new(FakeResolver(vec![]), FakeTransport::returning(OK));
    assert_eq!(
        empty
            .execute(&weather_allowlist(), &get("https://api.weather.gov/"), None)
            .unwrap_err(),
        FetchError::AddressRefused
    );
    let failing = NetFetch::new(FailingResolver, FakeTransport::returning(OK));
    assert_eq!(
        failing
            .execute(&weather_allowlist(), &get("https://api.weather.gov/"), None)
            .unwrap_err(),
        FetchError::Network
    );
}

#[test]
fn the_connection_goes_to_the_checked_address_with_the_host_as_server_name() {
    let transport = FakeTransport::returning(OK);
    NetFetch::new(routable(), &transport)
        .execute(
            &weather_allowlist(),
            &get("https://api.weather.gov/x"),
            None,
        )
        .unwrap();
    let (server_name, address, _) = transport.sent();
    assert_eq!(server_name, "api.weather.gov");
    assert_eq!(address, "93.184.215.14:443".parse().unwrap());
}

// ── The wire and the response (V-R3..R7) ─────────────────────────────────────

#[test]
fn an_unauthenticated_get_sends_only_daemon_owned_and_allowlisted_headers() {
    let transport = FakeTransport::returning(OK);
    let response = NetFetch::new(routable(), &transport)
        .execute(
            &weather_allowlist(),
            &request(r#"{"url":"https://api.weather.gov/p?q=1","method":"GET","headers":{"accept":"application/json"}}"#),
            None,
        )
        .unwrap();
    let (_, _, wire) = transport.sent();
    assert_eq!(
        wire,
        "GET /p?q=1 HTTP/1.1\r\nhost: api.weather.gov\r\naccept: application/json\r\n\
         user-agent: chief-of-staff-net-fetch/0.1\r\naccept-encoding: identity\r\n\
         connection: close\r\n\r\n"
    );
    assert_eq!(response.status, 200);
    assert_eq!(response.body, r#"{"rain":true}"#);
    assert!(!response.truncated);
    // V-R6: set-cookie never reaches the model.
    assert_eq!(
        response.headers,
        vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("content-length".to_string(), "13".to_string()),
        ]
    );
    let json = response.to_json();
    assert!(matches!(&json, JsonValue::Object(fields) if fields.len() == 4));
}

#[test]
fn post_sends_its_body_with_a_length_and_a_custom_user_agent_wins() {
    let allowlist = NetAllowlist::from_capabilities(&[
        capability("dns", "api.example.com"),
        capability("connect", "api.example.com:8443"),
    ])
    .unwrap()
    .unwrap();
    let transport = FakeTransport::returning(OK);
    NetFetch::new(routable(), &transport)
        .execute(
            &allowlist,
            &request(r#"{"url":"https://api.example.com:8443/v","method":"POST","headers":{"user-agent":"me/1"},"body":"{\"a\":1}"}"#),
            None,
        )
        .unwrap();
    let (_, address, wire) = transport.sent();
    assert_eq!(address.port(), 8443);
    assert!(
        wire.starts_with("POST /v HTTP/1.1\r\nhost: api.example.com:8443\r\nuser-agent: me/1\r\n")
    );
    assert!(!wire.contains("chief-of-staff-net-fetch"));
    assert!(wire.ends_with("content-length: 7\r\n\r\n{\"a\":1}"));
}

#[test]
fn redirects_are_returned_not_followed() {
    let transport = FakeTransport::returning(
        b"HTTP/1.1 302 Found\r\nLocation: https://evil.example/steal\r\nContent-Length: 0\r\n\r\n",
    );
    let response = NetFetch::new(routable(), &transport)
        .execute(&weather_allowlist(), &get("https://api.weather.gov/"), None)
        .unwrap();
    assert_eq!(response.status, 302);
    assert_eq!(
        response.headers,
        vec![
            (
                "location".to_string(),
                "https://evil.example/steal".to_string()
            ),
            ("content-length".to_string(), "0".to_string()),
        ]
    );
}

fn fetch_raw(raw: &[u8]) -> Result<chief_of_staff_net_fetch::FetchResponse, FetchError> {
    NetFetch::new(routable(), FakeTransport::returning(raw)).execute(
        &weather_allowlist(),
        &get("https://api.weather.gov/"),
        None,
    )
}

#[test]
fn bodies_decode_by_length_chunks_or_eof() {
    let chunked = fetch_raw(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nwiki\r\n5\r\npedia\r\n0\r\n\r\n",
    )
    .unwrap();
    assert_eq!(chunked.body, "wikipedia");
    let eof = fetch_raw(b"HTTP/1.0 200 OK\r\n\r\nuntil the end").unwrap();
    assert_eq!(eof.body, "until the end");
    let empty = fetch_raw(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap();
    assert_eq!(empty.body, "");
}

#[test]
fn malformed_responses_are_protocol_errors() {
    for raw in [
        &b"not http at all"[..],
        b"HTTP/1.1 200 OK\r\nContent-Length: 50\r\n\r\nshort",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nwi",
        b"garbage\r\n\r\n",
    ] {
        assert_eq!(fetch_raw(raw).unwrap_err(), FetchError::Protocol, "{raw:?}");
    }
    let mut huge_head = b"HTTP/1.1 200 OK\r\n".to_vec();
    huge_head.extend(std::iter::repeat_n(b'x', 40 * 1024));
    huge_head.extend_from_slice(b": y\r\n\r\n");
    assert_eq!(fetch_raw(&huge_head).unwrap_err(), FetchError::Protocol);
}

#[test]
fn binary_bodies_are_refused() {
    assert_eq!(
        fetch_raw(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n\xff\xfe").unwrap_err(),
        FetchError::BinaryBody
    );
}

#[test]
fn oversized_bodies_are_truncated_at_the_bound() {
    let body = "a".repeat(MAX_RESPONSE_BODY_BYTES + 100);
    let mut raw = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
    raw.extend_from_slice(body.as_bytes());
    let response = fetch_raw(&raw).unwrap();
    assert!(response.truncated);
    assert_eq!(response.body.len(), MAX_RESPONSE_BODY_BYTES);

    // A stream that never ends is cut at the wire limit, not read forever.
    let mut endless = b"HTTP/1.0 200 OK\r\n\r\n".to_vec();
    endless.extend(std::iter::repeat_n(b'b', 400 * 1024));
    let response = fetch_raw(&endless).unwrap();
    assert!(response.truncated);
    assert_eq!(response.body.len(), MAX_RESPONSE_BODY_BYTES);
}

// ── Credentials (V-S1..S6) ───────────────────────────────────────────────────

const LEASED: &str = r#"{"url":"https://api.weather.gov/k","method":"GET",
    "credential":{"vault_ref":"vault-lease:abc","header":"authorization","scheme":"Bearer"}}"#;

#[test]
fn a_leased_secret_goes_into_exactly_one_header_and_is_consumed_once() {
    let lease = OneLease::new(b"tok-1234567890");
    let transport = FakeTransport::returning(OK);
    NetFetch::new(routable(), &transport)
        .execute(&weather_allowlist(), &request(LEASED), Some(&lease))
        .unwrap();
    let (_, _, wire) = transport.sent();
    assert_eq!(wire.matches("tok-1234567890").count(), 1);
    assert!(wire.contains("\r\nauthorization: Bearer tok-1234567890\r\n"));
    assert_eq!(*lease.redeemed.borrow(), 1);

    // The lease is single-use: a second call is refused by the source.
    let again = NetFetch::new(routable(), FakeTransport::returning(OK))
        .execute(&weather_allowlist(), &request(LEASED), Some(&lease))
        .unwrap_err();
    assert_eq!(again, FetchError::CredentialRefused);
}

#[test]
fn every_echo_of_the_secret_is_scrubbed() {
    let echo = b"HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain\r\nETag: tok-1234567890\r\n\
                 Content-Length: 66\r\n\r\nbad header 'Bearer tok-1234567890' (token tok-1234567890 is unknown)";
    let lease = OneLease::new(b"tok-1234567890");
    let response = NetFetch::new(routable(), FakeTransport::returning(echo))
        .execute(&weather_allowlist(), &request(LEASED), Some(&lease))
        .unwrap();
    assert!(
        !response.body.contains("tok-1234567890"),
        "{}",
        response.body
    );
    assert!(
        !response.body.contains("Bearer"),
        "the whole header value is one redaction"
    );
    assert_eq!(response.body.matches("[redacted]").count(), 2);
    assert!(response
        .headers
        .iter()
        .all(|(_, value)| !value.contains("tok-1234567890")));
}

#[test]
fn refusals_before_the_network_never_consume_the_lease() {
    // V-S6: an unauthorized URL, then an internal address.
    let lease = OneLease::new(b"tok-1234567890");
    let unauthorized = request(
        r#"{"url":"https://evil.example/","method":"GET",
            "credential":{"vault_ref":"vault-lease:abc","header":"x-api-key"}}"#,
    );
    let transport = FakeTransport::returning(OK);
    assert_eq!(
        NetFetch::new(routable(), &transport)
            .execute(&weather_allowlist(), &unauthorized, Some(&lease))
            .unwrap_err(),
        FetchError::Unauthorized
    );
    let internal = FakeResolver(vec!["10.0.0.5".parse().unwrap()]);
    assert_eq!(
        NetFetch::new(internal, &transport)
            .execute(&weather_allowlist(), &request(LEASED), Some(&lease))
            .unwrap_err(),
        FetchError::AddressRefused
    );
    assert_eq!(*lease.redeemed.borrow(), 0);
    assert!(transport.nothing_sent());
}

#[test]
fn credentials_without_a_source_or_with_unusable_secrets_are_refused() {
    let transport = FakeTransport::returning(OK);
    let fetch = NetFetch::new(routable(), &transport);
    assert_eq!(
        fetch
            .execute(&weather_allowlist(), &request(LEASED), None)
            .unwrap_err(),
        FetchError::CredentialRefused
    );
    for bad in [&b"short"[..], b"has a space in it", b"new\r\nline-header"] {
        let lease = OneLease::new(Box::leak(bad.to_vec().into_boxed_slice()));
        assert_eq!(
            fetch
                .execute(&weather_allowlist(), &request(LEASED), Some(&lease))
                .unwrap_err(),
            FetchError::CredentialRefused
        );
    }
    assert!(
        transport.nothing_sent(),
        "nothing reaches the wire with a bad secret"
    );
}

#[test]
fn errors_carry_no_request_or_response_text() {
    for error in [
        FetchError::Unauthorized,
        FetchError::CredentialRefused,
        FetchError::AddressRefused,
        FetchError::Network,
        FetchError::Tls,
        FetchError::Protocol,
        FetchError::BinaryBody,
        FetchError::Timeout,
    ] {
        assert_eq!(error.to_string(), format!("net.fetch {}", error.kind()));
    }
}

// ── The D18D definition ──────────────────────────────────────────────────────

#[test]
fn the_definition_publishes_the_header_allowlist_and_names_no_agent() {
    let definition = tool_definition();
    assert_eq!(definition.tool_id, NET_FETCH_TOOL_ID);
    let schema = format!("{:?}", definition.input_schema);
    for header in ALLOWED_REQUEST_HEADERS {
        assert!(schema.contains(header));
    }
    assert!(!schema.contains("cookie"));
    // D18S S-I7: the tool surface names no agent identity.
    assert!(chief_of_staff_tool_api::tools_naming_another_agent(&[definition]).is_empty());
}

// Transports by reference, so tests can inspect what was sent afterwards.
impl Transport for &FakeTransport {
    fn exchange(
        &self,
        server_name: &str,
        address: SocketAddr,
        request: &[u8],
        limits: &Limits,
    ) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        (**self).exchange(server_name, address, request, limits)
    }
}

// ── Remaining branches: every header and scheme, the production pieces ───────

#[test]
fn every_credential_header_and_scheme_is_written_verbatim() {
    for (header, scheme, expected) in [
        ("x-api-key", None, "x-api-key: tok-1234567890"),
        (
            "x-auth-token",
            Some("Token"),
            "x-auth-token: Token tok-1234567890",
        ),
        (
            "authorization",
            Some("Basic"),
            "authorization: Basic tok-1234567890",
        ),
    ] {
        let scheme = scheme.map_or(String::new(), |s| format!(r#","scheme":"{s}""#));
        let call = request(&format!(
            r#"{{"url":"https://api.weather.gov/","method":"GET",
                "credential":{{"vault_ref":"vault-lease:abc","header":"{header}"{scheme}}}}}"#
        ));
        let transport = FakeTransport::returning(OK);
        NetFetch::new(routable(), &transport)
            .execute(
                &weather_allowlist(),
                &call,
                Some(&OneLease::new(b"tok-1234567890")),
            )
            .unwrap();
        assert!(transport.sent().2.contains(expected), "{expected}");
    }
}

#[test]
fn bodies_past_the_wire_limit_are_truncated_whatever_their_framing() {
    let big = "c".repeat(400 * 1024);
    let mut by_length =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", big.len()).into_bytes();
    by_length.extend_from_slice(big.as_bytes());
    let response = fetch_raw(&by_length).unwrap();
    assert!(response.truncated);
    assert_eq!(response.body.len(), MAX_RESPONSE_BODY_BYTES);

    let mut chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    for _ in 0..3 {
        chunked.extend_from_slice(format!("{:x}\r\n", 100 * 1024).as_bytes());
        chunked.extend(std::iter::repeat_n(b'd', 100 * 1024));
        chunked.extend_from_slice(b"\r\n");
    }
    chunked.extend_from_slice(b"0\r\n\r\n");
    let response = fetch_raw(&chunked).unwrap();
    assert!(response.truncated);
    assert_eq!(response.body.len(), MAX_RESPONSE_BODY_BYTES);
}

struct BrokenTransport;

impl Transport for BrokenTransport {
    fn exchange(
        &self,
        _: &str,
        _: SocketAddr,
        _: &[u8],
        _: &Limits,
    ) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        Err(FetchError::Timeout)
    }
}

#[test]
fn transport_failures_surface_as_their_kind() {
    assert_eq!(
        NetFetch::new(routable(), BrokenTransport)
            .execute(&weather_allowlist(), &get("https://api.weather.gov/"), None)
            .unwrap_err(),
        FetchError::Timeout
    );
}

#[test]
fn read_limited_retries_interrupts_tolerates_unclean_eof_and_reports_errors() {
    use std::io::{Error, ErrorKind, Read};
    struct Script(Vec<Result<&'static [u8], ErrorKind>>);
    impl Read for Script {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.0.pop() {
                None => Ok(0),
                Some(Ok(bytes)) => {
                    buf[..bytes.len()].copy_from_slice(bytes);
                    Ok(bytes.len())
                }
                Some(Err(kind)) => Err(Error::from(kind)),
            }
        }
    }
    // Popped from the back: "ab", an interrupt, "cd", then an unclean EOF.
    let mut script = Script(vec![
        Err(ErrorKind::UnexpectedEof),
        Ok(b"cd"),
        Err(ErrorKind::Interrupted),
        Ok(b"ab"),
    ]);
    assert_eq!(&*read_limited(&mut script, 64).unwrap(), b"abcd");
    let mut reset = Script(vec![Err(ErrorKind::ConnectionReset)]);
    assert_eq!(
        read_limited(&mut reset, 64).err().unwrap(),
        FetchError::Network
    );
    let mut timeout = Script(vec![Err(ErrorKind::TimedOut)]);
    assert_eq!(
        read_limited(&mut timeout, 64).err().unwrap(),
        FetchError::Timeout
    );
}

#[test]
fn the_system_resolver_answers_from_the_host_table() {
    // `localhost` resolves from /etc/hosts (or its platform equivalent)
    // without touching the network.
    let answers = chief_of_staff_net_fetch::SystemResolver
        .resolve("localhost", 443)
        .unwrap();
    assert!(answers.iter().all(|address| address.ip().is_loopback()));
    assert!(chief_of_staff_net_fetch::SystemResolver
        .resolve("", 443)
        .is_err());
}

#[test]
fn the_tls_transport_reports_a_refused_connection_as_network() {
    // Port 1 on loopback is closed in any sane test environment; the call
    // must fail at TCP connect, before any TLS byte, with a bounded kind.
    let closed: SocketAddr = "127.0.0.1:1".parse().unwrap();
    let error = chief_of_staff_net_fetch::TlsTransport
        .exchange(
            "localhost",
            closed,
            b"GET / HTTP/1.1\r\n\r\n",
            &Limits::default(),
        )
        .err().unwrap();
    assert!(
        matches!(
            error,
            FetchError::Network | FetchError::Timeout | FetchError::Tls
        ),
        "{error:?}"
    );
    // And the production composition is constructible.
    let _ = NetFetch::production();
}
