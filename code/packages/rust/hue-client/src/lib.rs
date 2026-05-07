//! Transport-neutral Philips Hue CLIP v2 client core.
//!
//! The crate deliberately stops at HTTP-shaped messages. A runtime can bind the
//! transport to blocking sockets, async TLS, a simulator, or a capability cage
//! without changing the Hue request/response semantics.

#![forbid(unsafe_code)]

use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{parse as parse_json, JsonNumber, JsonValue};
use http_core::{find_header, Header};
use hue_core::{
    validate_brightness, HueCommand, HueLightResource, HueLightStateUpdate, HueMethod, HueRequest,
    HueRequestBody, HueResourceId, HueResourceRef, HueResourceType, CLIP_V2_EVENT_STREAM_PATH,
    CLIP_V2_RESOURCE_ROOT, HUE_APPLICATION_KEY_HEADER,
};
use std::fmt;

const CONTENT_TYPE_JSON: &str = "application/json";
const ACCEPT_EVENT_STREAM: &str = "text/event-stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HueClientError {
    MissingApplicationKey,
    InvalidRequest {
        message: String,
    },
    JsonEncode {
        message: String,
    },
    JsonDecode {
        message: String,
    },
    UnexpectedJson {
        message: String,
    },
    HttpStatus {
        status: u16,
        errors: Vec<HueApiError>,
    },
    ApiErrors(Vec<HueApiError>),
    Transport {
        message: String,
    },
}

impl HueClientError {
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
        }
    }

    fn unexpected_json(message: impl Into<String>) -> Self {
        Self::UnexpectedJson {
            message: message.into(),
        }
    }
}

impl fmt::Display for HueClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApplicationKey => write!(f, "Hue application key is required"),
            Self::InvalidRequest { message } => write!(f, "invalid Hue request: {message}"),
            Self::JsonEncode { message } => write!(f, "failed to encode Hue JSON: {message}"),
            Self::JsonDecode { message } => write!(f, "failed to decode Hue JSON: {message}"),
            Self::UnexpectedJson { message } => write!(f, "unexpected Hue JSON shape: {message}"),
            Self::HttpStatus { status, errors } => {
                write!(f, "Hue bridge returned HTTP {status}")?;
                if !errors.is_empty() {
                    write!(f, " with {} API error(s)", errors.len())?;
                }
                Ok(())
            }
            Self::ApiErrors(errors) => {
                write!(f, "Hue bridge returned {} API error(s)", errors.len())
            }
            Self::Transport { message } => write!(f, "Hue transport failed: {message}"),
        }
    }
}

impl std::error::Error for HueClientError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueApiError {
    pub error_type: Option<String>,
    pub address: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueRegistrationResult {
    pub application_key: String,
    pub client_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueEnvelope {
    pub data: Vec<JsonValue>,
    pub errors: Vec<HueApiError>,
}

impl HueEnvelope {
    pub fn ensure_success(self) -> Result<Self, HueClientError> {
        if self.errors.is_empty() {
            Ok(self)
        } else {
            Err(HueClientError::ApiErrors(self.errors))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueEventRecord {
    pub id: Option<String>,
    pub event_type: Option<String>,
    pub creation_time: Option<String>,
    pub data: Vec<JsonValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueEventStreamBatch {
    pub sse_id: Option<String>,
    pub sse_event_type: Option<String>,
    pub retry_ms: Option<u64>,
    pub events: Vec<HueEventRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueHttpRequest {
    pub method: HueMethod,
    pub path: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

impl HueHttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }

    pub fn method_name(&self) -> &'static str {
        hue_method_name(self.method)
    }

    pub fn has_body(&self) -> bool {
        !self.body.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueHttpResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

impl HueHttpResponse {
    pub fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: vec![Header {
                name: "Content-Type".to_string(),
                value: CONTENT_TYPE_JSON.to_string(),
            }],
            body: body.into(),
        }
    }
}

pub trait HueTransport {
    fn send(&mut self, request: HueHttpRequest) -> Result<HueHttpResponse, HueClientError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HueClientConfig {
    pub application_key: Option<String>,
}

impl HueClientConfig {
    pub fn paired(application_key: impl Into<String>) -> Self {
        Self {
            application_key: Some(application_key.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HueClient<T> {
    config: HueClientConfig,
    transport: T,
}

impl<T> HueClient<T> {
    pub fn new(config: HueClientConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn into_transport(self) -> T {
        self.transport
    }
}

impl<T: HueTransport> HueClient<T> {
    pub fn register_application(
        &mut self,
        app_name: impl Into<String>,
        instance_name: impl Into<String>,
    ) -> Result<HueRegistrationResult, HueClientError> {
        let response = self
            .transport
            .send(registration_request(app_name, instance_name)?)?;
        ensure_success_status(&response)?;
        parse_registration_response(&response.body)
    }

    pub fn get_resources(&mut self) -> Result<HueEnvelope, HueClientError> {
        let response = self
            .transport
            .send(resource_snapshot_request(self.application_key()?)?)?;
        parse_envelope_response(response)
    }

    pub fn get_collection(
        &mut self,
        resource_type: HueResourceType,
    ) -> Result<HueEnvelope, HueClientError> {
        let response = self.transport.send(resource_collection_request(
            self.application_key()?,
            &resource_type,
        )?)?;
        parse_envelope_response(response)
    }

    pub fn get_light_resources(&mut self) -> Result<Vec<HueLightResource>, HueClientError> {
        let envelope = self.get_collection(HueResourceType::Light)?;
        parse_lights_from_envelope(&envelope)
    }

    pub fn get_light_state_updates(&mut self) -> Result<Vec<HueLightStateUpdate>, HueClientError> {
        let envelope = self.get_collection(HueResourceType::Light)?;
        parse_light_state_updates_from_envelope(&envelope)
    }

    pub fn send_command(&mut self, command: HueCommand) -> Result<HueEnvelope, HueClientError> {
        let request = command.to_request();
        let response = self
            .transport
            .send(hue_request_to_http(request, Some(self.application_key()?))?)?;
        parse_envelope_response(response)
    }

    pub fn event_stream_request(&self) -> Result<HueHttpRequest, HueClientError> {
        event_stream_request(self.application_key()?)
    }

    fn application_key(&self) -> Result<&str, HueClientError> {
        self.config
            .application_key
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or(HueClientError::MissingApplicationKey)
    }
}

pub fn registration_request(
    app_name: impl Into<String>,
    instance_name: impl Into<String>,
) -> Result<HueHttpRequest, HueClientError> {
    hue_request_to_http(
        HueRequest {
            method: HueMethod::Post,
            path: "/api".to_string(),
            body: Some(HueRequestBody::RegisterApplication {
                app_name: app_name.into(),
                instance_name: instance_name.into(),
            }),
        },
        None,
    )
}

pub fn resource_snapshot_request(application_key: &str) -> Result<HueHttpRequest, HueClientError> {
    hue_request_to_http(
        HueRequest {
            method: HueMethod::Get,
            path: CLIP_V2_RESOURCE_ROOT.to_string(),
            body: None,
        },
        Some(application_key),
    )
}

pub fn resource_collection_request(
    application_key: &str,
    resource_type: &HueResourceType,
) -> Result<HueHttpRequest, HueClientError> {
    hue_request_to_http(
        HueRequest {
            method: HueMethod::Get,
            path: HueResourceRef::collection_path(resource_type),
            body: None,
        },
        Some(application_key),
    )
}

pub fn resource_request(
    application_key: &str,
    resource: &HueResourceRef,
) -> Result<HueHttpRequest, HueClientError> {
    hue_request_to_http(
        HueRequest {
            method: HueMethod::Get,
            path: resource.path(),
            body: None,
        },
        Some(application_key),
    )
}

pub fn event_stream_request(application_key: &str) -> Result<HueHttpRequest, HueClientError> {
    let mut request = hue_request_to_http(
        HueRequest {
            method: HueMethod::Get,
            path: CLIP_V2_EVENT_STREAM_PATH.to_string(),
            body: None,
        },
        Some(application_key),
    )?;
    request.headers.push(Header {
        name: "Accept".to_string(),
        value: ACCEPT_EVENT_STREAM.to_string(),
    });
    Ok(request)
}

pub fn hue_request_to_http(
    request: HueRequest,
    application_key: Option<&str>,
) -> Result<HueHttpRequest, HueClientError> {
    let mut headers = Vec::new();
    if let Some(application_key) = application_key {
        let application_key = application_key.trim();
        if application_key.is_empty() {
            return Err(HueClientError::MissingApplicationKey);
        }
        headers.push(Header {
            name: HUE_APPLICATION_KEY_HEADER.to_string(),
            value: application_key.to_string(),
        });
    }

    let body = match request.body {
        Some(body) => encode_request_body(&body)?.into_bytes(),
        None => Vec::new(),
    };

    if !body.is_empty() {
        headers.push(Header {
            name: "Content-Type".to_string(),
            value: CONTENT_TYPE_JSON.to_string(),
        });
        headers.push(Header {
            name: "Content-Length".to_string(),
            value: body.len().to_string(),
        });
    }

    Ok(HueHttpRequest {
        method: request.method,
        path: request.path,
        headers,
        body,
    })
}

pub fn hue_method_name(method: HueMethod) -> &'static str {
    match method {
        HueMethod::Get => "GET",
        HueMethod::Post => "POST",
        HueMethod::Put => "PUT",
        HueMethod::Delete => "DELETE",
    }
}

pub fn encode_request_body(body: &HueRequestBody) -> Result<String, HueClientError> {
    let value = match body {
        HueRequestBody::RegisterApplication {
            app_name,
            instance_name,
        } => JsonValue::Object(vec![
            (
                "devicetype".to_string(),
                JsonValue::String(format!("{app_name}#{instance_name}")),
            ),
            ("generateclientkey".to_string(), JsonValue::Bool(true)),
        ]),
        HueRequestBody::SetOn { on } => JsonValue::Object(vec![(
            "on".to_string(),
            JsonValue::Object(vec![("on".to_string(), JsonValue::Bool(*on))]),
        )]),
        HueRequestBody::SetBrightness { brightness } => {
            let brightness = validate_brightness(u16::from(*brightness)).map_err(|error| {
                HueClientError::InvalidRequest {
                    message: error.to_string(),
                }
            })?;
            JsonValue::Object(vec![(
                "dimming".to_string(),
                JsonValue::Object(vec![(
                    "brightness".to_string(),
                    JsonValue::Number(JsonNumber::Integer(i64::from(brightness))),
                )]),
            )])
        }
        HueRequestBody::SetColorTemperature { mirek } => JsonValue::Object(vec![(
            "color_temperature".to_string(),
            JsonValue::Object(vec![(
                "mirek".to_string(),
                JsonValue::Number(JsonNumber::Integer(i64::from(*mirek))),
            )]),
        )]),
        HueRequestBody::RecallScene => JsonValue::Object(vec![(
            "recall".to_string(),
            JsonValue::Object(vec![(
                "action".to_string(),
                JsonValue::String("active".to_string()),
            )]),
        )]),
    };

    serialize(&value).map_err(|error| HueClientError::JsonEncode {
        message: error.message,
    })
}

pub fn parse_hue_envelope(body: &[u8]) -> Result<HueEnvelope, HueClientError> {
    let value = parse_body(body)?;
    let JsonValue::Object(_) = value else {
        return Err(HueClientError::unexpected_json(
            "Hue v2 response envelope must be a JSON object",
        ));
    };

    let data = match object_field(&value, "data") {
        Some(JsonValue::Array(values)) => values.clone(),
        Some(_) => {
            return Err(HueClientError::unexpected_json(
                "Hue v2 envelope data field must be an array",
            ))
        }
        None => Vec::new(),
    };
    let errors = match object_field(&value, "errors") {
        Some(JsonValue::Array(values)) => parse_api_errors(values)?,
        Some(_) => {
            return Err(HueClientError::unexpected_json(
                "Hue v2 envelope errors field must be an array",
            ))
        }
        None => Vec::new(),
    };

    Ok(HueEnvelope { data, errors })
}

pub fn parse_registration_response(body: &[u8]) -> Result<HueRegistrationResult, HueClientError> {
    let value = parse_body(body)?;
    let JsonValue::Array(entries) = value else {
        return Err(HueClientError::unexpected_json(
            "Hue registration response must be a JSON array",
        ));
    };

    let mut errors = Vec::new();
    for entry in &entries {
        if let Some(success) = object_field(entry, "success") {
            let application_key = object_string_field(success, "username")
                .or_else(|| object_string_field(success, "application_key"))
                .ok_or_else(|| {
                    HueClientError::unexpected_json(
                        "Hue registration success is missing username/application_key",
                    )
                })?;
            return Ok(HueRegistrationResult {
                application_key: application_key.to_string(),
                client_key: object_string_field(success, "clientkey").map(str::to_string),
            });
        }

        if let Some(error) = object_field(entry, "error") {
            errors.push(parse_api_error(error)?);
        }
    }

    if errors.is_empty() {
        Err(HueClientError::unexpected_json(
            "Hue registration response contained no success or error entries",
        ))
    } else {
        Err(HueClientError::ApiErrors(errors))
    }
}

pub fn parse_lights_from_envelope(
    envelope: &HueEnvelope,
) -> Result<Vec<HueLightResource>, HueClientError> {
    let mut lights = Vec::new();
    for resource in &envelope.data {
        if object_string_field(resource, "type") != Some(HueResourceType::Light.as_hue_type()) {
            continue;
        }

        let update = parse_light_state_update(resource)?;
        let owner_device_id = update.owner_device_id.ok_or_else(|| {
            HueClientError::unexpected_json("Hue light resource is missing owner.rid")
        })?;
        let name = update
            .name
            .unwrap_or_else(|| update.id.as_str().to_string());

        lights.push(HueLightResource {
            id: update.id,
            owner_device_id,
            name,
            on: update.on,
            brightness: update.brightness,
            color_temperature_mirek: update.color_temperature_mirek,
        });
    }
    Ok(lights)
}

pub fn parse_light_state_updates_from_envelope(
    envelope: &HueEnvelope,
) -> Result<Vec<HueLightStateUpdate>, HueClientError> {
    let mut updates = Vec::new();
    for resource in &envelope.data {
        if object_string_field(resource, "type") == Some(HueResourceType::Light.as_hue_type()) {
            updates.push(parse_light_state_update(resource)?);
        }
    }
    Ok(updates)
}

pub fn parse_light_state_updates_from_event_batches(
    batches: &[HueEventStreamBatch],
) -> Result<Vec<HueLightStateUpdate>, HueClientError> {
    let mut updates = Vec::new();
    for batch in batches {
        for event in &batch.events {
            for resource in &event.data {
                if object_string_field(resource, "type")
                    == Some(HueResourceType::Light.as_hue_type())
                {
                    updates.push(parse_light_state_update(resource)?);
                }
            }
        }
    }
    Ok(updates)
}

pub fn parse_event_stream(body: &[u8]) -> Result<Vec<HueEventStreamBatch>, HueClientError> {
    let text = std::str::from_utf8(body).map_err(|error| HueClientError::JsonDecode {
        message: error.to_string(),
    })?;
    let mut batches = Vec::new();
    let mut current = PartialSseEvent::default();

    for line in text.lines() {
        if line.is_empty() {
            if let Some(batch) = current.finish()? {
                batches.push(batch);
            }
            continue;
        }
        current.push_line(line)?;
    }
    if let Some(batch) = current.finish()? {
        batches.push(batch);
    }

    Ok(batches)
}

fn parse_envelope_response(response: HueHttpResponse) -> Result<HueEnvelope, HueClientError> {
    ensure_success_status(&response)?;
    parse_hue_envelope(&response.body)?.ensure_success()
}

fn ensure_success_status(response: &HueHttpResponse) -> Result<(), HueClientError> {
    if (200..300).contains(&response.status) {
        return Ok(());
    }

    let errors = parse_hue_envelope(&response.body)
        .map(|envelope| envelope.errors)
        .unwrap_or_default();
    Err(HueClientError::HttpStatus {
        status: response.status,
        errors,
    })
}

fn parse_body(body: &[u8]) -> Result<JsonValue, HueClientError> {
    let text = std::str::from_utf8(body).map_err(|error| HueClientError::JsonDecode {
        message: error.to_string(),
    })?;
    parse_json(text).map_err(|error| HueClientError::JsonDecode {
        message: error.message,
    })
}

#[derive(Debug, Default)]
struct PartialSseEvent {
    sse_id: Option<String>,
    sse_event_type: Option<String>,
    retry_ms: Option<u64>,
    data_lines: Vec<String>,
    saw_field: bool,
}

impl PartialSseEvent {
    fn push_line(&mut self, line: &str) -> Result<(), HueClientError> {
        if line.starts_with(':') {
            return Ok(());
        }
        self.saw_field = true;
        let (field, value) = line.split_once(':').map_or((line, ""), |(field, value)| {
            (field, value.strip_prefix(' ').unwrap_or(value))
        });
        match field {
            "id" => self.sse_id = Some(value.to_string()),
            "event" => self.sse_event_type = Some(value.to_string()),
            "retry" => {
                self.retry_ms = Some(value.parse::<u64>().map_err(|error| {
                    HueClientError::unexpected_json(format!("invalid Hue SSE retry field: {error}"))
                })?);
            }
            "data" => self.data_lines.push(value.to_string()),
            _ => {}
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<Option<HueEventStreamBatch>, HueClientError> {
        if !self.saw_field {
            return Ok(None);
        }
        let sse_id = self.sse_id.take();
        let sse_event_type = self.sse_event_type.take();
        let retry_ms = self.retry_ms.take();
        let data = self.data_lines.join("\n");
        let events = if data.trim().is_empty() {
            Vec::new()
        } else {
            parse_event_stream_records(&data)?
        };
        *self = Self::default();
        Ok(Some(HueEventStreamBatch {
            sse_id,
            sse_event_type,
            retry_ms,
            events,
        }))
    }
}

fn parse_event_stream_records(data: &str) -> Result<Vec<HueEventRecord>, HueClientError> {
    let value = parse_json(data).map_err(|error| HueClientError::JsonDecode {
        message: error.message,
    })?;
    let JsonValue::Array(entries) = value else {
        return Err(HueClientError::unexpected_json(
            "Hue event-stream data field must be a JSON array",
        ));
    };
    entries.iter().map(parse_event_record).collect()
}

fn parse_event_record(value: &JsonValue) -> Result<HueEventRecord, HueClientError> {
    let JsonValue::Object(_) = value else {
        return Err(HueClientError::unexpected_json(
            "Hue event-stream entry must be a JSON object",
        ));
    };
    let data = match object_field(value, "data") {
        Some(JsonValue::Array(values)) => values.clone(),
        Some(_) => {
            return Err(HueClientError::unexpected_json(
                "Hue event-stream entry data field must be an array",
            ))
        }
        None => Vec::new(),
    };
    Ok(HueEventRecord {
        id: object_string_field(value, "id").map(str::to_string),
        event_type: object_string_field(value, "type").map(str::to_string),
        creation_time: object_string_field(value, "creationtime").map(str::to_string),
        data,
    })
}

fn parse_light_state_update(resource: &JsonValue) -> Result<HueLightStateUpdate, HueClientError> {
    let id = object_string_field(resource, "id")
        .ok_or_else(|| HueClientError::unexpected_json("Hue light resource is missing id"))?;
    let owner_device_id = object_field(resource, "owner")
        .and_then(|owner| object_string_field(owner, "rid"))
        .map(HueResourceId::trusted);
    let name = object_field(resource, "metadata")
        .and_then(|metadata| object_string_field(metadata, "name"))
        .map(str::to_string);
    let on = object_field(resource, "on").and_then(|on| object_bool_field(on, "on"));
    let brightness = object_field(resource, "dimming")
        .and_then(|dimming| object_field(dimming, "brightness"))
        .map(json_number_to_percent)
        .transpose()?;
    let color_temperature_mirek = object_field(resource, "color_temperature")
        .and_then(|color_temperature| object_field(color_temperature, "mirek"))
        .map(json_number_to_u16)
        .transpose()?;

    Ok(HueLightStateUpdate {
        id: HueResourceId::trusted(id),
        owner_device_id,
        name,
        on,
        brightness,
        color_temperature_mirek,
    })
}

fn parse_api_errors(values: &[JsonValue]) -> Result<Vec<HueApiError>, HueClientError> {
    values.iter().map(parse_api_error).collect()
}

fn parse_api_error(value: &JsonValue) -> Result<HueApiError, HueClientError> {
    let JsonValue::Object(_) = value else {
        return Err(HueClientError::unexpected_json(
            "Hue API error entry must be an object",
        ));
    };

    let description = object_string_field(value, "description")
        .map(str::to_string)
        .unwrap_or_else(|| "unknown Hue API error".to_string());
    Ok(HueApiError {
        error_type: object_field(value, "type").and_then(json_scalar_to_string),
        address: object_string_field(value, "address").map(str::to_string),
        description,
    })
}

fn object_field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    match value {
        JsonValue::Object(fields) => fields
            .iter()
            .find(|(field_name, _)| field_name == name)
            .map(|(_, field_value)| field_value),
        _ => None,
    }
}

fn object_string_field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a str> {
    match object_field(value, name) {
        Some(JsonValue::String(value)) => Some(value),
        _ => None,
    }
}

fn object_bool_field(value: &JsonValue, name: &str) -> Option<bool> {
    match object_field(value, name) {
        Some(JsonValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn json_scalar_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Number(JsonNumber::Integer(value)) => Some(value.to_string()),
        JsonValue::Number(JsonNumber::Float(value)) => Some(value.to_string()),
        JsonValue::Bool(value) => Some(value.to_string()),
        JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_) => None,
    }
}

fn json_number_to_percent(value: &JsonValue) -> Result<u8, HueClientError> {
    let value = json_number_to_f64(value)?;
    if !(0.0..=100.0).contains(&value) || !value.is_finite() {
        return Err(HueClientError::unexpected_json(
            "Hue brightness must be a finite number in 0..=100",
        ));
    }
    validate_brightness(value.round() as u16).map_err(|error| {
        HueClientError::unexpected_json(format!("invalid Hue brightness: {error}"))
    })
}

fn json_number_to_u16(value: &JsonValue) -> Result<u16, HueClientError> {
    let value = json_number_to_f64(value)?;
    if !(0.0..=f64::from(u16::MAX)).contains(&value) || !value.is_finite() {
        return Err(HueClientError::unexpected_json(
            "Hue numeric field is outside u16 range",
        ));
    }
    Ok(value.round() as u16)
}

fn json_number_to_f64(value: &JsonValue) -> Result<f64, HueClientError> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(*value as f64),
        JsonValue::Number(JsonNumber::Float(value)) => Ok(*value),
        _ => Err(HueClientError::unexpected_json(
            "expected Hue numeric field",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct RecordingTransport {
        requests: Vec<HueHttpRequest>,
        responses: Vec<HueHttpResponse>,
    }

    impl RecordingTransport {
        fn with_response(body: &'static str) -> Self {
            Self {
                requests: Vec::new(),
                responses: vec![HueHttpResponse::json(200, body.as_bytes())],
            }
        }
    }

    impl HueTransport for RecordingTransport {
        fn send(&mut self, request: HueHttpRequest) -> Result<HueHttpResponse, HueClientError> {
            self.requests.push(request);
            if self.responses.is_empty() {
                return Err(HueClientError::transport("no mock response queued"));
            }
            Ok(self.responses.remove(0))
        }
    }

    #[test]
    fn registration_request_uses_clip_v1_pairing_shape_without_app_key() {
        let request = registration_request("chief-of-staff", "desk").unwrap();

        assert_eq!(request.method, HueMethod::Post);
        assert_eq!(request.method_name(), "POST");
        assert_eq!(request.path, "/api");
        assert_eq!(request.header(HUE_APPLICATION_KEY_HEADER), None);
        assert_eq!(
            std::str::from_utf8(&request.body).unwrap(),
            r#"{"devicetype":"chief-of-staff#desk","generateclientkey":true}"#
        );
    }

    #[test]
    fn resource_snapshot_request_includes_application_key() {
        let request = resource_snapshot_request("app-key").unwrap();

        assert_eq!(request.method, HueMethod::Get);
        assert_eq!(request.path, "/clip/v2/resource");
        assert_eq!(request.header(HUE_APPLICATION_KEY_HEADER), Some("app-key"));
        assert!(!request.has_body());
    }

    #[test]
    fn command_requests_encode_structured_json_bodies() {
        let command = HueCommand::SetLightBrightness {
            light_id: HueResourceId::trusted("light-1"),
            brightness: 70,
        };
        let request = hue_request_to_http(command.to_request(), Some("app-key")).unwrap();

        assert_eq!(request.method, HueMethod::Put);
        assert_eq!(request.path, "/clip/v2/resource/light/light-1");
        assert_eq!(request.header("Content-Type"), Some(CONTENT_TYPE_JSON));
        assert_eq!(
            std::str::from_utf8(&request.body).unwrap(),
            r#"{"dimming":{"brightness":70}}"#
        );
    }

    #[test]
    fn command_requests_reject_invalid_brightness() {
        let command = HueCommand::SetLightBrightness {
            light_id: HueResourceId::trusted("light-1"),
            brightness: 255,
        };

        assert!(matches!(
            hue_request_to_http(command.to_request(), Some("app-key")),
            Err(HueClientError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn event_stream_request_uses_sse_accept_header() {
        let request = event_stream_request("app-key").unwrap();

        assert_eq!(request.method, HueMethod::Get);
        assert_eq!(request.path, "/eventstream/clip/v2");
        assert_eq!(request.header("Accept"), Some(ACCEPT_EVENT_STREAM));
        assert_eq!(request.header(HUE_APPLICATION_KEY_HEADER), Some("app-key"));
    }

    #[test]
    fn parses_hue_event_stream_batches() {
        let batches = parse_event_stream(
            b": keepalive\nid: stream-1\nevent: update\nretry: 5000\ndata: [{\"creationtime\":\"2026-05-07T01:00:00Z\",\"data\":[{\"id\":\"light-1\",\"type\":\"light\",\"on\":{\"on\":true}}],\"id\":\"event-1\",\"type\":\"update\"}]\n\n",
        )
        .unwrap();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].sse_id.as_deref(), Some("stream-1"));
        assert_eq!(batches[0].sse_event_type.as_deref(), Some("update"));
        assert_eq!(batches[0].retry_ms, Some(5_000));
        assert_eq!(batches[0].events.len(), 1);
        assert_eq!(batches[0].events[0].id.as_deref(), Some("event-1"));
        assert_eq!(batches[0].events[0].event_type.as_deref(), Some("update"));
        assert_eq!(
            batches[0].events[0].creation_time.as_deref(),
            Some("2026-05-07T01:00:00Z")
        );
        assert_eq!(
            object_string_field(&batches[0].events[0].data[0], "id"),
            Some("light-1")
        );
    }

    #[test]
    fn parses_light_state_updates_from_event_stream_batches() {
        let batches = parse_event_stream(
            b"data: [{\"id\":\"event-1\",\"type\":\"update\",\"data\":[{\"id\":\"light-1\",\"type\":\"light\",\"on\":{\"on\":false},\"dimming\":{\"brightness\":5},\"color_temperature\":{\"mirek\":250}}]}]\n\n",
        )
        .unwrap();

        let updates = parse_light_state_updates_from_event_batches(&batches).unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].id.as_str(), "light-1");
        assert_eq!(updates[0].owner_device_id, None);
        assert_eq!(updates[0].name, None);
        assert_eq!(updates[0].on, Some(false));
        assert_eq!(updates[0].brightness, Some(5));
        assert_eq!(updates[0].color_temperature_mirek, Some(250));

        let deltas = updates[0].state_deltas();
        assert_eq!(deltas.len(), 3);
        assert_eq!(deltas[0].capability_id.as_str(), "light.on_off");
        assert_eq!(deltas[1].capability_id.as_str(), "light.brightness");
        assert_eq!(deltas[2].capability_id.as_str(), "light.color_temperature");
    }

    #[test]
    fn event_stream_parser_accepts_multiline_data() {
        let batches = parse_event_stream(
            b"data: [{\"id\":\"event-1\",\"type\":\"update\",\"data\":[]}\ndata: ,{\"id\":\"event-2\",\"type\":\"add\",\"data\":[]}]\n\n",
        )
        .unwrap();

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].events.len(), 2);
        assert_eq!(batches[0].events[1].event_type.as_deref(), Some("add"));
    }

    #[test]
    fn event_stream_parser_rejects_invalid_data_shape() {
        assert!(matches!(
            parse_event_stream(b"data: {\"type\":\"update\"}\n\n"),
            Err(HueClientError::UnexpectedJson { .. })
        ));
    }

    #[test]
    fn client_sends_request_through_injected_transport() {
        let transport = RecordingTransport::with_response(r#"{"data":[],"errors":[]}"#);
        let mut client = HueClient::new(HueClientConfig::paired("app-key"), transport);

        let envelope = client.get_resources().unwrap();
        let transport = client.into_transport();

        assert_eq!(envelope.data, Vec::new());
        assert_eq!(transport.requests.len(), 1);
        assert_eq!(transport.requests[0].path, "/clip/v2/resource");
    }

    #[test]
    fn parses_registration_success() {
        let registration = parse_registration_response(
            br#"[{"success":{"username":"app-key","clientkey":"client-key"}}]"#,
        )
        .unwrap();

        assert_eq!(registration.application_key, "app-key");
        assert_eq!(registration.client_key.as_deref(), Some("client-key"));
    }

    #[test]
    fn parses_hue_v2_envelope_errors() {
        let envelope = parse_hue_envelope(
            br#"{"data":[],"errors":[{"type":7,"address":"/lights/1","description":"denied"}]}"#,
        )
        .unwrap();

        assert_eq!(envelope.errors.len(), 1);
        assert_eq!(envelope.errors[0].error_type.as_deref(), Some("7"));
        assert_eq!(envelope.errors[0].description, "denied");
    }

    #[test]
    fn parses_light_resources_from_snapshot_envelope() {
        let envelope = parse_hue_envelope(
            br#"{"data":[{"id":"light-1","type":"light","metadata":{"name":"Kitchen"},"owner":{"rid":"device-1","rtype":"device"},"on":{"on":true},"dimming":{"brightness":42},"color_temperature":{"mirek":366}},{"id":"room-1","type":"room"}],"errors":[]}"#,
        )
        .unwrap();

        let lights = parse_lights_from_envelope(&envelope).unwrap();

        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].id.as_str(), "light-1");
        assert_eq!(lights[0].owner_device_id.as_str(), "device-1");
        assert_eq!(lights[0].name, "Kitchen");
        assert_eq!(lights[0].on, Some(true));
        assert_eq!(lights[0].brightness, Some(42));
        assert_eq!(lights[0].color_temperature_mirek, Some(366));
    }

    #[test]
    fn parses_light_state_updates_from_snapshot_envelope() {
        let envelope = parse_hue_envelope(
            br#"{"data":[{"id":"light-1","type":"light","on":{"on":true}},{"id":"room-1","type":"room"}],"errors":[]}"#,
        )
        .unwrap();

        let updates = parse_light_state_updates_from_envelope(&envelope).unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].id.as_str(), "light-1");
        assert_eq!(updates[0].on, Some(true));
        assert!(updates[0].has_state());
    }

    #[test]
    fn paired_client_requires_application_key() {
        let transport = RecordingTransport::default();
        let mut client = HueClient::new(HueClientConfig::default(), transport);

        assert_eq!(
            client.get_resources().unwrap_err(),
            HueClientError::MissingApplicationKey
        );
    }
}
