//! Production ONVIF discovery and camera integration for D23.

#![forbid(unsafe_code)]

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{SecondsFormat, Utc};
use coding_adventures_sha1::sum1;
use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};
use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use rand::{rngs::OsRng, RngCore};
use smart_home_camera_media::{
    CameraMediaEndpointRegistry, CameraMediaKind, SNAPSHOT_CAPABILITY_ID, STREAM_CAPABILITY_ID,
};
use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device, DeviceId,
    Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, StateConfidence, StateSnapshot, StateSource, Value, ValueKind, VaultRef,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use udp_client::{send_to_and_collect, UdpError, UdpOptions};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.1";
pub const INTEGRATION_ID: &str = "onvif";
pub const WS_DISCOVERY_PORT: u16 = 3702;
pub const WS_DISCOVERY_IPV4: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 64;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

const SOAP_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
const WSA_NAMESPACE: &str = "http://www.w3.org/2005/08/addressing";
const WSD_NAMESPACE: &str = "http://schemas.xmlsoap.org/ws/2005/04/discovery";
const DEVICE_NAMESPACE: &str = "http://www.onvif.org/ver10/device/wsdl";
const MEDIA_NAMESPACE: &str = "http://www.onvif.org/ver10/media/wsdl";
const SCHEMA_NAMESPACE: &str = "http://www.onvif.org/ver10/schema";
const WSSE_NAMESPACE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
const WSU_NAMESPACE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";
const PASSWORD_DIGEST_TYPE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";
const BASE64_BINARY_TYPE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary";

const GET_DEVICE_INFORMATION_ACTION: &str =
    "http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation";
const GET_CAPABILITIES_ACTION: &str = "http://www.onvif.org/ver10/device/wsdl/GetCapabilities";
const GET_PROFILES_ACTION: &str = "http://www.onvif.org/ver10/media/wsdl/GetProfiles";
const GET_SNAPSHOT_URI_ACTION: &str = "http://www.onvif.org/ver10/media/wsdl/GetSnapshotUri";
const GET_STREAM_URI_ACTION: &str = "http://www.onvif.org/ver10/media/wsdl/GetStreamUri";

#[derive(Debug, Clone, PartialEq)]
pub enum OnvifError {
    Validation(String),
    Udp(UdpError),
    Xml(String),
    Url(UrlError),
    Io(String),
    Tls(String),
    Http(String),
    HttpStatus(u16),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    SoapFault(String),
    MissingField(&'static str),
    NoMediaProfiles,
    Runtime(String),
    CameraMedia(String),
}

impl fmt::Display for OnvifError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid ONVIF input: {message}"),
            Self::Udp(error) => write!(formatter, "ONVIF discovery failed: {error}"),
            Self::Xml(message) => write!(formatter, "invalid ONVIF XML: {message}"),
            Self::Url(error) => write!(formatter, "invalid ONVIF URL: {error}"),
            Self::Io(message) => write!(formatter, "ONVIF LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "ONVIF TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid ONVIF HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "ONVIF endpoint returned HTTP {status}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "ONVIF response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "ONVIF response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::SoapFault(reason) => write!(formatter, "ONVIF SOAP fault: {reason}"),
            Self::MissingField(field) => write!(formatter, "ONVIF response is missing {field}"),
            Self::NoMediaProfiles => formatter.write_str("ONVIF camera returned no media profiles"),
            Self::Runtime(message) => {
                write!(formatter, "D23 runtime rejected ONVIF data: {message}")
            }
            Self::CameraMedia(message) => {
                write!(
                    formatter,
                    "camera media broker rejected ONVIF data: {message}"
                )
            }
        }
    }
}

impl std::error::Error for OnvifError {}

impl From<UdpError> for OnvifError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<UrlError> for OnvifError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<RuntimeError> for OnvifError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnvifDiscoveryMatch {
    pub endpoint_reference: String,
    pub types: Vec<String>,
    pub scopes: Vec<String>,
    pub xaddrs: Vec<String>,
    pub metadata_version: Option<u64>,
    pub source: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnvifDiscoveryFailure {
    pub source: SocketAddr,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OnvifDiscoveryReport {
    pub matches: Vec<OnvifDiscoveryMatch>,
    pub failures: Vec<OnvifDiscoveryFailure>,
}

impl OnvifDiscoveryReport {
    pub fn discovery_records(
        &self,
        discovered_at_ms: u64,
    ) -> Result<Vec<DiscoveryRecord>, OnvifError> {
        self.matches
            .iter()
            .map(|matched| discovery_record(matched, discovered_at_ms))
            .collect()
    }
}

pub fn ws_discovery_ipv4_destination() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(WS_DISCOVERY_IPV4), WS_DISCOVERY_PORT)
}

pub fn random_message_id() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    format!(
        "urn:uuid:{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes(bytes[0..4].try_into().expect("four bytes")),
        u16::from_be_bytes(bytes[4..6].try_into().expect("two bytes")),
        u16::from_be_bytes(bytes[6..8].try_into().expect("two bytes")),
        u16::from_be_bytes(bytes[8..10].try_into().expect("two bytes")),
        u64::from_be_bytes([
            0, 0, bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ])
    )
}

pub fn build_ws_discovery_probe(message_id: &str) -> Result<String, OnvifError> {
    if message_id.trim().is_empty() {
        return Err(OnvifError::Validation(
            "WS-Discovery message id must not be empty".to_string(),
        ));
    }
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{SOAP_NAMESPACE}" xmlns:a="{WSA_NAMESPACE}" xmlns:d="{WSD_NAMESPACE}" xmlns:dn="{SCHEMA_NAMESPACE}">
  <s:Header>
    <a:Action s:mustUnderstand="1">{WSD_NAMESPACE}/Probe</a:Action>
    <a:MessageID>{}</a:MessageID>
    <a:ReplyTo><a:Address>http://www.w3.org/2005/08/addressing/anonymous</a:Address></a:ReplyTo>
    <a:To s:mustUnderstand="1">urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
  </s:Header>
  <s:Body><d:Probe><d:Types>dn:NetworkVideoTransmitter</d:Types></d:Probe></s:Body>
</s:Envelope>"#,
        xml_escape(message_id)
    ))
}

pub fn scan_ws_discovery(
    destination: SocketAddr,
    timeout: Duration,
    max_responses: usize,
) -> Result<OnvifDiscoveryReport, OnvifError> {
    if timeout.is_zero() {
        return Err(OnvifError::Validation(
            "WS-Discovery timeout must be positive".to_string(),
        ));
    }
    let probe = build_ws_discovery_probe(&random_message_id())?;
    let options = UdpOptions {
        bind_addr: None,
        max_datagram_size: 65_535,
        read_timeout: Some(timeout),
        write_timeout: Some(timeout),
    };
    let datagrams = send_to_and_collect(destination, probe.as_bytes(), options, max_responses)?;
    let mut report = OnvifDiscoveryReport::default();
    let mut seen = BTreeSet::new();
    for datagram in datagrams {
        let text = match std::str::from_utf8(&datagram.payload) {
            Ok(text) => text,
            Err(error) => {
                report.failures.push(OnvifDiscoveryFailure {
                    source: datagram.source,
                    message: format!("response is not UTF-8: {error}"),
                });
                continue;
            }
        };
        match parse_probe_matches(text, datagram.source) {
            Ok(matches) => {
                for matched in matches {
                    if seen.insert(matched.endpoint_reference.clone()) {
                        report.matches.push(matched);
                    }
                }
            }
            Err(error) => report.failures.push(OnvifDiscoveryFailure {
                source: datagram.source,
                message: error.to_string(),
            }),
        }
    }
    report
        .matches
        .sort_by(|left, right| left.endpoint_reference.cmp(&right.endpoint_reference));
    Ok(report)
}

pub fn parse_probe_matches(
    source: &str,
    sender: SocketAddr,
) -> Result<Vec<OnvifDiscoveryMatch>, OnvifError> {
    let document = parse_xml(source).map_err(|error| OnvifError::Xml(error.to_string()))?;
    soap_fault(&document.root)?;
    let mut elements = Vec::new();
    collect_descendants(&document.root, "ProbeMatch", &mut elements);
    let mut matches = Vec::with_capacity(elements.len());
    for element in elements {
        let endpoint_reference = descendant_text(element, "Address").ok_or(
            OnvifError::MissingField("ProbeMatch/EndpointReference/Address"),
        )?;
        let xaddrs = descendant_text(element, "XAddrs")
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if xaddrs.is_empty() {
            return Err(OnvifError::MissingField("ProbeMatch/XAddrs"));
        }
        matches.push(OnvifDiscoveryMatch {
            endpoint_reference,
            types: descendant_text(element, "Types")
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_string)
                .collect(),
            scopes: descendant_text(element, "Scopes")
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_string)
                .collect(),
            xaddrs,
            metadata_version: descendant_text(element, "MetadataVersion")
                .and_then(|value| value.parse().ok()),
            source: sender,
        });
    }
    if matches.is_empty() {
        return Err(OnvifError::MissingField("ProbeMatches/ProbeMatch"));
    }
    Ok(matches)
}

fn discovery_record(
    matched: &OnvifDiscoveryMatch,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, OnvifError> {
    let display_name =
        camera_name_from_scopes(&matched.scopes).unwrap_or_else(|| "ONVIF Camera".to_string());
    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Onvif,
        matched.endpoint_reference.clone(),
        DiscoverySource::WsDiscovery,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| OnvifError::Validation(error.to_string()))?
    .with_display_name(display_name)
    .with_address(matched.xaddrs[0].clone())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("onvif.discovery.sender", matched.source.to_string());
    if let Some(version) = matched.metadata_version {
        record = record.with_metadata("onvif.metadata_version", version.to_string());
    }
    record = record.with_metadata("onvif.xaddr_count", matched.xaddrs.len().to_string());
    Ok(record)
}

fn camera_name_from_scopes(scopes: &[String]) -> Option<String> {
    scopes.iter().find_map(|scope| {
        scope
            .strip_prefix("onvif://www.onvif.org/name/")
            .map(|name| name.replace("%20", " "))
            .filter(|name| !name.trim().is_empty())
    })
}

pub struct OnvifCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl OnvifCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, OnvifError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(OnvifError::Validation(
                "ONVIF username and password must not be empty".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for OnvifCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OnvifCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnvifDeviceInformation {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
}

pub struct OnvifMediaProfile {
    pub token: String,
    pub name: String,
    pub encoding: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate_limit: Option<u32>,
    snapshot_uri: Option<Zeroizing<String>>,
    stream_uri: Option<Zeroizing<String>>,
}

impl OnvifMediaProfile {
    pub fn has_snapshot_uri(&self) -> bool {
        self.snapshot_uri.is_some()
    }

    pub fn has_stream_uri(&self) -> bool {
        self.stream_uri.is_some()
    }

    fn snapshot_uri(&self) -> Option<&str> {
        self.snapshot_uri.as_deref().map(String::as_str)
    }

    fn stream_uri(&self) -> Option<&str> {
        self.stream_uri.as_deref().map(String::as_str)
    }
}

impl fmt::Debug for OnvifMediaProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OnvifMediaProfile")
            .field("token", &self.token)
            .field("name", &self.name)
            .field("encoding", &self.encoding)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("frame_rate_limit", &self.frame_rate_limit)
            .field(
                "snapshot_uri",
                &self.snapshot_uri.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "stream_uri",
                &self.stream_uri.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

pub struct OnvifCameraSnapshot {
    pub device_information: OnvifDeviceInformation,
    pub device_service_url: String,
    pub media_service_url: String,
    pub profiles: Vec<OnvifMediaProfile>,
}

impl fmt::Debug for OnvifCameraSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OnvifCameraSnapshot")
            .field("device_information", &self.device_information)
            .field("device_service_url", &self.device_service_url)
            .field("media_service_url", &self.media_service_url)
            .field("profiles", &self.profiles)
            .finish()
    }
}

pub trait OnvifSoapTransport {
    fn post_soap(
        &mut self,
        endpoint: &str,
        action: &str,
        envelope: &[u8],
    ) -> Result<Vec<u8>, OnvifError>;
}

pub struct OnvifLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    timeout: Duration,
    max_response_bytes: usize,
}

impl Default for OnvifLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl OnvifLanTransport {
    pub fn new(connector: Box<dyn TlsConnector>, tls_config: TlsConfig) -> Self {
        Self {
            connector,
            tls_config,
            timeout: Duration::from_secs(5),
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_max_response_bytes(mut self, maximum: usize) -> Self {
        self.max_response_bytes = maximum.max(1);
        self
    }
}

impl OnvifSoapTransport for OnvifLanTransport {
    fn post_soap(
        &mut self,
        endpoint: &str,
        action: &str,
        envelope: &[u8],
    ) -> Result<Vec<u8>, OnvifError> {
        let url = Url::parse(endpoint)?;
        let host = url
            .host
            .as_deref()
            .ok_or_else(|| OnvifError::Validation("ONVIF URL is missing a host".to_string()))?;
        let port = url
            .effective_port()
            .ok_or_else(|| OnvifError::Validation("ONVIF URL is missing a port".to_string()))?;
        let request = Zeroizing::new(encode_http_request(&url, action, envelope)?);
        let response = match url.scheme.as_str() {
            "http" => {
                let mut stream = connect_tcp(host, port, self.timeout)?;
                stream
                    .write_all(&request)
                    .map_err(|error| OnvifError::Io(error.to_string()))?;
                stream
                    .flush()
                    .map_err(|error| OnvifError::Io(error.to_string()))?;
                read_bounded(&mut stream, self.max_response_bytes)?
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = self.timeout;
                config.read_timeout = Some(self.timeout);
                config.write_timeout = Some(self.timeout);
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(|error| OnvifError::Tls(error.to_string()))?;
                stream
                    .write_all(&request)
                    .map_err(|error| OnvifError::Io(error.to_string()))?;
                stream
                    .flush()
                    .map_err(|error| OnvifError::Io(error.to_string()))?;
                let bytes = read_bounded(&mut stream, self.max_response_bytes)?;
                stream
                    .close_notify()
                    .map_err(|error| OnvifError::Tls(error.to_string()))?;
                bytes
            }
            scheme => {
                return Err(OnvifError::Validation(format!(
                    "unsupported ONVIF URL scheme `{scheme}`"
                )))
            }
        };
        decode_http_response(&response, self.max_response_bytes)
    }
}

pub struct OnvifClient<T> {
    transport: T,
}

impl<T: OnvifSoapTransport> OnvifClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn inspect_camera(
        &mut self,
        device_service_url: &str,
        credentials: &OnvifCredentials,
    ) -> Result<OnvifCameraSnapshot, OnvifError> {
        Url::parse(device_service_url)?;
        let device_information = parse_device_information(&self.call(
            device_service_url,
            GET_DEVICE_INFORMATION_ACTION,
            "<tds:GetDeviceInformation/>",
            credentials,
        )?)?;
        let capabilities = self.call(
            device_service_url,
            GET_CAPABILITIES_ACTION,
            "<tds:GetCapabilities><tds:Category>Media</tds:Category></tds:GetCapabilities>",
            credentials,
        )?;
        let media_service_url = descendant_text(&capabilities, "XAddr")
            .ok_or(OnvifError::MissingField("Capabilities/Media/XAddr"))?;
        Url::parse(&media_service_url)?;
        let profiles_response = self.call(
            &media_service_url,
            GET_PROFILES_ACTION,
            "<trt:GetProfiles/>",
            credentials,
        )?;
        let mut profiles = parse_profiles(&profiles_response)?;
        if profiles.is_empty() {
            return Err(OnvifError::NoMediaProfiles);
        }
        for profile in &mut profiles {
            let token = xml_escape(&profile.token);
            let snapshot = self.call(
                &media_service_url,
                GET_SNAPSHOT_URI_ACTION,
                &format!(
                    "<trt:GetSnapshotUri><trt:ProfileToken>{token}</trt:ProfileToken></trt:GetSnapshotUri>"
                ),
                credentials,
            )?;
            profile.snapshot_uri = descendant_text(&snapshot, "Uri").map(Zeroizing::new);
            let stream = self.call(
                &media_service_url,
                GET_STREAM_URI_ACTION,
                &format!(
                    "<trt:GetStreamUri><trt:StreamSetup><tt:Stream>RTP-Unicast</tt:Stream><tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport></trt:StreamSetup><trt:ProfileToken>{token}</trt:ProfileToken></trt:GetStreamUri>"
                ),
                credentials,
            )?;
            profile.stream_uri = descendant_text(&stream, "Uri").map(Zeroizing::new);
        }
        Ok(OnvifCameraSnapshot {
            device_information,
            device_service_url: device_service_url.to_string(),
            media_service_url,
            profiles,
        })
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    fn call(
        &mut self,
        endpoint: &str,
        action: &str,
        body: &str,
        credentials: &OnvifCredentials,
    ) -> Result<XmlElement, OnvifError> {
        let mut nonce = [0u8; 20];
        OsRng.fill_bytes(&mut nonce);
        let created = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let envelope = Zeroizing::new(
            build_authenticated_envelope(endpoint, action, body, credentials, &nonce, &created)?
                .into_bytes(),
        );
        let response = self.transport.post_soap(endpoint, action, &envelope)?;
        let source = std::str::from_utf8(&response)
            .map_err(|error| OnvifError::Xml(format!("SOAP body is not UTF-8: {error}")))?;
        let document = parse_xml(source).map_err(|error| OnvifError::Xml(error.to_string()))?;
        soap_fault(&document.root)?;
        Ok(document.root)
    }
}

pub fn build_authenticated_envelope(
    endpoint: &str,
    action: &str,
    body: &str,
    credentials: &OnvifCredentials,
    nonce: &[u8],
    created: &str,
) -> Result<String, OnvifError> {
    if nonce.is_empty() || created.trim().is_empty() {
        return Err(OnvifError::Validation(
            "WS-Security nonce and created timestamp must not be empty".to_string(),
        ));
    }
    let mut digest_input =
        Vec::with_capacity(nonce.len() + created.len() + credentials.password.len());
    digest_input.extend_from_slice(nonce);
    digest_input.extend_from_slice(created.as_bytes());
    digest_input.extend_from_slice(credentials.password.as_bytes());
    let digest = BASE64.encode(sum1(&digest_input));
    digest_input.fill(0);
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{SOAP_NAMESPACE}" xmlns:a="{WSA_NAMESPACE}" xmlns:tds="{DEVICE_NAMESPACE}" xmlns:trt="{MEDIA_NAMESPACE}" xmlns:tt="{SCHEMA_NAMESPACE}" xmlns:wsse="{WSSE_NAMESPACE}" xmlns:wsu="{WSU_NAMESPACE}">
  <s:Header>
    <a:Action s:mustUnderstand="1">{}</a:Action>
    <a:To s:mustUnderstand="1">{}</a:To>
    <wsse:Security s:mustUnderstand="1"><wsse:UsernameToken>
      <wsse:Username>{}</wsse:Username>
      <wsse:Password Type="{PASSWORD_DIGEST_TYPE}">{digest}</wsse:Password>
      <wsse:Nonce EncodingType="{BASE64_BINARY_TYPE}">{}</wsse:Nonce>
      <wsu:Created>{}</wsu:Created>
    </wsse:UsernameToken></wsse:Security>
  </s:Header>
  <s:Body>{body}</s:Body>
</s:Envelope>"#,
        xml_escape(action),
        xml_escape(endpoint),
        xml_escape(&credentials.username),
        BASE64.encode(nonce),
        xml_escape(created),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnvifCameraConfig {
    pub bridge_id: BridgeId,
    pub endpoint_reference: String,
    pub credential_ref: VaultRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledOnvifCamera {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
    pub snapshot_endpoint_count: usize,
    pub stream_endpoint_count: usize,
}

pub fn install_camera_snapshot(
    runtime: &mut SmartHomeRuntime,
    media_registry: &mut impl CameraMediaEndpointRegistry,
    config: &OnvifCameraConfig,
    snapshot: &OnvifCameraSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledOnvifCamera, OnvifError> {
    if config.endpoint_reference.trim().is_empty() {
        return Err(OnvifError::Validation(
            "ONVIF endpoint reference must not be empty".to_string(),
        ));
    }
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(snapshot.device_service_url.clone());
    bridge.hardware_model = Some(snapshot.device_information.model.clone());
    bridge.firmware_version = Some(snapshot.device_information.firmware_version.clone());
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier(
        "endpoint_reference",
        &config.endpoint_reference,
    )?];
    bridge.metadata = vec![
        Metadata::new(
            "onvif.media_profile_count",
            snapshot.profiles.len().to_string(),
        ),
        Metadata::new("onvif.transport", "soap_http_ws_security"),
    ];
    runtime.upsert_bridge(bridge)?;

    let native_id = stable_component(
        if snapshot.device_information.serial_number.trim().is_empty() {
            &snapshot.device_information.hardware_id
        } else {
            &snapshot.device_information.serial_number
        },
    );
    let device_id = DeviceId::trusted(format!("onvif:{native_id}"));
    let mut entities = Vec::with_capacity(snapshot.profiles.len());
    let mut entity_ids = Vec::with_capacity(snapshot.profiles.len());
    let mut snapshot_endpoint_count = 0usize;
    let mut stream_endpoint_count = 0usize;
    for profile in &snapshot.profiles {
        let entity_id = EntityId::trusted(format!(
            "onvif:{native_id}:{}",
            stable_component(&profile.token)
        ));
        let mut capabilities = Vec::new();
        if profile.has_snapshot_uri() {
            capabilities.push(Capability::new(
                CapabilityId::trusted(SNAPSHOT_CAPABILITY_ID),
                CapabilityMode::Command,
                ValueKind::Text,
            ));
        }
        if profile.has_stream_uri() {
            capabilities.push(Capability::new(
                CapabilityId::trusted(STREAM_CAPABILITY_ID),
                CapabilityMode::Command,
                ValueKind::Text,
            ));
        }
        capabilities.push(Capability::new(
            CapabilityId::trusted("camera.video_profile"),
            CapabilityMode::Observe,
            ValueKind::Object,
        ));
        let mut metadata = vec![Metadata::new("onvif.profile_token", &profile.token)];
        if let Some(encoding) = &profile.encoding {
            metadata.push(Metadata::new("onvif.video_encoding", encoding));
        }
        if let (Some(width), Some(height)) = (profile.width, profile.height) {
            metadata.push(Metadata::new(
                "onvif.resolution",
                format!("{width}x{height}"),
            ));
        }
        entities.push(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Camera,
            name: profile.name.clone(),
            capabilities,
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: profile_state(profile),
                observed_at_ms,
                received_at_ms: observed_at_ms,
                source: StateSource::Poll,
                confidence: StateConfidence::Confirmed,
                expires_at_ms: None,
            }),
            metadata,
        });
        if let Some(uri) = profile.snapshot_uri() {
            media_registry
                .register_camera_endpoint(entity_id.clone(), CameraMediaKind::Snapshot, uri)
                .map_err(|error| OnvifError::CameraMedia(error.to_string()))?;
            snapshot_endpoint_count += 1;
        }
        if let Some(uri) = profile.stream_uri() {
            media_registry
                .register_camera_endpoint(entity_id.clone(), CameraMediaKind::Stream, uri)
                .map_err(|error| OnvifError::CameraMedia(error.to_string()))?;
            stream_endpoint_count += 1;
        }
        entity_ids.push(entity_id);
    }
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: snapshot.device_information.manufacturer.clone(),
        model: snapshot.device_information.model.clone(),
        name: snapshot.device_information.model.clone(),
        serial: Some(snapshot.device_information.serial_number.clone()),
        firmware_version: Some(snapshot.device_information.firmware_version.clone()),
        room_id: None,
        entity_ids: entity_ids.clone(),
        identifiers: vec![
            protocol_identifier("serial_number", &snapshot.device_information.serial_number)?,
            protocol_identifier("hardware_id", &snapshot.device_information.hardware_id)?,
        ],
        health: Health::Online,
        metadata: vec![Metadata::new(
            "onvif.profile_count",
            snapshot.profiles.len().to_string(),
        )],
    })?;
    for entity in entities {
        runtime.upsert_entity(entity)?;
    }
    Ok(InstalledOnvifCamera {
        bridge_id: config.bridge_id.clone(),
        device_id,
        entity_ids,
        snapshot_endpoint_count,
        stream_endpoint_count,
    })
}

fn profile_state(profile: &OnvifMediaProfile) -> Value {
    let mut fields = vec![
        ("available".to_string(), Value::Bool(true)),
        (
            "snapshot_available".to_string(),
            Value::Bool(profile.has_snapshot_uri()),
        ),
        (
            "stream_available".to_string(),
            Value::Bool(profile.has_stream_uri()),
        ),
    ];
    if let Some(encoding) = &profile.encoding {
        fields.push(("encoding".to_string(), Value::Text(encoding.clone())));
    }
    if let Some(width) = profile.width {
        fields.push(("width".to_string(), Value::Integer(i64::from(width))));
    }
    if let Some(height) = profile.height {
        fields.push(("height".to_string(), Value::Integer(i64::from(height))));
    }
    Value::Object(fields)
}

fn parse_device_information(root: &XmlElement) -> Result<OnvifDeviceInformation, OnvifError> {
    Ok(OnvifDeviceInformation {
        manufacturer: required_descendant_text(root, "Manufacturer")?,
        model: required_descendant_text(root, "Model")?,
        firmware_version: required_descendant_text(root, "FirmwareVersion")?,
        serial_number: required_descendant_text(root, "SerialNumber")?,
        hardware_id: required_descendant_text(root, "HardwareId")?,
    })
}

fn parse_profiles(root: &XmlElement) -> Result<Vec<OnvifMediaProfile>, OnvifError> {
    let mut elements = Vec::new();
    collect_descendants(root, "Profiles", &mut elements);
    let mut profiles = Vec::with_capacity(elements.len());
    for element in elements {
        let token = element
            .get_attr(None, "token")
            .map(str::to_string)
            .ok_or(OnvifError::MissingField("Profiles@token"))?;
        let resolution = descendant(element, "Resolution");
        let rate_control = descendant(element, "RateControl");
        profiles.push(OnvifMediaProfile {
            name: descendant_text(element, "Name").unwrap_or_else(|| token.clone()),
            token,
            encoding: descendant_text(element, "Encoding"),
            width: resolution
                .and_then(|node| descendant_text(node, "Width"))
                .and_then(|value| value.parse().ok()),
            height: resolution
                .and_then(|node| descendant_text(node, "Height"))
                .and_then(|value| value.parse().ok()),
            frame_rate_limit: rate_control
                .and_then(|node| descendant_text(node, "FrameRateLimit"))
                .and_then(|value| value.parse().ok()),
            snapshot_uri: None,
            stream_uri: None,
        });
    }
    Ok(profiles)
}

fn required_descendant_text(root: &XmlElement, name: &'static str) -> Result<String, OnvifError> {
    descendant_text(root, name).ok_or(OnvifError::MissingField(name))
}

fn descendant<'a>(root: &'a XmlElement, local_name: &str) -> Option<&'a XmlElement> {
    if root.local_name == local_name {
        return Some(root);
    }
    root.children.iter().find_map(|child| match child {
        XmlNode::Element(element) => descendant(element, local_name),
        _ => None,
    })
}

fn descendant_text(root: &XmlElement, local_name: &str) -> Option<String> {
    descendant(root, local_name)
        .map(XmlElement::text_content)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn collect_descendants<'a>(
    root: &'a XmlElement,
    local_name: &str,
    output: &mut Vec<&'a XmlElement>,
) {
    if root.local_name == local_name {
        output.push(root);
    }
    for child in &root.children {
        if let XmlNode::Element(element) = child {
            collect_descendants(element, local_name, output);
        }
    }
}

fn soap_fault(root: &XmlElement) -> Result<(), OnvifError> {
    let Some(fault) = descendant(root, "Fault") else {
        return Ok(());
    };
    let reason = descendant_text(fault, "Text")
        .or_else(|| descendant_text(fault, "Reason"))
        .unwrap_or_else(|| "unspecified SOAP fault".to_string());
    Err(OnvifError::SoapFault(reason))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn stable_component(value: &str) -> String {
    let component = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let component = component.trim_matches('-');
    if component.is_empty() {
        "camera".to_string()
    } else {
        component.to_string()
    }
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, OnvifError> {
    ProtocolIdentifier::new(ProtocolFamily::Onvif, kind, value)
        .map_err(|error| OnvifError::Validation(error.to_string()))
}

fn encode_http_request(url: &Url, action: &str, body: &[u8]) -> Result<Vec<u8>, OnvifError> {
    let host = url
        .host
        .as_deref()
        .ok_or_else(|| OnvifError::Validation("ONVIF URL is missing a host".to_string()))?;
    let port = url
        .effective_port()
        .ok_or_else(|| OnvifError::Validation("ONVIF URL is missing a port".to_string()))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(action) {
        return Err(OnvifError::Validation(
            "ONVIF request target or action contains unsafe HTTP text".to_string(),
        ));
    }
    let default_port = match url.scheme.as_str() {
        "http" => 80,
        "https" => 443,
        scheme => {
            return Err(OnvifError::Validation(format!(
                "unsupported ONVIF URL scheme `{scheme}`"
            )))
        }
    };
    let host_header = if url.port.is_some() && port != default_port {
        format!("{host}:{port}")
    } else {
        host.to_string()
    };
    let mut request = format!(
        "POST {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\nAccept: application/soap+xml\r\nContent-Type: application/soap+xml; charset=utf-8; action=\"{action}\"\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, OnvifError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| OnvifError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| OnvifError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| OnvifError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(OnvifError::Io(
        last_error
            .unwrap_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "host resolved to no addresses",
                )
            })
            .to_string(),
    ))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, OnvifError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| OnvifError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(OnvifError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, OnvifError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| OnvifError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(OnvifError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(OnvifError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => decode_chunked(input, maximum)?,
    };
    if body.len() > maximum {
        return Err(OnvifError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, OnvifError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| OnvifError::Http("missing chunk-size terminator".to_string()))?;
        let line_end = cursor + line_offset;
        let size_text = std::str::from_utf8(&input[cursor..line_end])
            .map_err(|_| OnvifError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| OnvifError::Http("invalid chunk size".to_string()))?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(OnvifError::ResponseTooLarge { limit: maximum });
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| OnvifError::Http("chunk size overflow".to_string()))?;
        if end + 2 > input.len() || &input[end..end + 2] != b"\r\n" {
            return Err(OnvifError::Http("truncated chunk payload".to_string()));
        }
        output.extend_from_slice(&input[cursor..end]);
        cursor = end + 2;
    }
}

fn has_unsafe_http_text(value: &str) -> bool {
    value
        .bytes()
        .any(|byte| byte == b'\r' || byte == b'\n' || byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_camera_media::{
        CameraMediaAccessRequest, CameraMediaClock, CameraMediaError, CameraMediaExecution,
        CameraMediaExecutionError, CameraMediaExecutionResult, CameraMediaExecutor,
        CameraMediaNonceError, CameraMediaNonceSource, CameraMediaPolicy,
        CameraMediaPrincipalSource, CameraMediaService,
    };
    use smart_home_core::{AgentId, CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::cell::Cell;
    use std::io::{BufRead, BufReader};
    use std::net::{TcpListener, UdpSocket};
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    use std::thread;

    const USERNAME: &str = "operator";
    const PASSWORD: &str = "fixture-password";

    struct FixedNonce([u8; 16]);

    impl CameraMediaNonceSource for FixedNonce {
        fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), CameraMediaNonceError> {
            output.copy_from_slice(&self.0);
            Ok(())
        }
    }

    #[derive(Clone)]
    struct TestClock(Rc<Cell<u64>>);

    impl CameraMediaClock for TestClock {
        fn now_ms(&self) -> u64 {
            self.0.get()
        }
    }

    struct FixedPrincipal(AgentId);

    impl CameraMediaPrincipalSource for FixedPrincipal {
        fn current_principal(&self) -> Option<AgentId> {
            Some(self.0.clone())
        }
    }

    struct SnapshotDeliveryHost {
        saw_snapshot_endpoint: Rc<Cell<bool>>,
    }

    impl CameraMediaExecutor for SnapshotDeliveryHost {
        type Stream = ();

        fn deliver(
            &mut self,
            execution: CameraMediaExecution<'_>,
        ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
            self.saw_snapshot_endpoint
                .set(execution.endpoint_uri().contains("snapshot.jpg"));
            Ok(CameraMediaExecutionResult::snapshot(vec![0x5a; 128]))
        }

        fn close_stream(
            &mut self,
            _stream: &mut Self::Stream,
        ) -> Result<(), CameraMediaExecutionError> {
            Ok(())
        }
    }

    fn soap(body: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><s:Envelope xmlns:s="{SOAP_NAMESPACE}" xmlns:tds="{DEVICE_NAMESPACE}" xmlns:trt="{MEDIA_NAMESPACE}" xmlns:tt="{SCHEMA_NAMESPACE}"><s:Body>{body}</s:Body></s:Envelope>"#
        )
    }

    #[test]
    fn ws_discovery_probe_match_becomes_d23_record() {
        let sender: SocketAddr = "192.0.2.10:3702".parse().unwrap();
        let response = format!(
            r#"<s:Envelope xmlns:s="{SOAP_NAMESPACE}" xmlns:a="{WSA_NAMESPACE}" xmlns:d="{WSD_NAMESPACE}"><s:Body><d:ProbeMatches><d:ProbeMatch><a:EndpointReference><a:Address>urn:uuid:camera-1</a:Address></a:EndpointReference><d:Types>dn:NetworkVideoTransmitter</d:Types><d:Scopes>onvif://www.onvif.org/name/Front%20Door</d:Scopes><d:XAddrs>http://192.0.2.10/onvif/device_service</d:XAddrs><d:MetadataVersion>4</d:MetadataVersion></d:ProbeMatch></d:ProbeMatches></s:Body></s:Envelope>"#
        );
        let matched = parse_probe_matches(&response, sender).unwrap();
        let report = OnvifDiscoveryReport {
            matches: matched,
            failures: Vec::new(),
        };
        let records = report.discovery_records(100).unwrap();
        assert_eq!(records[0].source, DiscoverySource::WsDiscovery);
        assert_eq!(records[0].protocol_family, ProtocolFamily::Onvif);
        assert_eq!(records[0].display_name.as_deref(), Some("Front Door"));
        assert_eq!(
            records[0].pairing_requirement,
            PairingRequirement::Credentials
        );
    }

    #[test]
    fn real_udp_scan_sends_probe_and_collects_response() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let destination = socket.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let (size, peer) = socket.recv_from(&mut buffer).unwrap();
            let probe = std::str::from_utf8(&buffer[..size]).unwrap();
            assert!(probe.contains("NetworkVideoTransmitter"));
            let response = format!(
                r#"<s:Envelope xmlns:s="{SOAP_NAMESPACE}" xmlns:a="{WSA_NAMESPACE}" xmlns:d="{WSD_NAMESPACE}"><s:Body><d:ProbeMatches><d:ProbeMatch><a:EndpointReference><a:Address>urn:uuid:loopback-camera</a:Address></a:EndpointReference><d:Types>dn:NetworkVideoTransmitter</d:Types><d:Scopes>onvif://www.onvif.org/name/Loopback</d:Scopes><d:XAddrs>http://127.0.0.1/onvif/device_service</d:XAddrs><d:MetadataVersion>1</d:MetadataVersion></d:ProbeMatch></d:ProbeMatches></s:Body></s:Envelope>"#
            );
            socket.send_to(response.as_bytes(), peer).unwrap();
        });
        let report = scan_ws_discovery(destination, Duration::from_millis(100), 4).unwrap();
        server.join().unwrap();
        assert_eq!(report.matches.len(), 1);
        assert!(report.failures.is_empty());
    }

    #[test]
    fn ws_security_uses_password_digest_without_exposing_password() {
        let credentials = OnvifCredentials::new(USERNAME, PASSWORD).unwrap();
        let envelope = build_authenticated_envelope(
            "http://camera/onvif/device_service",
            GET_DEVICE_INFORMATION_ACTION,
            "<tds:GetDeviceInformation/>",
            &credentials,
            b"fixture-nonce",
            "2026-08-01T00:00:00.000Z",
        )
        .unwrap();
        assert!(envelope.contains(USERNAME));
        assert!(envelope.contains("PasswordDigest"));
        assert!(!envelope.contains(PASSWORD));
        assert!(!format!("{credentials:?}").contains(PASSWORD));
    }

    #[test]
    fn real_http_camera_inspection_projects_runtime_and_private_media_leases() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base = format!("http://{address}");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let server_base = base.clone();
        let server = thread::spawn(move || {
            for _ in 0..5 {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut headers = String::new();
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        content_length = value.trim().parse().unwrap();
                    }
                    headers.push_str(&line);
                }
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                server_requests.lock().unwrap().push(body.clone());
                let response_body = if body.contains("GetDeviceInformation") {
                    soap("<tds:GetDeviceInformationResponse><tds:Manufacturer>Acme</tds:Manufacturer><tds:Model>DoorCam</tds:Model><tds:FirmwareVersion>1.2.3</tds:FirmwareVersion><tds:SerialNumber>CAM-001</tds:SerialNumber><tds:HardwareId>HW-9</tds:HardwareId></tds:GetDeviceInformationResponse>")
                } else if body.contains("GetCapabilities") {
                    soap(&format!("<tds:GetCapabilitiesResponse><tds:Capabilities><tt:Media><tt:XAddr>{server_base}/onvif/media_service</tt:XAddr></tt:Media></tds:Capabilities></tds:GetCapabilitiesResponse>"))
                } else if body.contains("GetProfiles") {
                    soap("<trt:GetProfilesResponse><trt:Profiles token=\"profile-main\"><tt:Name>Main Stream</tt:Name><tt:VideoEncoderConfiguration><tt:Encoding>H264</tt:Encoding><tt:Resolution><tt:Width>1920</tt:Width><tt:Height>1080</tt:Height></tt:Resolution><tt:RateControl><tt:FrameRateLimit>30</tt:FrameRateLimit></tt:RateControl></tt:VideoEncoderConfiguration></trt:Profiles></trt:GetProfilesResponse>")
                } else if body.contains("GetSnapshotUri") {
                    soap(&format!("<trt:GetSnapshotUriResponse><trt:MediaUri><tt:Uri>{server_base}/snapshot.jpg</tt:Uri></trt:MediaUri></trt:GetSnapshotUriResponse>"))
                } else {
                    soap("<trt:GetStreamUriResponse><trt:MediaUri><tt:Uri>rtsp://127.0.0.1/private-stream</tt:Uri></trt:MediaUri></trt:GetStreamUriResponse>")
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/soap+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                reader.get_mut().write_all(response.as_bytes()).unwrap();
            }
        });

        let credentials = OnvifCredentials::new(USERNAME, PASSWORD).unwrap();
        let mut client =
            OnvifClient::new(OnvifLanTransport::default().with_timeout(Duration::from_secs(2)));
        let snapshot = client
            .inspect_camera(&format!("{base}/onvif/device_service"), &credentials)
            .unwrap();
        server.join().unwrap();
        assert_eq!(snapshot.device_information.model, "DoorCam");
        assert_eq!(snapshot.profiles.len(), 1);
        assert_eq!(snapshot.profiles[0].width, Some(1920));
        assert!(snapshot.profiles[0].has_snapshot_uri());
        assert!(snapshot.profiles[0].has_stream_uri());

        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 5);
        assert!(captured
            .iter()
            .all(|request| request.contains("PasswordDigest")));
        assert!(captured.iter().all(|request| !request.contains(PASSWORD)));
        drop(captured);

        let mut runtime = SmartHomeRuntime::default();
        let principal = AgentId::trusted("dashboard-user");
        let clock_value = Rc::new(Cell::new(1_000));
        let saw_snapshot_endpoint = Rc::new(Cell::new(false));
        let mut media = CameraMediaService::new(
            CameraMediaPolicy {
                allow_plaintext_loopback: true,
                ..CameraMediaPolicy::default()
            },
            TestClock(Rc::clone(&clock_value)),
            FixedNonce([0x44; 16]),
            FixedPrincipal(principal.clone()),
            SnapshotDeliveryHost {
                saw_snapshot_endpoint: Rc::clone(&saw_snapshot_endpoint),
            },
        );
        let installed = install_camera_snapshot(
            &mut runtime,
            &mut media,
            &OnvifCameraConfig {
                bridge_id: BridgeId::trusted("onvif-loopback"),
                endpoint_reference: "urn:uuid:loopback-camera".to_string(),
                credential_ref: VaultRef::trusted("vault:onvif:camera-1"),
            },
            &snapshot,
            1_000,
        )
        .unwrap();
        assert_eq!(installed.entity_ids.len(), 1);
        assert_eq!(installed.snapshot_endpoint_count, 1);
        assert_eq!(installed.stream_endpoint_count, 1);
        assert_eq!(runtime.registry().devices().count(), 1);
        assert_eq!(runtime.registry().entities().count(), 1);
        assert_eq!(
            runtime
                .registry()
                .entity(&installed.entity_ids[0])
                .unwrap()
                .kind,
            EntityKind::Camera
        );

        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("camera-preview"),
                principal.clone(),
                installed.entity_ids[0].clone(),
                CapabilityId::trusted(SNAPSHOT_CAPABILITY_ID),
                PrivilegeTier::HumanApproval,
                "operator",
                1_000,
            ));
        clock_value.set(1_001);
        let lease = media
            .issue_lease(
                &runtime,
                CameraMediaAccessRequest::new(
                    installed.entity_ids[0].clone(),
                    CameraMediaKind::Snapshot,
                    "door preview",
                    5_000,
                ),
            )
            .unwrap();
        clock_value.set(1_002);
        let delivery = media.deliver_lease(&runtime, &lease.lease_id).unwrap();
        assert_eq!(delivery.snapshot_bytes().unwrap().len(), 128);
        assert!(saw_snapshot_endpoint.get());
        assert!(!format!("{:?}", runtime.durable_snapshot()).contains("snapshot.jpg"));
        assert!(
            !format!("{:?}", media.audit_records().collect::<Vec<_>>()).contains("private-stream")
        );
        clock_value.set(1_003);
        assert!(matches!(
            media.deliver_lease(&runtime, &lease.lease_id),
            Err(CameraMediaError::UnknownLease)
        ));
    }
}
