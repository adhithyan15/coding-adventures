use serde_json::Value;
use smart_home_onvif_integration::{
    build_authenticated_envelope, validate_discovery_origin, validate_onvif_http_status,
    OnvifClient, OnvifCredentials, OnvifLanTransport, OnvifOriginPolicy,
};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use url_parser::Url;

const CASES: &str =
    include_str!("../../../../specs/fixtures/smart-home-onvif-origin-v1/cases.json");

fn strings(value: &Value, field: &str) -> Vec<IpAddr> {
    value[field]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| item.as_str().unwrap().parse().unwrap())
        .collect()
}

fn error_code<T>(result: &Result<T, smart_home_onvif_integration::OnvifError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(error) => error.origin_policy_code().unwrap_or("unexpected_error"),
    }
}

#[test]
fn language_neutral_origin_cases_match_v1_contract() {
    let document: Value = serde_json::from_str(CASES).unwrap();
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["contract"], "smart-home-onvif-origin-v1");
    let cases = document["cases"].as_array().unwrap();
    assert!(cases.len() >= 12);

    for case in cases {
        let input = &case["input"];
        let expected = case["expected"]["code"].as_str().unwrap();
        let allow_loopback_http = input["allow_loopback_http"].as_bool().unwrap_or(false);
        let result = match case["operation"].as_str().unwrap() {
            "discovery" => {
                let sender_ip = input["sender_ip"].as_str().unwrap().parse().unwrap();
                validate_discovery_origin(
                    input["probe_message_id"].as_str().unwrap(),
                    input["relates_to"].as_str().unwrap(),
                    SocketAddr::new(sender_ip, 3702),
                    input["url"].as_str().unwrap(),
                    allow_loopback_http,
                )
                .map(|policy| policy.approved_endpoint().pinned_address())
            }
            "soap_origin" => OnvifOriginPolicy::review(
                input["url"].as_str().unwrap(),
                &strings(input, "resolved_addresses"),
                allow_loopback_http,
            )
            .map(|policy| policy.approved_endpoint().pinned_address()),
            "derived_origin" => {
                let approved_url = input["approved_url"].as_str().unwrap();
                let parsed = Url::parse(approved_url).unwrap();
                let approved_ip: IpAddr = parsed.host.unwrap().parse().unwrap();
                OnvifOriginPolicy::review(approved_url, &[approved_ip], allow_loopback_http)
                    .and_then(|policy| policy.approve_resource(input["url"].as_str().unwrap()))
                    .map(|endpoint| endpoint.pinned_address())
            }
            "http_status" => validate_onvif_http_status(input["status"].as_u64().unwrap() as u16)
                .map(|()| SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1)),
            "size_policy" => {
                let observed = input["observed_bytes"].as_u64().unwrap() as usize;
                match input["field"].as_str().unwrap() {
                    "url" => OnvifOriginPolicy::review(
                        &format!("https://10.0.0.8/{}", "x".repeat(observed)),
                        &["10.0.0.8".parse().unwrap()],
                        false,
                    )
                    .map(|policy| policy.approved_endpoint().pinned_address()),
                    "credential" => OnvifCredentials::new("fixture", "x".repeat(observed))
                        .map(|_| SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1)),
                    "request" => {
                        let credentials = OnvifCredentials::new("fixture", "secret").unwrap();
                        build_authenticated_envelope(
                            "https://10.0.0.8/onvif/device_service",
                            "fixture-action",
                            &"x".repeat(observed),
                            &credentials,
                            b"nonce",
                            "2026-08-01T00:00:00.000Z",
                        )
                        .map(|_| SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1))
                    }
                    field => panic!("unknown size field {field}"),
                }
            }
            operation => panic!("unknown fixture operation {operation}"),
        };
        assert_eq!(error_code(&result), expected, "fixture {}", case["id"]);
        assert_eq!(
            result.is_ok(),
            case["expected"]["accepted"].as_bool().unwrap(),
            "fixture {} acceptance",
            case["id"]
        );
        if let Some(expected_address) = case["expected"]["pinned_address"].as_str() {
            assert_eq!(
                result.unwrap().to_string(),
                expected_address,
                "fixture {} pinned address",
                case["id"]
            );
        }
    }
}

#[test]
fn credential_digest_is_not_sent_to_cross_origin_media_service() {
    let device = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let device_addr = device.local_addr().unwrap();
    let attacker = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    attacker.set_nonblocking(true).unwrap();
    let attacker_addr = attacker.local_addr().unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();

    let server = thread::spawn(move || {
        ready_tx.send(()).unwrap();
        for response_body in [
            "<tds:GetDeviceInformationResponse><tds:Manufacturer>Fixture</tds:Manufacturer><tds:Model>Camera</tds:Model><tds:FirmwareVersion>1</tds:FirmwareVersion><tds:SerialNumber>one</tds:SerialNumber><tds:HardwareId>one</tds:HardwareId></tds:GetDeviceInformationResponse>".to_string(),
            format!("<tds:GetCapabilitiesResponse><tds:Capabilities><tt:Media><tt:XAddr>http://{attacker_addr}/onvif/media_service</tt:XAddr></tt:Media></tds:Capabilities></tds:GetCapabilitiesResponse>"),
        ] {
            let (mut stream, _) = device.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let soap = format!(
                "<s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tds=\"http://www.onvif.org/ver10/device/wsdl\" xmlns:tt=\"http://www.onvif.org/ver10/schema\"><s:Body>{response_body}</s:Body></s:Envelope>"
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                soap.len(),
                soap
            )
            .unwrap();
        }
    });

    ready_rx.recv().unwrap();
    let endpoint = format!("http://{device_addr}/onvif/device_service");
    let policy = OnvifOriginPolicy::review(&endpoint, &[device_addr.ip()], true).unwrap();
    let transport = OnvifLanTransport::default().with_timeout(Duration::from_secs(2));
    let mut client = OnvifClient::new(transport, policy);
    let credentials = OnvifCredentials::new("fixture-user", "fixture-secret").unwrap();
    let error = client.inspect_camera(&endpoint, &credentials).unwrap_err();
    assert_eq!(error.origin_policy_code(), Some("unreviewed_origin"));
    server.join().unwrap();

    thread::sleep(Duration::from_millis(50));
    assert!(
        attacker.accept().is_err(),
        "attacker origin received a request"
    );
}
