//! Home Assistant-compatible local HTTP API routes for the smart-home platform.
//!
//! The crate builds a `web-core::WebApp` over runtime-owned smart-home registry
//! snapshots. It deliberately uses the repo's own HTTP server stack and keeps
//! mutation routes out until they can be wired through runtime command and
//! desired-state authorization paths.

#![forbid(unsafe_code)]

use smart_home_core::{
    Capability, CapabilityMode, Entity, EntityKind, Scene, StateConfidence, StateSource, Value,
};
use smart_home_runtime::SmartHomeRuntime;
use std::collections::BTreeMap;
use std::sync::Arc;
use web_core::{WebApp, WebResponse};

pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomePlatformHttpConfig {
    pub location_name: String,
    pub unit_system: String,
    pub time_zone: String,
    pub version: String,
}

impl SmartHomePlatformHttpConfig {
    pub fn new(location_name: impl Into<String>) -> Self {
        Self {
            location_name: location_name.into(),
            unit_system: "metric".to_string(),
            time_zone: "UTC".to_string(),
            version: VERSION.to_string(),
        }
    }

    pub fn with_unit_system(mut self, unit_system: impl Into<String>) -> Self {
        self.unit_system = unit_system.into();
        self
    }

    pub fn with_time_zone(mut self, time_zone: impl Into<String>) -> Self {
        self.time_zone = time_zone.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmartHomePlatformHttpState {
    pub config: SmartHomePlatformHttpConfig,
    pub entities: Vec<Entity>,
    pub scenes: Vec<Scene>,
    pub event_types: Vec<String>,
    pub generated_at_ms: u64,
}

impl SmartHomePlatformHttpState {
    pub fn from_runtime(
        runtime: &SmartHomeRuntime,
        config: SmartHomePlatformHttpConfig,
        event_types: impl IntoIterator<Item = impl Into<String>>,
        generated_at_ms: u64,
    ) -> Self {
        let mut event_types = event_types.into_iter().map(Into::into).collect::<Vec<_>>();
        event_types.sort();
        event_types.dedup();

        Self {
            config,
            entities: runtime.registry().entities().cloned().collect(),
            scenes: runtime.registry().scenes().cloned().collect(),
            event_types,
            generated_at_ms,
        }
    }

    pub fn summary(&self) -> SmartHomePlatformHttpSummary {
        SmartHomePlatformHttpSummary::from_state(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartHomePlatformHttpSummary {
    pub state_count: usize,
    pub known_state_count: usize,
    pub unknown_state_count: usize,
    pub stale_state_count: usize,
    pub optimistic_state_count: usize,
    pub service_count: usize,
    pub event_type_count: usize,
    pub scene_count: usize,
}

impl SmartHomePlatformHttpSummary {
    pub fn from_state(state: &SmartHomePlatformHttpState) -> Self {
        let mut summary = Self {
            state_count: state.entities.len(),
            event_type_count: state.event_types.len(),
            scene_count: state.scenes.len(),
            service_count: platform_services(state).len(),
            ..Self::default()
        };

        for entity in &state.entities {
            match &entity.state {
                Some(snapshot) if snapshot.confidence == StateConfidence::Stale => {
                    summary.stale_state_count += 1;
                }
                Some(snapshot) if snapshot.confidence == StateConfidence::Optimistic => {
                    summary.optimistic_state_count += 1;
                    summary.known_state_count += 1;
                }
                Some(_) => summary.known_state_count += 1,
                None => summary.unknown_state_count += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomePlatformService {
    pub domain: String,
    pub service: String,
    pub description: String,
    pub target_entity_ids: Vec<String>,
    pub target_scene_ids: Vec<String>,
    pub capability_ids: Vec<String>,
}

pub fn home_assistant_web_app(state: SmartHomePlatformHttpState) -> WebApp {
    let state = Arc::new(state);
    let mut app = WebApp::new();

    {
        let state = Arc::clone(&state);
        app.get("/api/config", move |_| {
            WebResponse::json(config_json(&state).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/states", move |_| {
            WebResponse::json(states_json(&state.entities, state.generated_at_ms).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/states/:entity_id", move |request| {
            let Some(entity_id) = request.route_params.get("entity_id") else {
                return WebResponse::new(400, br#"{"error":"missing entity_id"}"#.to_vec())
                    .with_content_type("application/json");
            };
            match state
                .entities
                .iter()
                .find(|entity| entity.entity_id.as_str() == entity_id)
            {
                Some(entity) => {
                    WebResponse::json(state_json(entity, state.generated_at_ms).into_bytes())
                }
                None => WebResponse::new(404, br#"{"error":"entity not found"}"#.to_vec())
                    .with_content_type("application/json"),
            }
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/services", move |_| {
            WebResponse::json(services_json(&platform_services(&state)).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/events", move |_| {
            WebResponse::json(events_json(&state.event_types).into_bytes())
        });
    }

    app
}

pub fn platform_services(state: &SmartHomePlatformHttpState) -> Vec<SmartHomePlatformService> {
    let mut services = BTreeMap::<(String, String), SmartHomePlatformService>::new();

    for entity in &state.entities {
        let domain = entity_domain(entity.kind).to_string();
        for capability in entity
            .capabilities
            .iter()
            .filter(|capability| capability_allows_command(capability))
        {
            for service in services_for_capability(&domain, capability) {
                let key = (domain.clone(), service.to_string());
                let entry = services
                    .entry(key)
                    .or_insert_with(|| SmartHomePlatformService {
                        domain: domain.clone(),
                        service: service.to_string(),
                        description: format!("{} {}", service.replace('_', " "), domain),
                        target_entity_ids: Vec::new(),
                        target_scene_ids: Vec::new(),
                        capability_ids: Vec::new(),
                    });
                push_unique_string(&mut entry.target_entity_ids, entity.entity_id.as_str());
                push_unique_string(&mut entry.capability_ids, capability.capability_id.as_str());
            }
        }
    }

    if !state.scenes.is_empty() {
        let entry = services
            .entry(("scene".to_string(), "turn_on".to_string()))
            .or_insert_with(|| SmartHomePlatformService {
                domain: "scene".to_string(),
                service: "turn_on".to_string(),
                description: "activate scene".to_string(),
                target_entity_ids: Vec::new(),
                target_scene_ids: Vec::new(),
                capability_ids: vec!["scene.recall".to_string()],
            });
        for scene in &state.scenes {
            push_unique_string(&mut entry.target_scene_ids, scene.scene_id.as_str());
        }
    }

    services.into_values().collect()
}

fn config_json(state: &SmartHomePlatformHttpState) -> String {
    let summary = state.summary();
    format!(
        "{{\"location_name\":{},\"unit_system\":{},\"time_zone\":{},\"version\":{},\"components\":[\"smart_home\"],\"state_count\":{},\"service_count\":{},\"event_type_count\":{},\"generated_at_ms\":{}}}",
        json_string(&state.config.location_name),
        json_string(&state.config.unit_system),
        json_string(&state.config.time_zone),
        json_string(&state.config.version),
        summary.state_count,
        summary.service_count,
        summary.event_type_count,
        state.generated_at_ms,
    )
}

fn states_json(entities: &[Entity], now_ms: u64) -> String {
    format!(
        "[{}]",
        entities
            .iter()
            .map(|entity| state_json(entity, now_ms))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn state_json(entity: &Entity, now_ms: u64) -> String {
    let (state_value, last_changed_ms, last_updated_ms, source, confidence, stale) =
        match &entity.state {
            Some(snapshot) => (
                value_json(&snapshot.value),
                snapshot.observed_at_ms,
                snapshot.received_at_ms,
                state_source_label(snapshot.source),
                state_confidence_label(snapshot.confidence),
                snapshot.is_stale_at(now_ms),
            ),
            None => (json_string("unknown"), 0, 0, "unknown", "unknown", true),
        };

    let capability_ids = entity
        .capabilities
        .iter()
        .map(|capability| json_string(capability.capability_id.as_str()))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"entity_id\":{},\"state\":{},\"attributes\":{{\"friendly_name\":{},\"device_id\":{},\"domain\":{},\"entity_kind\":{},\"capability_count\":{},\"capabilities\":[{}],\"stale\":{}}},\"last_changed_ms\":{},\"last_updated_ms\":{},\"context\":{{\"source\":{},\"confidence\":{}}}}}",
        json_string(entity.entity_id.as_str()),
        state_value,
        json_string(&entity.name),
        json_string(entity.device_id.as_str()),
        json_string(entity_domain(entity.kind)),
        json_string(entity_kind_label(entity.kind)),
        entity.capabilities.len(),
        capability_ids,
        stale,
        last_changed_ms,
        last_updated_ms,
        json_string(source),
        json_string(confidence),
    )
}

fn services_json(services: &[SmartHomePlatformService]) -> String {
    let mut domains = BTreeMap::<&str, Vec<&SmartHomePlatformService>>::new();
    for service in services {
        domains.entry(&service.domain).or_default().push(service);
    }

    format!(
        "[{}]",
        domains
            .into_iter()
            .map(|(domain, services)| {
                format!(
                    "{{\"domain\":{},\"services\":[{}]}}",
                    json_string(domain),
                    services
                        .into_iter()
                        .map(service_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn service_json(service: &SmartHomePlatformService) -> String {
    format!(
        "{{\"service\":{},\"description\":{},\"target_entity_ids\":[{}],\"target_scene_ids\":[{}],\"capability_ids\":[{}]}}",
        json_string(&service.service),
        json_string(&service.description),
        json_string_array(&service.target_entity_ids),
        json_string_array(&service.target_scene_ids),
        json_string_array(&service.capability_ids),
    )
}

fn events_json(event_types: &[String]) -> String {
    format!(
        "[{}]",
        event_types
            .iter()
            .map(|event_type| {
                format!(
                    "{{\"event\":{},\"description\":{}}}",
                    json_string(event_type),
                    json_string(format!("{event_type} platform event")),
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn services_for_capability(domain: &str, capability: &Capability) -> Vec<&'static str> {
    match capability.capability_id.as_str() {
        "light.on_off" => vec!["turn_on", "turn_off"],
        "light.brightness" => vec!["set_brightness"],
        "light.color" => vec!["set_color"],
        "light.color_temperature" => vec!["set_color_temperature"],
        "lock.state" => vec!["lock", "unlock"],
        "climate.setpoint" => vec!["set_temperature"],
        "scene.recall" => vec!["turn_on"],
        _ if domain == "input" => vec!["set_value"],
        _ => vec!["set_value"],
    }
}

fn capability_allows_command(capability: &Capability) -> bool {
    matches!(
        capability.mode,
        CapabilityMode::Command | CapabilityMode::ObserveAndCommand
    )
}

fn entity_domain(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Light => "light",
        EntityKind::LightGroup => "light",
        EntityKind::Switch => "switch",
        EntityKind::Sensor => "sensor",
        EntityKind::Lock => "lock",
        EntityKind::Thermostat => "climate",
        EntityKind::Scene => "scene",
        EntityKind::Input => "input",
        EntityKind::BridgeHealth => "binary_sensor",
        EntityKind::NetworkDiagnostic => "diagnostic",
        EntityKind::Unknown => "unknown",
    }
}

fn entity_kind_label(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Light => "light",
        EntityKind::LightGroup => "light_group",
        EntityKind::Switch => "switch",
        EntityKind::Sensor => "sensor",
        EntityKind::Lock => "lock",
        EntityKind::Thermostat => "thermostat",
        EntityKind::Scene => "scene",
        EntityKind::Input => "input",
        EntityKind::BridgeHealth => "bridge_health",
        EntityKind::NetworkDiagnostic => "network_diagnostic",
        EntityKind::Unknown => "unknown",
    }
}

fn state_source_label(source: StateSource) -> &'static str {
    match source {
        StateSource::EventStream => "event_stream",
        StateSource::Poll => "poll",
        StateSource::OptimisticCommand => "optimistic_command",
        StateSource::Manual => "manual",
    }
}

fn state_confidence_label(confidence: StateConfidence) -> &'static str {
    match confidence {
        StateConfidence::Confirmed => "confirmed",
        StateConfidence::Optimistic => "optimistic",
        StateConfidence::Stale => "stale",
        StateConfidence::Unknown => "unknown",
    }
}

fn value_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Number(value) if value.is_finite() => value.to_string(),
        Value::Number(_) => "null".to_string(),
        Value::Percentage(value) => value.to_string(),
        Value::Text(value) => json_string(value),
        Value::Object(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(key, value)| format!("{}:{}", json_string(key), value_json(value)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Array(values) => format!(
            "[{}]",
            values.iter().map(value_json).collect::<Vec<_>>().join(",")
        ),
    }
}

fn json_string_array(values: &[String]) -> String {
    values.iter().map(json_string).collect::<Vec<_>>().join(",")
}

fn json_string(value: impl AsRef<str>) -> String {
    let mut escaped = String::from("\"");
    for ch in value.as_ref().chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddable_http_server::{HttpRequest, HttpServerOptions};
    use http_core::{Header, HttpVersion, RequestHead};
    use smart_home_testkit::hue_lighting_runtime;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use tcp_runtime::{ConnectionId, TcpConnectionInfo};
    use web_core::WebServer;

    fn request(method: &str, target: &str) -> HttpRequest {
        HttpRequest {
            connection: TcpConnectionInfo {
                id: ConnectionId(0),
                peer_addr: SocketAddr::from(([127, 0, 0, 1], 10_000)),
                local_addr: SocketAddr::from(([127, 0, 0, 1], 8123)),
            },
            head: RequestHead {
                method: method.to_string(),
                target: target.to_string(),
                version: HttpVersion { major: 1, minor: 1 },
                headers: vec![Header {
                    name: "Host".to_string(),
                    value: "localhost".to_string(),
                }],
            },
            body: Vec::new(),
        }
    }

    fn response_body(response: web_core::WebResponse) -> String {
        String::from_utf8(response.body).expect("json response is utf8")
    }

    fn http_get(port: u16, path: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");

        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).expect("write request");

        let mut reader = BufReader::new(&stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .expect("read status line");
        let status = status_line
            .split_whitespace()
            .nth(1)
            .expect("status code")
            .parse()
            .expect("parse status code");

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header");
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if trimmed.to_ascii_lowercase().starts_with("content-length:") {
                content_length = trimmed
                    .split_once(':')
                    .map(|(_, value)| value.trim().parse().unwrap_or(0))
                    .unwrap_or(0);
            }
        }

        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).expect("read response body");
        (
            status,
            String::from_utf8(body).expect("json response is utf8"),
        )
    }

    fn start_server(app: WebApp) -> (u16, tcp_runtime::StopHandle) {
        let app = Arc::new(app);

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        let mut server = WebServer::bind_kqueue(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind kqueue");

        #[cfg(target_os = "linux")]
        let mut server = WebServer::bind_epoll(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind epoll");

        #[cfg(target_os = "windows")]
        let mut server = WebServer::bind_windows(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind windows");

        let port = server.local_addr().port();
        let stop = server.stop_handle();
        thread::spawn(move || {
            let _ = server.serve();
        });
        thread::sleep(Duration::from_millis(20));
        (port, stop)
    }

    fn fixture_state() -> SmartHomePlatformHttpState {
        let runtime = hue_lighting_runtime();
        SmartHomePlatformHttpState::from_runtime(
            &runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
            ["state_changed", "call_service"],
            5_000,
        )
    }

    #[test]
    fn platform_http_summary_counts_runtime_snapshot_shape() {
        let state = fixture_state();
        let summary = state.summary();

        assert_eq!(summary.state_count, 2);
        assert_eq!(summary.scene_count, 1);
        assert_eq!(summary.unknown_state_count, 2);
        assert_eq!(summary.event_type_count, 2);
        assert!(summary.service_count >= 4);
    }

    #[test]
    fn home_assistant_web_app_serves_config_states_services_and_events() {
        let state = fixture_state();
        let app = home_assistant_web_app(state);

        let config = response_body(app.handle(request("GET", "/api/config")).into());
        assert!(config.contains(r#""location_name":"Codex Home""#));
        assert!(config.contains(r#""state_count":2"#));

        let states = response_body(app.handle(request("GET", "/api/states")).into());
        assert!(states.contains(r#""entity_id":"entity-light-1""#));
        assert!(states.contains(r#""domain":"light""#));
        assert!(states.contains(r#""state":"unknown""#));

        let one_state = response_body(
            app.handle(request("GET", "/api/states/entity-light-1"))
                .into(),
        );
        assert!(one_state.contains(r#""friendly_name":"Kitchen Light""#));
        assert!(one_state.contains(r#""light.on_off""#));
        assert!(one_state.contains(r#""light.brightness""#));
        assert!(one_state.contains(r#""light.color_temperature""#));

        let services = response_body(app.handle(request("GET", "/api/services")).into());
        assert!(services.contains(r#""domain":"light""#));
        assert!(services.contains(r#""service":"turn_on""#));
        assert!(services.contains(r#""service":"set_brightness""#));
        assert!(services.contains(r#""domain":"scene""#));

        let events = response_body(app.handle(request("GET", "/api/events")).into());
        assert!(events.contains(r#""event":"call_service""#));
        assert!(events.contains(r#""event":"state_changed""#));
    }

    #[test]
    fn home_assistant_web_app_serves_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_web_app(fixture_state()));
        let (status, body) = http_get(port, "/api/states/entity-light-1");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""domain":"light""#));
        assert!(body.contains(r#""friendly_name":"Kitchen Light""#));
    }

    #[test]
    fn home_assistant_web_app_reports_missing_state_as_json_404() {
        let app = home_assistant_web_app(fixture_state());
        let response: web_core::WebResponse = app
            .handle(request("GET", "/api/states/missing.entity"))
            .into();

        assert_eq!(response.status, 404);
        assert_eq!(response_body(response), r#"{"error":"entity not found"}"#);
    }

    #[test]
    fn value_json_escapes_strings_and_projects_nested_values() {
        let value = Value::Object(vec![
            ("name".to_string(), Value::Text("Kitchen \"A\"".to_string())),
            (
                "levels".to_string(),
                Value::Array(vec![Value::Percentage(50)]),
            ),
        ]);

        assert_eq!(
            value_json(&value),
            r#"{"name":"Kitchen \"A\"","levels":[50]}"#
        );
    }
}
