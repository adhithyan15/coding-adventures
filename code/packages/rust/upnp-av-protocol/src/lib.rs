//! Strict fixed UPnP AV MediaRenderer description and SOAP codec.

#![forbid(unsafe_code)]

use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};
use std::fmt;

pub const MEDIA_RENDERER_DEVICE_TYPE: &str = "urn:schemas-upnp-org:device:MediaRenderer:1";
pub const AV_TRANSPORT_SERVICE_TYPE: &str = "urn:schemas-upnp-org:service:AVTransport:1";
pub const RENDERING_CONTROL_SERVICE_TYPE: &str = "urn:schemas-upnp-org:service:RenderingControl:1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpnpAvError {
    Validation(String),
    Xml(String),
    MissingService(&'static str),
}

impl fmt::Display for UpnpAvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid UPnP AV value: {message}"),
            Self::Xml(message) => write!(formatter, "invalid UPnP AV XML: {message}"),
            Self::MissingService(service) => {
                write!(formatter, "UPnP description has no {service} service")
            }
        }
    }
}

impl std::error::Error for UpnpAvError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub service_type: String,
    pub control_url: String,
    pub event_subscription_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererDescription {
    pub friendly_name: String,
    pub manufacturer: Option<String>,
    pub model_name: String,
    pub model_number: Option<String>,
    pub serial_number: Option<String>,
    pub udn: String,
    pub firmware_version: Option<String>,
    pub room_name: Option<String>,
    pub av_transport: Service,
    pub rendering_control: Service,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Play,
    Pause,
    Stop,
    Transitioning,
    NoMedia,
    Other(String),
}

impl PlaybackState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Stop => "stop",
            Self::Transitioning => "transitioning",
            Self::NoMedia => "no_media",
            Self::Other(state) => state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionInfo {
    pub track_uri: Option<String>,
    pub track_metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoapRequest {
    pub action_header: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    GetTransportInfo,
    GetPositionInfo,
    GetVolume,
    GetMute,
    Play,
    Pause,
    Stop,
    SetVolume(u8),
    SetMute(bool),
}

impl Action {
    pub fn service_type(self) -> &'static str {
        match self {
            Self::GetTransportInfo
            | Self::GetPositionInfo
            | Self::Play
            | Self::Pause
            | Self::Stop => AV_TRANSPORT_SERVICE_TYPE,
            Self::GetVolume | Self::GetMute | Self::SetVolume(_) | Self::SetMute(_) => {
                RENDERING_CONTROL_SERVICE_TYPE
            }
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::GetTransportInfo => "GetTransportInfo",
            Self::GetPositionInfo => "GetPositionInfo",
            Self::GetVolume => "GetVolume",
            Self::GetMute => "GetMute",
            Self::Play => "Play",
            Self::Pause => "Pause",
            Self::Stop => "Stop",
            Self::SetVolume(_) => "SetVolume",
            Self::SetMute(_) => "SetMute",
        }
    }

    fn arguments(self) -> Result<String, UpnpAvError> {
        match self {
            Self::GetTransportInfo | Self::GetPositionInfo | Self::Pause | Self::Stop => {
                Ok("<InstanceID>0</InstanceID>".to_string())
            }
            Self::GetVolume | Self::GetMute => {
                Ok("<InstanceID>0</InstanceID><Channel>Master</Channel>".to_string())
            }
            Self::Play => Ok("<InstanceID>0</InstanceID><Speed>1</Speed>".to_string()),
            Self::SetVolume(volume) if volume <= 100 => Ok(format!(
                "<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredVolume>{volume}</DesiredVolume>"
            )),
            Self::SetVolume(volume) => Err(UpnpAvError::Validation(format!(
                "volume {volume} exceeds 100"
            ))),
            Self::SetMute(muted) => Ok(format!(
                "<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredMute>{}</DesiredMute>",
                u8::from(muted)
            )),
        }
    }
}

pub fn encode_action(action: Action) -> Result<SoapRequest, UpnpAvError> {
    let service_type = action.service_type();
    let name = action.name();
    let arguments = action.arguments()?;
    Ok(SoapRequest {
        action_header: format!("\"{service_type}#{name}\""),
        body: format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{name} xmlns:u=\"{service_type}\">{arguments}</u:{name}></s:Body></s:Envelope>"
        )
        .into_bytes(),
    })
}

pub fn parse_renderer_description(
    bytes: &[u8],
    expected_device_type: &str,
) -> Result<RendererDescription, UpnpAvError> {
    if expected_device_type.trim().is_empty() {
        return Err(UpnpAvError::Validation(
            "expected device type is empty".to_string(),
        ));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|_| UpnpAvError::Xml("device description is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| UpnpAvError::Xml(error.to_string()))?;
    let device = descendant(&document.root, "device")
        .ok_or_else(|| UpnpAvError::Xml("description is missing device".to_string()))?;
    let device_type = required_child_text(device, "deviceType")?;
    if device_type != expected_device_type {
        return Err(UpnpAvError::Validation(format!(
            "expected device type `{expected_device_type}`, got `{device_type}`"
        )));
    }
    let mut services = Vec::new();
    collect_descendants(device, "service", &mut services);
    let service = |service_type: &'static str| {
        services
            .iter()
            .find_map(|service| {
                let found_type = child_text(service, "serviceType")?;
                if found_type != service_type {
                    return None;
                }
                Some(Service {
                    service_type: found_type,
                    control_url: child_text(service, "controlURL").unwrap_or_default(),
                    event_subscription_url: child_text(service, "eventSubURL"),
                })
            })
            .filter(|service| !service.control_url.is_empty())
            .ok_or(UpnpAvError::MissingService(service_type))
    };
    Ok(RendererDescription {
        friendly_name: required_child_text(device, "friendlyName")?,
        manufacturer: child_text(device, "manufacturer"),
        model_name: required_child_text(device, "modelName")?,
        model_number: child_text(device, "modelNumber"),
        serial_number: child_text(device, "serialNumber"),
        udn: required_child_text(device, "UDN")?,
        firmware_version: child_text(device, "softwareVersion")
            .or_else(|| child_text(device, "firmwareVersion")),
        room_name: child_text(device, "roomName"),
        av_transport: service(AV_TRANSPORT_SERVICE_TYPE)?,
        rendering_control: service(RENDERING_CONTROL_SERVICE_TYPE)?,
    })
}

pub fn decode_transport_info(bytes: &[u8]) -> Result<PlaybackState, UpnpAvError> {
    let values = response_fields(
        bytes,
        "GetTransportInfoResponse",
        &["CurrentTransportState"],
    )?;
    let state = required_field(&values, 0, "CurrentTransportState")?;
    Ok(parse_playback_state(state))
}

pub fn decode_position_info(bytes: &[u8]) -> Result<PositionInfo, UpnpAvError> {
    let values = response_fields(
        bytes,
        "GetPositionInfoResponse",
        &["TrackURI", "TrackMetaData"],
    )?;
    Ok(PositionInfo {
        track_uri: values[0].clone(),
        track_metadata: values[1].clone(),
    })
}

pub fn decode_volume(bytes: &[u8]) -> Result<u8, UpnpAvError> {
    let values = response_fields(bytes, "GetVolumeResponse", &["CurrentVolume"])?;
    let value = required_field(&values, 0, "CurrentVolume")?;
    value
        .parse::<u8>()
        .ok()
        .filter(|value| *value <= 100)
        .ok_or_else(|| UpnpAvError::Xml(format!("invalid percentage `{value}`")))
}

pub fn decode_mute(bytes: &[u8]) -> Result<bool, UpnpAvError> {
    let values = response_fields(bytes, "GetMuteResponse", &["CurrentMute"])?;
    match required_field(&values, 0, "CurrentMute")? {
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        value => Err(UpnpAvError::Xml(format!("invalid boolean `{value}`"))),
    }
}

pub fn decode_action_response(bytes: &[u8], action: Action) -> Result<(), UpnpAvError> {
    match action {
        Action::Play | Action::Pause | Action::Stop | Action::SetVolume(_) | Action::SetMute(_) => {
            let response_name = format!("{}Response", action.name());
            response_fields(bytes, &response_name, &[]).map(|_| ())
        }
        _ => Err(UpnpAvError::Validation(
            "read action requires its typed decoder".to_string(),
        )),
    }
}

pub fn parse_didl_metadata(source: &str) -> Result<(Option<String>, Option<String>), UpnpAvError> {
    if source.trim().is_empty() {
        return Ok((None, None));
    }
    let document = parse_xml(source).map_err(|error| UpnpAvError::Xml(error.to_string()))?;
    Ok((
        descendant(&document.root, "title")
            .map(XmlElement::text_content)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        descendant(&document.root, "creator")
            .or_else(|| descendant(&document.root, "artist"))
            .map(XmlElement::text_content)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    ))
}

fn response_fields(
    bytes: &[u8],
    response_name: &str,
    names: &[&str],
) -> Result<Vec<Option<String>>, UpnpAvError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| UpnpAvError::Xml("SOAP response is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| UpnpAvError::Xml(error.to_string()))?;
    if descendant(&document.root, "Fault").is_some() {
        return Err(UpnpAvError::Xml(
            "SOAP response contains a fault".to_string(),
        ));
    }
    let response = descendant(&document.root, response_name)
        .ok_or_else(|| UpnpAvError::Xml(format!("SOAP response is missing {response_name}")))?;
    Ok(names
        .iter()
        .map(|name| {
            descendant(response, name)
                .map(XmlElement::text_content)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty() && value != "NOT_IMPLEMENTED")
        })
        .collect())
}

fn required_field<'a>(
    values: &'a [Option<String>],
    index: usize,
    name: &str,
) -> Result<&'a str, UpnpAvError> {
    values
        .get(index)
        .and_then(Option::as_deref)
        .ok_or_else(|| UpnpAvError::Xml(format!("SOAP response is missing {name}")))
}

pub fn parse_playback_state(value: &str) -> PlaybackState {
    match value {
        "PLAYING" => PlaybackState::Play,
        "PAUSED_PLAYBACK" => PlaybackState::Pause,
        "STOPPED" => PlaybackState::Stop,
        "TRANSITIONING" => PlaybackState::Transitioning,
        "NO_MEDIA_PRESENT" => PlaybackState::NoMedia,
        state => PlaybackState::Other(state.to_ascii_lowercase()),
    }
}

fn child_text(root: &XmlElement, name: &str) -> Option<String> {
    root.children
        .iter()
        .find_map(|child| match child {
            XmlNode::Element(element) if element.local_name == name => Some(element),
            _ => None,
        })
        .map(|element| element.text_content())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_child_text(root: &XmlElement, name: &str) -> Result<String, UpnpAvError> {
    child_text(root, name).ok_or_else(|| UpnpAvError::Xml(format!("device is missing {name}")))
}

fn descendant<'a>(root: &'a XmlElement, name: &str) -> Option<&'a XmlElement> {
    if root.local_name == name {
        return Some(root);
    }
    root.children.iter().find_map(|child| match child {
        XmlNode::Element(element) => descendant(element, name),
        XmlNode::Text(_)
        | XmlNode::CData(_)
        | XmlNode::Comment(_)
        | XmlNode::ProcessingInstruction { .. } => None,
    })
}

fn collect_descendants<'a>(root: &'a XmlElement, name: &str, output: &mut Vec<&'a XmlElement>) {
    if root.local_name == name {
        output.push(root);
    }
    for child in &root.children {
        if let XmlNode::Element(element) = child {
            collect_descendants(element, name, output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESCRIPTION: &str = r#"<root><device><deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType><friendlyName>Living Room TV</friendlyName><manufacturer>Example</manufacturer><modelName>MR-1</modelName><serialNumber>abc</serialNumber><UDN>uuid:renderer-1</UDN><serviceList><service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/av</controlURL></service><service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType><controlURL>/render</controlURL></service></serviceList></device></root>"#;

    #[test]
    fn parses_exact_renderer_and_services() {
        let description =
            parse_renderer_description(DESCRIPTION.as_bytes(), MEDIA_RENDERER_DEVICE_TYPE).unwrap();
        assert_eq!(description.friendly_name, "Living Room TV");
        assert_eq!(description.manufacturer.as_deref(), Some("Example"));
        assert_eq!(description.av_transport.control_url, "/av");
        assert_eq!(description.rendering_control.control_url, "/render");
    }

    #[test]
    fn rejects_wrong_device_type_and_missing_service() {
        assert!(parse_renderer_description(DESCRIPTION.as_bytes(), "wrong").is_err());
        let missing = DESCRIPTION.replace(
            "urn:schemas-upnp-org:service:RenderingControl:1",
            "urn:example:missing",
        );
        assert!(matches!(
            parse_renderer_description(missing.as_bytes(), MEDIA_RENDERER_DEVICE_TYPE),
            Err(UpnpAvError::MissingService(RENDERING_CONTROL_SERVICE_TYPE))
        ));
    }

    #[test]
    fn emits_only_fixed_actions() {
        let play = encode_action(Action::Play).unwrap();
        assert_eq!(
            play.action_header,
            "\"urn:schemas-upnp-org:service:AVTransport:1#Play\""
        );
        assert!(String::from_utf8(play.body)
            .unwrap()
            .contains("<Speed>1</Speed>"));
        assert!(encode_action(Action::SetVolume(101)).is_err());
    }

    #[test]
    fn decodes_typed_responses_and_rejects_faults() {
        let transport = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetTransportInfoResponse xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><CurrentTransportState>PLAYING</CurrentTransportState></u:GetTransportInfoResponse></s:Body></s:Envelope>"#;
        assert_eq!(
            decode_transport_info(transport.as_bytes()).unwrap(),
            PlaybackState::Play
        );
        let volume = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetVolumeResponse xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1"><CurrentVolume>42</CurrentVolume></u:GetVolumeResponse></s:Body></s:Envelope>"#;
        assert_eq!(decode_volume(volume.as_bytes()).unwrap(), 42);
        let mute = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetMuteResponse xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1"><CurrentMute>1</CurrentMute></u:GetMuteResponse></s:Body></s:Envelope>"#;
        assert!(decode_mute(mute.as_bytes()).unwrap());
        let fault = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault/></s:Body></s:Envelope>"#;
        assert!(decode_transport_info(fault.as_bytes()).is_err());
    }
}
