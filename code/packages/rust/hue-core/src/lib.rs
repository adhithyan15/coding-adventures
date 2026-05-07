//! Philips Hue CLIP v2 resource and mapping primitives.
//!
//! This crate deliberately has no network I/O. It owns Hue resource names,
//! endpoint paths, structured command intents, and projection into
//! `smart-home-core`. A later `hue-client` crate can attach HTTPS, TLS policy,
//! Vault-leased application keys, and event-stream transport.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, Device, DeviceId, Entity,
    EntityId, EntityKind, Health, IntegrationDescriptor, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, RuntimeKind, StateConfidence, StateDelta, StateSnapshot, StateSource,
    Value,
};
use std::fmt;

pub const HUE_INTEGRATION_ID: &str = "hue";
pub const CLIP_V2_RESOURCE_ROOT: &str = "/clip/v2/resource";
pub const CLIP_V2_EVENT_STREAM_PATH: &str = "/eventstream/clip/v2";
pub const HUE_APPLICATION_KEY_HEADER: &str = "hue-application-key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HueError {
    EmptyResourceId,
    UnsupportedCommandTarget { resource_type: HueResourceType },
    InvalidBrightness { value: u16 },
}

impl fmt::Display for HueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyResourceId => write!(f, "Hue resource id must not be empty"),
            Self::UnsupportedCommandTarget { resource_type } => {
                write!(f, "Hue command target {resource_type:?} is not supported")
            }
            Self::InvalidBrightness { value } => {
                write!(f, "Hue brightness {value} is outside 0..=100")
            }
        }
    }
}

impl std::error::Error for HueError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueResourceId(String);

impl HueResourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, HueError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(HueError::EmptyResourceId);
        }
        Ok(Self(value))
    }

    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HueResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HueResourceType {
    Bridge,
    Device,
    Light,
    GroupedLight,
    Room,
    Zone,
    Scene,
    Motion,
    Button,
    SmartScene,
    Unknown(String),
}

impl HueResourceType {
    pub fn from_hue_type(value: &str) -> Self {
        match value {
            "bridge" => Self::Bridge,
            "device" => Self::Device,
            "light" => Self::Light,
            "grouped_light" => Self::GroupedLight,
            "room" => Self::Room,
            "zone" => Self::Zone,
            "scene" => Self::Scene,
            "motion" | "motion_sensor" => Self::Motion,
            "button" => Self::Button,
            "smart_scene" => Self::SmartScene,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_hue_type(&self) -> &str {
        match self {
            Self::Bridge => "bridge",
            Self::Device => "device",
            Self::Light => "light",
            Self::GroupedLight => "grouped_light",
            Self::Room => "room",
            Self::Zone => "zone",
            Self::Scene => "scene",
            Self::Motion => "motion",
            Self::Button => "button",
            Self::SmartScene => "smart_scene",
            Self::Unknown(value) => value.as_str(),
        }
    }

    pub fn maps_to_entity_kind(&self) -> Option<EntityKind> {
        match self {
            Self::Light => Some(EntityKind::Light),
            Self::GroupedLight => Some(EntityKind::LightGroup),
            Self::Scene | Self::SmartScene => Some(EntityKind::Scene),
            Self::Motion => Some(EntityKind::Sensor),
            Self::Button => Some(EntityKind::Input),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueResourceRef {
    pub resource_type: HueResourceType,
    pub id: HueResourceId,
}

impl HueResourceRef {
    pub fn new(resource_type: HueResourceType, id: HueResourceId) -> Self {
        Self { resource_type, id }
    }

    pub fn collection_path(resource_type: &HueResourceType) -> String {
        format!("{CLIP_V2_RESOURCE_ROOT}/{}", resource_type.as_hue_type())
    }

    pub fn path(&self) -> String {
        format!(
            "{}/{}/{}",
            CLIP_V2_RESOURCE_ROOT,
            self.resource_type.as_hue_type(),
            self.id
        )
    }

    pub fn protocol_identifier(&self) -> ProtocolIdentifier {
        ProtocolIdentifier::new(
            ProtocolFamily::Hue,
            self.resource_type.as_hue_type(),
            self.id.as_str(),
        )
        .expect("Hue resource refs are constructed with non-empty resource ids")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HueRequestBody {
    RegisterApplication {
        app_name: String,
        instance_name: String,
    },
    SetOn {
        on: bool,
    },
    SetBrightness {
        brightness: u8,
    },
    SetColorTemperature {
        mirek: u16,
    },
    RecallScene,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueRequest {
    pub method: HueMethod,
    pub path: String,
    pub body: Option<HueRequestBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HueCommand {
    SetLightOn {
        light_id: HueResourceId,
        on: bool,
    },
    SetGroupedLightOn {
        grouped_light_id: HueResourceId,
        on: bool,
    },
    SetLightBrightness {
        light_id: HueResourceId,
        brightness: u8,
    },
    SetGroupedLightBrightness {
        grouped_light_id: HueResourceId,
        brightness: u8,
    },
    SetLightColorTemperature {
        light_id: HueResourceId,
        mirek: u16,
    },
    RecallScene {
        scene_id: HueResourceId,
    },
}

impl HueCommand {
    pub fn to_request(&self) -> HueRequest {
        match self {
            Self::SetLightOn { light_id, on } => {
                set_on_request(HueResourceType::Light, light_id, *on)
            }
            Self::SetGroupedLightOn {
                grouped_light_id,
                on,
            } => set_on_request(HueResourceType::GroupedLight, grouped_light_id, *on),
            Self::SetLightBrightness {
                light_id,
                brightness,
            } => set_brightness_request(HueResourceType::Light, light_id, *brightness),
            Self::SetGroupedLightBrightness {
                grouped_light_id,
                brightness,
            } => {
                set_brightness_request(HueResourceType::GroupedLight, grouped_light_id, *brightness)
            }
            Self::SetLightColorTemperature { light_id, mirek } => HueRequest {
                method: HueMethod::Put,
                path: HueResourceRef::new(HueResourceType::Light, light_id.clone()).path(),
                body: Some(HueRequestBody::SetColorTemperature { mirek: *mirek }),
            },
            Self::RecallScene { scene_id } => HueRequest {
                method: HueMethod::Put,
                path: HueResourceRef::new(HueResourceType::Scene, scene_id.clone()).path(),
                body: Some(HueRequestBody::RecallScene),
            },
        }
    }
}

fn set_on_request(resource_type: HueResourceType, id: &HueResourceId, on: bool) -> HueRequest {
    HueRequest {
        method: HueMethod::Put,
        path: HueResourceRef::new(resource_type, id.clone()).path(),
        body: Some(HueRequestBody::SetOn { on }),
    }
}

fn set_brightness_request(
    resource_type: HueResourceType,
    id: &HueResourceId,
    brightness: u8,
) -> HueRequest {
    HueRequest {
        method: HueMethod::Put,
        path: HueResourceRef::new(resource_type, id.clone()).path(),
        body: Some(HueRequestBody::SetBrightness { brightness }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredHueBridge {
    pub bridge_id: String,
    pub address: String,
    pub hardware_model: Option<String>,
    pub firmware_version: Option<String>,
}

pub fn hue_integration_descriptor() -> IntegrationDescriptor {
    IntegrationDescriptor {
        integration_id: IntegrationId::trusted(HUE_INTEGRATION_ID),
        display_name: "Philips Hue".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_kind: RuntimeKind::RustWorkerProcess,
        capabilities: vec![
            smart_home_core::CapabilityId::trusted("smart_home.read"),
            smart_home_core::CapabilityId::trusted("smart_home.command.light"),
            smart_home_core::CapabilityId::trusted("smart_home.pair"),
        ],
        discovery_roles: vec!["hue-bridge".to_string()],
        pairing_roles: vec!["hue-bridge".to_string()],
    }
}

pub fn discovered_bridge_to_core(discovered: DiscoveredHueBridge) -> Bridge {
    let mut bridge = Bridge::new(
        BridgeId::trusted(format!("hue.bridge.{}", discovered.bridge_id)),
        IntegrationId::trusted(HUE_INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(discovered.address);
    bridge.hardware_model = discovered.hardware_model;
    bridge.firmware_version = discovered.firmware_version;
    bridge.health = Health::Unpaired;
    bridge.identifiers.push(
        ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", discovered.bridge_id)
            .expect("discovered Hue bridge id is non-empty"),
    );
    bridge
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueLightResource {
    pub id: HueResourceId,
    pub owner_device_id: HueResourceId,
    pub name: String,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
    pub color_temperature_mirek: Option<u16>,
}

impl HueLightResource {
    pub fn command_set_on(&self, on: bool) -> HueCommand {
        HueCommand::SetLightOn {
            light_id: self.id.clone(),
            on,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueLightStateUpdate {
    pub id: HueResourceId,
    pub owner_device_id: Option<HueResourceId>,
    pub name: Option<String>,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
    pub color_temperature_mirek: Option<u16>,
}

impl HueLightStateUpdate {
    pub fn from_light_resource(light: &HueLightResource) -> Self {
        Self {
            id: light.id.clone(),
            owner_device_id: Some(light.owner_device_id.clone()),
            name: Some(light.name.clone()),
            on: light.on,
            brightness: light.brightness,
            color_temperature_mirek: light.color_temperature_mirek,
        }
    }

    pub fn has_state(&self) -> bool {
        self.on.is_some() || self.brightness.is_some() || self.color_temperature_mirek.is_some()
    }

    pub fn state_deltas(&self) -> Vec<StateDelta> {
        hue_light_state_deltas(self)
    }
}

pub fn hue_light_state_deltas(update: &HueLightStateUpdate) -> Vec<StateDelta> {
    let mut deltas = Vec::new();
    if let Some(on) = update.on {
        deltas.push(StateDelta {
            capability_id: CapabilityId::trusted("light.on_off"),
            value: Value::Bool(on),
        });
    }
    if let Some(brightness) = update.brightness {
        deltas.push(StateDelta {
            capability_id: CapabilityId::trusted("light.brightness"),
            value: Value::Percentage(brightness),
        });
    }
    if let Some(mirek) = update.color_temperature_mirek {
        deltas.push(StateDelta {
            capability_id: CapabilityId::trusted("light.color_temperature"),
            value: Value::Integer(i64::from(mirek)),
        });
    }
    deltas
}

pub fn hue_device_to_core(
    bridge_id: &BridgeId,
    hue_device_id: HueResourceId,
    manufacturer: impl Into<String>,
    model: impl Into<String>,
    name: impl Into<String>,
) -> Device {
    Device {
        device_id: DeviceId::trusted(format!("hue.device.{}.{}", bridge_id, hue_device_id)),
        bridge_id: bridge_id.clone(),
        manufacturer: manufacturer.into(),
        model: model.into(),
        name: name.into(),
        serial: None,
        firmware_version: None,
        room_id: None,
        entity_ids: Vec::new(),
        identifiers: vec![
            HueResourceRef::new(HueResourceType::Device, hue_device_id).protocol_identifier()
        ],
        health: Health::Online,
        metadata: Vec::new(),
    }
}

pub fn hue_light_to_entity(
    bridge_id: &BridgeId,
    device_id: DeviceId,
    light: HueLightResource,
    received_at_ms: u64,
) -> Entity {
    let entity_id = EntityId::trusted(format!("hue.light.{}.{}", bridge_id, light.id));
    let mut capabilities = vec![Capability::light_on_off(), Capability::light_brightness()];
    if light.color_temperature_mirek.is_some() {
        capabilities.push(Capability::light_color_temperature());
    }

    let state = light.on.map(|on| StateSnapshot {
        entity_id: entity_id.clone(),
        value: Value::Object(vec![
            ("on".to_string(), Value::Bool(on)),
            (
                "brightness".to_string(),
                light
                    .brightness
                    .map(Value::Percentage)
                    .unwrap_or(Value::Null),
            ),
        ]),
        source: StateSource::Poll,
        observed_at_ms: received_at_ms,
        received_at_ms,
        expires_at_ms: None,
        confidence: StateConfidence::Confirmed,
    });

    Entity {
        entity_id,
        device_id,
        kind: EntityKind::Light,
        name: light.name,
        capabilities,
        state,
        metadata: vec![
            Metadata::new("hue.resource_type", "light"),
            Metadata::new("hue.resource_id", light.id.as_str()),
        ],
    }
}

pub fn validate_brightness(value: u16) -> Result<u8, HueError> {
    if value > 100 {
        return Err(HueError::InvalidBrightness { value });
    }
    Ok(value as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_paths_match_clip_v2_shape() {
        let light = HueResourceRef::new(HueResourceType::Light, HueResourceId::trusted("abc"));

        assert_eq!(
            HueResourceRef::collection_path(&HueResourceType::Light),
            "/clip/v2/resource/light"
        );
        assert_eq!(light.path(), "/clip/v2/resource/light/abc");
        assert_eq!(CLIP_V2_EVENT_STREAM_PATH, "/eventstream/clip/v2");
    }

    #[test]
    fn commands_build_structured_requests() {
        let command = HueCommand::SetLightBrightness {
            light_id: HueResourceId::trusted("light-1"),
            brightness: validate_brightness(70).unwrap(),
        };

        assert_eq!(
            command.to_request(),
            HueRequest {
                method: HueMethod::Put,
                path: "/clip/v2/resource/light/light-1".to_string(),
                body: Some(HueRequestBody::SetBrightness { brightness: 70 }),
            }
        );
    }

    #[test]
    fn discovered_bridge_projects_to_unpaired_core_bridge() {
        let bridge = discovered_bridge_to_core(DiscoveredHueBridge {
            bridge_id: "001788fffeabcdef".to_string(),
            address: "https://192.0.2.10".to_string(),
            hardware_model: Some("BSB002".to_string()),
            firmware_version: None,
        });

        assert_eq!(bridge.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(bridge.health, Health::Unpaired);
        assert_eq!(bridge.transport, BridgeTransport::LanHttp);
        assert_eq!(bridge.identifiers[0].kind, "bridge");
    }

    #[test]
    fn hue_light_maps_to_normalized_light_entity() {
        let bridge_id = BridgeId::trusted("hue.bridge.001788");
        let entity = hue_light_to_entity(
            &bridge_id,
            DeviceId::trusted("hue.device.1"),
            HueLightResource {
                id: HueResourceId::trusted("light-1"),
                owner_device_id: HueResourceId::trusted("device-1"),
                name: "Kitchen".to_string(),
                on: Some(true),
                brightness: Some(42),
                color_temperature_mirek: Some(366),
            },
            1_000,
        );

        assert_eq!(entity.kind, EntityKind::Light);
        assert!(entity
            .capabilities
            .iter()
            .any(|capability| capability.capability_id.as_str() == "light.color_temperature"));
        assert_eq!(entity.metadata[1].value, "light-1");
        assert_eq!(entity.state.unwrap().confidence, StateConfidence::Confirmed);
    }

    #[test]
    fn hue_light_state_update_maps_known_fields_to_deltas() {
        let update = HueLightStateUpdate {
            id: HueResourceId::trusted("light-1"),
            owner_device_id: None,
            name: None,
            on: Some(false),
            brightness: Some(12),
            color_temperature_mirek: Some(366),
        };

        assert!(update.has_state());
        assert_eq!(
            update.state_deltas(),
            vec![
                StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(false),
                },
                StateDelta {
                    capability_id: CapabilityId::trusted("light.brightness"),
                    value: Value::Percentage(12),
                },
                StateDelta {
                    capability_id: CapabilityId::trusted("light.color_temperature"),
                    value: Value::Integer(366),
                },
            ]
        );
    }

    #[test]
    fn hue_integration_declares_agent_facing_capabilities() {
        let descriptor = hue_integration_descriptor();

        assert_eq!(descriptor.integration_id, IntegrationId::trusted("hue"));
        assert!(descriptor
            .capabilities
            .iter()
            .any(|capability| capability.as_str() == "smart_home.command.light"));
        assert_eq!(descriptor.discovery_roles, vec!["hue-bridge"]);
    }
}
