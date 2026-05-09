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
    ProtocolIdentifier, RuntimeKind, Scene, SceneAction, SceneId, SceneScope, StateConfidence,
    StateDelta, StateSnapshot, StateSource, Value,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueBridgeResource {
    pub id: HueResourceId,
    pub owner_device_id: Option<HueResourceId>,
    pub bridge_id: Option<String>,
    pub time_zone: Option<String>,
}

impl HueBridgeResource {
    pub fn to_core(&self, address: Option<String>) -> Bridge {
        let bridge_identifier = self
            .bridge_id
            .as_deref()
            .unwrap_or_else(|| self.id.as_str());
        let mut bridge = Bridge::new(
            BridgeId::trusted(format!("hue.bridge.{bridge_identifier}")),
            IntegrationId::trusted(HUE_INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = address;
        bridge.health = Health::Online;
        bridge.identifiers.push(
            ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", bridge_identifier)
                .expect("Hue bridge resources have a non-empty identifier"),
        );
        bridge
            .metadata
            .push(Metadata::new("hue.resource_id", self.id.as_str()));
        if let Some(owner_device_id) = &self.owner_device_id {
            bridge.metadata.push(Metadata::new(
                "hue.owner_device_id",
                owner_device_id.as_str(),
            ));
        }
        if let Some(time_zone) = &self.time_zone {
            bridge
                .metadata
                .push(Metadata::new("hue.time_zone", time_zone));
        }
        bridge
    }
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

    pub fn command_set_brightness(&self, brightness: u8) -> HueCommand {
        HueCommand::SetLightBrightness {
            light_id: self.id.clone(),
            brightness,
        }
    }

    pub fn command_set_color_temperature(&self, mirek: u16) -> HueCommand {
        HueCommand::SetLightColorTemperature {
            light_id: self.id.clone(),
            mirek,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueGroupedLightResource {
    pub id: HueResourceId,
    pub owner: HueResourceRef,
    pub name: String,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
}

impl HueGroupedLightResource {
    pub fn command_set_on(&self, on: bool) -> HueCommand {
        HueCommand::SetGroupedLightOn {
            grouped_light_id: self.id.clone(),
            on,
        }
    }

    pub fn command_set_brightness(&self, brightness: u8) -> HueCommand {
        HueCommand::SetGroupedLightBrightness {
            grouped_light_id: self.id.clone(),
            brightness,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueRoomResource {
    pub id: HueResourceId,
    pub name: String,
    pub archetype: Option<String>,
    pub children: Vec<HueResourceRef>,
    pub services: Vec<HueResourceRef>,
}

impl HueRoomResource {
    pub fn grouped_light_service(&self) -> Option<&HueResourceRef> {
        self.services
            .iter()
            .find(|service| service.resource_type == HueResourceType::GroupedLight)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueZoneResource {
    pub id: HueResourceId,
    pub name: String,
    pub archetype: Option<String>,
    pub children: Vec<HueResourceRef>,
    pub services: Vec<HueResourceRef>,
}

impl HueZoneResource {
    pub fn grouped_light_service(&self) -> Option<&HueResourceRef> {
        self.services
            .iter()
            .find(|service| service.resource_type == HueResourceType::GroupedLight)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueSceneAction {
    pub target: HueResourceRef,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
    pub color_temperature_mirek: Option<u16>,
}

impl HueSceneAction {
    pub fn has_state(&self) -> bool {
        self.on.is_some() || self.brightness.is_some() || self.color_temperature_mirek.is_some()
    }

    pub fn desired_state(&self) -> Value {
        let mut fields = Vec::new();
        if let Some(on) = self.on {
            fields.push(("light.on_off".to_string(), Value::Bool(on)));
        }
        if let Some(brightness) = self.brightness {
            fields.push((
                "light.brightness".to_string(),
                Value::Percentage(brightness),
            ));
        }
        if let Some(mirek) = self.color_temperature_mirek {
            fields.push((
                "light.color_temperature".to_string(),
                Value::Integer(i64::from(mirek)),
            ));
        }
        Value::Object(fields)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueSceneResource {
    pub id: HueResourceId,
    pub group: HueResourceRef,
    pub name: String,
    pub actions: Vec<HueSceneAction>,
}

impl HueSceneResource {
    pub fn command_recall(&self) -> HueCommand {
        HueCommand::RecallScene {
            scene_id: self.id.clone(),
        }
    }

    pub fn to_core(&self, bridge_id: &BridgeId) -> Scene {
        Scene {
            scene_id: SceneId::trusted(format!("hue.scene.{}.{}", bridge_id, self.id)),
            scope: scene_scope_for_group(&self.group),
            native_ref: Some(
                HueResourceRef::new(HueResourceType::Scene, self.id.clone()).protocol_identifier(),
            ),
            actions: self
                .actions
                .iter()
                .filter(|action| action.has_state())
                .map(|action| SceneAction {
                    entity_id: hue_entity_id_for_resource_ref(bridge_id, &action.target),
                    desired_state: action.desired_state(),
                })
                .collect(),
            metadata: vec![
                Metadata::new("hue.resource_type", "scene"),
                Metadata::new("hue.resource_id", self.id.as_str()),
                Metadata::new("hue.name", &self.name),
                Metadata::new("hue.group_type", self.group.resource_type.as_hue_type()),
                Metadata::new("hue.group_id", self.group.id.as_str()),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueMotionResource {
    pub id: HueResourceId,
    pub owner_device_id: HueResourceId,
    pub name: String,
    pub motion: Option<bool>,
    pub motion_valid: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueButtonResource {
    pub id: HueResourceId,
    pub owner_device_id: HueResourceId,
    pub name: String,
    pub last_event: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueMotionStateUpdate {
    pub id: HueResourceId,
    pub owner_device_id: Option<HueResourceId>,
    pub name: Option<String>,
    pub motion: Option<bool>,
    pub motion_valid: Option<bool>,
}

impl HueMotionStateUpdate {
    pub fn from_motion_resource(motion: &HueMotionResource) -> Self {
        Self {
            id: motion.id.clone(),
            owner_device_id: Some(motion.owner_device_id.clone()),
            name: Some(motion.name.clone()),
            motion: motion.motion,
            motion_valid: motion.motion_valid,
        }
    }

    pub fn has_state(&self) -> bool {
        self.motion.is_some()
    }

    pub fn state_deltas(&self) -> Vec<StateDelta> {
        hue_motion_state_deltas(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueButtonStateUpdate {
    pub id: HueResourceId,
    pub owner_device_id: Option<HueResourceId>,
    pub name: Option<String>,
    pub last_event: Option<String>,
}

impl HueButtonStateUpdate {
    pub fn from_button_resource(button: &HueButtonResource) -> Self {
        Self {
            id: button.id.clone(),
            owner_device_id: Some(button.owner_device_id.clone()),
            name: Some(button.name.clone()),
            last_event: button.last_event.clone(),
        }
    }

    pub fn has_state(&self) -> bool {
        self.last_event.is_some()
    }

    pub fn state_deltas(&self) -> Vec<StateDelta> {
        hue_button_state_deltas(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueDeviceResource {
    pub id: HueResourceId,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub product_name: Option<String>,
    pub software_version: Option<String>,
    pub services: Vec<HueResourceRef>,
}

impl HueDeviceResource {
    pub fn to_core(&self, bridge_id: &BridgeId) -> Device {
        let manufacturer = self
            .manufacturer
            .clone()
            .unwrap_or_else(|| "Philips Hue".to_string());
        let model = self
            .model
            .clone()
            .or_else(|| self.product_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let mut device = hue_device_to_core(
            bridge_id,
            self.id.clone(),
            manufacturer,
            model,
            self.name.clone(),
        );
        device.firmware_version = self.software_version.clone();
        if let Some(product_name) = &self.product_name {
            device
                .metadata
                .push(Metadata::new("hue.product_name", product_name));
        }
        for service in &self.services {
            device.metadata.push(Metadata::new(
                "hue.service",
                format!("{}:{}", service.resource_type.as_hue_type(), service.id),
            ));
        }
        device
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

pub fn hue_motion_state_deltas(update: &HueMotionStateUpdate) -> Vec<StateDelta> {
    update
        .motion
        .map(|motion| StateDelta {
            capability_id: CapabilityId::trusted("sensor.occupancy"),
            value: Value::Bool(motion),
        })
        .into_iter()
        .collect()
}

pub fn hue_button_state_deltas(update: &HueButtonStateUpdate) -> Vec<StateDelta> {
    update
        .last_event
        .as_ref()
        .map(|last_event| StateDelta {
            capability_id: CapabilityId::trusted("input.button"),
            value: Value::Text(last_event.clone()),
        })
        .into_iter()
        .collect()
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
            ("light.on_off".to_string(), Value::Bool(on)),
            (
                "light.brightness".to_string(),
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

pub fn hue_motion_to_entity(
    bridge_id: &BridgeId,
    device_id: DeviceId,
    motion: HueMotionResource,
    received_at_ms: u64,
) -> Entity {
    let entity_id = EntityId::trusted(format!("hue.motion.{}.{}", bridge_id, motion.id));
    let state = motion.motion.map(|active| StateSnapshot {
        entity_id: entity_id.clone(),
        value: Value::Object(vec![
            ("sensor.occupancy".to_string(), Value::Bool(active)),
            (
                "hue.motion_valid".to_string(),
                motion.motion_valid.map(Value::Bool).unwrap_or(Value::Null),
            ),
        ]),
        source: StateSource::Poll,
        observed_at_ms: received_at_ms,
        received_at_ms,
        expires_at_ms: None,
        confidence: if motion.motion_valid == Some(false) {
            StateConfidence::Stale
        } else {
            StateConfidence::Confirmed
        },
    });

    Entity {
        entity_id,
        device_id,
        kind: EntityKind::Sensor,
        name: motion.name,
        capabilities: vec![Capability::sensor_occupancy()],
        state,
        metadata: vec![
            Metadata::new("hue.resource_type", "motion"),
            Metadata::new("hue.resource_id", motion.id.as_str()),
            Metadata::new("hue.owner_device_id", motion.owner_device_id.as_str()),
        ],
    }
}

pub fn hue_button_to_entity(
    bridge_id: &BridgeId,
    device_id: DeviceId,
    button: HueButtonResource,
    received_at_ms: u64,
) -> Entity {
    let entity_id = EntityId::trusted(format!("hue.button.{}.{}", bridge_id, button.id));
    let state = button.last_event.clone().map(|last_event| StateSnapshot {
        entity_id: entity_id.clone(),
        value: Value::Object(vec![("input.button".to_string(), Value::Text(last_event))]),
        source: StateSource::Poll,
        observed_at_ms: received_at_ms,
        received_at_ms,
        expires_at_ms: None,
        confidence: StateConfidence::Confirmed,
    });

    Entity {
        entity_id,
        device_id,
        kind: EntityKind::Input,
        name: button.name,
        capabilities: vec![Capability::input_button()],
        state,
        metadata: vec![
            Metadata::new("hue.resource_type", "button"),
            Metadata::new("hue.resource_id", button.id.as_str()),
            Metadata::new("hue.owner_device_id", button.owner_device_id.as_str()),
        ],
    }
}

fn scene_scope_for_group(group: &HueResourceRef) -> SceneScope {
    match group.resource_type {
        HueResourceType::Room => SceneScope::Room,
        HueResourceType::Zone => SceneScope::Zone,
        HueResourceType::Bridge => SceneScope::Bridge,
        _ => SceneScope::Custom,
    }
}

fn hue_entity_id_for_resource_ref(bridge_id: &BridgeId, resource: &HueResourceRef) -> EntityId {
    EntityId::trusted(format!(
        "hue.{}.{}.{}",
        resource.resource_type.as_hue_type(),
        bridge_id,
        resource.id
    ))
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
    fn hue_bridge_resource_projects_to_online_core_bridge() {
        let bridge = HueBridgeResource {
            id: HueResourceId::trusted("bridge-resource-1"),
            owner_device_id: Some(HueResourceId::trusted("device-bridge")),
            bridge_id: Some("001788fffeabcdef".to_string()),
            time_zone: Some("America/Los_Angeles".to_string()),
        }
        .to_core(Some("https://192.0.2.10".to_string()));

        assert_eq!(bridge.bridge_id.as_str(), "hue.bridge.001788fffeabcdef");
        assert_eq!(bridge.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(bridge.health, Health::Online);
        assert_eq!(bridge.address.as_deref(), Some("https://192.0.2.10"));
        assert_eq!(bridge.identifiers[0].value, "001788fffeabcdef");
        assert!(bridge
            .metadata
            .iter()
            .any(|metadata| metadata.value == "bridge-resource-1"));
        assert!(bridge
            .metadata
            .iter()
            .any(|metadata| metadata.value == "America/Los_Angeles"));
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
        let state = entity.state.unwrap();
        assert_eq!(state.confidence, StateConfidence::Confirmed);
        assert_eq!(
            state.value,
            Value::Object(vec![
                ("light.on_off".to_string(), Value::Bool(true)),
                ("light.brightness".to_string(), Value::Percentage(42)),
            ])
        );
    }

    #[test]
    fn hue_light_resource_builds_direct_light_commands() {
        let light = HueLightResource {
            id: HueResourceId::trusted("light-1"),
            owner_device_id: HueResourceId::trusted("device-1"),
            name: "Kitchen".to_string(),
            on: Some(true),
            brightness: Some(42),
            color_temperature_mirek: Some(366),
        };

        assert_eq!(
            light.command_set_on(false).to_request().body,
            Some(HueRequestBody::SetOn { on: false })
        );
        assert_eq!(
            light.command_set_brightness(55).to_request().body,
            Some(HueRequestBody::SetBrightness { brightness: 55 })
        );
        assert_eq!(
            light.command_set_color_temperature(370).to_request().body,
            Some(HueRequestBody::SetColorTemperature { mirek: 370 })
        );
    }

    #[test]
    fn hue_grouped_light_resource_builds_group_commands() {
        let grouped = HueGroupedLightResource {
            id: HueResourceId::trusted("grouped-light-1"),
            owner: HueResourceRef::new(HueResourceType::Room, HueResourceId::trusted("room-1")),
            name: "Kitchen".to_string(),
            on: Some(false),
            brightness: Some(20),
        };

        assert_eq!(
            grouped.command_set_on(true).to_request(),
            HueRequest {
                method: HueMethod::Put,
                path: "/clip/v2/resource/grouped_light/grouped-light-1".to_string(),
                body: Some(HueRequestBody::SetOn { on: true }),
            }
        );
        assert_eq!(
            grouped.command_set_brightness(55).to_request().body,
            Some(HueRequestBody::SetBrightness { brightness: 55 })
        );
        assert_eq!(grouped.owner.resource_type, HueResourceType::Room);
    }

    #[test]
    fn hue_room_and_zone_resources_expose_grouped_light_services() {
        let room = HueRoomResource {
            id: HueResourceId::trusted("room-1"),
            name: "Kitchen".to_string(),
            archetype: Some("kitchen".to_string()),
            children: vec![HueResourceRef::new(
                HueResourceType::Device,
                HueResourceId::trusted("device-1"),
            )],
            services: vec![HueResourceRef::new(
                HueResourceType::GroupedLight,
                HueResourceId::trusted("grouped-light-1"),
            )],
        };
        let zone = HueZoneResource {
            id: HueResourceId::trusted("zone-1"),
            name: "Downstairs".to_string(),
            archetype: None,
            children: vec![HueResourceRef::new(
                HueResourceType::Room,
                HueResourceId::trusted("room-1"),
            )],
            services: room.services.clone(),
        };

        assert_eq!(
            room.grouped_light_service().unwrap().id.as_str(),
            "grouped-light-1"
        );
        assert_eq!(
            zone.grouped_light_service().unwrap().resource_type,
            HueResourceType::GroupedLight
        );
    }

    #[test]
    fn hue_scene_resource_builds_recall_command_and_core_scene() {
        let bridge_id = BridgeId::trusted("hue.bridge.001788");
        let scene = HueSceneResource {
            id: HueResourceId::trusted("scene-1"),
            group: HueResourceRef::new(HueResourceType::Room, HueResourceId::trusted("room-1")),
            name: "Dinner".to_string(),
            actions: vec![HueSceneAction {
                target: HueResourceRef::new(
                    HueResourceType::Light,
                    HueResourceId::trusted("light-1"),
                ),
                on: Some(true),
                brightness: Some(66),
                color_temperature_mirek: Some(366),
            }],
        };

        assert_eq!(
            scene.command_recall().to_request().path,
            "/clip/v2/resource/scene/scene-1"
        );

        let core_scene = scene.to_core(&bridge_id);

        assert_eq!(
            core_scene.scene_id.as_str(),
            "hue.scene.hue.bridge.001788.scene-1"
        );
        assert_eq!(core_scene.scope, SceneScope::Room);
        assert_eq!(core_scene.native_ref.as_ref().unwrap().kind, "scene");
        assert_eq!(core_scene.actions.len(), 1);
        assert_eq!(
            core_scene.actions[0].entity_id.as_str(),
            "hue.light.hue.bridge.001788.light-1"
        );
        assert_eq!(
            core_scene.actions[0].desired_state,
            Value::Object(vec![
                ("light.on_off".to_string(), Value::Bool(true)),
                ("light.brightness".to_string(), Value::Percentage(66)),
                ("light.color_temperature".to_string(), Value::Integer(366)),
            ])
        );
    }

    #[test]
    fn hue_motion_resource_maps_to_occupancy_entity() {
        let bridge_id = BridgeId::trusted("hue.bridge.001788");
        let entity = hue_motion_to_entity(
            &bridge_id,
            DeviceId::trusted("hue.device.1"),
            HueMotionResource {
                id: HueResourceId::trusted("motion-1"),
                owner_device_id: HueResourceId::trusted("device-1"),
                name: "Hallway motion".to_string(),
                motion: Some(true),
                motion_valid: Some(true),
            },
            1_000,
        );

        assert_eq!(entity.kind, EntityKind::Sensor);
        assert_eq!(
            entity.capabilities[0].capability_id.as_str(),
            "sensor.occupancy"
        );
        assert_eq!(entity.metadata[1].value, "motion-1");
        assert_eq!(
            entity.state.unwrap().value,
            Value::Object(vec![
                ("sensor.occupancy".to_string(), Value::Bool(true)),
                ("hue.motion_valid".to_string(), Value::Bool(true)),
            ])
        );
    }

    #[test]
    fn hue_invalid_motion_marks_state_stale() {
        let bridge_id = BridgeId::trusted("hue.bridge.001788");
        let entity = hue_motion_to_entity(
            &bridge_id,
            DeviceId::trusted("hue.device.1"),
            HueMotionResource {
                id: HueResourceId::trusted("motion-1"),
                owner_device_id: HueResourceId::trusted("device-1"),
                name: "Hallway motion".to_string(),
                motion: Some(false),
                motion_valid: Some(false),
            },
            1_000,
        );

        assert_eq!(entity.state.unwrap().confidence, StateConfidence::Stale);
    }

    #[test]
    fn hue_button_resource_maps_to_input_entity() {
        let bridge_id = BridgeId::trusted("hue.bridge.001788");
        let entity = hue_button_to_entity(
            &bridge_id,
            DeviceId::trusted("hue.device.1"),
            HueButtonResource {
                id: HueResourceId::trusted("button-1"),
                owner_device_id: HueResourceId::trusted("device-1"),
                name: "Dimmer button".to_string(),
                last_event: Some("short_release".to_string()),
            },
            1_000,
        );

        assert_eq!(entity.kind, EntityKind::Input);
        assert_eq!(
            entity.capabilities[0].capability_id.as_str(),
            "input.button"
        );
        assert_eq!(
            entity.state.unwrap().value,
            Value::Object(vec![(
                "input.button".to_string(),
                Value::Text("short_release".to_string()),
            )])
        );
    }

    #[test]
    fn hue_device_resource_maps_to_normalized_device() {
        let bridge_id = BridgeId::trusted("hue.bridge.001788");
        let device = HueDeviceResource {
            id: HueResourceId::trusted("device-1"),
            name: "Kitchen lamp".to_string(),
            manufacturer: Some("Signify Netherlands B.V.".to_string()),
            model: Some("LCA001".to_string()),
            product_name: Some("Hue color lamp".to_string()),
            software_version: Some("1.116.3".to_string()),
            services: vec![HueResourceRef::new(
                HueResourceType::Light,
                HueResourceId::trusted("light-1"),
            )],
        }
        .to_core(&bridge_id);

        assert_eq!(device.bridge_id, bridge_id);
        assert_eq!(
            device.device_id.as_str(),
            "hue.device.hue.bridge.001788.device-1"
        );
        assert_eq!(device.model, "LCA001");
        assert_eq!(device.firmware_version.as_deref(), Some("1.116.3"));
        assert_eq!(device.identifiers[0].kind, "device");
        assert!(device
            .metadata
            .iter()
            .any(|metadata| metadata.value == "light:light-1"));
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
    fn hue_motion_state_update_maps_occupancy_delta() {
        let motion = HueMotionResource {
            id: HueResourceId::trusted("motion-1"),
            owner_device_id: HueResourceId::trusted("device-1"),
            name: "Hallway motion".to_string(),
            motion: Some(true),
            motion_valid: Some(true),
        };
        let update = HueMotionStateUpdate::from_motion_resource(&motion);

        assert!(update.has_state());
        assert_eq!(
            update.owner_device_id.as_ref().unwrap().as_str(),
            "device-1"
        );
        assert_eq!(
            update.state_deltas(),
            vec![StateDelta {
                capability_id: CapabilityId::trusted("sensor.occupancy"),
                value: Value::Bool(true),
            }]
        );
    }

    #[test]
    fn hue_button_state_update_maps_last_event_delta() {
        let button = HueButtonResource {
            id: HueResourceId::trusted("button-1"),
            owner_device_id: HueResourceId::trusted("device-1"),
            name: "Dimmer button".to_string(),
            last_event: Some("short_release".to_string()),
        };
        let update = HueButtonStateUpdate::from_button_resource(&button);

        assert!(update.has_state());
        assert_eq!(update.name.as_deref(), Some("Dimmer button"));
        assert_eq!(
            update.state_deltas(),
            vec![StateDelta {
                capability_id: CapabilityId::trusted("input.button"),
                value: Value::Text("short_release".to_string()),
            }]
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
