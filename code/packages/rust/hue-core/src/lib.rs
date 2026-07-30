//! Philips Hue CLIP v2 resource and mapping primitives.
//!
//! This crate deliberately has no network I/O. It owns Hue resource names,
//! endpoint paths, structured command intents, and projection into
//! `smart-home-core`. A later `hue-client` crate can attach HTTPS, TLS policy,
//! Vault-leased application keys, and event-stream transport.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroize;
use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, Device, DeviceId, Entity,
    EntityId, EntityKind, Health, IntegrationDescriptor, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, RuntimeKind, Scene, SceneAction, SceneId, SceneScope, StateConfidence,
    StateDelta, StateSnapshot, StateSource, Value, VaultRef,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource, DiscoveryWorkerFailure,
    DiscoveryWorkerId, DiscoveryWorkerKind, DiscoveryWorkerRun, MdnsAdvertisement, MdnsScanResult,
    MdnsWorkerScanReport, PairingRequirement,
};
use smart_home_local_http::{
    LocalHttpEndpoint, LocalHttpError, LocalHttpMethod, LocalHttpRequestPlan,
    LocalHttpRequestTemplate,
};
use std::fmt;

pub const HUE_INTEGRATION_ID: &str = "hue";
pub const HUE_READ_CAPABILITY_ID: &str = "smart_home.read";
pub const HUE_LIGHT_COMMAND_CAPABILITY_ID: &str = "smart_home.command.light";
pub const HUE_PAIRING_CAPABILITY_ID: &str = "smart_home.pair";
pub const HUE_BRIDGE_ROLE: &str = "hue-bridge";
pub const CLIP_V2_RESOURCE_ROOT: &str = "/clip/v2/resource";
pub const CLIP_V2_EVENT_STREAM_PATH: &str = "/eventstream/clip/v2";
pub const HUE_APPLICATION_KEY_HEADER: &str = "hue-application-key";
pub const HUE_APPLICATION_REGISTRATION_PATH: &str = "/api";
pub const HUE_MDNS_SERVICE_TYPE: &str = "_hue._tcp.local";
pub const HUE_DEFAULT_HTTPS_PORT: u16 = 443;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HueError {
    EmptyResourceId,
    UnsupportedCommandTarget {
        resource_type: HueResourceType,
    },
    InvalidBrightness {
        value: u16,
    },
    InvalidCommandValue {
        capability_id: CapabilityId,
        expected: &'static str,
    },
    MissingDiscoveryField {
        field: &'static str,
    },
    MissingPairingCredential {
        field: &'static str,
    },
    EmptyPairingCredential {
        field: &'static str,
    },
    InvalidPairingResponse {
        reason: String,
    },
    PairingRejected {
        error_type: Option<i64>,
        description: String,
    },
    PairingBridgeMismatch {
        plan_bridge_id: BridgeId,
        endpoint_bridge_id: BridgeId,
    },
    UnsupportedDiscoveryService {
        service_type: String,
    },
    Discovery(DiscoveryError),
    LocalHttp(LocalHttpError),
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
            Self::InvalidCommandValue {
                capability_id,
                expected,
            } => write!(
                f,
                "Hue command value for capability {} must be {expected}",
                capability_id.as_str()
            ),
            Self::MissingDiscoveryField { field } => {
                write!(f, "Hue discovery field {field} is required")
            }
            Self::MissingPairingCredential { field } => {
                write!(f, "Hue pairing credential field {field} is required")
            }
            Self::EmptyPairingCredential { field } => {
                write!(f, "Hue pairing credential field {field} must not be empty")
            }
            Self::InvalidPairingResponse { reason } => {
                write!(f, "Hue pairing response is invalid: {reason}")
            }
            Self::PairingRejected {
                error_type,
                description,
            } => match error_type {
                Some(error_type) => write!(
                    f,
                    "Hue pairing was rejected with error {error_type}: {description}"
                ),
                None => write!(f, "Hue pairing was rejected: {description}"),
            },
            Self::PairingBridgeMismatch {
                plan_bridge_id,
                endpoint_bridge_id,
            } => write!(
                f,
                "Hue pairing plan bridge {plan_bridge_id} does not match local HTTP endpoint bridge {endpoint_bridge_id}"
            ),
            Self::UnsupportedDiscoveryService { service_type } => {
                write!(
                    f,
                    "mDNS service `{service_type}` is not a Hue bridge service"
                )
            }
            Self::Discovery(error) => write!(f, "{error}"),
            Self::LocalHttp(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for HueError {}

impl From<DiscoveryError> for HueError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<LocalHttpError> for HueError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

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

impl HueMethod {
    pub fn as_local_http_method(self) -> LocalHttpMethod {
        match self {
            Self::Get => LocalHttpMethod::Get,
            Self::Post => LocalHttpMethod::Post,
            Self::Put => LocalHttpMethod::Put,
            Self::Delete => LocalHttpMethod::Delete,
        }
    }

    pub fn is_idempotent_by_default(self) -> bool {
        matches!(self, Self::Get | Self::Put | Self::Delete)
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueRequestBodyKind {
    RegisterApplication,
    SetOn,
    SetBrightness,
    SetColorTemperature,
    RecallScene,
}

impl HueRequestBodyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegisterApplication => "register_application",
            Self::SetOn => "set_on",
            Self::SetBrightness => "set_brightness",
            Self::SetColorTemperature => "set_color_temperature",
            Self::RecallScene => "recall_scene",
        }
    }
}

impl HueRequestBody {
    pub fn kind(&self) -> HueRequestBodyKind {
        match self {
            Self::RegisterApplication { .. } => HueRequestBodyKind::RegisterApplication,
            Self::SetOn { .. } => HueRequestBodyKind::SetOn,
            Self::SetBrightness { .. } => HueRequestBodyKind::SetBrightness,
            Self::SetColorTemperature { .. } => HueRequestBodyKind::SetColorTemperature,
            Self::RecallScene => HueRequestBodyKind::RecallScene,
        }
    }
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
    SetGroupedLightColorTemperature {
        grouped_light_id: HueResourceId,
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
            Self::SetLightColorTemperature { light_id, mirek } => {
                set_color_temperature_request(HueResourceType::Light, light_id, *mirek)
            }
            Self::SetGroupedLightColorTemperature {
                grouped_light_id,
                mirek,
            } => set_color_temperature_request(
                HueResourceType::GroupedLight,
                grouped_light_id,
                *mirek,
            ),
            Self::RecallScene { scene_id } => HueRequest {
                method: HueMethod::Put,
                path: HueResourceRef::new(HueResourceType::Scene, scene_id.clone()).path(),
                body: Some(HueRequestBody::RecallScene),
            },
        }
    }

    pub fn summary(&self) -> HueCommandSummary {
        HueCommandSummary::from_command(self)
    }

    pub fn from_state_delta(
        target: &HueResourceRef,
        delta: &StateDelta,
    ) -> Result<Option<Self>, HueError> {
        hue_command_from_state_delta(target, delta)
    }
}

pub fn hue_command_from_state_delta(
    target: &HueResourceRef,
    delta: &StateDelta,
) -> Result<Option<HueCommand>, HueError> {
    match delta.capability_id.as_str() {
        "light.on_off" => {
            let Value::Bool(on) = &delta.value else {
                return Err(HueError::InvalidCommandValue {
                    capability_id: delta.capability_id.clone(),
                    expected: "a boolean",
                });
            };
            match &target.resource_type {
                HueResourceType::Light => Ok(Some(HueCommand::SetLightOn {
                    light_id: target.id.clone(),
                    on: *on,
                })),
                HueResourceType::GroupedLight => Ok(Some(HueCommand::SetGroupedLightOn {
                    grouped_light_id: target.id.clone(),
                    on: *on,
                })),
                _ => Err(HueError::UnsupportedCommandTarget {
                    resource_type: target.resource_type.clone(),
                }),
            }
        }
        "light.brightness" => {
            let Value::Percentage(brightness) = &delta.value else {
                return Err(HueError::InvalidCommandValue {
                    capability_id: delta.capability_id.clone(),
                    expected: "a percentage",
                });
            };
            match &target.resource_type {
                HueResourceType::Light => Ok(Some(HueCommand::SetLightBrightness {
                    light_id: target.id.clone(),
                    brightness: *brightness,
                })),
                HueResourceType::GroupedLight => Ok(Some(HueCommand::SetGroupedLightBrightness {
                    grouped_light_id: target.id.clone(),
                    brightness: *brightness,
                })),
                _ => Err(HueError::UnsupportedCommandTarget {
                    resource_type: target.resource_type.clone(),
                }),
            }
        }
        "light.color_temperature" => {
            let Value::Integer(mirek) = &delta.value else {
                return Err(HueError::InvalidCommandValue {
                    capability_id: delta.capability_id.clone(),
                    expected: "an integer mirek value",
                });
            };
            let mirek = u16::try_from(*mirek).map_err(|_| HueError::InvalidCommandValue {
                capability_id: delta.capability_id.clone(),
                expected: "an integer in 0..=65535",
            })?;
            match &target.resource_type {
                HueResourceType::Light => Ok(Some(HueCommand::SetLightColorTemperature {
                    light_id: target.id.clone(),
                    mirek,
                })),
                HueResourceType::GroupedLight => {
                    Ok(Some(HueCommand::SetGroupedLightColorTemperature {
                        grouped_light_id: target.id.clone(),
                        mirek,
                    }))
                }
                _ => Err(HueError::UnsupportedCommandTarget {
                    resource_type: target.resource_type.clone(),
                }),
            }
        }
        _ => Ok(None),
    }
}

pub fn hue_commands_from_state_deltas<'a>(
    target: &HueResourceRef,
    deltas: impl IntoIterator<Item = &'a StateDelta>,
) -> Result<Vec<HueCommand>, HueError> {
    let mut commands = Vec::new();
    for delta in deltas {
        if let Some(command) = hue_command_from_state_delta(target, delta)? {
            commands.push(command);
        }
    }
    Ok(commands)
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueCommandPlan {
    pub target: HueResourceRef,
    pub commands: Vec<HueCommand>,
    pub ignored_capability_ids: Vec<CapabilityId>,
}

impl HueCommandPlan {
    pub fn empty(target: HueResourceRef) -> Self {
        Self {
            target,
            commands: Vec::new(),
            ignored_capability_ids: Vec::new(),
        }
    }

    pub fn from_state_deltas<'a>(
        target: &HueResourceRef,
        deltas: impl IntoIterator<Item = &'a StateDelta>,
    ) -> Result<Self, HueError> {
        hue_command_plan_from_state_deltas(target, deltas)
    }

    pub fn summary(&self) -> HueCommandPlanSummary {
        HueCommandPlanSummary::from_commands(&self.commands)
    }

    pub fn projection_summary(&self) -> HueCommandPlanProjectionSummary {
        HueCommandPlanProjectionSummary::from_plan(self)
    }

    pub fn requests(&self) -> Vec<HueRequest> {
        self.commands.iter().map(HueCommand::to_request).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn has_ignored_deltas(&self) -> bool {
        !self.ignored_capability_ids.is_empty()
    }

    pub fn ignored_delta_count(&self) -> usize {
        self.ignored_capability_ids.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueCommandPlanProjectionSummary {
    pub target_resource_type: HueResourceType,
    pub requested_delta_count: usize,
    pub generated_command_count: usize,
    pub ignored_delta_count: usize,
    pub command_summary: HueCommandPlanSummary,
}

impl HueCommandPlanProjectionSummary {
    pub fn from_plan(plan: &HueCommandPlan) -> Self {
        let command_summary = plan.summary();
        Self {
            target_resource_type: plan.target.resource_type.clone(),
            requested_delta_count: command_summary.total_commands + plan.ignored_delta_count(),
            generated_command_count: command_summary.total_commands,
            ignored_delta_count: plan.ignored_delta_count(),
            command_summary,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.requested_delta_count == 0
    }

    pub fn has_generated_commands(&self) -> bool {
        self.generated_command_count > 0
    }

    pub fn has_ignored_deltas(&self) -> bool {
        self.ignored_delta_count > 0
    }

    pub fn projected_all_requested_deltas(&self) -> bool {
        self.requested_delta_count > 0 && self.ignored_delta_count == 0
    }

    pub fn has_partial_projection(&self) -> bool {
        self.generated_command_count > 0 && self.ignored_delta_count > 0
    }

    pub fn target_is_light_surface(&self) -> bool {
        matches!(
            self.target_resource_type,
            HueResourceType::Light | HueResourceType::GroupedLight
        )
    }

    pub fn target_is_scene_surface(&self) -> bool {
        self.target_resource_type == HueResourceType::Scene
    }
}

pub fn hue_command_plan_from_state_deltas<'a>(
    target: &HueResourceRef,
    deltas: impl IntoIterator<Item = &'a StateDelta>,
) -> Result<HueCommandPlan, HueError> {
    let mut plan = HueCommandPlan::empty(target.clone());
    for delta in deltas {
        if let Some(command) = hue_command_from_state_delta(target, delta)? {
            plan.commands.push(command);
        } else {
            plan.ignored_capability_ids
                .push(delta.capability_id.clone());
        }
    }
    Ok(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueCommandTarget {
    Light,
    GroupedLight,
    Scene,
}

impl HueCommandTarget {
    pub fn resource_type(self) -> HueResourceType {
        match self {
            Self::Light => HueResourceType::Light,
            Self::GroupedLight => HueResourceType::GroupedLight,
            Self::Scene => HueResourceType::Scene,
        }
    }

    pub fn is_light_surface(self) -> bool {
        matches!(self, Self::Light | Self::GroupedLight)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueCommandSummary {
    pub target: HueCommandTarget,
    pub method: HueMethod,
    pub body_kind: HueRequestBodyKind,
}

impl HueCommandSummary {
    pub fn from_command(command: &HueCommand) -> Self {
        match command {
            HueCommand::SetLightOn { .. } => Self {
                target: HueCommandTarget::Light,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetOn,
            },
            HueCommand::SetGroupedLightOn { .. } => Self {
                target: HueCommandTarget::GroupedLight,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetOn,
            },
            HueCommand::SetLightBrightness { .. } => Self {
                target: HueCommandTarget::Light,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetBrightness,
            },
            HueCommand::SetGroupedLightBrightness { .. } => Self {
                target: HueCommandTarget::GroupedLight,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetBrightness,
            },
            HueCommand::SetLightColorTemperature { .. } => Self {
                target: HueCommandTarget::Light,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetColorTemperature,
            },
            HueCommand::SetGroupedLightColorTemperature { .. } => Self {
                target: HueCommandTarget::GroupedLight,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetColorTemperature,
            },
            HueCommand::RecallScene { .. } => Self {
                target: HueCommandTarget::Scene,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::RecallScene,
            },
        }
    }

    pub fn writes_light_state(&self) -> bool {
        self.target.is_light_surface()
    }

    pub fn recalls_scene(&self) -> bool {
        self.body_kind == HueRequestBodyKind::RecallScene
    }

    pub fn targets_direct_light(&self) -> bool {
        self.target == HueCommandTarget::Light
    }

    pub fn targets_grouped_light(&self) -> bool {
        self.target == HueCommandTarget::GroupedLight
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HueCommandPlanSummary {
    pub total_commands: usize,
    pub light_commands: usize,
    pub grouped_light_commands: usize,
    pub scene_commands: usize,
    pub on_off_commands: usize,
    pub brightness_commands: usize,
    pub color_temperature_commands: usize,
    pub scene_recall_commands: usize,
}

impl HueCommandPlanSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_commands<'a>(commands: impl IntoIterator<Item = &'a HueCommand>) -> Self {
        let mut summary = Self::empty();
        for command in commands {
            summary.record_summary(&command.summary());
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &HueCommandSummary) {
        self.total_commands += 1;
        match summary.target {
            HueCommandTarget::Light => self.light_commands += 1,
            HueCommandTarget::GroupedLight => self.grouped_light_commands += 1,
            HueCommandTarget::Scene => self.scene_commands += 1,
        }
        match summary.body_kind {
            HueRequestBodyKind::SetOn => self.on_off_commands += 1,
            HueRequestBodyKind::SetBrightness => self.brightness_commands += 1,
            HueRequestBodyKind::SetColorTemperature => self.color_temperature_commands += 1,
            HueRequestBodyKind::RecallScene => self.scene_recall_commands += 1,
            HueRequestBodyKind::RegisterApplication => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_commands == 0
    }

    pub fn has_lighting_writes(&self) -> bool {
        self.light_commands > 0 || self.grouped_light_commands > 0
    }

    pub fn lighting_write_count(&self) -> usize {
        self.light_commands + self.grouped_light_commands
    }

    pub fn target_surface_count(&self) -> usize {
        usize::from(self.light_commands > 0)
            + usize::from(self.grouped_light_commands > 0)
            + usize::from(self.scene_commands > 0)
    }

    pub fn light_capability_write_count(&self) -> usize {
        self.on_off_commands + self.brightness_commands + self.color_temperature_commands
    }

    pub fn light_capability_kind_count(&self) -> usize {
        usize::from(self.on_off_commands > 0)
            + usize::from(self.brightness_commands > 0)
            + usize::from(self.color_temperature_commands > 0)
    }

    pub fn has_direct_light_commands(&self) -> bool {
        self.light_commands > 0
    }

    pub fn has_group_commands(&self) -> bool {
        self.grouped_light_commands > 0
    }

    pub fn mixes_direct_and_grouped_light_writes(&self) -> bool {
        self.light_commands > 0 && self.grouped_light_commands > 0
    }

    pub fn has_color_temperature_writes(&self) -> bool {
        self.color_temperature_commands > 0
    }

    pub fn writes_multiple_light_capability_kinds(&self) -> bool {
        self.light_capability_kind_count() > 1
    }

    pub fn has_scene_recalls(&self) -> bool {
        self.scene_recall_commands > 0
    }

    pub fn has_only_light_surface_writes(&self) -> bool {
        self.has_lighting_writes() && self.scene_commands == 0
    }

    pub fn has_only_scene_recalls(&self) -> bool {
        self.scene_commands > 0 && self.scene_commands == self.total_commands
    }

    pub fn touches_multiple_surfaces(&self) -> bool {
        self.target_surface_count() > 1
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

fn set_color_temperature_request(
    resource_type: HueResourceType,
    id: &HueResourceId,
    mirek: u16,
) -> HueRequest {
    HueRequest {
        method: HueMethod::Put,
        path: HueResourceRef::new(resource_type, id.clone()).path(),
        body: Some(HueRequestBody::SetColorTemperature { mirek }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredHueBridge {
    pub bridge_id: String,
    pub address: String,
    pub hardware_model: Option<String>,
    pub firmware_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueCloudDiscoveryBridge {
    pub bridge_id: String,
    pub internal_ip_address: String,
    pub port: Option<u16>,
    pub hardware_model: Option<String>,
    pub firmware_version: Option<String>,
    pub discovered_at_ms: u64,
}

impl HueCloudDiscoveryBridge {
    pub fn new(
        bridge_id: impl Into<String>,
        internal_ip_address: impl Into<String>,
        discovered_at_ms: u64,
    ) -> Result<Self, HueError> {
        Ok(Self {
            bridge_id: non_empty_discovery_field("bridge_id", bridge_id)?,
            internal_ip_address: non_empty_discovery_field(
                "internal_ip_address",
                internal_ip_address,
            )?,
            port: None,
            hardware_model: None,
            firmware_version: None,
            discovered_at_ms,
        })
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_hardware_model(mut self, hardware_model: impl Into<String>) -> Self {
        self.hardware_model = Some(hardware_model.into());
        self
    }

    pub fn with_firmware_version(mut self, firmware_version: impl Into<String>) -> Self {
        self.firmware_version = Some(firmware_version.into());
        self
    }

    pub fn address(&self) -> String {
        hue_https_endpoint(
            &self.internal_ip_address,
            self.port.unwrap_or(HUE_DEFAULT_HTTPS_PORT),
        )
    }

    pub fn into_record(self) -> Result<DiscoveryRecord, HueError> {
        let address = self.address();
        hue_discovery_record(
            self.bridge_id,
            DiscoverySource::CloudFallback,
            address,
            DiscoveryConfidence::Candidate,
            self.hardware_model,
            self.firmware_version,
            self.discovered_at_ms,
            vec![Metadata::new("hue.discovery.source", "cloud_fallback")],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueDiscoveryBatch {
    pub generated_at_ms: u64,
    pub records: Vec<DiscoveryRecord>,
}

impl HueDiscoveryBatch {
    pub fn new(generated_at_ms: u64) -> Self {
        Self {
            generated_at_ms,
            records: Vec::new(),
        }
    }

    pub fn from_mdns_advertisements<'a>(
        advertisements: impl IntoIterator<Item = &'a MdnsAdvertisement>,
        generated_at_ms: u64,
    ) -> Result<Self, HueError> {
        let mut batch = Self::new(generated_at_ms);
        for advertisement in advertisements {
            batch
                .records
                .push(hue_discovery_record_from_mdns(advertisement)?);
        }
        Ok(batch)
    }

    pub fn from_cloud_bridges(
        bridges: impl IntoIterator<Item = HueCloudDiscoveryBridge>,
        generated_at_ms: u64,
    ) -> Result<Self, HueError> {
        let mut batch = Self::new(generated_at_ms);
        for bridge in bridges {
            batch.records.push(bridge.into_record()?);
        }
        Ok(batch)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn bridge_candidates(&self) -> Vec<Bridge> {
        self.records
            .iter()
            .map(DiscoveryRecord::to_bridge_candidate)
            .collect()
    }
}

pub fn hue_discovery_worker_run_from_observations<'a>(
    worker_id: impl Into<String>,
    mdns_advertisements: impl IntoIterator<Item = &'a MdnsAdvertisement>,
    cloud_bridges: impl IntoIterator<Item = HueCloudDiscoveryBridge>,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<DiscoveryWorkerRun, HueError> {
    let mdns_advertisements = mdns_advertisements.into_iter().collect::<Vec<_>>();
    let cloud_bridges = cloud_bridges.into_iter().collect::<Vec<_>>();
    let mut run = DiscoveryWorkerRun::new(
        DiscoveryWorkerId::new(worker_id)?,
        IntegrationId::trusted(HUE_INTEGRATION_ID),
        hue_discovery_worker_kind(!mdns_advertisements.is_empty(), !cloud_bridges.is_empty()),
        started_at_ms,
        completed_at_ms,
    )
    .with_metadata("hue.discovery.worker", "true")
    .with_metadata("hue.discovery.worker_version", env!("CARGO_PKG_VERSION"));

    for advertisement in mdns_advertisements {
        match hue_discovery_record_from_mdns(advertisement) {
            Ok(record) => run.push_record(record)?,
            Err(error) => run.push_failure(
                DiscoveryWorkerFailure::new(DiscoverySource::Mdns, error.to_string())?
                    .with_metadata("hue.discovery.service_type", &advertisement.service_type)
                    .with_metadata("hue.discovery.instance_name", &advertisement.instance_name)
                    .with_metadata("hue.discovery.host_name", &advertisement.host_name),
            ),
        }
    }

    for bridge in cloud_bridges {
        let bridge_id = bridge.bridge_id.clone();
        let internal_ip_address = bridge.internal_ip_address.clone();
        match bridge.into_record() {
            Ok(record) => run.push_record(record)?,
            Err(error) => run.push_failure(
                DiscoveryWorkerFailure::new(DiscoverySource::CloudFallback, error.to_string())?
                    .with_metadata("hue.discovery.bridge_id", bridge_id)
                    .with_metadata("hue.discovery.internal_ip_address", internal_ip_address),
            ),
        }
    }

    Ok(run)
}

pub fn hue_discovery_worker_run_from_mdns_scan(
    worker_id: impl Into<String>,
    scan: &MdnsScanResult,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<DiscoveryWorkerRun, HueError> {
    let mut run = DiscoveryWorkerRun::new(
        DiscoveryWorkerId::new(worker_id)?,
        IntegrationId::trusted(HUE_INTEGRATION_ID),
        DiscoveryWorkerKind::MdnsScan,
        started_at_ms,
        completed_at_ms,
    )
    .with_metadata("hue.discovery.worker", "true")
    .with_metadata("hue.discovery.worker_version", env!("CARGO_PKG_VERSION"))
    .with_metadata("hue.discovery.scan_service_type", &scan.service_type)
    .with_metadata(
        "hue.discovery.scan_datagram_count",
        scan.datagram_count.to_string(),
    )
    .with_metadata(
        "hue.discovery.scan_advertisement_count",
        scan.advertisements.len().to_string(),
    )
    .with_metadata(
        "hue.discovery.scan_failure_count",
        scan.failures.len().to_string(),
    );

    for advertisement in &scan.advertisements {
        match hue_discovery_record_from_mdns(advertisement) {
            Ok(record) => run.push_record(record)?,
            Err(error) => run.push_failure(
                DiscoveryWorkerFailure::new(DiscoverySource::Mdns, error.to_string())?
                    .with_metadata("hue.discovery.service_type", &advertisement.service_type)
                    .with_metadata("hue.discovery.instance_name", &advertisement.instance_name)
                    .with_metadata("hue.discovery.host_name", &advertisement.host_name),
            ),
        }
    }

    for failure in &scan.failures {
        let mut worker_failure =
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, failure.message.clone())?
                .with_metadata("hue.discovery.scan_failure", "true");
        if let Some(source) = &failure.source {
            worker_failure = worker_failure.with_metadata("hue.discovery.scan_source", source);
        }
        run.push_failure(worker_failure);
    }

    Ok(run)
}

pub fn hue_discovery_worker_run_from_mdns_scan_report(
    report: &MdnsWorkerScanReport,
) -> Result<DiscoveryWorkerRun, HueError> {
    if report.integration_id != IntegrationId::trusted(HUE_INTEGRATION_ID) {
        return Err(DiscoveryError::WorkerIntegrationMismatch {
            worker_integration_id: HUE_INTEGRATION_ID.to_string(),
            record_integration_id: report.integration_id.as_str().to_string(),
        }
        .into());
    }
    if report.service_type != HUE_MDNS_SERVICE_TYPE {
        return Err(HueError::UnsupportedDiscoveryService {
            service_type: report.service_type.clone(),
        });
    }

    let scan = report.aggregate_result();
    let mut run = hue_discovery_worker_run_from_mdns_scan(
        report.worker_id.as_str().to_string(),
        &scan,
        report.started_at_ms,
        report.completed_at_ms,
    )?
    .with_metadata("hue.discovery.scan_report", "true")
    .with_metadata(
        "hue.discovery.scan_request_success_count",
        report.completed_scan_count().to_string(),
    )
    .with_metadata(
        "hue.discovery.scan_request_failure_count",
        report.failed_scan_count().to_string(),
    )
    .with_metadata(
        "hue.discovery.scan_packet_failure_count",
        report.packet_failure_count().to_string(),
    );
    run.metadata.extend(report.metadata.iter().cloned());
    Ok(run)
}

fn hue_discovery_worker_kind(has_mdns: bool, has_cloud: bool) -> DiscoveryWorkerKind {
    match (has_mdns, has_cloud) {
        (true, true) => DiscoveryWorkerKind::Composite,
        (true, false) => DiscoveryWorkerKind::MdnsScan,
        (false, true) => DiscoveryWorkerKind::CloudFallback,
        (false, false) => DiscoveryWorkerKind::Composite,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueBridgePairingPlan {
    pub bridge: Bridge,
    pub registration_request: HueRequest,
    pub application_key_header: String,
    pub event_stream_path: String,
    pub requires_user_presence: bool,
}

impl HueBridgePairingPlan {
    pub fn bridge_id(&self) -> &BridgeId {
        &self.bridge.bridge_id
    }

    pub fn summary(&self) -> HueBridgePairingPlanSummary {
        HueBridgePairingPlanSummary::from_plan(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueBridgePairingPlanSummary {
    pub registration_method: HueMethod,
    pub has_bridge_address: bool,
    pub bridge_is_unpaired: bool,
    pub registration_path_is_api: bool,
    pub registration_body_is_application: bool,
    pub uses_hue_application_key_header: bool,
    pub uses_event_stream_path: bool,
    pub requires_user_presence: bool,
}

impl HueBridgePairingPlanSummary {
    pub fn from_plan(plan: &HueBridgePairingPlan) -> Self {
        Self {
            registration_method: plan.registration_request.method,
            has_bridge_address: plan.bridge.address.is_some(),
            bridge_is_unpaired: plan.bridge.health == Health::Unpaired,
            registration_path_is_api: plan.registration_request.path
                == HUE_APPLICATION_REGISTRATION_PATH,
            registration_body_is_application: matches!(
                &plan.registration_request.body,
                Some(HueRequestBody::RegisterApplication { .. })
            ),
            uses_hue_application_key_header: plan.application_key_header
                == HUE_APPLICATION_KEY_HEADER,
            uses_event_stream_path: plan.event_stream_path == CLIP_V2_EVENT_STREAM_PATH,
            requires_user_presence: plan.requires_user_presence,
        }
    }

    pub fn uses_physical_presence(self) -> bool {
        self.requires_user_presence
    }

    pub fn posts_registration_request(self) -> bool {
        self.registration_method == HueMethod::Post
            && self.registration_path_is_api
            && self.registration_body_is_application
    }

    pub fn ready_for_local_registration(self) -> bool {
        self.has_bridge_address
            && self.bridge_is_unpaired
            && self.posts_registration_request()
            && self.uses_hue_application_key_header
            && self.uses_event_stream_path
            && self.uses_physical_presence()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HueApplicationCredentials {
    pub application_key: String,
    pub client_key: Option<String>,
}

impl fmt::Debug for HueApplicationCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HueApplicationCredentials")
            .field("application_key", &"[REDACTED]")
            .field(
                "client_key",
                &self.client_key.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl HueApplicationCredentials {
    pub fn new(
        application_key: impl Into<String>,
        client_key: Option<String>,
    ) -> Result<Self, HueError> {
        let application_key = application_key.into();
        if application_key.trim().is_empty() {
            return Err(HueError::EmptyPairingCredential { field: "username" });
        }

        Ok(Self {
            application_key,
            client_key,
        })
    }

    pub fn has_client_key(&self) -> bool {
        self.client_key.is_some()
    }

    pub fn from_vault_secret_json(secret: &[u8]) -> Result<Self, HueError> {
        let value: serde_json::Value =
            serde_json::from_slice(secret).map_err(|error| HueError::InvalidPairingResponse {
                reason: error.to_string(),
            })?;
        let application_key = value
            .get("application_key")
            .and_then(serde_json::Value::as_str)
            .ok_or(HueError::MissingPairingCredential {
                field: "application_key",
            })?;
        let client_key = value
            .get("client_key")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        Self::new(application_key, client_key)
    }

    pub fn vault_secret_json(&self) -> Vec<u8> {
        let mut object = serde_json::Map::new();
        object.insert(
            "application_key".to_string(),
            serde_json::Value::String(self.application_key.clone()),
        );
        if let Some(client_key) = &self.client_key {
            object.insert(
                "client_key".to_string(),
                serde_json::Value::String(client_key.clone()),
            );
        }
        serde_json::to_vec(&serde_json::Value::Object(object))
            .expect("Hue credential vault payload is valid JSON")
    }

    pub fn vault_handoff(
        &self,
        plan: &HueBridgePairingPlan,
        vault_ref: VaultRef,
        stored_at_ms: u64,
    ) -> HuePairingVaultHandoff {
        HuePairingVaultHandoff {
            bridge_id: plan.bridge_id().clone(),
            vault_ref,
            stored_at_ms,
            application_key_header: plan.application_key_header.clone(),
            event_stream_path: plan.event_stream_path.clone(),
            metadata: vec![
                Metadata::new("hue.pairing.phase", "credential_stored"),
                Metadata::new("hue.pairing.credential_kind", "application_key"),
                Metadata::new(
                    "hue.pairing.application_key_header",
                    plan.application_key_header.as_str(),
                ),
                Metadata::new(
                    "hue.pairing.client_key_present",
                    self.has_client_key().to_string(),
                ),
                Metadata::new(
                    "hue.pairing.event_stream_path",
                    plan.event_stream_path.as_str(),
                ),
                Metadata::new("hue.pairing.stored_at_ms", stored_at_ms.to_string()),
            ],
        }
    }
}

impl Drop for HueApplicationCredentials {
    fn drop(&mut self) {
        self.application_key.zeroize();
        if let Some(client_key) = &mut self.client_key {
            client_key.zeroize();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuePairingVaultHandoff {
    pub bridge_id: BridgeId,
    pub vault_ref: VaultRef,
    pub stored_at_ms: u64,
    pub application_key_header: String,
    pub event_stream_path: String,
    pub metadata: Vec<Metadata>,
}

impl HuePairingVaultHandoff {
    pub fn summary(&self) -> HuePairingVaultHandoffSummary {
        HuePairingVaultHandoffSummary::from_handoff(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePairingVaultHandoffSummary {
    pub metadata_count: usize,
    pub stored_at_ms: u64,
    pub has_vault_reference: bool,
    pub uses_hue_application_key_header: bool,
    pub uses_event_stream_path: bool,
    pub has_credential_stored_phase: bool,
    pub has_application_key_credential_kind: bool,
    pub reports_client_key_presence: bool,
}

impl HuePairingVaultHandoffSummary {
    pub fn from_handoff(handoff: &HuePairingVaultHandoff) -> Self {
        Self {
            metadata_count: handoff.metadata.len(),
            stored_at_ms: handoff.stored_at_ms,
            has_vault_reference: !handoff.vault_ref.as_str().trim().is_empty(),
            uses_hue_application_key_header: handoff.application_key_header
                == HUE_APPLICATION_KEY_HEADER,
            uses_event_stream_path: handoff.event_stream_path == CLIP_V2_EVENT_STREAM_PATH,
            has_credential_stored_phase: metadata_contains(
                &handoff.metadata,
                "hue.pairing.phase",
                "credential_stored",
            ),
            has_application_key_credential_kind: metadata_contains(
                &handoff.metadata,
                "hue.pairing.credential_kind",
                "application_key",
            ),
            reports_client_key_presence: metadata_has_key(
                &handoff.metadata,
                "hue.pairing.client_key_present",
            ),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.has_vault_reference
            && self.uses_hue_application_key_header
            && self.uses_event_stream_path
            && self.has_credential_stored_phase
            && self.has_application_key_credential_kind
            && self.reports_client_key_presence
    }

    pub fn has_metadata(self) -> bool {
        self.metadata_count > 0
    }

    pub fn was_stored(self) -> bool {
        self.stored_at_ms > 0
    }
}

pub fn hue_integration_descriptor() -> IntegrationDescriptor {
    IntegrationDescriptor {
        integration_id: IntegrationId::trusted(HUE_INTEGRATION_ID),
        display_name: "Philips Hue".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_kind: RuntimeKind::RustWorkerProcess,
        capabilities: vec![
            smart_home_core::CapabilityId::trusted(HUE_READ_CAPABILITY_ID),
            smart_home_core::CapabilityId::trusted(HUE_LIGHT_COMMAND_CAPABILITY_ID),
            smart_home_core::CapabilityId::trusted(HUE_PAIRING_CAPABILITY_ID),
        ],
        discovery_roles: vec![HUE_BRIDGE_ROLE.to_string()],
        pairing_roles: vec![HUE_BRIDGE_ROLE.to_string()],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueIntegrationDescriptorSummary {
    pub runtime_kind: RuntimeKind,
    pub capability_count: usize,
    pub discovery_role_count: usize,
    pub pairing_role_count: usize,
    pub integration_id_is_hue: bool,
    pub declares_read: bool,
    pub declares_light_command: bool,
    pub declares_pairing: bool,
    pub declares_bridge_discovery: bool,
    pub declares_bridge_pairing: bool,
}

impl HueIntegrationDescriptorSummary {
    pub fn from_descriptor(descriptor: &IntegrationDescriptor) -> Self {
        Self {
            runtime_kind: descriptor.runtime_kind,
            capability_count: descriptor.capabilities.len(),
            discovery_role_count: descriptor.discovery_roles.len(),
            pairing_role_count: descriptor.pairing_roles.len(),
            integration_id_is_hue: descriptor.integration_id.as_str() == HUE_INTEGRATION_ID,
            declares_read: descriptor_declares_capability(descriptor, HUE_READ_CAPABILITY_ID),
            declares_light_command: descriptor_declares_capability(
                descriptor,
                HUE_LIGHT_COMMAND_CAPABILITY_ID,
            ),
            declares_pairing: descriptor_declares_capability(descriptor, HUE_PAIRING_CAPABILITY_ID),
            declares_bridge_discovery: descriptor_declares_role(
                &descriptor.discovery_roles,
                HUE_BRIDGE_ROLE,
            ),
            declares_bridge_pairing: descriptor_declares_role(
                &descriptor.pairing_roles,
                HUE_BRIDGE_ROLE,
            ),
        }
    }

    pub fn runs_as_worker_process(&self) -> bool {
        self.runtime_kind == RuntimeKind::RustWorkerProcess
    }

    pub fn has_canonical_identity(&self) -> bool {
        self.integration_id_is_hue
    }

    pub fn has_agent_facing_capabilities(&self) -> bool {
        self.declares_read || self.declares_light_command || self.declares_pairing
    }

    pub fn has_bridge_roles(&self) -> bool {
        self.declares_bridge_discovery || self.declares_bridge_pairing
    }

    pub fn supports_local_pairing_flow(&self) -> bool {
        self.declares_pairing && self.declares_bridge_pairing
    }

    pub fn supports_light_command_flow(&self) -> bool {
        self.declares_read && self.declares_light_command && self.declares_bridge_discovery
    }
}

pub fn hue_integration_descriptor_summary() -> HueIntegrationDescriptorSummary {
    HueIntegrationDescriptorSummary::from_descriptor(&hue_integration_descriptor())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueIntegrationPackageSummary {
    pub descriptor_summary: HueIntegrationDescriptorSummary,
    pub pairing_plan_summary: HueBridgePairingPlanSummary,
    pub worker_process_ready: bool,
    pub command_flow_declared: bool,
    pub local_pairing_declared: bool,
    pub local_pairing_ready: bool,
    pub package_ready: bool,
    pub requires_physical_presence: bool,
}

impl HueIntegrationPackageSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_summaries(hue_integration_descriptor_summary(), plan.summary())
    }

    pub fn from_summaries(
        descriptor_summary: HueIntegrationDescriptorSummary,
        pairing_plan_summary: HueBridgePairingPlanSummary,
    ) -> Self {
        let worker_process_ready = descriptor_summary.runs_as_worker_process();
        let command_flow_declared = descriptor_summary.supports_light_command_flow();
        let local_pairing_declared = descriptor_summary.supports_local_pairing_flow();
        let local_pairing_ready =
            local_pairing_declared && pairing_plan_summary.ready_for_local_registration();
        let package_ready = worker_process_ready && command_flow_declared && local_pairing_ready;

        Self {
            descriptor_summary,
            pairing_plan_summary,
            worker_process_ready,
            command_flow_declared,
            local_pairing_declared,
            local_pairing_ready,
            package_ready,
            requires_physical_presence: pairing_plan_summary.requires_user_presence,
        }
    }

    pub fn has_agent_facing_capabilities(&self) -> bool {
        self.descriptor_summary.has_agent_facing_capabilities()
    }

    pub fn has_bridge_roles(&self) -> bool {
        self.descriptor_summary.has_bridge_roles()
    }

    pub fn uses_local_event_stream(&self) -> bool {
        self.pairing_plan_summary.uses_event_stream_path
    }
}

pub fn hue_integration_package_summary(
    plan: &HueBridgePairingPlan,
) -> HueIntegrationPackageSummary {
    HueIntegrationPackageSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseReadinessSummary {
    pub package_summary: HueIntegrationPackageSummary,
    pub required_check_count: usize,
    pub passed_check_count: usize,
    pub failed_check_count: usize,
    pub worker_process_ready: bool,
    pub command_flow_ready: bool,
    pub pairing_flow_ready: bool,
    pub event_stream_ready: bool,
    pub physical_presence_required: bool,
    pub release_ready: bool,
}

impl HuePackageReleaseReadinessSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_package_summary(hue_integration_package_summary(plan))
    }

    pub fn from_package_summary(package_summary: HueIntegrationPackageSummary) -> Self {
        let checks = [
            package_summary.worker_process_ready,
            package_summary.command_flow_declared,
            package_summary.local_pairing_ready,
            package_summary.uses_local_event_stream(),
            package_summary.requires_physical_presence,
        ];
        let passed_check_count = checks.iter().filter(|ready| **ready).count();
        let required_check_count = checks.len();
        let failed_check_count = required_check_count - passed_check_count;
        let release_ready = failed_check_count == 0 && package_summary.package_ready;

        Self {
            package_summary,
            required_check_count,
            passed_check_count,
            failed_check_count,
            worker_process_ready: package_summary.worker_process_ready,
            command_flow_ready: package_summary.command_flow_declared,
            pairing_flow_ready: package_summary.local_pairing_ready,
            event_stream_ready: package_summary.uses_local_event_stream(),
            physical_presence_required: package_summary.requires_physical_presence,
            release_ready,
        }
    }

    pub fn is_release_ready(self) -> bool {
        self.release_ready
    }

    pub fn has_failed_checks(self) -> bool {
        self.failed_check_count > 0
    }
}

pub fn hue_package_release_readiness_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseReadinessSummary {
    HuePackageReleaseReadinessSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageSpecSummary {
    pub release_readiness: HuePackageReleaseReadinessSummary,
    pub required_spec_check_count: usize,
    pub passed_spec_check_count: usize,
    pub missing_spec_check_count: usize,
    pub canonical_integration_id: bool,
    pub clip_v2_resource_root: bool,
    pub registration_endpoint_ready: bool,
    pub application_key_header_ready: bool,
    pub event_stream_path_ready: bool,
    pub read_model_declared: bool,
    pub command_model_declared: bool,
    pub pairing_model_declared: bool,
    pub spec_ready: bool,
}

impl HuePackageSpecSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_release_readiness(hue_package_release_readiness_summary(plan))
    }

    pub fn from_release_readiness(release_readiness: HuePackageReleaseReadinessSummary) -> Self {
        let descriptor = release_readiness.package_summary.descriptor_summary;
        let pairing = release_readiness.package_summary.pairing_plan_summary;
        let canonical_integration_id = descriptor.has_canonical_identity();
        let clip_v2_resource_root = CLIP_V2_RESOURCE_ROOT == "/clip/v2/resource";
        let registration_endpoint_ready = pairing.registration_path_is_api;
        let application_key_header_ready = pairing.uses_hue_application_key_header;
        let event_stream_path_ready = pairing.uses_event_stream_path;
        let read_model_declared = descriptor.declares_read;
        let command_model_declared = descriptor.declares_light_command;
        let pairing_model_declared = descriptor.declares_pairing;
        let release_ready = release_readiness.is_release_ready();
        let checks = [
            canonical_integration_id,
            clip_v2_resource_root,
            registration_endpoint_ready,
            application_key_header_ready,
            event_stream_path_ready,
            read_model_declared,
            command_model_declared,
            pairing_model_declared,
            release_ready,
        ];
        let passed_spec_check_count = checks.iter().filter(|ready| **ready).count();
        let required_spec_check_count = checks.len();
        let missing_spec_check_count = required_spec_check_count - passed_spec_check_count;
        let spec_ready = missing_spec_check_count == 0 && release_ready;

        Self {
            release_readiness,
            required_spec_check_count,
            passed_spec_check_count,
            missing_spec_check_count,
            canonical_integration_id,
            clip_v2_resource_root,
            registration_endpoint_ready,
            application_key_header_ready,
            event_stream_path_ready,
            read_model_declared,
            command_model_declared,
            pairing_model_declared,
            spec_ready,
        }
    }

    pub fn is_spec_ready(self) -> bool {
        self.spec_ready
    }

    pub fn has_missing_spec_checks(self) -> bool {
        self.missing_spec_check_count > 0
    }

    pub fn declares_runtime_model_surface(self) -> bool {
        self.read_model_declared && self.command_model_declared && self.pairing_model_declared
    }
}

pub fn hue_package_spec_summary(plan: &HueBridgePairingPlan) -> HuePackageSpecSummary {
    HuePackageSpecSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageSpecGapSummary {
    pub spec_summary: HuePackageSpecSummary,
    pub blocking_spec_check_count: usize,
    pub release_blocked: bool,
    pub identity_blocked: bool,
    pub clip_v2_root_blocked: bool,
    pub registration_endpoint_blocked: bool,
    pub application_key_header_blocked: bool,
    pub event_stream_path_blocked: bool,
    pub runtime_model_blocked: bool,
    pub spec_ready: bool,
}

impl HuePackageSpecGapSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_spec_summary(hue_package_spec_summary(plan))
    }

    pub fn from_spec_summary(spec_summary: HuePackageSpecSummary) -> Self {
        let release_blocked = !spec_summary.release_readiness.is_release_ready();
        let identity_blocked = !spec_summary.canonical_integration_id;
        let clip_v2_root_blocked = !spec_summary.clip_v2_resource_root;
        let registration_endpoint_blocked = !spec_summary.registration_endpoint_ready;
        let application_key_header_blocked = !spec_summary.application_key_header_ready;
        let event_stream_path_blocked = !spec_summary.event_stream_path_ready;
        let runtime_model_blocked = !spec_summary.declares_runtime_model_surface();
        let blockers = [
            release_blocked,
            identity_blocked,
            clip_v2_root_blocked,
            registration_endpoint_blocked,
            application_key_header_blocked,
            event_stream_path_blocked,
            runtime_model_blocked,
        ];
        let blocking_spec_check_count = blockers.iter().filter(|blocked| **blocked).count();
        let spec_ready = spec_summary.is_spec_ready() && blocking_spec_check_count == 0;

        Self {
            spec_summary,
            blocking_spec_check_count,
            release_blocked,
            identity_blocked,
            clip_v2_root_blocked,
            registration_endpoint_blocked,
            application_key_header_blocked,
            event_stream_path_blocked,
            runtime_model_blocked,
            spec_ready,
        }
    }

    pub fn is_clear(self) -> bool {
        self.spec_ready
    }

    pub fn has_blockers(self) -> bool {
        self.blocking_spec_check_count > 0
    }

    pub fn needs_release_review(self) -> bool {
        self.release_blocked
    }

    pub fn needs_transport_review(self) -> bool {
        self.clip_v2_root_blocked
            || self.registration_endpoint_blocked
            || self.application_key_header_blocked
            || self.event_stream_path_blocked
    }

    pub fn needs_runtime_model_review(self) -> bool {
        self.runtime_model_blocked
    }
}

pub fn hue_package_spec_gap_summary(plan: &HueBridgePairingPlan) -> HuePackageSpecGapSummary {
    HuePackageSpecGapSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueCatalogPackageReadinessSummary {
    pub spec_summary: HuePackageSpecSummary,
    pub required_catalog_check_count: usize,
    pub passed_catalog_check_count: usize,
    pub missing_catalog_check_count: usize,
    pub package_spec_ready: bool,
    pub release_ready: bool,
    pub catalog_identity_ready: bool,
    pub clip_v2_transport_ready: bool,
    pub runtime_model_ready: bool,
    pub pairing_handoff_ready: bool,
    pub catalog_ready: bool,
}

impl HueCatalogPackageReadinessSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_spec_summary(hue_package_spec_summary(plan))
    }

    pub fn from_spec_summary(spec_summary: HuePackageSpecSummary) -> Self {
        let descriptor = spec_summary
            .release_readiness
            .package_summary
            .descriptor_summary;
        let pairing = spec_summary
            .release_readiness
            .package_summary
            .pairing_plan_summary;
        let package_spec_ready = spec_summary.is_spec_ready();
        let release_ready = spec_summary.release_readiness.is_release_ready();
        let catalog_identity_ready = spec_summary.canonical_integration_id
            && descriptor.has_agent_facing_capabilities()
            && descriptor.has_bridge_roles();
        let clip_v2_transport_ready = spec_summary.clip_v2_resource_root
            && spec_summary.registration_endpoint_ready
            && spec_summary.application_key_header_ready
            && spec_summary.event_stream_path_ready;
        let runtime_model_ready = spec_summary.declares_runtime_model_surface();
        let pairing_handoff_ready = pairing.ready_for_local_registration()
            && spec_summary.release_readiness.physical_presence_required;
        let checks = [
            package_spec_ready,
            release_ready,
            catalog_identity_ready,
            clip_v2_transport_ready,
            runtime_model_ready,
            pairing_handoff_ready,
        ];
        let passed_catalog_check_count = checks.iter().filter(|ready| **ready).count();
        let required_catalog_check_count = checks.len();
        let missing_catalog_check_count = required_catalog_check_count - passed_catalog_check_count;
        let catalog_ready = missing_catalog_check_count == 0;

        Self {
            spec_summary,
            required_catalog_check_count,
            passed_catalog_check_count,
            missing_catalog_check_count,
            package_spec_ready,
            release_ready,
            catalog_identity_ready,
            clip_v2_transport_ready,
            runtime_model_ready,
            pairing_handoff_ready,
            catalog_ready,
        }
    }

    pub fn is_catalog_ready(self) -> bool {
        self.catalog_ready
    }

    pub fn has_missing_catalog_checks(self) -> bool {
        self.missing_catalog_check_count > 0
    }

    pub fn transport_or_runtime_blocked(self) -> bool {
        !self.clip_v2_transport_ready || !self.runtime_model_ready
    }
}

pub fn hue_catalog_package_readiness_summary(
    plan: &HueBridgePairingPlan,
) -> HueCatalogPackageReadinessSummary {
    HueCatalogPackageReadinessSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueCatalogPackageGapSummary {
    pub catalog_readiness: HueCatalogPackageReadinessSummary,
    pub blocking_check_count: usize,
    pub package_spec_blocked: bool,
    pub release_blocked: bool,
    pub identity_blocked: bool,
    pub transport_or_runtime_blocked: bool,
    pub pairing_handoff_blocked: bool,
    pub catalog_ready: bool,
}

impl HueCatalogPackageGapSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_catalog_readiness(hue_catalog_package_readiness_summary(plan))
    }

    pub fn from_catalog_readiness(catalog_readiness: HueCatalogPackageReadinessSummary) -> Self {
        let package_spec_blocked = !catalog_readiness.package_spec_ready;
        let release_blocked = !catalog_readiness.release_ready;
        let identity_blocked = !catalog_readiness.catalog_identity_ready;
        let transport_or_runtime_blocked = catalog_readiness.transport_or_runtime_blocked();
        let pairing_handoff_blocked = !catalog_readiness.pairing_handoff_ready;
        let blockers = [
            package_spec_blocked,
            release_blocked,
            identity_blocked,
            transport_or_runtime_blocked,
            pairing_handoff_blocked,
        ];
        let blocking_check_count = blockers.iter().filter(|blocked| **blocked).count();
        let catalog_ready = catalog_readiness.is_catalog_ready() && blocking_check_count == 0;

        Self {
            catalog_readiness,
            blocking_check_count,
            package_spec_blocked,
            release_blocked,
            identity_blocked,
            transport_or_runtime_blocked,
            pairing_handoff_blocked,
            catalog_ready,
        }
    }

    pub fn is_clear(self) -> bool {
        self.catalog_ready
    }

    pub fn has_blockers(self) -> bool {
        self.blocking_check_count > 0
    }

    pub fn needs_spec_review(self) -> bool {
        self.package_spec_blocked || self.identity_blocked
    }

    pub fn needs_runtime_handoff_review(self) -> bool {
        self.transport_or_runtime_blocked || self.pairing_handoff_blocked
    }
}

pub fn hue_catalog_package_gap_summary(plan: &HueBridgePairingPlan) -> HueCatalogPackageGapSummary {
    HueCatalogPackageGapSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HueCatalogSpecHandoffSummary {
    pub gap_summary: HueCatalogPackageGapSummary,
    pub required_handoff_check_count: usize,
    pub passed_handoff_check_count: usize,
    pub missing_handoff_check_count: usize,
    pub catalog_ready: bool,
    pub spec_review_clear: bool,
    pub release_review_clear: bool,
    pub runtime_handoff_clear: bool,
    pub handoff_accepted: bool,
}

impl HueCatalogSpecHandoffSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_gap_summary(hue_catalog_package_gap_summary(plan))
    }

    pub fn from_gap_summary(gap_summary: HueCatalogPackageGapSummary) -> Self {
        let catalog_ready = gap_summary.is_clear();
        let spec_review_clear = !gap_summary.needs_spec_review();
        let release_review_clear = !gap_summary.release_blocked;
        let runtime_handoff_clear = !gap_summary.needs_runtime_handoff_review();
        let checks = [
            catalog_ready,
            spec_review_clear,
            release_review_clear,
            runtime_handoff_clear,
        ];
        let passed_handoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_handoff_check_count = checks.len();
        let missing_handoff_check_count = required_handoff_check_count - passed_handoff_check_count;
        let handoff_accepted = missing_handoff_check_count == 0;

        Self {
            gap_summary,
            required_handoff_check_count,
            passed_handoff_check_count,
            missing_handoff_check_count,
            catalog_ready,
            spec_review_clear,
            release_review_clear,
            runtime_handoff_clear,
            handoff_accepted,
        }
    }

    pub fn is_handoff_accepted(self) -> bool {
        self.handoff_accepted
    }

    pub fn has_missing_handoff_checks(self) -> bool {
        self.missing_handoff_check_count > 0
    }

    pub fn needs_catalog_spec_review(self) -> bool {
        !self.spec_review_clear
    }

    pub fn needs_release_review(self) -> bool {
        !self.release_review_clear
    }

    pub fn needs_runtime_pairing_review(self) -> bool {
        !self.runtime_handoff_clear
    }
}

pub fn hue_catalog_spec_handoff_summary(
    plan: &HueBridgePairingPlan,
) -> HueCatalogSpecHandoffSummary {
    HueCatalogSpecHandoffSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackagePublishGateSummary {
    pub handoff_summary: HueCatalogSpecHandoffSummary,
    pub required_publish_check_count: usize,
    pub passed_publish_check_count: usize,
    pub blocked_publish_check_count: usize,
    pub handoff_accepted: bool,
    pub catalog_spec_review_clear: bool,
    pub release_review_clear: bool,
    pub runtime_pairing_review_clear: bool,
    pub publish_ready: bool,
}

impl HuePackagePublishGateSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_handoff_summary(hue_catalog_spec_handoff_summary(plan))
    }

    pub fn from_handoff_summary(handoff_summary: HueCatalogSpecHandoffSummary) -> Self {
        let handoff_accepted = handoff_summary.is_handoff_accepted();
        let catalog_spec_review_clear = !handoff_summary.needs_catalog_spec_review();
        let release_review_clear = !handoff_summary.needs_release_review();
        let runtime_pairing_review_clear = !handoff_summary.needs_runtime_pairing_review();
        let checks = [
            handoff_accepted,
            catalog_spec_review_clear,
            release_review_clear,
            runtime_pairing_review_clear,
        ];
        let passed_publish_check_count = checks.iter().filter(|ready| **ready).count();
        let required_publish_check_count = checks.len();
        let blocked_publish_check_count = required_publish_check_count - passed_publish_check_count;
        let publish_ready = blocked_publish_check_count == 0;

        Self {
            handoff_summary,
            required_publish_check_count,
            passed_publish_check_count,
            blocked_publish_check_count,
            handoff_accepted,
            catalog_spec_review_clear,
            release_review_clear,
            runtime_pairing_review_clear,
            publish_ready,
        }
    }

    pub fn is_publish_ready(self) -> bool {
        self.publish_ready
    }

    pub fn has_publish_blockers(self) -> bool {
        self.blocked_publish_check_count > 0
    }

    pub fn needs_catalog_spec_queue(self) -> bool {
        !self.catalog_spec_review_clear
    }

    pub fn needs_release_queue(self) -> bool {
        !self.release_review_clear
    }

    pub fn needs_runtime_pairing_queue(self) -> bool {
        !self.runtime_pairing_review_clear
    }
}

pub fn hue_package_publish_gate_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackagePublishGateSummary {
    HuePackagePublishGateSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageLifecycleSummary {
    pub publish_gate: HuePackagePublishGateSummary,
    pub required_lifecycle_stage_count: usize,
    pub passed_lifecycle_stage_count: usize,
    pub blocked_lifecycle_stage_count: usize,
    pub release_ready: bool,
    pub spec_ready: bool,
    pub catalog_ready: bool,
    pub handoff_accepted: bool,
    pub publish_ready: bool,
    pub lifecycle_complete: bool,
}

impl HuePackageLifecycleSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_publish_gate(hue_package_publish_gate_summary(plan))
    }

    pub fn from_publish_gate(publish_gate: HuePackagePublishGateSummary) -> Self {
        let catalog_readiness = publish_gate.handoff_summary.gap_summary.catalog_readiness;
        let spec_summary = catalog_readiness.spec_summary;
        let release_ready = spec_summary.release_readiness.is_release_ready();
        let spec_ready = spec_summary.is_spec_ready();
        let catalog_ready = catalog_readiness.is_catalog_ready();
        let handoff_accepted = publish_gate.handoff_accepted;
        let publish_ready = publish_gate.is_publish_ready();
        let stages = [
            release_ready,
            spec_ready,
            catalog_ready,
            handoff_accepted,
            publish_ready,
        ];
        let passed_lifecycle_stage_count = stages.iter().filter(|ready| **ready).count();
        let required_lifecycle_stage_count = stages.len();
        let blocked_lifecycle_stage_count =
            required_lifecycle_stage_count - passed_lifecycle_stage_count;
        let lifecycle_complete = blocked_lifecycle_stage_count == 0;

        Self {
            publish_gate,
            required_lifecycle_stage_count,
            passed_lifecycle_stage_count,
            blocked_lifecycle_stage_count,
            release_ready,
            spec_ready,
            catalog_ready,
            handoff_accepted,
            publish_ready,
            lifecycle_complete,
        }
    }

    pub fn is_lifecycle_complete(self) -> bool {
        self.lifecycle_complete
    }

    pub fn has_blocked_lifecycle_stages(self) -> bool {
        self.blocked_lifecycle_stage_count > 0
    }

    pub fn needs_release_stage(self) -> bool {
        !self.release_ready
    }

    pub fn needs_spec_stage(self) -> bool {
        !self.spec_ready
    }

    pub fn needs_catalog_stage(self) -> bool {
        !self.catalog_ready
    }

    pub fn needs_handoff_stage(self) -> bool {
        !self.handoff_accepted
    }

    pub fn needs_publish_stage(self) -> bool {
        !self.publish_ready
    }
}

pub fn hue_package_lifecycle_summary(plan: &HueBridgePairingPlan) -> HuePackageLifecycleSummary {
    HuePackageLifecycleSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReviewQueueSummary {
    pub lifecycle_summary: HuePackageLifecycleSummary,
    pub total_review_queue_count: usize,
    pub active_review_queue_count: usize,
    pub clear_review_queue_count: usize,
    pub release_queue_active: bool,
    pub spec_queue_active: bool,
    pub catalog_queue_active: bool,
    pub handoff_queue_active: bool,
    pub publish_queue_active: bool,
    pub package_acceptance_ready: bool,
}

impl HuePackageReviewQueueSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_lifecycle_summary(hue_package_lifecycle_summary(plan))
    }

    pub fn from_lifecycle_summary(lifecycle_summary: HuePackageLifecycleSummary) -> Self {
        let release_queue_active = lifecycle_summary.needs_release_stage();
        let spec_queue_active = lifecycle_summary.needs_spec_stage();
        let catalog_queue_active = lifecycle_summary.needs_catalog_stage();
        let handoff_queue_active = lifecycle_summary.needs_handoff_stage();
        let publish_queue_active = lifecycle_summary.needs_publish_stage();
        let queues = [
            release_queue_active,
            spec_queue_active,
            catalog_queue_active,
            handoff_queue_active,
            publish_queue_active,
        ];
        let active_review_queue_count = queues.iter().filter(|active| **active).count();
        let total_review_queue_count = queues.len();
        let clear_review_queue_count = total_review_queue_count - active_review_queue_count;
        let package_acceptance_ready =
            lifecycle_summary.is_lifecycle_complete() && active_review_queue_count == 0;

        Self {
            lifecycle_summary,
            total_review_queue_count,
            active_review_queue_count,
            clear_review_queue_count,
            release_queue_active,
            spec_queue_active,
            catalog_queue_active,
            handoff_queue_active,
            publish_queue_active,
            package_acceptance_ready,
        }
    }

    pub fn has_active_review_queues(self) -> bool {
        self.active_review_queue_count > 0
    }

    pub fn is_package_acceptance_ready(self) -> bool {
        self.package_acceptance_ready
    }

    pub fn needs_release_queue(self) -> bool {
        self.release_queue_active
    }

    pub fn needs_spec_queue(self) -> bool {
        self.spec_queue_active
    }

    pub fn needs_catalog_queue(self) -> bool {
        self.catalog_queue_active
    }

    pub fn needs_handoff_queue(self) -> bool {
        self.handoff_queue_active
    }

    pub fn needs_publish_queue(self) -> bool {
        self.publish_queue_active
    }
}

pub fn hue_package_review_queue_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReviewQueueSummary {
    HuePackageReviewQueueSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageAcceptanceSummary {
    pub review_queue_summary: HuePackageReviewQueueSummary,
    pub required_acceptance_check_count: usize,
    pub passed_acceptance_check_count: usize,
    pub failed_acceptance_check_count: usize,
    pub lifecycle_complete: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub package_accepted: bool,
}

impl HuePackageAcceptanceSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_review_queue_summary(hue_package_review_queue_summary(plan))
    }

    pub fn from_review_queue_summary(review_queue_summary: HuePackageReviewQueueSummary) -> Self {
        let lifecycle_complete = review_queue_summary
            .lifecycle_summary
            .is_lifecycle_complete();
        let review_queues_clear = !review_queue_summary.has_active_review_queues();
        let publish_gate_ready = review_queue_summary
            .lifecycle_summary
            .publish_gate
            .is_publish_ready();
        let package_acceptance_ready = review_queue_summary.is_package_acceptance_ready();
        let checks = [
            lifecycle_complete,
            review_queues_clear,
            publish_gate_ready,
            package_acceptance_ready,
        ];
        let passed_acceptance_check_count = checks.iter().filter(|ready| **ready).count();
        let required_acceptance_check_count = checks.len();
        let failed_acceptance_check_count =
            required_acceptance_check_count - passed_acceptance_check_count;
        let package_accepted = failed_acceptance_check_count == 0;

        Self {
            review_queue_summary,
            required_acceptance_check_count,
            passed_acceptance_check_count,
            failed_acceptance_check_count,
            lifecycle_complete,
            review_queues_clear,
            publish_gate_ready,
            package_accepted,
        }
    }

    pub fn is_package_accepted(self) -> bool {
        self.package_accepted
    }

    pub fn has_acceptance_failures(self) -> bool {
        self.failed_acceptance_check_count > 0
    }

    pub fn needs_lifecycle_completion(self) -> bool {
        !self.lifecycle_complete
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_acceptance_summary(plan: &HueBridgePairingPlan) -> HuePackageAcceptanceSummary {
    HuePackageAcceptanceSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseHandoffSummary {
    pub acceptance_summary: HuePackageAcceptanceSummary,
    pub required_handoff_check_count: usize,
    pub passed_handoff_check_count: usize,
    pub blocked_handoff_check_count: usize,
    pub package_accepted: bool,
    pub lifecycle_complete: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_handoff_ready: bool,
}

impl HuePackageReleaseHandoffSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_acceptance_summary(hue_package_acceptance_summary(plan))
    }

    pub fn from_acceptance_summary(acceptance_summary: HuePackageAcceptanceSummary) -> Self {
        let package_accepted = acceptance_summary.is_package_accepted();
        let lifecycle_complete = !acceptance_summary.needs_lifecycle_completion();
        let review_queues_clear = !acceptance_summary.needs_review_queue_clearance();
        let publish_gate_ready = !acceptance_summary.needs_publish_gate();
        let checks = [
            package_accepted,
            lifecycle_complete,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_handoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_handoff_check_count = checks.len();
        let blocked_handoff_check_count = required_handoff_check_count - passed_handoff_check_count;
        let release_handoff_ready = blocked_handoff_check_count == 0;

        Self {
            acceptance_summary,
            required_handoff_check_count,
            passed_handoff_check_count,
            blocked_handoff_check_count,
            package_accepted,
            lifecycle_complete,
            review_queues_clear,
            publish_gate_ready,
            release_handoff_ready,
        }
    }

    pub fn is_release_handoff_ready(self) -> bool {
        self.release_handoff_ready
    }

    pub fn has_blocked_handoff_checks(self) -> bool {
        self.blocked_handoff_check_count > 0
    }

    pub fn needs_package_acceptance(self) -> bool {
        !self.package_accepted
    }

    pub fn needs_lifecycle_completion(self) -> bool {
        !self.lifecycle_complete
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_handoff_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseHandoffSummary {
    HuePackageReleaseHandoffSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseQueueSummary {
    pub handoff_summary: HuePackageReleaseHandoffSummary,
    pub required_release_queue_check_count: usize,
    pub queued_release_check_count: usize,
    pub blocked_release_queue_check_count: usize,
    pub release_handoff_ready: bool,
    pub package_accepted: bool,
    pub lifecycle_complete: bool,
    pub publish_gate_ready: bool,
    pub release_queue_ready: bool,
}

impl HuePackageReleaseQueueSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_handoff_summary(hue_package_release_handoff_summary(plan))
    }

    pub fn from_handoff_summary(handoff_summary: HuePackageReleaseHandoffSummary) -> Self {
        let release_handoff_ready = handoff_summary.is_release_handoff_ready();
        let package_accepted = !handoff_summary.needs_package_acceptance();
        let lifecycle_complete = !handoff_summary.needs_lifecycle_completion();
        let publish_gate_ready = !handoff_summary.needs_publish_gate();
        let checks = [
            release_handoff_ready,
            package_accepted,
            lifecycle_complete,
            publish_gate_ready,
        ];
        let queued_release_check_count = checks.iter().filter(|ready| **ready).count();
        let required_release_queue_check_count = checks.len();
        let blocked_release_queue_check_count =
            required_release_queue_check_count - queued_release_check_count;
        let release_queue_ready = blocked_release_queue_check_count == 0;

        Self {
            handoff_summary,
            required_release_queue_check_count,
            queued_release_check_count,
            blocked_release_queue_check_count,
            release_handoff_ready,
            package_accepted,
            lifecycle_complete,
            publish_gate_ready,
            release_queue_ready,
        }
    }

    pub fn is_release_queue_ready(self) -> bool {
        self.release_queue_ready
    }

    pub fn has_blocked_release_queue_checks(self) -> bool {
        self.blocked_release_queue_check_count > 0
    }

    pub fn needs_release_handoff(self) -> bool {
        !self.release_handoff_ready
    }

    pub fn needs_package_acceptance(self) -> bool {
        !self.package_accepted
    }

    pub fn needs_lifecycle_completion(self) -> bool {
        !self.lifecycle_complete
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_queue_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseQueueSummary {
    HuePackageReleaseQueueSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseCoordinationSummary {
    pub release_queue_summary: HuePackageReleaseQueueSummary,
    pub required_coordination_check_count: usize,
    pub passed_coordination_check_count: usize,
    pub blocked_coordination_check_count: usize,
    pub release_queue_ready: bool,
    pub release_handoff_ready: bool,
    pub package_accepted: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_coordination_ready: bool,
}

impl HuePackageReleaseCoordinationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_release_queue_summary(hue_package_release_queue_summary(plan))
    }

    pub fn from_release_queue_summary(
        release_queue_summary: HuePackageReleaseQueueSummary,
    ) -> Self {
        let release_queue_ready = release_queue_summary.is_release_queue_ready();
        let release_handoff_ready = !release_queue_summary.needs_release_handoff();
        let package_accepted = !release_queue_summary.needs_package_acceptance();
        let review_queues_clear = !release_queue_summary
            .handoff_summary
            .needs_review_queue_clearance();
        let publish_gate_ready = !release_queue_summary.needs_publish_gate();
        let checks = [
            release_queue_ready,
            release_handoff_ready,
            package_accepted,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_coordination_check_count = checks.iter().filter(|ready| **ready).count();
        let required_coordination_check_count = checks.len();
        let blocked_coordination_check_count =
            required_coordination_check_count - passed_coordination_check_count;
        let release_coordination_ready = blocked_coordination_check_count == 0;

        Self {
            release_queue_summary,
            required_coordination_check_count,
            passed_coordination_check_count,
            blocked_coordination_check_count,
            release_queue_ready,
            release_handoff_ready,
            package_accepted,
            review_queues_clear,
            publish_gate_ready,
            release_coordination_ready,
        }
    }

    pub fn is_release_coordination_ready(self) -> bool {
        self.release_coordination_ready
    }

    pub fn has_blocked_coordination_checks(self) -> bool {
        self.blocked_coordination_check_count > 0
    }

    pub fn needs_release_queue(self) -> bool {
        !self.release_queue_ready
    }

    pub fn needs_release_handoff(self) -> bool {
        !self.release_handoff_ready
    }

    pub fn needs_package_acceptance(self) -> bool {
        !self.package_accepted
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_coordination_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseCoordinationSummary {
    HuePackageReleaseCoordinationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseDispatchSummary {
    pub coordination_summary: HuePackageReleaseCoordinationSummary,
    pub required_dispatch_check_count: usize,
    pub passed_dispatch_check_count: usize,
    pub blocked_dispatch_check_count: usize,
    pub coordination_ready: bool,
    pub release_queue_ready: bool,
    pub package_accepted: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_dispatch_ready: bool,
}

impl HuePackageReleaseDispatchSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_coordination_summary(hue_package_release_coordination_summary(plan))
    }

    pub fn from_coordination_summary(
        coordination_summary: HuePackageReleaseCoordinationSummary,
    ) -> Self {
        let coordination_ready = coordination_summary.is_release_coordination_ready();
        let release_queue_ready = !coordination_summary.needs_release_queue();
        let package_accepted = !coordination_summary.needs_package_acceptance();
        let review_queues_clear = !coordination_summary.needs_review_queue_clearance();
        let publish_gate_ready = !coordination_summary.needs_publish_gate();
        let checks = [
            coordination_ready,
            release_queue_ready,
            package_accepted,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_dispatch_check_count = checks.iter().filter(|ready| **ready).count();
        let required_dispatch_check_count = checks.len();
        let blocked_dispatch_check_count =
            required_dispatch_check_count - passed_dispatch_check_count;
        let release_dispatch_ready = blocked_dispatch_check_count == 0;

        Self {
            coordination_summary,
            required_dispatch_check_count,
            passed_dispatch_check_count,
            blocked_dispatch_check_count,
            coordination_ready,
            release_queue_ready,
            package_accepted,
            review_queues_clear,
            publish_gate_ready,
            release_dispatch_ready,
        }
    }

    pub fn is_release_dispatch_ready(self) -> bool {
        self.release_dispatch_ready
    }

    pub fn has_blocked_dispatch_checks(self) -> bool {
        self.blocked_dispatch_check_count > 0
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_release_queue(self) -> bool {
        !self.release_queue_ready
    }

    pub fn needs_package_acceptance(self) -> bool {
        !self.package_accepted
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_dispatch_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseDispatchSummary {
    HuePackageReleaseDispatchSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseOperatorSummary {
    pub dispatch_summary: HuePackageReleaseDispatchSummary,
    pub required_operator_check_count: usize,
    pub passed_operator_check_count: usize,
    pub blocked_operator_check_count: usize,
    pub dispatch_ready: bool,
    pub coordination_ready: bool,
    pub package_accepted: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_operator_ready: bool,
}

impl HuePackageReleaseOperatorSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_dispatch_summary(hue_package_release_dispatch_summary(plan))
    }

    pub fn from_dispatch_summary(dispatch_summary: HuePackageReleaseDispatchSummary) -> Self {
        let dispatch_ready = dispatch_summary.is_release_dispatch_ready();
        let coordination_ready = !dispatch_summary.needs_coordination();
        let package_accepted = !dispatch_summary.needs_package_acceptance();
        let review_queues_clear = !dispatch_summary.needs_review_queue_clearance();
        let publish_gate_ready = !dispatch_summary.needs_publish_gate();
        let checks = [
            dispatch_ready,
            coordination_ready,
            package_accepted,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_operator_check_count = checks.iter().filter(|ready| **ready).count();
        let required_operator_check_count = checks.len();
        let blocked_operator_check_count =
            required_operator_check_count - passed_operator_check_count;
        let release_operator_ready = blocked_operator_check_count == 0;

        Self {
            dispatch_summary,
            required_operator_check_count,
            passed_operator_check_count,
            blocked_operator_check_count,
            dispatch_ready,
            coordination_ready,
            package_accepted,
            review_queues_clear,
            publish_gate_ready,
            release_operator_ready,
        }
    }

    pub fn is_release_operator_ready(self) -> bool {
        self.release_operator_ready
    }

    pub fn has_blocked_operator_checks(self) -> bool {
        self.blocked_operator_check_count > 0
    }

    pub fn needs_dispatch(self) -> bool {
        !self.dispatch_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_package_acceptance(self) -> bool {
        !self.package_accepted
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_operator_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseOperatorSummary {
    HuePackageReleaseOperatorSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseAuditSummary {
    pub operator_summary: HuePackageReleaseOperatorSummary,
    pub required_audit_check_count: usize,
    pub passed_audit_check_count: usize,
    pub blocked_audit_check_count: usize,
    pub operator_ready: bool,
    pub dispatch_ready: bool,
    pub coordination_ready: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_audit_ready: bool,
}

impl HuePackageReleaseAuditSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_operator_summary(hue_package_release_operator_summary(plan))
    }

    pub fn from_operator_summary(operator_summary: HuePackageReleaseOperatorSummary) -> Self {
        let operator_ready = operator_summary.is_release_operator_ready();
        let dispatch_ready = !operator_summary.needs_dispatch();
        let coordination_ready = !operator_summary.needs_coordination();
        let review_queues_clear = !operator_summary.needs_review_queue_clearance();
        let publish_gate_ready = !operator_summary.needs_publish_gate();
        let checks = [
            operator_ready,
            dispatch_ready,
            coordination_ready,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_audit_check_count = checks.iter().filter(|ready| **ready).count();
        let required_audit_check_count = checks.len();
        let blocked_audit_check_count = required_audit_check_count - passed_audit_check_count;
        let release_audit_ready = blocked_audit_check_count == 0;

        Self {
            operator_summary,
            required_audit_check_count,
            passed_audit_check_count,
            blocked_audit_check_count,
            operator_ready,
            dispatch_ready,
            coordination_ready,
            review_queues_clear,
            publish_gate_ready,
            release_audit_ready,
        }
    }

    pub fn is_release_audit_ready(self) -> bool {
        self.release_audit_ready
    }

    pub fn has_blocked_audit_checks(self) -> bool {
        self.blocked_audit_check_count > 0
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_dispatch(self) -> bool {
        !self.dispatch_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_audit_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseAuditSummary {
    HuePackageReleaseAuditSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseSignoffSummary {
    pub audit_summary: HuePackageReleaseAuditSummary,
    pub required_signoff_check_count: usize,
    pub passed_signoff_check_count: usize,
    pub blocked_signoff_check_count: usize,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub dispatch_ready: bool,
    pub coordination_ready: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_signoff_ready: bool,
}

impl HuePackageReleaseSignoffSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_audit_summary(hue_package_release_audit_summary(plan))
    }

    pub fn from_audit_summary(audit_summary: HuePackageReleaseAuditSummary) -> Self {
        let release_audit_ready = audit_summary.is_release_audit_ready();
        let operator_ready = !audit_summary.needs_operator_readiness();
        let dispatch_ready = !audit_summary.needs_dispatch();
        let coordination_ready = !audit_summary.needs_coordination();
        let review_queues_clear = !audit_summary.needs_review_queue_clearance();
        let publish_gate_ready = !audit_summary.needs_publish_gate();
        let checks = [
            release_audit_ready,
            operator_ready,
            dispatch_ready,
            coordination_ready,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_signoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_signoff_check_count = checks.len();
        let blocked_signoff_check_count = required_signoff_check_count - passed_signoff_check_count;
        let release_signoff_ready = blocked_signoff_check_count == 0;

        Self {
            audit_summary,
            required_signoff_check_count,
            passed_signoff_check_count,
            blocked_signoff_check_count,
            release_audit_ready,
            operator_ready,
            dispatch_ready,
            coordination_ready,
            review_queues_clear,
            publish_gate_ready,
            release_signoff_ready,
        }
    }

    pub fn is_release_signoff_ready(self) -> bool {
        self.release_signoff_ready
    }

    pub fn has_blocked_signoff_checks(self) -> bool {
        self.blocked_signoff_check_count > 0
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_dispatch(self) -> bool {
        !self.dispatch_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_signoff_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseSignoffSummary {
    HuePackageReleaseSignoffSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseClosureSummary {
    pub signoff_summary: HuePackageReleaseSignoffSummary,
    pub required_closure_check_count: usize,
    pub passed_closure_check_count: usize,
    pub blocked_closure_check_count: usize,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub dispatch_ready: bool,
    pub coordination_ready: bool,
    pub review_queues_clear: bool,
    pub publish_gate_ready: bool,
    pub release_closure_ready: bool,
}

impl HuePackageReleaseClosureSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_signoff_summary(hue_package_release_signoff_summary(plan))
    }

    pub fn from_signoff_summary(signoff_summary: HuePackageReleaseSignoffSummary) -> Self {
        let release_signoff_ready = signoff_summary.is_release_signoff_ready();
        let release_audit_ready = !signoff_summary.needs_release_audit();
        let operator_ready = !signoff_summary.needs_operator_readiness();
        let dispatch_ready = !signoff_summary.needs_dispatch();
        let coordination_ready = !signoff_summary.needs_coordination();
        let review_queues_clear = !signoff_summary.needs_review_queue_clearance();
        let publish_gate_ready = !signoff_summary.needs_publish_gate();
        let checks = [
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            dispatch_ready,
            coordination_ready,
            review_queues_clear,
            publish_gate_ready,
        ];
        let passed_closure_check_count = checks.iter().filter(|ready| **ready).count();
        let required_closure_check_count = checks.len();
        let blocked_closure_check_count = required_closure_check_count - passed_closure_check_count;
        let release_closure_ready = blocked_closure_check_count == 0;

        Self {
            signoff_summary,
            required_closure_check_count,
            passed_closure_check_count,
            blocked_closure_check_count,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            dispatch_ready,
            coordination_ready,
            review_queues_clear,
            publish_gate_ready,
            release_closure_ready,
        }
    }

    pub fn is_release_closure_ready(self) -> bool {
        self.release_closure_ready
    }

    pub fn has_blocked_closure_checks(self) -> bool {
        self.blocked_closure_check_count > 0
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_dispatch(self) -> bool {
        !self.dispatch_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_closure_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseClosureSummary {
    HuePackageReleaseClosureSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveSummary {
    pub closure_summary: HuePackageReleaseClosureSummary,
    pub required_archive_check_count: usize,
    pub passed_archive_check_count: usize,
    pub blocked_archive_check_count: usize,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_ready: bool,
}

impl HuePackageReleaseArchiveSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_closure_summary(hue_package_release_closure_summary(plan))
    }

    pub fn from_closure_summary(closure_summary: HuePackageReleaseClosureSummary) -> Self {
        let release_closure_ready = closure_summary.is_release_closure_ready();
        let release_signoff_ready = !closure_summary.needs_release_signoff();
        let release_audit_ready = !closure_summary.needs_release_audit();
        let operator_ready = !closure_summary.needs_operator_readiness();
        let coordination_ready = !closure_summary.needs_coordination();
        let publish_gate_ready = !closure_summary.needs_publish_gate();
        let checks = [
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_check_count = checks.len();
        let blocked_archive_check_count = required_archive_check_count - passed_archive_check_count;
        let release_archive_ready = blocked_archive_check_count == 0;

        Self {
            closure_summary,
            required_archive_check_count,
            passed_archive_check_count,
            blocked_archive_check_count,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_ready,
        }
    }

    pub fn is_release_archive_ready(self) -> bool {
        self.release_archive_ready
    }

    pub fn has_blocked_archive_checks(self) -> bool {
        self.blocked_archive_check_count > 0
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveSummary {
    HuePackageReleaseArchiveSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveSignoffSummary {
    pub archive_summary: HuePackageReleaseArchiveSummary,
    pub required_archive_signoff_check_count: usize,
    pub passed_archive_signoff_check_count: usize,
    pub blocked_archive_signoff_check_count: usize,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_signoff_ready: bool,
}

impl HuePackageReleaseArchiveSignoffSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_summary(hue_package_release_archive_summary(plan))
    }

    pub fn from_archive_summary(archive_summary: HuePackageReleaseArchiveSummary) -> Self {
        let release_archive_ready = archive_summary.is_release_archive_ready();
        let release_closure_ready = !archive_summary.needs_release_closure();
        let release_signoff_ready = !archive_summary.needs_release_signoff();
        let release_audit_ready = !archive_summary.needs_release_audit();
        let operator_ready = !archive_summary.needs_operator_readiness();
        let coordination_ready = !archive_summary.needs_coordination();
        let publish_gate_ready = !archive_summary.needs_publish_gate();
        let checks = [
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_signoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_signoff_check_count = checks.len();
        let blocked_archive_signoff_check_count =
            required_archive_signoff_check_count - passed_archive_signoff_check_count;
        let release_archive_signoff_ready = blocked_archive_signoff_check_count == 0;

        Self {
            archive_summary,
            required_archive_signoff_check_count,
            passed_archive_signoff_check_count,
            blocked_archive_signoff_check_count,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_signoff_ready,
        }
    }

    pub fn is_release_archive_signoff_ready(self) -> bool {
        self.release_archive_signoff_ready
    }

    pub fn has_blocked_archive_signoff_checks(self) -> bool {
        self.blocked_archive_signoff_check_count > 0
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_signoff_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveSignoffSummary {
    HuePackageReleaseArchiveSignoffSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveClosureSummary {
    pub archive_signoff_summary: HuePackageReleaseArchiveSignoffSummary,
    pub required_archive_closure_check_count: usize,
    pub passed_archive_closure_check_count: usize,
    pub blocked_archive_closure_check_count: usize,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_closure_ready: bool,
}

impl HuePackageReleaseArchiveClosureSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_signoff_summary(hue_package_release_archive_signoff_summary(plan))
    }

    pub fn from_archive_signoff_summary(
        archive_signoff_summary: HuePackageReleaseArchiveSignoffSummary,
    ) -> Self {
        let release_archive_signoff_ready =
            archive_signoff_summary.is_release_archive_signoff_ready();
        let release_archive_ready = !archive_signoff_summary.needs_release_archive();
        let release_closure_ready = !archive_signoff_summary.needs_release_closure();
        let release_signoff_ready = !archive_signoff_summary.needs_release_signoff();
        let release_audit_ready = !archive_signoff_summary.needs_release_audit();
        let operator_ready = !archive_signoff_summary.needs_operator_readiness();
        let coordination_ready = !archive_signoff_summary.needs_coordination();
        let publish_gate_ready = !archive_signoff_summary.needs_publish_gate();
        let checks = [
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_closure_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_closure_check_count = checks.len();
        let blocked_archive_closure_check_count =
            required_archive_closure_check_count - passed_archive_closure_check_count;
        let release_archive_closure_ready = blocked_archive_closure_check_count == 0;

        Self {
            archive_signoff_summary,
            required_archive_closure_check_count,
            passed_archive_closure_check_count,
            blocked_archive_closure_check_count,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_closure_ready,
        }
    }

    pub fn is_release_archive_closure_ready(self) -> bool {
        self.release_archive_closure_ready
    }

    pub fn has_blocked_archive_closure_checks(self) -> bool {
        self.blocked_archive_closure_check_count > 0
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_closure_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveClosureSummary {
    HuePackageReleaseArchiveClosureSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveHandoffSummary {
    pub archive_closure_summary: HuePackageReleaseArchiveClosureSummary,
    pub required_archive_handoff_check_count: usize,
    pub passed_archive_handoff_check_count: usize,
    pub blocked_archive_handoff_check_count: usize,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_handoff_ready: bool,
}

impl HuePackageReleaseArchiveHandoffSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_closure_summary(hue_package_release_archive_closure_summary(plan))
    }

    pub fn from_archive_closure_summary(
        archive_closure_summary: HuePackageReleaseArchiveClosureSummary,
    ) -> Self {
        let release_archive_closure_ready =
            archive_closure_summary.is_release_archive_closure_ready();
        let release_archive_signoff_ready =
            !archive_closure_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_closure_summary.needs_release_archive();
        let release_closure_ready = !archive_closure_summary.needs_release_closure();
        let release_signoff_ready = !archive_closure_summary.needs_release_signoff();
        let release_audit_ready = !archive_closure_summary.needs_release_audit();
        let operator_ready = !archive_closure_summary.needs_operator_readiness();
        let coordination_ready = !archive_closure_summary.needs_coordination();
        let publish_gate_ready = !archive_closure_summary.needs_publish_gate();
        let checks = [
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_handoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_handoff_check_count = checks.len();
        let blocked_archive_handoff_check_count =
            required_archive_handoff_check_count - passed_archive_handoff_check_count;
        let release_archive_handoff_ready = blocked_archive_handoff_check_count == 0;

        Self {
            archive_closure_summary,
            required_archive_handoff_check_count,
            passed_archive_handoff_check_count,
            blocked_archive_handoff_check_count,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_handoff_ready,
        }
    }

    pub fn is_release_archive_handoff_ready(self) -> bool {
        self.release_archive_handoff_ready
    }

    pub fn has_blocked_archive_handoff_checks(self) -> bool {
        self.blocked_archive_handoff_check_count > 0
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_handoff_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveHandoffSummary {
    HuePackageReleaseArchiveHandoffSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveDispatchSummary {
    pub archive_handoff_summary: HuePackageReleaseArchiveHandoffSummary,
    pub required_archive_dispatch_check_count: usize,
    pub passed_archive_dispatch_check_count: usize,
    pub blocked_archive_dispatch_check_count: usize,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_dispatch_ready: bool,
}

impl HuePackageReleaseArchiveDispatchSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_handoff_summary(hue_package_release_archive_handoff_summary(plan))
    }

    pub fn from_archive_handoff_summary(
        archive_handoff_summary: HuePackageReleaseArchiveHandoffSummary,
    ) -> Self {
        let release_archive_handoff_ready =
            archive_handoff_summary.is_release_archive_handoff_ready();
        let release_archive_closure_ready =
            !archive_handoff_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_handoff_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_handoff_summary.needs_release_archive();
        let release_closure_ready = !archive_handoff_summary.needs_release_closure();
        let release_signoff_ready = !archive_handoff_summary.needs_release_signoff();
        let release_audit_ready = !archive_handoff_summary.needs_release_audit();
        let operator_ready = !archive_handoff_summary.needs_operator_readiness();
        let coordination_ready = !archive_handoff_summary.needs_coordination();
        let publish_gate_ready = !archive_handoff_summary.needs_publish_gate();
        let checks = [
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_dispatch_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_dispatch_check_count = checks.len();
        let blocked_archive_dispatch_check_count =
            required_archive_dispatch_check_count - passed_archive_dispatch_check_count;
        let release_archive_dispatch_ready = blocked_archive_dispatch_check_count == 0;

        Self {
            archive_handoff_summary,
            required_archive_dispatch_check_count,
            passed_archive_dispatch_check_count,
            blocked_archive_dispatch_check_count,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_dispatch_ready,
        }
    }

    pub fn is_release_archive_dispatch_ready(self) -> bool {
        self.release_archive_dispatch_ready
    }

    pub fn has_blocked_archive_dispatch_checks(self) -> bool {
        self.blocked_archive_dispatch_check_count > 0
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_dispatch_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveDispatchSummary {
    HuePackageReleaseArchiveDispatchSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveOperatorSummary {
    pub archive_dispatch_summary: HuePackageReleaseArchiveDispatchSummary,
    pub required_archive_operator_check_count: usize,
    pub passed_archive_operator_check_count: usize,
    pub blocked_archive_operator_check_count: usize,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_operator_ready: bool,
}

impl HuePackageReleaseArchiveOperatorSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_dispatch_summary(hue_package_release_archive_dispatch_summary(plan))
    }

    pub fn from_archive_dispatch_summary(
        archive_dispatch_summary: HuePackageReleaseArchiveDispatchSummary,
    ) -> Self {
        let release_archive_dispatch_ready =
            archive_dispatch_summary.is_release_archive_dispatch_ready();
        let release_archive_handoff_ready =
            !archive_dispatch_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_dispatch_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_dispatch_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_dispatch_summary.needs_release_archive();
        let release_closure_ready = !archive_dispatch_summary.needs_release_closure();
        let release_signoff_ready = !archive_dispatch_summary.needs_release_signoff();
        let release_audit_ready = !archive_dispatch_summary.needs_release_audit();
        let operator_ready = !archive_dispatch_summary.needs_operator_readiness();
        let coordination_ready = !archive_dispatch_summary.needs_coordination();
        let publish_gate_ready = !archive_dispatch_summary.needs_publish_gate();
        let checks = [
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_operator_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_operator_check_count = checks.len();
        let blocked_archive_operator_check_count =
            required_archive_operator_check_count - passed_archive_operator_check_count;
        let release_archive_operator_ready = blocked_archive_operator_check_count == 0;

        Self {
            archive_dispatch_summary,
            required_archive_operator_check_count,
            passed_archive_operator_check_count,
            blocked_archive_operator_check_count,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_operator_ready,
        }
    }

    pub fn is_release_archive_operator_ready(self) -> bool {
        self.release_archive_operator_ready
    }

    pub fn has_blocked_archive_operator_checks(self) -> bool {
        self.blocked_archive_operator_check_count > 0
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_operator_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveOperatorSummary {
    HuePackageReleaseArchiveOperatorSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveSupervisorSummary {
    pub archive_operator_summary: HuePackageReleaseArchiveOperatorSummary,
    pub required_archive_supervisor_check_count: usize,
    pub passed_archive_supervisor_check_count: usize,
    pub blocked_archive_supervisor_check_count: usize,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_supervisor_ready: bool,
}

impl HuePackageReleaseArchiveSupervisorSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_operator_summary(hue_package_release_archive_operator_summary(plan))
    }

    pub fn from_archive_operator_summary(
        archive_operator_summary: HuePackageReleaseArchiveOperatorSummary,
    ) -> Self {
        let release_archive_operator_ready =
            archive_operator_summary.is_release_archive_operator_ready();
        let release_archive_dispatch_ready =
            !archive_operator_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_operator_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_operator_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_operator_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_operator_summary.needs_release_archive();
        let release_closure_ready = !archive_operator_summary.needs_release_closure();
        let release_signoff_ready = !archive_operator_summary.needs_release_signoff();
        let release_audit_ready = !archive_operator_summary.needs_release_audit();
        let operator_ready = !archive_operator_summary.needs_operator_readiness();
        let coordination_ready = !archive_operator_summary.needs_coordination();
        let publish_gate_ready = !archive_operator_summary.needs_publish_gate();
        let checks = [
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_supervisor_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_supervisor_check_count = checks.len();
        let blocked_archive_supervisor_check_count =
            required_archive_supervisor_check_count - passed_archive_supervisor_check_count;
        let release_archive_supervisor_ready = blocked_archive_supervisor_check_count == 0;

        Self {
            archive_operator_summary,
            required_archive_supervisor_check_count,
            passed_archive_supervisor_check_count,
            blocked_archive_supervisor_check_count,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_supervisor_ready,
        }
    }

    pub fn is_release_archive_supervisor_ready(self) -> bool {
        self.release_archive_supervisor_ready
    }

    pub fn has_blocked_archive_supervisor_checks(self) -> bool {
        self.blocked_archive_supervisor_check_count > 0
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_supervisor_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveSupervisorSummary {
    HuePackageReleaseArchiveSupervisorSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveCompletionSummary {
    pub archive_supervisor_summary: HuePackageReleaseArchiveSupervisorSummary,
    pub required_archive_completion_check_count: usize,
    pub passed_archive_completion_check_count: usize,
    pub blocked_archive_completion_check_count: usize,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_completion_ready: bool,
}

impl HuePackageReleaseArchiveCompletionSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_supervisor_summary(hue_package_release_archive_supervisor_summary(plan))
    }

    pub fn from_archive_supervisor_summary(
        archive_supervisor_summary: HuePackageReleaseArchiveSupervisorSummary,
    ) -> Self {
        let release_archive_supervisor_ready =
            archive_supervisor_summary.is_release_archive_supervisor_ready();
        let release_archive_operator_ready =
            !archive_supervisor_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_supervisor_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_supervisor_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_supervisor_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_supervisor_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_supervisor_summary.needs_release_archive();
        let release_closure_ready = !archive_supervisor_summary.needs_release_closure();
        let release_signoff_ready = !archive_supervisor_summary.needs_release_signoff();
        let release_audit_ready = !archive_supervisor_summary.needs_release_audit();
        let operator_ready = !archive_supervisor_summary.needs_operator_readiness();
        let coordination_ready = !archive_supervisor_summary.needs_coordination();
        let publish_gate_ready = !archive_supervisor_summary.needs_publish_gate();
        let checks = [
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_completion_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_completion_check_count = checks.len();
        let blocked_archive_completion_check_count =
            required_archive_completion_check_count - passed_archive_completion_check_count;
        let release_archive_completion_ready = blocked_archive_completion_check_count == 0;

        Self {
            archive_supervisor_summary,
            required_archive_completion_check_count,
            passed_archive_completion_check_count,
            blocked_archive_completion_check_count,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_completion_ready,
        }
    }

    pub fn is_release_archive_completion_ready(self) -> bool {
        self.release_archive_completion_ready
    }

    pub fn has_blocked_archive_completion_checks(self) -> bool {
        self.blocked_archive_completion_check_count > 0
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_completion_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveCompletionSummary {
    HuePackageReleaseArchiveCompletionSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchivePublicationSummary {
    pub archive_completion_summary: HuePackageReleaseArchiveCompletionSummary,
    pub required_archive_publication_check_count: usize,
    pub passed_archive_publication_check_count: usize,
    pub blocked_archive_publication_check_count: usize,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_publication_ready: bool,
}

impl HuePackageReleaseArchivePublicationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_completion_summary(hue_package_release_archive_completion_summary(plan))
    }

    pub fn from_archive_completion_summary(
        archive_completion_summary: HuePackageReleaseArchiveCompletionSummary,
    ) -> Self {
        let release_archive_completion_ready =
            archive_completion_summary.is_release_archive_completion_ready();
        let release_archive_supervisor_ready =
            !archive_completion_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_completion_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_completion_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_completion_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_completion_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_completion_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_completion_summary.needs_release_archive();
        let release_closure_ready = !archive_completion_summary.needs_release_closure();
        let release_signoff_ready = !archive_completion_summary.needs_release_signoff();
        let release_audit_ready = !archive_completion_summary.needs_release_audit();
        let operator_ready = !archive_completion_summary.needs_operator_readiness();
        let coordination_ready = !archive_completion_summary.needs_coordination();
        let publish_gate_ready = !archive_completion_summary.needs_publish_gate();
        let checks = [
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_publication_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_publication_check_count = checks.len();
        let blocked_archive_publication_check_count =
            required_archive_publication_check_count - passed_archive_publication_check_count;
        let release_archive_publication_ready = blocked_archive_publication_check_count == 0;

        Self {
            archive_completion_summary,
            required_archive_publication_check_count,
            passed_archive_publication_check_count,
            blocked_archive_publication_check_count,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_publication_ready,
        }
    }

    pub fn is_release_archive_publication_ready(self) -> bool {
        self.release_archive_publication_ready
    }

    pub fn has_blocked_archive_publication_checks(self) -> bool {
        self.blocked_archive_publication_check_count > 0
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_publication_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchivePublicationSummary {
    HuePackageReleaseArchivePublicationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveVerificationSummary {
    pub archive_publication_summary: HuePackageReleaseArchivePublicationSummary,
    pub required_archive_verification_check_count: usize,
    pub passed_archive_verification_check_count: usize,
    pub blocked_archive_verification_check_count: usize,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_verification_ready: bool,
}

impl HuePackageReleaseArchiveVerificationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_publication_summary(hue_package_release_archive_publication_summary(
            plan,
        ))
    }

    pub fn from_archive_publication_summary(
        archive_publication_summary: HuePackageReleaseArchivePublicationSummary,
    ) -> Self {
        let release_archive_publication_ready =
            archive_publication_summary.is_release_archive_publication_ready();
        let release_archive_completion_ready =
            !archive_publication_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_publication_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_publication_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_publication_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_publication_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_publication_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_publication_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_publication_summary.needs_release_archive();
        let release_closure_ready = !archive_publication_summary.needs_release_closure();
        let release_signoff_ready = !archive_publication_summary.needs_release_signoff();
        let release_audit_ready = !archive_publication_summary.needs_release_audit();
        let operator_ready = !archive_publication_summary.needs_operator_readiness();
        let coordination_ready = !archive_publication_summary.needs_coordination();
        let publish_gate_ready = !archive_publication_summary.needs_publish_gate();
        let checks = [
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_verification_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_verification_check_count = checks.len();
        let blocked_archive_verification_check_count =
            required_archive_verification_check_count - passed_archive_verification_check_count;
        let release_archive_verification_ready = blocked_archive_verification_check_count == 0;

        Self {
            archive_publication_summary,
            required_archive_verification_check_count,
            passed_archive_verification_check_count,
            blocked_archive_verification_check_count,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_verification_ready,
        }
    }

    pub fn is_release_archive_verification_ready(self) -> bool {
        self.release_archive_verification_ready
    }

    pub fn has_blocked_archive_verification_checks(self) -> bool {
        self.blocked_archive_verification_check_count > 0
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_verification_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveVerificationSummary {
    HuePackageReleaseArchiveVerificationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveValidationSummary {
    pub archive_verification_summary: HuePackageReleaseArchiveVerificationSummary,
    pub required_archive_validation_check_count: usize,
    pub passed_archive_validation_check_count: usize,
    pub blocked_archive_validation_check_count: usize,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_validation_ready: bool,
}

impl HuePackageReleaseArchiveValidationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_verification_summary(hue_package_release_archive_verification_summary(
            plan,
        ))
    }

    pub fn from_archive_verification_summary(
        archive_verification_summary: HuePackageReleaseArchiveVerificationSummary,
    ) -> Self {
        let release_archive_verification_ready =
            archive_verification_summary.is_release_archive_verification_ready();
        let release_archive_publication_ready =
            !archive_verification_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_verification_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_verification_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_verification_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_verification_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_verification_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_verification_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_verification_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_verification_summary.needs_release_archive();
        let release_closure_ready = !archive_verification_summary.needs_release_closure();
        let release_signoff_ready = !archive_verification_summary.needs_release_signoff();
        let release_audit_ready = !archive_verification_summary.needs_release_audit();
        let operator_ready = !archive_verification_summary.needs_operator_readiness();
        let coordination_ready = !archive_verification_summary.needs_coordination();
        let publish_gate_ready = !archive_verification_summary.needs_publish_gate();
        let checks = [
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_validation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_validation_check_count = checks.len();
        let blocked_archive_validation_check_count =
            required_archive_validation_check_count - passed_archive_validation_check_count;
        let release_archive_validation_ready = blocked_archive_validation_check_count == 0;

        Self {
            archive_verification_summary,
            required_archive_validation_check_count,
            passed_archive_validation_check_count,
            blocked_archive_validation_check_count,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_validation_ready,
        }
    }

    pub fn is_release_archive_validation_ready(self) -> bool {
        self.release_archive_validation_ready
    }

    pub fn has_blocked_archive_validation_checks(self) -> bool {
        self.blocked_archive_validation_check_count > 0
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_validation_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveValidationSummary {
    HuePackageReleaseArchiveValidationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveCertificationSummary {
    pub archive_validation_summary: HuePackageReleaseArchiveValidationSummary,
    pub required_archive_certification_check_count: usize,
    pub passed_archive_certification_check_count: usize,
    pub blocked_archive_certification_check_count: usize,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_certification_ready: bool,
}

impl HuePackageReleaseArchiveCertificationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_validation_summary(hue_package_release_archive_validation_summary(plan))
    }

    pub fn from_archive_validation_summary(
        archive_validation_summary: HuePackageReleaseArchiveValidationSummary,
    ) -> Self {
        let release_archive_validation_ready =
            archive_validation_summary.is_release_archive_validation_ready();
        let release_archive_verification_ready =
            !archive_validation_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_validation_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_validation_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_validation_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_validation_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_validation_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_validation_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_validation_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_validation_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_validation_summary.needs_release_archive();
        let release_closure_ready = !archive_validation_summary.needs_release_closure();
        let release_signoff_ready = !archive_validation_summary.needs_release_signoff();
        let release_audit_ready = !archive_validation_summary.needs_release_audit();
        let operator_ready = !archive_validation_summary.needs_operator_readiness();
        let coordination_ready = !archive_validation_summary.needs_coordination();
        let publish_gate_ready = !archive_validation_summary.needs_publish_gate();
        let checks = [
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_certification_check_count =
            checks.iter().filter(|ready| **ready).count();
        let required_archive_certification_check_count = checks.len();
        let blocked_archive_certification_check_count =
            required_archive_certification_check_count - passed_archive_certification_check_count;
        let release_archive_certification_ready = blocked_archive_certification_check_count == 0;

        Self {
            archive_validation_summary,
            required_archive_certification_check_count,
            passed_archive_certification_check_count,
            blocked_archive_certification_check_count,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_certification_ready,
        }
    }

    pub fn is_release_archive_certification_ready(self) -> bool {
        self.release_archive_certification_ready
    }

    pub fn has_blocked_archive_certification_checks(self) -> bool {
        self.blocked_archive_certification_check_count > 0
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_certification_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveCertificationSummary {
    HuePackageReleaseArchiveCertificationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveApprovalSummary {
    pub archive_certification_summary: HuePackageReleaseArchiveCertificationSummary,
    pub required_archive_approval_check_count: usize,
    pub passed_archive_approval_check_count: usize,
    pub blocked_archive_approval_check_count: usize,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_approval_ready: bool,
}

impl HuePackageReleaseArchiveApprovalSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_certification_summary(hue_package_release_archive_certification_summary(
            plan,
        ))
    }

    pub fn from_archive_certification_summary(
        archive_certification_summary: HuePackageReleaseArchiveCertificationSummary,
    ) -> Self {
        let release_archive_certification_ready =
            archive_certification_summary.is_release_archive_certification_ready();
        let release_archive_validation_ready =
            !archive_certification_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_certification_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_certification_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_certification_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_certification_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_certification_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_certification_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_certification_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_certification_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_certification_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_certification_summary.needs_release_archive();
        let release_closure_ready = !archive_certification_summary.needs_release_closure();
        let release_signoff_ready = !archive_certification_summary.needs_release_signoff();
        let release_audit_ready = !archive_certification_summary.needs_release_audit();
        let operator_ready = !archive_certification_summary.needs_operator_readiness();
        let coordination_ready = !archive_certification_summary.needs_coordination();
        let publish_gate_ready = !archive_certification_summary.needs_publish_gate();
        let checks = [
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_approval_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_approval_check_count = checks.len();
        let blocked_archive_approval_check_count =
            required_archive_approval_check_count - passed_archive_approval_check_count;
        let release_archive_approval_ready = blocked_archive_approval_check_count == 0;

        Self {
            archive_certification_summary,
            required_archive_approval_check_count,
            passed_archive_approval_check_count,
            blocked_archive_approval_check_count,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_approval_ready,
        }
    }

    pub fn is_release_archive_approval_ready(self) -> bool {
        self.release_archive_approval_ready
    }

    pub fn has_blocked_archive_approval_checks(self) -> bool {
        self.blocked_archive_approval_check_count > 0
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_approval_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveApprovalSummary {
    HuePackageReleaseArchiveApprovalSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveActivationSummary {
    pub archive_approval_summary: HuePackageReleaseArchiveApprovalSummary,
    pub required_archive_activation_check_count: usize,
    pub passed_archive_activation_check_count: usize,
    pub blocked_archive_activation_check_count: usize,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_activation_ready: bool,
}

impl HuePackageReleaseArchiveActivationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_approval_summary(hue_package_release_archive_approval_summary(plan))
    }

    pub fn from_archive_approval_summary(
        archive_approval_summary: HuePackageReleaseArchiveApprovalSummary,
    ) -> Self {
        let release_archive_approval_ready =
            archive_approval_summary.is_release_archive_approval_ready();
        let release_archive_certification_ready =
            !archive_approval_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_approval_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_approval_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_approval_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_approval_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_approval_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_approval_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_approval_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_approval_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_approval_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_approval_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_approval_summary.needs_release_archive();
        let release_closure_ready = !archive_approval_summary.needs_release_closure();
        let release_signoff_ready = !archive_approval_summary.needs_release_signoff();
        let release_audit_ready = !archive_approval_summary.needs_release_audit();
        let operator_ready = !archive_approval_summary.needs_operator_readiness();
        let coordination_ready = !archive_approval_summary.needs_coordination();
        let publish_gate_ready = !archive_approval_summary.needs_publish_gate();
        let checks = [
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_activation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_activation_check_count = checks.len();
        let blocked_archive_activation_check_count =
            required_archive_activation_check_count - passed_archive_activation_check_count;
        let release_archive_activation_ready = blocked_archive_activation_check_count == 0;

        Self {
            archive_approval_summary,
            required_archive_activation_check_count,
            passed_archive_activation_check_count,
            blocked_archive_activation_check_count,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_activation_ready,
        }
    }

    pub fn is_release_archive_activation_ready(self) -> bool {
        self.release_archive_activation_ready
    }

    pub fn has_blocked_archive_activation_checks(self) -> bool {
        self.blocked_archive_activation_check_count > 0
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_activation_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveActivationSummary {
    HuePackageReleaseArchiveActivationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveRolloutSummary {
    pub archive_activation_summary: HuePackageReleaseArchiveActivationSummary,
    pub required_archive_rollout_check_count: usize,
    pub passed_archive_rollout_check_count: usize,
    pub blocked_archive_rollout_check_count: usize,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_rollout_ready: bool,
}

impl HuePackageReleaseArchiveRolloutSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_activation_summary(hue_package_release_archive_activation_summary(plan))
    }

    pub fn from_archive_activation_summary(
        archive_activation_summary: HuePackageReleaseArchiveActivationSummary,
    ) -> Self {
        let release_archive_activation_ready =
            archive_activation_summary.is_release_archive_activation_ready();
        let release_archive_approval_ready =
            !archive_activation_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_activation_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_activation_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_activation_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_activation_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_activation_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_activation_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_activation_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_activation_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_activation_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_activation_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_activation_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_activation_summary.needs_release_archive();
        let release_closure_ready = !archive_activation_summary.needs_release_closure();
        let release_signoff_ready = !archive_activation_summary.needs_release_signoff();
        let release_audit_ready = !archive_activation_summary.needs_release_audit();
        let operator_ready = !archive_activation_summary.needs_operator_readiness();
        let coordination_ready = !archive_activation_summary.needs_coordination();
        let publish_gate_ready = !archive_activation_summary.needs_publish_gate();
        let checks = [
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_rollout_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_rollout_check_count = checks.len();
        let blocked_archive_rollout_check_count =
            required_archive_rollout_check_count - passed_archive_rollout_check_count;
        let release_archive_rollout_ready = blocked_archive_rollout_check_count == 0;

        Self {
            archive_activation_summary,
            required_archive_rollout_check_count,
            passed_archive_rollout_check_count,
            blocked_archive_rollout_check_count,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_rollout_ready,
        }
    }

    pub fn is_release_archive_rollout_ready(self) -> bool {
        self.release_archive_rollout_ready
    }

    pub fn has_blocked_archive_rollout_checks(self) -> bool {
        self.blocked_archive_rollout_check_count > 0
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_rollout_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveRolloutSummary {
    HuePackageReleaseArchiveRolloutSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveAdoptionSummary {
    pub archive_rollout_summary: HuePackageReleaseArchiveRolloutSummary,
    pub required_archive_adoption_check_count: usize,
    pub passed_archive_adoption_check_count: usize,
    pub blocked_archive_adoption_check_count: usize,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_adoption_ready: bool,
}

impl HuePackageReleaseArchiveAdoptionSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_rollout_summary(hue_package_release_archive_rollout_summary(plan))
    }

    pub fn from_archive_rollout_summary(
        archive_rollout_summary: HuePackageReleaseArchiveRolloutSummary,
    ) -> Self {
        let release_archive_rollout_ready =
            archive_rollout_summary.is_release_archive_rollout_ready();
        let release_archive_activation_ready =
            !archive_rollout_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_rollout_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_rollout_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_rollout_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_rollout_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_rollout_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_rollout_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_rollout_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_rollout_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_rollout_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_rollout_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_rollout_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_rollout_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_rollout_summary.needs_release_archive();
        let release_closure_ready = !archive_rollout_summary.needs_release_closure();
        let release_signoff_ready = !archive_rollout_summary.needs_release_signoff();
        let release_audit_ready = !archive_rollout_summary.needs_release_audit();
        let operator_ready = !archive_rollout_summary.needs_operator_readiness();
        let coordination_ready = !archive_rollout_summary.needs_coordination();
        let publish_gate_ready = !archive_rollout_summary.needs_publish_gate();
        let checks = [
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_adoption_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_adoption_check_count = checks.len();
        let blocked_archive_adoption_check_count =
            required_archive_adoption_check_count - passed_archive_adoption_check_count;
        let release_archive_adoption_ready = blocked_archive_adoption_check_count == 0;

        Self {
            archive_rollout_summary,
            required_archive_adoption_check_count,
            passed_archive_adoption_check_count,
            blocked_archive_adoption_check_count,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_adoption_ready,
        }
    }

    pub fn is_release_archive_adoption_ready(self) -> bool {
        self.release_archive_adoption_ready
    }

    pub fn has_blocked_archive_adoption_checks(self) -> bool {
        self.blocked_archive_adoption_check_count > 0
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_adoption_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveAdoptionSummary {
    HuePackageReleaseArchiveAdoptionSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveAcceptanceSummary {
    pub archive_adoption_summary: HuePackageReleaseArchiveAdoptionSummary,
    pub required_archive_acceptance_check_count: usize,
    pub passed_archive_acceptance_check_count: usize,
    pub blocked_archive_acceptance_check_count: usize,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_acceptance_ready: bool,
}

impl HuePackageReleaseArchiveAcceptanceSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_adoption_summary(hue_package_release_archive_adoption_summary(plan))
    }

    pub fn from_archive_adoption_summary(
        archive_adoption_summary: HuePackageReleaseArchiveAdoptionSummary,
    ) -> Self {
        let release_archive_adoption_ready =
            archive_adoption_summary.is_release_archive_adoption_ready();
        let release_archive_rollout_ready =
            !archive_adoption_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_adoption_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_adoption_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_adoption_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_adoption_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_adoption_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_adoption_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_adoption_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_adoption_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_adoption_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_adoption_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_adoption_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_adoption_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_adoption_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_adoption_summary.needs_release_archive();
        let release_closure_ready = !archive_adoption_summary.needs_release_closure();
        let release_signoff_ready = !archive_adoption_summary.needs_release_signoff();
        let release_audit_ready = !archive_adoption_summary.needs_release_audit();
        let operator_ready = !archive_adoption_summary.needs_operator_readiness();
        let coordination_ready = !archive_adoption_summary.needs_coordination();
        let publish_gate_ready = !archive_adoption_summary.needs_publish_gate();
        let checks = [
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_acceptance_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_acceptance_check_count = checks.len();
        let blocked_archive_acceptance_check_count =
            required_archive_acceptance_check_count - passed_archive_acceptance_check_count;
        let release_archive_acceptance_ready = blocked_archive_acceptance_check_count == 0;

        Self {
            archive_adoption_summary,
            required_archive_acceptance_check_count,
            passed_archive_acceptance_check_count,
            blocked_archive_acceptance_check_count,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_acceptance_ready,
        }
    }

    pub fn is_release_archive_acceptance_ready(self) -> bool {
        self.release_archive_acceptance_ready
    }

    pub fn has_blocked_archive_acceptance_checks(self) -> bool {
        self.blocked_archive_acceptance_check_count > 0
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_acceptance_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveAcceptanceSummary {
    HuePackageReleaseArchiveAcceptanceSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveDistributionSummary {
    pub archive_acceptance_summary: HuePackageReleaseArchiveAcceptanceSummary,
    pub required_archive_distribution_check_count: usize,
    pub passed_archive_distribution_check_count: usize,
    pub blocked_archive_distribution_check_count: usize,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_distribution_ready: bool,
}

impl HuePackageReleaseArchiveDistributionSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_acceptance_summary(hue_package_release_archive_acceptance_summary(plan))
    }

    pub fn from_archive_acceptance_summary(
        archive_acceptance_summary: HuePackageReleaseArchiveAcceptanceSummary,
    ) -> Self {
        let release_archive_acceptance_ready =
            archive_acceptance_summary.is_release_archive_acceptance_ready();
        let release_archive_adoption_ready =
            !archive_acceptance_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_acceptance_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_acceptance_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_acceptance_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_acceptance_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_acceptance_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_acceptance_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_acceptance_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_acceptance_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_acceptance_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_acceptance_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_acceptance_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_acceptance_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_acceptance_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_acceptance_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_acceptance_summary.needs_release_archive();
        let release_closure_ready = !archive_acceptance_summary.needs_release_closure();
        let release_signoff_ready = !archive_acceptance_summary.needs_release_signoff();
        let release_audit_ready = !archive_acceptance_summary.needs_release_audit();
        let operator_ready = !archive_acceptance_summary.needs_operator_readiness();
        let coordination_ready = !archive_acceptance_summary.needs_coordination();
        let publish_gate_ready = !archive_acceptance_summary.needs_publish_gate();
        let checks = [
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_distribution_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_distribution_check_count = checks.len();
        let blocked_archive_distribution_check_count =
            required_archive_distribution_check_count - passed_archive_distribution_check_count;
        let release_archive_distribution_ready = blocked_archive_distribution_check_count == 0;

        Self {
            archive_acceptance_summary,
            required_archive_distribution_check_count,
            passed_archive_distribution_check_count,
            blocked_archive_distribution_check_count,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_distribution_ready,
        }
    }

    pub fn is_release_archive_distribution_ready(self) -> bool {
        self.release_archive_distribution_ready
    }

    pub fn has_blocked_archive_distribution_checks(self) -> bool {
        self.blocked_archive_distribution_check_count > 0
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_distribution_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveDistributionSummary {
    HuePackageReleaseArchiveDistributionSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveExportSummary {
    pub archive_distribution_summary: HuePackageReleaseArchiveDistributionSummary,
    pub required_archive_export_check_count: usize,
    pub passed_archive_export_check_count: usize,
    pub blocked_archive_export_check_count: usize,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_export_ready: bool,
}

impl HuePackageReleaseArchiveExportSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_distribution_summary(hue_package_release_archive_distribution_summary(
            plan,
        ))
    }

    pub fn from_archive_distribution_summary(
        archive_distribution_summary: HuePackageReleaseArchiveDistributionSummary,
    ) -> Self {
        let release_archive_distribution_ready =
            archive_distribution_summary.is_release_archive_distribution_ready();
        let release_archive_acceptance_ready =
            !archive_distribution_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_distribution_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_distribution_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_distribution_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_distribution_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_distribution_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_distribution_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_distribution_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_distribution_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_distribution_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_distribution_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_distribution_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_distribution_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_distribution_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_distribution_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_distribution_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_distribution_summary.needs_release_archive();
        let release_closure_ready = !archive_distribution_summary.needs_release_closure();
        let release_signoff_ready = !archive_distribution_summary.needs_release_signoff();
        let release_audit_ready = !archive_distribution_summary.needs_release_audit();
        let operator_ready = !archive_distribution_summary.needs_operator_readiness();
        let coordination_ready = !archive_distribution_summary.needs_coordination();
        let publish_gate_ready = !archive_distribution_summary.needs_publish_gate();
        let checks = [
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_export_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_export_check_count = checks.len();
        let blocked_archive_export_check_count =
            required_archive_export_check_count - passed_archive_export_check_count;
        let release_archive_export_ready = blocked_archive_export_check_count == 0;

        Self {
            archive_distribution_summary,
            required_archive_export_check_count,
            passed_archive_export_check_count,
            blocked_archive_export_check_count,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_export_ready,
        }
    }

    pub fn is_release_archive_export_ready(self) -> bool {
        self.release_archive_export_ready
    }

    pub fn has_blocked_archive_export_checks(self) -> bool {
        self.blocked_archive_export_check_count > 0
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_export_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveExportSummary {
    HuePackageReleaseArchiveExportSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveImportSummary {
    pub archive_export_summary: HuePackageReleaseArchiveExportSummary,
    pub required_archive_import_check_count: usize,
    pub passed_archive_import_check_count: usize,
    pub blocked_archive_import_check_count: usize,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_import_ready: bool,
}

impl HuePackageReleaseArchiveImportSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_export_summary(hue_package_release_archive_export_summary(plan))
    }

    pub fn from_archive_export_summary(
        archive_export_summary: HuePackageReleaseArchiveExportSummary,
    ) -> Self {
        let release_archive_export_ready = archive_export_summary.is_release_archive_export_ready();
        let release_archive_distribution_ready =
            !archive_export_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_export_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_export_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready = !archive_export_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_export_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_export_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_export_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_export_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_export_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_export_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_export_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_export_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_export_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_export_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready = !archive_export_summary.needs_release_archive_handoff();
        let release_archive_closure_ready = !archive_export_summary.needs_release_archive_closure();
        let release_archive_signoff_ready = !archive_export_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_export_summary.needs_release_archive();
        let release_closure_ready = !archive_export_summary.needs_release_closure();
        let release_signoff_ready = !archive_export_summary.needs_release_signoff();
        let release_audit_ready = !archive_export_summary.needs_release_audit();
        let operator_ready = !archive_export_summary.needs_operator_readiness();
        let coordination_ready = !archive_export_summary.needs_coordination();
        let publish_gate_ready = !archive_export_summary.needs_publish_gate();
        let checks = [
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_import_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_import_check_count = checks.len();
        let blocked_archive_import_check_count =
            required_archive_import_check_count - passed_archive_import_check_count;
        let release_archive_import_ready = blocked_archive_import_check_count == 0;

        Self {
            archive_export_summary,
            required_archive_import_check_count,
            passed_archive_import_check_count,
            blocked_archive_import_check_count,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_import_ready,
        }
    }

    pub fn is_release_archive_import_ready(self) -> bool {
        self.release_archive_import_ready
    }

    pub fn has_blocked_archive_import_checks(self) -> bool {
        self.blocked_archive_import_check_count > 0
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_import_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveImportSummary {
    HuePackageReleaseArchiveImportSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveIngestSummary {
    pub archive_import_summary: HuePackageReleaseArchiveImportSummary,
    pub required_archive_ingest_check_count: usize,
    pub passed_archive_ingest_check_count: usize,
    pub blocked_archive_ingest_check_count: usize,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_ingest_ready: bool,
}

impl HuePackageReleaseArchiveIngestSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_import_summary(hue_package_release_archive_import_summary(plan))
    }

    pub fn from_archive_import_summary(
        archive_import_summary: HuePackageReleaseArchiveImportSummary,
    ) -> Self {
        let release_archive_import_ready = archive_import_summary.is_release_archive_import_ready();
        let release_archive_export_ready = !archive_import_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_import_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_import_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_import_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready = !archive_import_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_import_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_import_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_import_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_import_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_import_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_import_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_import_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_import_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_import_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_import_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready = !archive_import_summary.needs_release_archive_handoff();
        let release_archive_closure_ready = !archive_import_summary.needs_release_archive_closure();
        let release_archive_signoff_ready = !archive_import_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_import_summary.needs_release_archive();
        let release_closure_ready = !archive_import_summary.needs_release_closure();
        let release_signoff_ready = !archive_import_summary.needs_release_signoff();
        let release_audit_ready = !archive_import_summary.needs_release_audit();
        let operator_ready = !archive_import_summary.needs_operator_readiness();
        let coordination_ready = !archive_import_summary.needs_coordination();
        let publish_gate_ready = !archive_import_summary.needs_publish_gate();
        let checks = [
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_ingest_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_ingest_check_count = checks.len();
        let blocked_archive_ingest_check_count =
            required_archive_ingest_check_count - passed_archive_ingest_check_count;
        let release_archive_ingest_ready = blocked_archive_ingest_check_count == 0;

        Self {
            archive_import_summary,
            required_archive_ingest_check_count,
            passed_archive_ingest_check_count,
            blocked_archive_ingest_check_count,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_ingest_ready,
        }
    }

    pub fn is_release_archive_ingest_ready(self) -> bool {
        self.release_archive_ingest_ready
    }

    pub fn has_blocked_archive_ingest_checks(self) -> bool {
        self.blocked_archive_ingest_check_count > 0
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_ingest_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveIngestSummary {
    HuePackageReleaseArchiveIngestSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveLoadSummary {
    pub archive_ingest_summary: HuePackageReleaseArchiveIngestSummary,
    pub required_archive_load_check_count: usize,
    pub passed_archive_load_check_count: usize,
    pub blocked_archive_load_check_count: usize,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_load_ready: bool,
}

impl HuePackageReleaseArchiveLoadSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_ingest_summary(hue_package_release_archive_ingest_summary(plan))
    }

    pub fn from_archive_ingest_summary(
        archive_ingest_summary: HuePackageReleaseArchiveIngestSummary,
    ) -> Self {
        let release_archive_ingest_ready = archive_ingest_summary.is_release_archive_ingest_ready();
        let release_archive_import_ready = !archive_ingest_summary.needs_release_archive_import();
        let release_archive_export_ready = !archive_ingest_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_ingest_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_ingest_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_ingest_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready = !archive_ingest_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_ingest_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_ingest_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_ingest_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_ingest_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_ingest_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_ingest_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_ingest_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_ingest_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_ingest_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_ingest_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready = !archive_ingest_summary.needs_release_archive_handoff();
        let release_archive_closure_ready = !archive_ingest_summary.needs_release_archive_closure();
        let release_archive_signoff_ready = !archive_ingest_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_ingest_summary.needs_release_archive();
        let release_closure_ready = !archive_ingest_summary.needs_release_closure();
        let release_signoff_ready = !archive_ingest_summary.needs_release_signoff();
        let release_audit_ready = !archive_ingest_summary.needs_release_audit();
        let operator_ready = !archive_ingest_summary.needs_operator_readiness();
        let coordination_ready = !archive_ingest_summary.needs_coordination();
        let publish_gate_ready = !archive_ingest_summary.needs_publish_gate();
        let checks = [
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_load_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_load_check_count = checks.len();
        let blocked_archive_load_check_count =
            required_archive_load_check_count - passed_archive_load_check_count;
        let release_archive_load_ready = blocked_archive_load_check_count == 0;

        Self {
            archive_ingest_summary,
            required_archive_load_check_count,
            passed_archive_load_check_count,
            blocked_archive_load_check_count,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_load_ready,
        }
    }

    pub fn is_release_archive_load_ready(self) -> bool {
        self.release_archive_load_ready
    }

    pub fn has_blocked_archive_load_checks(self) -> bool {
        self.blocked_archive_load_check_count > 0
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_load_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveLoadSummary {
    HuePackageReleaseArchiveLoadSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveRestoreSummary {
    pub archive_load_summary: HuePackageReleaseArchiveLoadSummary,
    pub required_archive_restore_check_count: usize,
    pub passed_archive_restore_check_count: usize,
    pub blocked_archive_restore_check_count: usize,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_restore_ready: bool,
}

impl HuePackageReleaseArchiveRestoreSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_load_summary(hue_package_release_archive_load_summary(plan))
    }

    pub fn from_archive_load_summary(
        archive_load_summary: HuePackageReleaseArchiveLoadSummary,
    ) -> Self {
        let release_archive_load_ready = archive_load_summary.is_release_archive_load_ready();
        let release_archive_ingest_ready = !archive_load_summary.needs_release_archive_ingest();
        let release_archive_import_ready = !archive_load_summary.needs_release_archive_import();
        let release_archive_export_ready = !archive_load_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_load_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_load_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready = !archive_load_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready = !archive_load_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_load_summary.needs_release_archive_activation();
        let release_archive_approval_ready = !archive_load_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_load_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_load_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_load_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_load_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_load_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_load_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready = !archive_load_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready = !archive_load_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready = !archive_load_summary.needs_release_archive_handoff();
        let release_archive_closure_ready = !archive_load_summary.needs_release_archive_closure();
        let release_archive_signoff_ready = !archive_load_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_load_summary.needs_release_archive();
        let release_closure_ready = !archive_load_summary.needs_release_closure();
        let release_signoff_ready = !archive_load_summary.needs_release_signoff();
        let release_audit_ready = !archive_load_summary.needs_release_audit();
        let operator_ready = !archive_load_summary.needs_operator_readiness();
        let coordination_ready = !archive_load_summary.needs_coordination();
        let publish_gate_ready = !archive_load_summary.needs_publish_gate();
        let checks = [
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_restore_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_restore_check_count = checks.len();
        let blocked_archive_restore_check_count =
            required_archive_restore_check_count - passed_archive_restore_check_count;
        let release_archive_restore_ready = blocked_archive_restore_check_count == 0;

        Self {
            archive_load_summary,
            required_archive_restore_check_count,
            passed_archive_restore_check_count,
            blocked_archive_restore_check_count,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_restore_ready,
        }
    }

    pub fn is_release_archive_restore_ready(self) -> bool {
        self.release_archive_restore_ready
    }

    pub fn has_blocked_archive_restore_checks(self) -> bool {
        self.blocked_archive_restore_check_count > 0
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_restore_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveRestoreSummary {
    HuePackageReleaseArchiveRestoreSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveRecoverySummary {
    pub archive_restore_summary: HuePackageReleaseArchiveRestoreSummary,
    pub required_archive_recovery_check_count: usize,
    pub passed_archive_recovery_check_count: usize,
    pub blocked_archive_recovery_check_count: usize,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_recovery_ready: bool,
}

impl HuePackageReleaseArchiveRecoverySummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_restore_summary(hue_package_release_archive_restore_summary(plan))
    }

    pub fn from_archive_restore_summary(
        archive_restore_summary: HuePackageReleaseArchiveRestoreSummary,
    ) -> Self {
        let release_archive_restore_ready =
            archive_restore_summary.is_release_archive_restore_ready();
        let release_archive_load_ready = !archive_restore_summary.needs_release_archive_load();
        let release_archive_ingest_ready = !archive_restore_summary.needs_release_archive_ingest();
        let release_archive_import_ready = !archive_restore_summary.needs_release_archive_import();
        let release_archive_export_ready = !archive_restore_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_restore_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_restore_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_restore_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_restore_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_restore_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_restore_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_restore_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_restore_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_restore_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_restore_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_restore_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_restore_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_restore_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_restore_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_restore_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_restore_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_restore_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_restore_summary.needs_release_archive();
        let release_closure_ready = !archive_restore_summary.needs_release_closure();
        let release_signoff_ready = !archive_restore_summary.needs_release_signoff();
        let release_audit_ready = !archive_restore_summary.needs_release_audit();
        let operator_ready = !archive_restore_summary.needs_operator_readiness();
        let coordination_ready = !archive_restore_summary.needs_coordination();
        let publish_gate_ready = !archive_restore_summary.needs_publish_gate();
        let checks = [
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_recovery_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_recovery_check_count = checks.len();
        let blocked_archive_recovery_check_count =
            required_archive_recovery_check_count - passed_archive_recovery_check_count;
        let release_archive_recovery_ready = blocked_archive_recovery_check_count == 0;

        Self {
            archive_restore_summary,
            required_archive_recovery_check_count,
            passed_archive_recovery_check_count,
            blocked_archive_recovery_check_count,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_recovery_ready,
        }
    }

    pub fn is_release_archive_recovery_ready(self) -> bool {
        self.release_archive_recovery_ready
    }

    pub fn has_blocked_archive_recovery_checks(self) -> bool {
        self.blocked_archive_recovery_check_count > 0
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_recovery_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveRecoverySummary {
    HuePackageReleaseArchiveRecoverySummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveReplaySummary {
    pub archive_recovery_summary: HuePackageReleaseArchiveRecoverySummary,
    pub required_archive_replay_check_count: usize,
    pub passed_archive_replay_check_count: usize,
    pub blocked_archive_replay_check_count: usize,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_replay_ready: bool,
}

impl HuePackageReleaseArchiveReplaySummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_recovery_summary(hue_package_release_archive_recovery_summary(plan))
    }

    pub fn from_archive_recovery_summary(
        archive_recovery_summary: HuePackageReleaseArchiveRecoverySummary,
    ) -> Self {
        let release_archive_recovery_ready =
            archive_recovery_summary.is_release_archive_recovery_ready();
        let release_archive_restore_ready =
            !archive_recovery_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_recovery_summary.needs_release_archive_load();
        let release_archive_ingest_ready = !archive_recovery_summary.needs_release_archive_ingest();
        let release_archive_import_ready = !archive_recovery_summary.needs_release_archive_import();
        let release_archive_export_ready = !archive_recovery_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_recovery_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_recovery_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_recovery_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_recovery_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_recovery_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_recovery_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_recovery_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_recovery_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_recovery_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_recovery_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_recovery_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_recovery_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_recovery_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_recovery_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_recovery_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_recovery_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_recovery_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_recovery_summary.needs_release_archive();
        let release_closure_ready = !archive_recovery_summary.needs_release_closure();
        let release_signoff_ready = !archive_recovery_summary.needs_release_signoff();
        let release_audit_ready = !archive_recovery_summary.needs_release_audit();
        let operator_ready = !archive_recovery_summary.needs_operator_readiness();
        let coordination_ready = !archive_recovery_summary.needs_coordination();
        let publish_gate_ready = !archive_recovery_summary.needs_publish_gate();
        let checks = [
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_replay_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_replay_check_count = checks.len();
        let blocked_archive_replay_check_count =
            required_archive_replay_check_count - passed_archive_replay_check_count;
        let release_archive_replay_ready = blocked_archive_replay_check_count == 0;

        Self {
            archive_recovery_summary,
            required_archive_replay_check_count,
            passed_archive_replay_check_count,
            blocked_archive_replay_check_count,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_replay_ready,
        }
    }

    pub fn is_release_archive_replay_ready(self) -> bool {
        self.release_archive_replay_ready
    }

    pub fn has_blocked_archive_replay_checks(self) -> bool {
        self.blocked_archive_replay_check_count > 0
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_replay_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveReplaySummary {
    HuePackageReleaseArchiveReplaySummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveReconciliationSummary {
    pub archive_replay_summary: HuePackageReleaseArchiveReplaySummary,
    pub required_archive_reconciliation_check_count: usize,
    pub passed_archive_reconciliation_check_count: usize,
    pub blocked_archive_reconciliation_check_count: usize,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_reconciliation_ready: bool,
}

impl HuePackageReleaseArchiveReconciliationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_replay_summary(hue_package_release_archive_replay_summary(plan))
    }

    pub fn from_archive_replay_summary(
        archive_replay_summary: HuePackageReleaseArchiveReplaySummary,
    ) -> Self {
        let release_archive_replay_ready = archive_replay_summary.is_release_archive_replay_ready();
        let release_archive_recovery_ready =
            !archive_replay_summary.needs_release_archive_recovery();
        let release_archive_restore_ready = !archive_replay_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_replay_summary.needs_release_archive_load();
        let release_archive_ingest_ready = !archive_replay_summary.needs_release_archive_ingest();
        let release_archive_import_ready = !archive_replay_summary.needs_release_archive_import();
        let release_archive_export_ready = !archive_replay_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_replay_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_replay_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_replay_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready = !archive_replay_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_replay_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_replay_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_replay_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_replay_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_replay_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_replay_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_replay_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_replay_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_replay_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_replay_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready = !archive_replay_summary.needs_release_archive_handoff();
        let release_archive_closure_ready = !archive_replay_summary.needs_release_archive_closure();
        let release_archive_signoff_ready = !archive_replay_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_replay_summary.needs_release_archive();
        let release_closure_ready = !archive_replay_summary.needs_release_closure();
        let release_signoff_ready = !archive_replay_summary.needs_release_signoff();
        let release_audit_ready = !archive_replay_summary.needs_release_audit();
        let operator_ready = !archive_replay_summary.needs_operator_readiness();
        let coordination_ready = !archive_replay_summary.needs_coordination();
        let publish_gate_ready = !archive_replay_summary.needs_publish_gate();
        let checks = [
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_reconciliation_check_count =
            checks.iter().filter(|ready| **ready).count();
        let required_archive_reconciliation_check_count = checks.len();
        let blocked_archive_reconciliation_check_count =
            required_archive_reconciliation_check_count - passed_archive_reconciliation_check_count;
        let release_archive_reconciliation_ready = blocked_archive_reconciliation_check_count == 0;

        Self {
            archive_replay_summary,
            required_archive_reconciliation_check_count,
            passed_archive_reconciliation_check_count,
            blocked_archive_reconciliation_check_count,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_reconciliation_ready,
        }
    }

    pub fn is_release_archive_reconciliation_ready(self) -> bool {
        self.release_archive_reconciliation_ready
    }

    pub fn has_blocked_archive_reconciliation_checks(self) -> bool {
        self.blocked_archive_reconciliation_check_count > 0
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_reconciliation_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveReconciliationSummary {
    HuePackageReleaseArchiveReconciliationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveSettlementSummary {
    pub archive_reconciliation_summary: HuePackageReleaseArchiveReconciliationSummary,
    pub required_archive_settlement_check_count: usize,
    pub passed_archive_settlement_check_count: usize,
    pub blocked_archive_settlement_check_count: usize,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_settlement_ready: bool,
}

impl HuePackageReleaseArchiveSettlementSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_reconciliation_summary(
            hue_package_release_archive_reconciliation_summary(plan),
        )
    }

    pub fn from_archive_reconciliation_summary(
        archive_reconciliation_summary: HuePackageReleaseArchiveReconciliationSummary,
    ) -> Self {
        let release_archive_reconciliation_ready =
            archive_reconciliation_summary.is_release_archive_reconciliation_ready();
        let release_archive_replay_ready =
            !archive_reconciliation_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_reconciliation_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_reconciliation_summary.needs_release_archive_restore();
        let release_archive_load_ready =
            !archive_reconciliation_summary.needs_release_archive_load();
        let release_archive_ingest_ready =
            !archive_reconciliation_summary.needs_release_archive_ingest();
        let release_archive_import_ready =
            !archive_reconciliation_summary.needs_release_archive_import();
        let release_archive_export_ready =
            !archive_reconciliation_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_reconciliation_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_reconciliation_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_reconciliation_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_reconciliation_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_reconciliation_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_reconciliation_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_reconciliation_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_reconciliation_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_reconciliation_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_reconciliation_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_reconciliation_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_reconciliation_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_reconciliation_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_reconciliation_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_reconciliation_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_reconciliation_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_reconciliation_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_reconciliation_summary.needs_release_archive();
        let release_closure_ready = !archive_reconciliation_summary.needs_release_closure();
        let release_signoff_ready = !archive_reconciliation_summary.needs_release_signoff();
        let release_audit_ready = !archive_reconciliation_summary.needs_release_audit();
        let operator_ready = !archive_reconciliation_summary.needs_operator_readiness();
        let coordination_ready = !archive_reconciliation_summary.needs_coordination();
        let publish_gate_ready = !archive_reconciliation_summary.needs_publish_gate();
        let checks = [
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_settlement_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_settlement_check_count = checks.len();
        let blocked_archive_settlement_check_count =
            required_archive_settlement_check_count - passed_archive_settlement_check_count;
        let release_archive_settlement_ready = blocked_archive_settlement_check_count == 0;

        Self {
            archive_reconciliation_summary,
            required_archive_settlement_check_count,
            passed_archive_settlement_check_count,
            blocked_archive_settlement_check_count,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_settlement_ready,
        }
    }

    pub fn is_release_archive_settlement_ready(self) -> bool {
        self.release_archive_settlement_ready
    }

    pub fn has_blocked_archive_settlement_checks(self) -> bool {
        self.blocked_archive_settlement_check_count > 0
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_settlement_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveSettlementSummary {
    HuePackageReleaseArchiveSettlementSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveFinalizationSummary {
    pub archive_settlement_summary: HuePackageReleaseArchiveSettlementSummary,
    pub required_archive_finalization_check_count: usize,
    pub passed_archive_finalization_check_count: usize,
    pub blocked_archive_finalization_check_count: usize,
    pub release_archive_settlement_ready: bool,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_finalization_ready: bool,
}

impl HuePackageReleaseArchiveFinalizationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_settlement_summary(hue_package_release_archive_settlement_summary(plan))
    }

    pub fn from_archive_settlement_summary(
        archive_settlement_summary: HuePackageReleaseArchiveSettlementSummary,
    ) -> Self {
        let release_archive_settlement_ready =
            archive_settlement_summary.is_release_archive_settlement_ready();
        let release_archive_reconciliation_ready =
            !archive_settlement_summary.needs_release_archive_reconciliation();
        let release_archive_replay_ready =
            !archive_settlement_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_settlement_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_settlement_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_settlement_summary.needs_release_archive_load();
        let release_archive_ingest_ready =
            !archive_settlement_summary.needs_release_archive_ingest();
        let release_archive_import_ready =
            !archive_settlement_summary.needs_release_archive_import();
        let release_archive_export_ready =
            !archive_settlement_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_settlement_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_settlement_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_settlement_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_settlement_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_settlement_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_settlement_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_settlement_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_settlement_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_settlement_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_settlement_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_settlement_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_settlement_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_settlement_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_settlement_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_settlement_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_settlement_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_settlement_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_settlement_summary.needs_release_archive();
        let release_closure_ready = !archive_settlement_summary.needs_release_closure();
        let release_signoff_ready = !archive_settlement_summary.needs_release_signoff();
        let release_audit_ready = !archive_settlement_summary.needs_release_audit();
        let operator_ready = !archive_settlement_summary.needs_operator_readiness();
        let coordination_ready = !archive_settlement_summary.needs_coordination();
        let publish_gate_ready = !archive_settlement_summary.needs_publish_gate();
        let checks = [
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_finalization_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_finalization_check_count = checks.len();
        let blocked_archive_finalization_check_count =
            required_archive_finalization_check_count - passed_archive_finalization_check_count;
        let release_archive_finalization_ready = blocked_archive_finalization_check_count == 0;

        Self {
            archive_settlement_summary,
            required_archive_finalization_check_count,
            passed_archive_finalization_check_count,
            blocked_archive_finalization_check_count,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_finalization_ready,
        }
    }

    pub fn is_release_archive_finalization_ready(self) -> bool {
        self.release_archive_finalization_ready
    }

    pub fn has_blocked_archive_finalization_checks(self) -> bool {
        self.blocked_archive_finalization_check_count > 0
    }

    pub fn needs_release_archive_settlement(self) -> bool {
        !self.release_archive_settlement_ready
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_finalization_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveFinalizationSummary {
    HuePackageReleaseArchiveFinalizationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveConfirmationSummary {
    pub archive_finalization_summary: HuePackageReleaseArchiveFinalizationSummary,
    pub required_archive_confirmation_check_count: usize,
    pub passed_archive_confirmation_check_count: usize,
    pub blocked_archive_confirmation_check_count: usize,
    pub release_archive_finalization_ready: bool,
    pub release_archive_settlement_ready: bool,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_confirmation_ready: bool,
}

impl HuePackageReleaseArchiveConfirmationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_finalization_summary(hue_package_release_archive_finalization_summary(
            plan,
        ))
    }

    pub fn from_archive_finalization_summary(
        archive_finalization_summary: HuePackageReleaseArchiveFinalizationSummary,
    ) -> Self {
        let release_archive_finalization_ready =
            archive_finalization_summary.is_release_archive_finalization_ready();
        let release_archive_settlement_ready =
            !archive_finalization_summary.needs_release_archive_settlement();
        let release_archive_reconciliation_ready =
            !archive_finalization_summary.needs_release_archive_reconciliation();
        let release_archive_replay_ready =
            !archive_finalization_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_finalization_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_finalization_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_finalization_summary.needs_release_archive_load();
        let release_archive_ingest_ready =
            !archive_finalization_summary.needs_release_archive_ingest();
        let release_archive_import_ready =
            !archive_finalization_summary.needs_release_archive_import();
        let release_archive_export_ready =
            !archive_finalization_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_finalization_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_finalization_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_finalization_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_finalization_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_finalization_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_finalization_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_finalization_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_finalization_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_finalization_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_finalization_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_finalization_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_finalization_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_finalization_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_finalization_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_finalization_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_finalization_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_finalization_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_finalization_summary.needs_release_archive();
        let release_closure_ready = !archive_finalization_summary.needs_release_closure();
        let release_signoff_ready = !archive_finalization_summary.needs_release_signoff();
        let release_audit_ready = !archive_finalization_summary.needs_release_audit();
        let operator_ready = !archive_finalization_summary.needs_operator_readiness();
        let coordination_ready = !archive_finalization_summary.needs_coordination();
        let publish_gate_ready = !archive_finalization_summary.needs_publish_gate();
        let checks = [
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_confirmation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_confirmation_check_count = checks.len();
        let blocked_archive_confirmation_check_count =
            required_archive_confirmation_check_count - passed_archive_confirmation_check_count;
        let release_archive_confirmation_ready = blocked_archive_confirmation_check_count == 0;

        Self {
            archive_finalization_summary,
            required_archive_confirmation_check_count,
            passed_archive_confirmation_check_count,
            blocked_archive_confirmation_check_count,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_confirmation_ready,
        }
    }

    pub fn is_release_archive_confirmation_ready(self) -> bool {
        self.release_archive_confirmation_ready
    }

    pub fn has_blocked_archive_confirmation_checks(self) -> bool {
        self.blocked_archive_confirmation_check_count > 0
    }

    pub fn needs_release_archive_finalization(self) -> bool {
        !self.release_archive_finalization_ready
    }

    pub fn needs_release_archive_settlement(self) -> bool {
        !self.release_archive_settlement_ready
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_confirmation_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveConfirmationSummary {
    HuePackageReleaseArchiveConfirmationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveAttestationSummary {
    pub archive_confirmation_summary: HuePackageReleaseArchiveConfirmationSummary,
    pub required_archive_attestation_check_count: usize,
    pub passed_archive_attestation_check_count: usize,
    pub blocked_archive_attestation_check_count: usize,
    pub release_archive_confirmation_ready: bool,
    pub release_archive_finalization_ready: bool,
    pub release_archive_settlement_ready: bool,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_attestation_ready: bool,
}

impl HuePackageReleaseArchiveAttestationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_confirmation_summary(hue_package_release_archive_confirmation_summary(
            plan,
        ))
    }

    pub fn from_archive_confirmation_summary(
        archive_confirmation_summary: HuePackageReleaseArchiveConfirmationSummary,
    ) -> Self {
        let release_archive_confirmation_ready =
            archive_confirmation_summary.is_release_archive_confirmation_ready();
        let release_archive_finalization_ready =
            !archive_confirmation_summary.needs_release_archive_finalization();
        let release_archive_settlement_ready =
            !archive_confirmation_summary.needs_release_archive_settlement();
        let release_archive_reconciliation_ready =
            !archive_confirmation_summary.needs_release_archive_reconciliation();
        let release_archive_replay_ready =
            !archive_confirmation_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_confirmation_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_confirmation_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_confirmation_summary.needs_release_archive_load();
        let release_archive_ingest_ready =
            !archive_confirmation_summary.needs_release_archive_ingest();
        let release_archive_import_ready =
            !archive_confirmation_summary.needs_release_archive_import();
        let release_archive_export_ready =
            !archive_confirmation_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_confirmation_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_confirmation_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_confirmation_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_confirmation_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_confirmation_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_confirmation_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_confirmation_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_confirmation_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_confirmation_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_confirmation_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_confirmation_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_confirmation_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_confirmation_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_confirmation_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_confirmation_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_confirmation_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_confirmation_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_confirmation_summary.needs_release_archive();
        let release_closure_ready = !archive_confirmation_summary.needs_release_closure();
        let release_signoff_ready = !archive_confirmation_summary.needs_release_signoff();
        let release_audit_ready = !archive_confirmation_summary.needs_release_audit();
        let operator_ready = !archive_confirmation_summary.needs_operator_readiness();
        let coordination_ready = !archive_confirmation_summary.needs_coordination();
        let publish_gate_ready = !archive_confirmation_summary.needs_publish_gate();
        let checks = [
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_attestation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_attestation_check_count = checks.len();
        let blocked_archive_attestation_check_count =
            required_archive_attestation_check_count - passed_archive_attestation_check_count;
        let release_archive_attestation_ready = blocked_archive_attestation_check_count == 0;

        Self {
            archive_confirmation_summary,
            required_archive_attestation_check_count,
            passed_archive_attestation_check_count,
            blocked_archive_attestation_check_count,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_attestation_ready,
        }
    }

    pub fn is_release_archive_attestation_ready(self) -> bool {
        self.release_archive_attestation_ready
    }

    pub fn has_blocked_archive_attestation_checks(self) -> bool {
        self.blocked_archive_attestation_check_count > 0
    }

    pub fn needs_release_archive_confirmation(self) -> bool {
        !self.release_archive_confirmation_ready
    }

    pub fn needs_release_archive_finalization(self) -> bool {
        !self.release_archive_finalization_ready
    }

    pub fn needs_release_archive_settlement(self) -> bool {
        !self.release_archive_settlement_ready
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_attestation_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveAttestationSummary {
    HuePackageReleaseArchiveAttestationSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveEvidenceSummary {
    pub archive_attestation_summary: HuePackageReleaseArchiveAttestationSummary,
    pub required_archive_evidence_check_count: usize,
    pub passed_archive_evidence_check_count: usize,
    pub blocked_archive_evidence_check_count: usize,
    pub release_archive_attestation_ready: bool,
    pub release_archive_confirmation_ready: bool,
    pub release_archive_finalization_ready: bool,
    pub release_archive_settlement_ready: bool,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_evidence_ready: bool,
}

impl HuePackageReleaseArchiveEvidenceSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_attestation_summary(hue_package_release_archive_attestation_summary(
            plan,
        ))
    }

    pub fn from_archive_attestation_summary(
        archive_attestation_summary: HuePackageReleaseArchiveAttestationSummary,
    ) -> Self {
        let release_archive_attestation_ready =
            archive_attestation_summary.is_release_archive_attestation_ready();
        let release_archive_confirmation_ready =
            !archive_attestation_summary.needs_release_archive_confirmation();
        let release_archive_finalization_ready =
            !archive_attestation_summary.needs_release_archive_finalization();
        let release_archive_settlement_ready =
            !archive_attestation_summary.needs_release_archive_settlement();
        let release_archive_reconciliation_ready =
            !archive_attestation_summary.needs_release_archive_reconciliation();
        let release_archive_replay_ready =
            !archive_attestation_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_attestation_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_attestation_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_attestation_summary.needs_release_archive_load();
        let release_archive_ingest_ready =
            !archive_attestation_summary.needs_release_archive_ingest();
        let release_archive_import_ready =
            !archive_attestation_summary.needs_release_archive_import();
        let release_archive_export_ready =
            !archive_attestation_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_attestation_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_attestation_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_attestation_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_attestation_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_attestation_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_attestation_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_attestation_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_attestation_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_attestation_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_attestation_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_attestation_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_attestation_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_attestation_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_attestation_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_attestation_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_attestation_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_attestation_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_attestation_summary.needs_release_archive();
        let release_closure_ready = !archive_attestation_summary.needs_release_closure();
        let release_signoff_ready = !archive_attestation_summary.needs_release_signoff();
        let release_audit_ready = !archive_attestation_summary.needs_release_audit();
        let operator_ready = !archive_attestation_summary.needs_operator_readiness();
        let coordination_ready = !archive_attestation_summary.needs_coordination();
        let publish_gate_ready = !archive_attestation_summary.needs_publish_gate();
        let checks = [
            release_archive_attestation_ready,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_evidence_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_evidence_check_count = checks.len();
        let blocked_archive_evidence_check_count =
            required_archive_evidence_check_count - passed_archive_evidence_check_count;
        let release_archive_evidence_ready = blocked_archive_evidence_check_count == 0;

        Self {
            archive_attestation_summary,
            required_archive_evidence_check_count,
            passed_archive_evidence_check_count,
            blocked_archive_evidence_check_count,
            release_archive_attestation_ready,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_evidence_ready,
        }
    }

    pub fn is_release_archive_evidence_ready(self) -> bool {
        self.release_archive_evidence_ready
    }

    pub fn has_blocked_archive_evidence_checks(self) -> bool {
        self.blocked_archive_evidence_check_count > 0
    }

    pub fn needs_release_archive_attestation(self) -> bool {
        !self.release_archive_attestation_ready
    }

    pub fn needs_release_archive_confirmation(self) -> bool {
        !self.release_archive_confirmation_ready
    }

    pub fn needs_release_archive_finalization(self) -> bool {
        !self.release_archive_finalization_ready
    }

    pub fn needs_release_archive_settlement(self) -> bool {
        !self.release_archive_settlement_ready
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_evidence_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveEvidenceSummary {
    HuePackageReleaseArchiveEvidenceSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveEvidenceLedgerSummary {
    pub archive_evidence_summary: HuePackageReleaseArchiveEvidenceSummary,
    pub required_archive_evidence_ledger_check_count: usize,
    pub passed_archive_evidence_ledger_check_count: usize,
    pub blocked_archive_evidence_ledger_check_count: usize,
    pub release_archive_evidence_ready: bool,
    pub release_archive_attestation_ready: bool,
    pub release_archive_confirmation_ready: bool,
    pub release_archive_finalization_ready: bool,
    pub release_archive_settlement_ready: bool,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_archive_evidence_ledger_ready: bool,
}

impl HuePackageReleaseArchiveEvidenceLedgerSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_archive_evidence_summary(hue_package_release_archive_evidence_summary(plan))
    }

    pub fn from_archive_evidence_summary(
        archive_evidence_summary: HuePackageReleaseArchiveEvidenceSummary,
    ) -> Self {
        let release_archive_evidence_ready =
            archive_evidence_summary.is_release_archive_evidence_ready();
        let release_archive_attestation_ready =
            !archive_evidence_summary.needs_release_archive_attestation();
        let release_archive_confirmation_ready =
            !archive_evidence_summary.needs_release_archive_confirmation();
        let release_archive_finalization_ready =
            !archive_evidence_summary.needs_release_archive_finalization();
        let release_archive_settlement_ready =
            !archive_evidence_summary.needs_release_archive_settlement();
        let release_archive_reconciliation_ready =
            !archive_evidence_summary.needs_release_archive_reconciliation();
        let release_archive_replay_ready = !archive_evidence_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_evidence_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_evidence_summary.needs_release_archive_restore();
        let release_archive_load_ready = !archive_evidence_summary.needs_release_archive_load();
        let release_archive_ingest_ready = !archive_evidence_summary.needs_release_archive_ingest();
        let release_archive_import_ready = !archive_evidence_summary.needs_release_archive_import();
        let release_archive_export_ready = !archive_evidence_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_evidence_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_evidence_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_evidence_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_evidence_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_evidence_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_evidence_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_evidence_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_evidence_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_evidence_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_evidence_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_evidence_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_evidence_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_evidence_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_evidence_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_evidence_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_evidence_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_evidence_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_evidence_summary.needs_release_archive();
        let release_closure_ready = !archive_evidence_summary.needs_release_closure();
        let release_signoff_ready = !archive_evidence_summary.needs_release_signoff();
        let release_audit_ready = !archive_evidence_summary.needs_release_audit();
        let operator_ready = !archive_evidence_summary.needs_operator_readiness();
        let coordination_ready = !archive_evidence_summary.needs_coordination();
        let publish_gate_ready = !archive_evidence_summary.needs_publish_gate();
        let checks = [
            release_archive_evidence_ready,
            release_archive_attestation_ready,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_archive_evidence_ledger_check_count =
            checks.iter().filter(|ready| **ready).count();
        let required_archive_evidence_ledger_check_count = checks.len();
        let blocked_archive_evidence_ledger_check_count =
            required_archive_evidence_ledger_check_count
                - passed_archive_evidence_ledger_check_count;
        let release_archive_evidence_ledger_ready =
            blocked_archive_evidence_ledger_check_count == 0;

        Self {
            archive_evidence_summary,
            required_archive_evidence_ledger_check_count,
            passed_archive_evidence_ledger_check_count,
            blocked_archive_evidence_ledger_check_count,
            release_archive_evidence_ready,
            release_archive_attestation_ready,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_archive_evidence_ledger_ready,
        }
    }

    pub fn is_release_archive_evidence_ledger_ready(self) -> bool {
        self.release_archive_evidence_ledger_ready
    }

    pub fn has_blocked_archive_evidence_ledger_checks(self) -> bool {
        self.blocked_archive_evidence_ledger_check_count > 0
    }

    pub fn needs_release_archive_evidence(self) -> bool {
        !self.release_archive_evidence_ready
    }

    pub fn needs_release_archive_attestation(self) -> bool {
        !self.release_archive_attestation_ready
    }

    pub fn needs_release_archive_confirmation(self) -> bool {
        !self.release_archive_confirmation_ready
    }

    pub fn needs_release_archive_finalization(self) -> bool {
        !self.release_archive_finalization_ready
    }

    pub fn needs_release_archive_settlement(self) -> bool {
        !self.release_archive_settlement_ready
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_archive_evidence_ledger_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveEvidenceLedgerSummary {
    HuePackageReleaseArchiveEvidenceLedgerSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseReadinessEvidenceSummary {
    pub release_readiness_summary: HuePackageReleaseReadinessSummary,
    pub archive_evidence_ledger_summary: HuePackageReleaseArchiveEvidenceLedgerSummary,
    pub required_release_readiness_evidence_check_count: usize,
    pub passed_release_readiness_evidence_check_count: usize,
    pub blocked_release_readiness_evidence_check_count: usize,
    pub worker_process_ready: bool,
    pub command_flow_ready: bool,
    pub pairing_flow_ready: bool,
    pub event_stream_ready: bool,
    pub physical_presence_required: bool,
    pub package_release_ready: bool,
    pub release_archive_evidence_ledger_ready: bool,
    pub release_archive_evidence_ready: bool,
    pub release_archive_attestation_ready: bool,
    pub release_archive_confirmation_ready: bool,
    pub release_archive_finalization_ready: bool,
    pub release_archive_settlement_ready: bool,
    pub release_archive_reconciliation_ready: bool,
    pub release_archive_replay_ready: bool,
    pub release_archive_recovery_ready: bool,
    pub release_archive_restore_ready: bool,
    pub release_archive_load_ready: bool,
    pub release_archive_ingest_ready: bool,
    pub release_archive_import_ready: bool,
    pub release_archive_export_ready: bool,
    pub release_archive_distribution_ready: bool,
    pub release_archive_acceptance_ready: bool,
    pub release_archive_adoption_ready: bool,
    pub release_archive_rollout_ready: bool,
    pub release_archive_activation_ready: bool,
    pub release_archive_approval_ready: bool,
    pub release_archive_certification_ready: bool,
    pub release_archive_validation_ready: bool,
    pub release_archive_verification_ready: bool,
    pub release_archive_publication_ready: bool,
    pub release_archive_completion_ready: bool,
    pub release_archive_supervisor_ready: bool,
    pub release_archive_operator_ready: bool,
    pub release_archive_dispatch_ready: bool,
    pub release_archive_handoff_ready: bool,
    pub release_archive_closure_ready: bool,
    pub release_archive_signoff_ready: bool,
    pub release_archive_ready: bool,
    pub release_closure_ready: bool,
    pub release_signoff_ready: bool,
    pub release_audit_ready: bool,
    pub operator_ready: bool,
    pub coordination_ready: bool,
    pub publish_gate_ready: bool,
    pub release_readiness_evidence_ready: bool,
}

impl HuePackageReleaseReadinessEvidenceSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_summaries(
            hue_package_release_readiness_summary(plan),
            hue_package_release_archive_evidence_ledger_summary(plan),
        )
    }

    pub fn from_summaries(
        release_readiness_summary: HuePackageReleaseReadinessSummary,
        archive_evidence_ledger_summary: HuePackageReleaseArchiveEvidenceLedgerSummary,
    ) -> Self {
        let worker_process_ready = release_readiness_summary.worker_process_ready;
        let command_flow_ready = release_readiness_summary.command_flow_ready;
        let pairing_flow_ready = release_readiness_summary.pairing_flow_ready;
        let event_stream_ready = release_readiness_summary.event_stream_ready;
        let physical_presence_required = release_readiness_summary.physical_presence_required;
        let package_release_ready = release_readiness_summary.is_release_ready();
        let release_archive_evidence_ledger_ready =
            archive_evidence_ledger_summary.is_release_archive_evidence_ledger_ready();
        let release_archive_evidence_ready =
            !archive_evidence_ledger_summary.needs_release_archive_evidence();
        let release_archive_attestation_ready =
            !archive_evidence_ledger_summary.needs_release_archive_attestation();
        let release_archive_confirmation_ready =
            !archive_evidence_ledger_summary.needs_release_archive_confirmation();
        let release_archive_finalization_ready =
            !archive_evidence_ledger_summary.needs_release_archive_finalization();
        let release_archive_settlement_ready =
            !archive_evidence_ledger_summary.needs_release_archive_settlement();
        let release_archive_reconciliation_ready =
            !archive_evidence_ledger_summary.needs_release_archive_reconciliation();
        let release_archive_replay_ready =
            !archive_evidence_ledger_summary.needs_release_archive_replay();
        let release_archive_recovery_ready =
            !archive_evidence_ledger_summary.needs_release_archive_recovery();
        let release_archive_restore_ready =
            !archive_evidence_ledger_summary.needs_release_archive_restore();
        let release_archive_load_ready =
            !archive_evidence_ledger_summary.needs_release_archive_load();
        let release_archive_ingest_ready =
            !archive_evidence_ledger_summary.needs_release_archive_ingest();
        let release_archive_import_ready =
            !archive_evidence_ledger_summary.needs_release_archive_import();
        let release_archive_export_ready =
            !archive_evidence_ledger_summary.needs_release_archive_export();
        let release_archive_distribution_ready =
            !archive_evidence_ledger_summary.needs_release_archive_distribution();
        let release_archive_acceptance_ready =
            !archive_evidence_ledger_summary.needs_release_archive_acceptance();
        let release_archive_adoption_ready =
            !archive_evidence_ledger_summary.needs_release_archive_adoption();
        let release_archive_rollout_ready =
            !archive_evidence_ledger_summary.needs_release_archive_rollout();
        let release_archive_activation_ready =
            !archive_evidence_ledger_summary.needs_release_archive_activation();
        let release_archive_approval_ready =
            !archive_evidence_ledger_summary.needs_release_archive_approval();
        let release_archive_certification_ready =
            !archive_evidence_ledger_summary.needs_release_archive_certification();
        let release_archive_validation_ready =
            !archive_evidence_ledger_summary.needs_release_archive_validation();
        let release_archive_verification_ready =
            !archive_evidence_ledger_summary.needs_release_archive_verification();
        let release_archive_publication_ready =
            !archive_evidence_ledger_summary.needs_release_archive_publication();
        let release_archive_completion_ready =
            !archive_evidence_ledger_summary.needs_release_archive_completion();
        let release_archive_supervisor_ready =
            !archive_evidence_ledger_summary.needs_release_archive_supervisor();
        let release_archive_operator_ready =
            !archive_evidence_ledger_summary.needs_release_archive_operator();
        let release_archive_dispatch_ready =
            !archive_evidence_ledger_summary.needs_release_archive_dispatch();
        let release_archive_handoff_ready =
            !archive_evidence_ledger_summary.needs_release_archive_handoff();
        let release_archive_closure_ready =
            !archive_evidence_ledger_summary.needs_release_archive_closure();
        let release_archive_signoff_ready =
            !archive_evidence_ledger_summary.needs_release_archive_signoff();
        let release_archive_ready = !archive_evidence_ledger_summary.needs_release_archive();
        let release_closure_ready = !archive_evidence_ledger_summary.needs_release_closure();
        let release_signoff_ready = !archive_evidence_ledger_summary.needs_release_signoff();
        let release_audit_ready = !archive_evidence_ledger_summary.needs_release_audit();
        let operator_ready = !archive_evidence_ledger_summary.needs_operator_readiness();
        let coordination_ready = !archive_evidence_ledger_summary.needs_coordination();
        let publish_gate_ready = !archive_evidence_ledger_summary.needs_publish_gate();
        let checks = [
            worker_process_ready,
            command_flow_ready,
            pairing_flow_ready,
            event_stream_ready,
            physical_presence_required,
            package_release_ready,
            release_archive_evidence_ledger_ready,
            release_archive_evidence_ready,
            release_archive_attestation_ready,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
        ];
        let passed_release_readiness_evidence_check_count =
            checks.iter().filter(|ready| **ready).count();
        let required_release_readiness_evidence_check_count = checks.len();
        let blocked_release_readiness_evidence_check_count =
            required_release_readiness_evidence_check_count
                - passed_release_readiness_evidence_check_count;
        let release_readiness_evidence_ready = blocked_release_readiness_evidence_check_count == 0;

        Self {
            release_readiness_summary,
            archive_evidence_ledger_summary,
            required_release_readiness_evidence_check_count,
            passed_release_readiness_evidence_check_count,
            blocked_release_readiness_evidence_check_count,
            worker_process_ready,
            command_flow_ready,
            pairing_flow_ready,
            event_stream_ready,
            physical_presence_required,
            package_release_ready,
            release_archive_evidence_ledger_ready,
            release_archive_evidence_ready,
            release_archive_attestation_ready,
            release_archive_confirmation_ready,
            release_archive_finalization_ready,
            release_archive_settlement_ready,
            release_archive_reconciliation_ready,
            release_archive_replay_ready,
            release_archive_recovery_ready,
            release_archive_restore_ready,
            release_archive_load_ready,
            release_archive_ingest_ready,
            release_archive_import_ready,
            release_archive_export_ready,
            release_archive_distribution_ready,
            release_archive_acceptance_ready,
            release_archive_adoption_ready,
            release_archive_rollout_ready,
            release_archive_activation_ready,
            release_archive_approval_ready,
            release_archive_certification_ready,
            release_archive_validation_ready,
            release_archive_verification_ready,
            release_archive_publication_ready,
            release_archive_completion_ready,
            release_archive_supervisor_ready,
            release_archive_operator_ready,
            release_archive_dispatch_ready,
            release_archive_handoff_ready,
            release_archive_closure_ready,
            release_archive_signoff_ready,
            release_archive_ready,
            release_closure_ready,
            release_signoff_ready,
            release_audit_ready,
            operator_ready,
            coordination_ready,
            publish_gate_ready,
            release_readiness_evidence_ready,
        }
    }

    pub fn is_release_readiness_evidence_ready(self) -> bool {
        self.release_readiness_evidence_ready
    }

    pub fn has_blocked_release_readiness_evidence_checks(self) -> bool {
        self.blocked_release_readiness_evidence_check_count > 0
    }

    pub fn needs_worker_process(self) -> bool {
        !self.worker_process_ready
    }

    pub fn needs_command_flow(self) -> bool {
        !self.command_flow_ready
    }

    pub fn needs_pairing_flow(self) -> bool {
        !self.pairing_flow_ready
    }

    pub fn needs_event_stream(self) -> bool {
        !self.event_stream_ready
    }

    pub fn needs_physical_presence_requirement(self) -> bool {
        !self.physical_presence_required
    }

    pub fn needs_package_release(self) -> bool {
        !self.package_release_ready
    }

    pub fn needs_release_archive_evidence_ledger(self) -> bool {
        !self.release_archive_evidence_ledger_ready
    }

    pub fn needs_release_archive_evidence(self) -> bool {
        !self.release_archive_evidence_ready
    }

    pub fn needs_release_archive_attestation(self) -> bool {
        !self.release_archive_attestation_ready
    }

    pub fn needs_release_archive_confirmation(self) -> bool {
        !self.release_archive_confirmation_ready
    }

    pub fn needs_release_archive_finalization(self) -> bool {
        !self.release_archive_finalization_ready
    }

    pub fn needs_release_archive_settlement(self) -> bool {
        !self.release_archive_settlement_ready
    }

    pub fn needs_release_archive_reconciliation(self) -> bool {
        !self.release_archive_reconciliation_ready
    }

    pub fn needs_release_archive_replay(self) -> bool {
        !self.release_archive_replay_ready
    }

    pub fn needs_release_archive_recovery(self) -> bool {
        !self.release_archive_recovery_ready
    }

    pub fn needs_release_archive_restore(self) -> bool {
        !self.release_archive_restore_ready
    }

    pub fn needs_release_archive_load(self) -> bool {
        !self.release_archive_load_ready
    }

    pub fn needs_release_archive_ingest(self) -> bool {
        !self.release_archive_ingest_ready
    }

    pub fn needs_release_archive_import(self) -> bool {
        !self.release_archive_import_ready
    }

    pub fn needs_release_archive_export(self) -> bool {
        !self.release_archive_export_ready
    }

    pub fn needs_release_archive_distribution(self) -> bool {
        !self.release_archive_distribution_ready
    }

    pub fn needs_release_archive_acceptance(self) -> bool {
        !self.release_archive_acceptance_ready
    }

    pub fn needs_release_archive_adoption(self) -> bool {
        !self.release_archive_adoption_ready
    }

    pub fn needs_release_archive_rollout(self) -> bool {
        !self.release_archive_rollout_ready
    }

    pub fn needs_release_archive_activation(self) -> bool {
        !self.release_archive_activation_ready
    }

    pub fn needs_release_archive_approval(self) -> bool {
        !self.release_archive_approval_ready
    }

    pub fn needs_release_archive_certification(self) -> bool {
        !self.release_archive_certification_ready
    }

    pub fn needs_release_archive_validation(self) -> bool {
        !self.release_archive_validation_ready
    }

    pub fn needs_release_archive_verification(self) -> bool {
        !self.release_archive_verification_ready
    }

    pub fn needs_release_archive_publication(self) -> bool {
        !self.release_archive_publication_ready
    }

    pub fn needs_release_archive_completion(self) -> bool {
        !self.release_archive_completion_ready
    }

    pub fn needs_release_archive_supervisor(self) -> bool {
        !self.release_archive_supervisor_ready
    }

    pub fn needs_release_archive_operator(self) -> bool {
        !self.release_archive_operator_ready
    }

    pub fn needs_release_archive_dispatch(self) -> bool {
        !self.release_archive_dispatch_ready
    }

    pub fn needs_release_archive_handoff(self) -> bool {
        !self.release_archive_handoff_ready
    }

    pub fn needs_release_archive_closure(self) -> bool {
        !self.release_archive_closure_ready
    }

    pub fn needs_release_archive_signoff(self) -> bool {
        !self.release_archive_signoff_ready
    }

    pub fn needs_release_archive(self) -> bool {
        !self.release_archive_ready
    }

    pub fn needs_release_closure(self) -> bool {
        !self.release_closure_ready
    }

    pub fn needs_release_signoff(self) -> bool {
        !self.release_signoff_ready
    }

    pub fn needs_release_audit(self) -> bool {
        !self.release_audit_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_coordination(self) -> bool {
        !self.coordination_ready
    }

    pub fn needs_publish_gate(self) -> bool {
        !self.publish_gate_ready
    }
}

pub fn hue_package_release_readiness_evidence_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseReadinessEvidenceSummary {
    HuePackageReleaseReadinessEvidenceSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseEvidenceIndexSummary {
    pub release_readiness_evidence_summary: HuePackageReleaseReadinessEvidenceSummary,
    pub required_release_evidence_index_check_count: usize,
    pub passed_release_evidence_index_check_count: usize,
    pub blocked_release_evidence_index_check_count: usize,
    pub indexed_release_readiness_evidence_check_count: usize,
    pub indexed_archive_evidence_ledger_check_count: usize,
    pub blocked_indexed_release_evidence_check_count: usize,
    pub blocked_indexed_archive_evidence_ledger_check_count: usize,
    pub release_readiness_evidence_ready: bool,
    pub runtime_evidence_ready: bool,
    pub archive_evidence_ready: bool,
    pub release_closeout_ready: bool,
    pub operations_ready: bool,
    pub release_evidence_index_ready: bool,
}

impl HuePackageReleaseEvidenceIndexSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_release_readiness_evidence_summary(
            hue_package_release_readiness_evidence_summary(plan),
        )
    }

    pub fn from_release_readiness_evidence_summary(
        release_readiness_evidence_summary: HuePackageReleaseReadinessEvidenceSummary,
    ) -> Self {
        let release_readiness_evidence_ready =
            release_readiness_evidence_summary.is_release_readiness_evidence_ready();
        let runtime_evidence_ready = release_readiness_evidence_summary.worker_process_ready
            && release_readiness_evidence_summary.command_flow_ready
            && release_readiness_evidence_summary.pairing_flow_ready
            && release_readiness_evidence_summary.event_stream_ready
            && release_readiness_evidence_summary.physical_presence_required
            && release_readiness_evidence_summary.package_release_ready;
        let archive_evidence_ready = release_readiness_evidence_summary
            .release_archive_evidence_ledger_ready
            && release_readiness_evidence_summary.release_archive_evidence_ready
            && release_readiness_evidence_summary.release_archive_attestation_ready
            && release_readiness_evidence_summary.release_archive_confirmation_ready
            && release_readiness_evidence_summary.release_archive_finalization_ready
            && release_readiness_evidence_summary.release_archive_settlement_ready
            && release_readiness_evidence_summary.release_archive_reconciliation_ready
            && release_readiness_evidence_summary.release_archive_replay_ready
            && release_readiness_evidence_summary.release_archive_recovery_ready
            && release_readiness_evidence_summary.release_archive_restore_ready
            && release_readiness_evidence_summary.release_archive_load_ready
            && release_readiness_evidence_summary.release_archive_ingest_ready
            && release_readiness_evidence_summary.release_archive_import_ready
            && release_readiness_evidence_summary.release_archive_export_ready
            && release_readiness_evidence_summary.release_archive_distribution_ready
            && release_readiness_evidence_summary.release_archive_acceptance_ready
            && release_readiness_evidence_summary.release_archive_adoption_ready
            && release_readiness_evidence_summary.release_archive_rollout_ready
            && release_readiness_evidence_summary.release_archive_activation_ready
            && release_readiness_evidence_summary.release_archive_approval_ready
            && release_readiness_evidence_summary.release_archive_certification_ready
            && release_readiness_evidence_summary.release_archive_validation_ready
            && release_readiness_evidence_summary.release_archive_verification_ready
            && release_readiness_evidence_summary.release_archive_publication_ready
            && release_readiness_evidence_summary.release_archive_completion_ready
            && release_readiness_evidence_summary.release_archive_supervisor_ready
            && release_readiness_evidence_summary.release_archive_operator_ready
            && release_readiness_evidence_summary.release_archive_dispatch_ready
            && release_readiness_evidence_summary.release_archive_handoff_ready
            && release_readiness_evidence_summary.release_archive_closure_ready
            && release_readiness_evidence_summary.release_archive_signoff_ready
            && release_readiness_evidence_summary.release_archive_ready;
        let release_closeout_ready = release_readiness_evidence_summary.release_closure_ready
            && release_readiness_evidence_summary.release_signoff_ready
            && release_readiness_evidence_summary.release_audit_ready;
        let operations_ready = release_readiness_evidence_summary.operator_ready
            && release_readiness_evidence_summary.coordination_ready
            && release_readiness_evidence_summary.publish_gate_ready;
        let checks = [
            release_readiness_evidence_ready,
            runtime_evidence_ready,
            archive_evidence_ready,
            release_closeout_ready,
            operations_ready,
        ];
        let passed_release_evidence_index_check_count =
            checks.iter().filter(|ready| **ready).count();
        let required_release_evidence_index_check_count = checks.len();
        let blocked_release_evidence_index_check_count =
            required_release_evidence_index_check_count - passed_release_evidence_index_check_count;
        let release_evidence_index_ready = blocked_release_evidence_index_check_count == 0;
        let indexed_release_readiness_evidence_check_count =
            release_readiness_evidence_summary.required_release_readiness_evidence_check_count;
        let indexed_archive_evidence_ledger_check_count = release_readiness_evidence_summary
            .archive_evidence_ledger_summary
            .required_archive_evidence_ledger_check_count;
        let blocked_indexed_release_evidence_check_count =
            release_readiness_evidence_summary.blocked_release_readiness_evidence_check_count;
        let blocked_indexed_archive_evidence_ledger_check_count =
            release_readiness_evidence_summary
                .archive_evidence_ledger_summary
                .blocked_archive_evidence_ledger_check_count;

        Self {
            release_readiness_evidence_summary,
            required_release_evidence_index_check_count,
            passed_release_evidence_index_check_count,
            blocked_release_evidence_index_check_count,
            indexed_release_readiness_evidence_check_count,
            indexed_archive_evidence_ledger_check_count,
            blocked_indexed_release_evidence_check_count,
            blocked_indexed_archive_evidence_ledger_check_count,
            release_readiness_evidence_ready,
            runtime_evidence_ready,
            archive_evidence_ready,
            release_closeout_ready,
            operations_ready,
            release_evidence_index_ready,
        }
    }

    pub fn is_release_evidence_index_ready(self) -> bool {
        self.release_evidence_index_ready
    }

    pub fn has_blocked_release_evidence_index_checks(self) -> bool {
        self.blocked_release_evidence_index_check_count > 0
    }

    pub fn has_blocked_indexed_release_evidence_checks(self) -> bool {
        self.blocked_indexed_release_evidence_check_count > 0
    }

    pub fn has_blocked_indexed_archive_evidence_ledger_checks(self) -> bool {
        self.blocked_indexed_archive_evidence_ledger_check_count > 0
    }

    pub fn needs_release_readiness_evidence(self) -> bool {
        !self.release_readiness_evidence_ready
    }

    pub fn needs_runtime_evidence(self) -> bool {
        !self.runtime_evidence_ready
    }

    pub fn needs_archive_evidence(self) -> bool {
        !self.archive_evidence_ready
    }

    pub fn needs_release_closeout(self) -> bool {
        !self.release_closeout_ready
    }

    pub fn needs_operations(self) -> bool {
        !self.operations_ready
    }
}

pub fn hue_package_release_evidence_index_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseEvidenceIndexSummary {
    HuePackageReleaseEvidenceIndexSummary::from_pairing_plan(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuePackageReleaseArchiveNotarizationSummary {
    pub release_evidence_index_summary: HuePackageReleaseEvidenceIndexSummary,
    pub required_archive_notarization_check_count: usize,
    pub passed_archive_notarization_check_count: usize,
    pub blocked_archive_notarization_check_count: usize,
    pub release_readiness_evidence_ready: bool,
    pub runtime_evidence_ready: bool,
    pub archive_evidence_ready: bool,
    pub release_closeout_ready: bool,
    pub operations_ready: bool,
    pub release_evidence_index_ready: bool,
    pub release_archive_notarization_ready: bool,
}

impl HuePackageReleaseArchiveNotarizationSummary {
    pub fn from_pairing_plan(plan: &HueBridgePairingPlan) -> Self {
        Self::from_release_evidence_index_summary(hue_package_release_evidence_index_summary(plan))
    }

    pub fn from_release_evidence_index_summary(
        release_evidence_index_summary: HuePackageReleaseEvidenceIndexSummary,
    ) -> Self {
        let release_readiness_evidence_ready =
            release_evidence_index_summary.release_readiness_evidence_ready;
        let runtime_evidence_ready = release_evidence_index_summary.runtime_evidence_ready;
        let archive_evidence_ready = release_evidence_index_summary.archive_evidence_ready;
        let release_closeout_ready = release_evidence_index_summary.release_closeout_ready;
        let operations_ready = release_evidence_index_summary.operations_ready;
        let release_evidence_index_ready =
            release_evidence_index_summary.is_release_evidence_index_ready();
        let checks = [
            release_readiness_evidence_ready,
            runtime_evidence_ready,
            archive_evidence_ready,
            release_closeout_ready,
            operations_ready,
            release_evidence_index_ready,
        ];
        let passed_archive_notarization_check_count = checks.iter().filter(|ready| **ready).count();
        let required_archive_notarization_check_count = checks.len();
        let blocked_archive_notarization_check_count =
            required_archive_notarization_check_count - passed_archive_notarization_check_count;
        let release_archive_notarization_ready = blocked_archive_notarization_check_count == 0;

        Self {
            release_evidence_index_summary,
            required_archive_notarization_check_count,
            passed_archive_notarization_check_count,
            blocked_archive_notarization_check_count,
            release_readiness_evidence_ready,
            runtime_evidence_ready,
            archive_evidence_ready,
            release_closeout_ready,
            operations_ready,
            release_evidence_index_ready,
            release_archive_notarization_ready,
        }
    }

    pub fn is_release_archive_notarization_ready(self) -> bool {
        self.release_archive_notarization_ready
    }

    pub fn has_blocked_archive_notarization_checks(self) -> bool {
        self.blocked_archive_notarization_check_count > 0
    }

    pub fn needs_release_readiness_evidence(self) -> bool {
        !self.release_readiness_evidence_ready
    }

    pub fn needs_runtime_evidence(self) -> bool {
        !self.runtime_evidence_ready
    }

    pub fn needs_archive_evidence(self) -> bool {
        !self.archive_evidence_ready
    }

    pub fn needs_release_closeout(self) -> bool {
        !self.release_closeout_ready
    }

    pub fn needs_operations(self) -> bool {
        !self.operations_ready
    }

    pub fn needs_release_evidence_index(self) -> bool {
        !self.release_evidence_index_ready
    }
}

pub fn hue_package_release_archive_notarization_summary(
    plan: &HueBridgePairingPlan,
) -> HuePackageReleaseArchiveNotarizationSummary {
    HuePackageReleaseArchiveNotarizationSummary::from_pairing_plan(plan)
}

fn descriptor_declares_capability(descriptor: &IntegrationDescriptor, capability_id: &str) -> bool {
    descriptor
        .capabilities
        .iter()
        .any(|capability| capability.as_str() == capability_id)
}

fn descriptor_declares_role(roles: &[String], role: &str) -> bool {
    roles.iter().any(|candidate| candidate == role)
}

fn metadata_contains(metadata: &[Metadata], key: &str, value: &str) -> bool {
    metadata
        .iter()
        .any(|metadata| metadata.key == key && metadata.value == value)
}

fn metadata_has_key(metadata: &[Metadata], key: &str) -> bool {
    metadata.iter().any(|metadata| metadata.key == key)
}

pub fn hue_registration_request(
    app_name: impl Into<String>,
    instance_name: impl Into<String>,
) -> HueRequest {
    HueRequest {
        method: HueMethod::Post,
        path: HUE_APPLICATION_REGISTRATION_PATH.to_string(),
        body: Some(HueRequestBody::RegisterApplication {
            app_name: app_name.into(),
            instance_name: instance_name.into(),
        }),
    }
}

pub fn hue_request_body_json(body: &HueRequestBody) -> serde_json::Value {
    match body {
        HueRequestBody::RegisterApplication {
            app_name,
            instance_name,
        } => serde_json::json!({
            "devicetype": format!("{app_name}#{instance_name}"),
            "generateclientkey": true,
        }),
        HueRequestBody::SetOn { on } => serde_json::json!({
            "on": { "on": on },
        }),
        HueRequestBody::SetBrightness { brightness } => serde_json::json!({
            "dimming": { "brightness": brightness },
        }),
        HueRequestBody::SetColorTemperature { mirek } => serde_json::json!({
            "color_temperature": { "mirek": mirek },
        }),
        HueRequestBody::RecallScene => serde_json::json!({
            "recall": { "action": "active" },
        }),
    }
}

pub fn hue_request_body_json_bytes(body: &HueRequestBody) -> Vec<u8> {
    serde_json::to_vec(&hue_request_body_json(body)).expect("Hue request body is valid JSON")
}

pub fn hue_request_to_local_http_plan(
    request: &HueRequest,
    endpoint: &LocalHttpEndpoint,
) -> Result<LocalHttpRequestPlan, HueError> {
    let body = request
        .body
        .as_ref()
        .map(hue_request_body_json_bytes)
        .unwrap_or_default();
    let mut template =
        LocalHttpRequestTemplate::new(request.method.as_local_http_method(), request.path.clone())?
            .with_accept("application/json")
            .with_timeout_ms(5_000)
            .with_idempotent(request.method.is_idempotent_by_default())
            .with_metadata(Metadata::new("hue.request.path", request.path.as_str()));

    if let Some(body) = &request.body {
        template = template
            .with_content_type("application/json")
            .with_metadata(Metadata::new("hue.request.body_kind", body.kind().as_str()));
    }

    Ok(template.plan(endpoint, body)?)
}

pub fn hue_pairing_registration_request_plan(
    plan: &HueBridgePairingPlan,
    endpoint: &LocalHttpEndpoint,
) -> Result<LocalHttpRequestPlan, HueError> {
    if plan.bridge_id() != &endpoint.bridge_id {
        return Err(HueError::PairingBridgeMismatch {
            plan_bridge_id: plan.bridge_id().clone(),
            endpoint_bridge_id: endpoint.bridge_id.clone(),
        });
    }

    let mut request_plan = hue_request_to_local_http_plan(&plan.registration_request, endpoint)?;
    request_plan
        .metadata
        .push(Metadata::new("hue.pairing.phase", "registration"));
    request_plan.metadata.push(Metadata::new(
        "hue.pairing.requires_user_presence",
        plan.requires_user_presence.to_string(),
    ));
    Ok(request_plan)
}

pub fn hue_application_credentials_from_registration_response(
    body: &[u8],
) -> Result<HueApplicationCredentials, HueError> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|error| HueError::InvalidPairingResponse {
            reason: error.to_string(),
        })?;
    let entries = value
        .as_array()
        .ok_or_else(|| HueError::InvalidPairingResponse {
            reason: "expected top-level response array".to_string(),
        })?;

    for entry in entries {
        if let Some(error) = entry.get("error") {
            return Err(HueError::PairingRejected {
                error_type: json_i64_field(error, "type"),
                description: json_string_field(error, "description")
                    .unwrap_or("unknown Hue pairing error")
                    .to_string(),
            });
        }

        if let Some(success) = entry.get("success") {
            let application_key = json_string_field(success, "username")
                .or_else(|| json_string_field(success, "application_key"))
                .ok_or(HueError::MissingPairingCredential { field: "username" })?;
            let client_key = json_string_field(success, "clientkey")
                .or_else(|| json_string_field(success, "client_key"))
                .map(str::to_string);
            return HueApplicationCredentials::new(application_key, client_key);
        }
    }

    Err(HueError::MissingPairingCredential { field: "success" })
}

pub fn hue_pairing_plan_for_discovered_bridge(
    discovered: DiscoveredHueBridge,
    app_name: impl Into<String>,
    instance_name: impl Into<String>,
) -> HueBridgePairingPlan {
    HueBridgePairingPlan {
        bridge: discovered_bridge_to_core(discovered),
        registration_request: hue_registration_request(app_name, instance_name),
        application_key_header: HUE_APPLICATION_KEY_HEADER.to_string(),
        event_stream_path: CLIP_V2_EVENT_STREAM_PATH.to_string(),
        requires_user_presence: true,
    }
}

pub fn hue_pairing_plan_for_discovery_record(
    record: &DiscoveryRecord,
    app_name: impl Into<String>,
    instance_name: impl Into<String>,
) -> Option<HueBridgePairingPlan> {
    discovered_hue_bridge_from_record(record)
        .map(|bridge| hue_pairing_plan_for_discovered_bridge(bridge, app_name, instance_name))
}

pub fn hue_discovery_record_from_mdns(
    advertisement: &MdnsAdvertisement,
) -> Result<DiscoveryRecord, HueError> {
    if !hue_mdns_service_type_matches(&advertisement.service_type) {
        return Err(HueError::UnsupportedDiscoveryService {
            service_type: advertisement.service_type.clone(),
        });
    }
    let bridge_id = mdns_txt_value(advertisement, "bridgeid")
        .or_else(|| mdns_txt_value(advertisement, "bridge_id"))
        .ok_or(HueError::MissingDiscoveryField { field: "bridgeid" })?;
    let hardware_model = mdns_txt_value(advertisement, "modelid").map(str::to_string);
    let firmware_version = mdns_txt_value(advertisement, "swversion")
        .or_else(|| mdns_txt_value(advertisement, "version"))
        .map(str::to_string);
    let metadata = vec![
        Metadata::new("hue.discovery.source", "mdns"),
        Metadata::new("hue.discovery.service_type", &advertisement.service_type),
        Metadata::new("hue.discovery.instance_name", &advertisement.instance_name),
        Metadata::new("hue.discovery.host_name", &advertisement.host_name),
        Metadata::new("hue.discovery.port", advertisement.port.to_string()),
    ];

    hue_discovery_record(
        bridge_id.to_string(),
        DiscoverySource::Mdns,
        hue_https_endpoint(advertisement.preferred_address(), advertisement.port),
        DiscoveryConfidence::Verified,
        hardware_model,
        firmware_version,
        advertisement.discovered_at_ms,
        metadata,
    )
    .map(|record| record.with_display_name(advertisement.instance_name.clone()))
}

pub fn discovered_hue_bridge_from_record(record: &DiscoveryRecord) -> Option<DiscoveredHueBridge> {
    if record.integration_id != IntegrationId::trusted(HUE_INTEGRATION_ID) {
        return None;
    }
    Some(DiscoveredHueBridge {
        bridge_id: record.native_bridge_id.clone(),
        address: record.address.clone()?,
        hardware_model: record.hardware_model.clone(),
        firmware_version: record.firmware_version.clone(),
    })
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

// Discovery records carry many independent fields; passing them explicitly is
// clearer here than introducing a params struct, and avoids churn.
#[allow(clippy::too_many_arguments)]
fn hue_discovery_record(
    bridge_id: impl Into<String>,
    source: DiscoverySource,
    address: impl Into<String>,
    confidence: DiscoveryConfidence,
    hardware_model: Option<String>,
    firmware_version: Option<String>,
    discovered_at_ms: u64,
    metadata: Vec<Metadata>,
) -> Result<DiscoveryRecord, HueError> {
    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(HUE_INTEGRATION_ID),
        ProtocolFamily::Hue,
        non_empty_discovery_field("bridge_id", bridge_id)?,
        source,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )?
    .with_address(non_empty_discovery_field("address", address)?)
    .with_confidence(confidence)
    .with_pairing_requirement(PairingRequirement::PhysicalPresence);
    if let Some(hardware_model) = hardware_model {
        record = record.with_hardware_model(hardware_model);
    }
    if let Some(firmware_version) = firmware_version {
        record = record.with_firmware_version(firmware_version);
    }
    for metadata in metadata {
        record = record.with_metadata(metadata.key, metadata.value);
    }
    Ok(record)
}

fn hue_https_endpoint(host: &str, port: u16) -> String {
    if port == HUE_DEFAULT_HTTPS_PORT {
        format!("https://{host}")
    } else {
        format!("https://{host}:{port}")
    }
}

fn hue_mdns_service_type_matches(service_type: &str) -> bool {
    service_type
        .trim_end_matches('.')
        .eq_ignore_ascii_case(HUE_MDNS_SERVICE_TYPE)
}

fn mdns_txt_value<'a>(advertisement: &'a MdnsAdvertisement, key: &str) -> Option<&'a str> {
    advertisement
        .txt
        .iter()
        .find(|entry| entry.key.eq_ignore_ascii_case(key))
        .map(|entry| entry.value.as_str())
}

fn non_empty_discovery_field(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, HueError> {
    let value = value.into();
    if value.trim().is_empty() {
        Err(HueError::MissingDiscoveryField { field })
    } else {
        Ok(value)
    }
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

    pub fn command_set_color_temperature(&self, mirek: u16) -> HueCommand {
        HueCommand::SetGroupedLightColorTemperature {
            grouped_light_id: self.id.clone(),
            mirek,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueGroupedLightStateUpdate {
    pub id: HueResourceId,
    pub owner: Option<HueResourceRef>,
    pub name: Option<String>,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
}

impl HueGroupedLightStateUpdate {
    pub fn from_grouped_light_resource(grouped_light: &HueGroupedLightResource) -> Self {
        Self {
            id: grouped_light.id.clone(),
            owner: Some(grouped_light.owner.clone()),
            name: Some(grouped_light.name.clone()),
            on: grouped_light.on,
            brightness: grouped_light.brightness,
        }
    }

    pub fn has_state(&self) -> bool {
        self.on.is_some() || self.brightness.is_some()
    }

    pub fn state_deltas(&self) -> Vec<StateDelta> {
        hue_grouped_light_state_deltas(self)
    }

    pub fn summary(&self) -> HueStateUpdateSummary {
        HueStateUpdateSummary {
            resource: HueResourceRef::new(HueResourceType::GroupedLight, self.id.clone()),
            owner: self.owner.clone(),
            name: self.name.clone(),
            state_field_count: usize::from(self.on.is_some())
                + usize::from(self.brightness.is_some()),
            delta_count: self.state_deltas().len(),
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
    pub fn state_field_count(&self) -> usize {
        usize::from(self.on.is_some())
            + usize::from(self.brightness.is_some())
            + usize::from(self.color_temperature_mirek.is_some())
    }

    pub fn has_state(&self) -> bool {
        self.state_field_count() > 0
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

    pub fn summary(&self) -> HueSceneSummary {
        HueSceneSummary {
            scene: HueResourceRef::new(HueResourceType::Scene, self.id.clone()),
            group: self.group.clone(),
            scope: scene_scope_for_group(&self.group),
            action_count: self.actions.len(),
            stateful_action_count: self
                .actions
                .iter()
                .filter(|action| action.has_state())
                .count(),
            desired_state_field_count: self
                .actions
                .iter()
                .map(HueSceneAction::state_field_count)
                .sum(),
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
pub struct HueSceneSummary {
    pub scene: HueResourceRef,
    pub group: HueResourceRef,
    pub scope: SceneScope,
    pub action_count: usize,
    pub stateful_action_count: usize,
    pub desired_state_field_count: usize,
}

impl HueSceneSummary {
    pub fn has_actions(&self) -> bool {
        self.action_count > 0
    }

    pub fn projects_actions(&self) -> bool {
        self.stateful_action_count > 0
    }

    pub fn is_room_or_zone_scoped(&self) -> bool {
        matches!(self.scope, SceneScope::Room | SceneScope::Zone)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HueSceneSetSummary {
    pub total_scenes: usize,
    pub room_scoped_scenes: usize,
    pub zone_scoped_scenes: usize,
    pub home_scoped_scenes: usize,
    pub bridge_scoped_scenes: usize,
    pub custom_scoped_scenes: usize,
    pub scenes_with_actions: usize,
    pub scenes_projecting_actions: usize,
    pub action_count: usize,
    pub stateful_action_count: usize,
    pub desired_state_field_count: usize,
}

impl HueSceneSetSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_scenes<'a>(scenes: impl IntoIterator<Item = &'a HueSceneResource>) -> Self {
        let mut summary = Self::empty();
        for scene in scenes {
            summary.record_summary(&scene.summary());
        }
        summary
    }

    pub fn from_summaries<'a>(summaries: impl IntoIterator<Item = &'a HueSceneSummary>) -> Self {
        let mut summary = Self::empty();
        for scene_summary in summaries {
            summary.record_summary(scene_summary);
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &HueSceneSummary) {
        self.total_scenes += 1;
        match summary.scope {
            SceneScope::Room => self.room_scoped_scenes += 1,
            SceneScope::Zone => self.zone_scoped_scenes += 1,
            SceneScope::Home => self.home_scoped_scenes += 1,
            SceneScope::Bridge => self.bridge_scoped_scenes += 1,
            SceneScope::Custom => self.custom_scoped_scenes += 1,
        }
        if summary.has_actions() {
            self.scenes_with_actions += 1;
        }
        if summary.projects_actions() {
            self.scenes_projecting_actions += 1;
        }
        self.action_count += summary.action_count;
        self.stateful_action_count += summary.stateful_action_count;
        self.desired_state_field_count += summary.desired_state_field_count;
    }

    pub fn is_empty(&self) -> bool {
        self.total_scenes == 0
    }

    pub fn room_or_zone_scoped_count(&self) -> usize {
        self.room_scoped_scenes + self.zone_scoped_scenes
    }

    pub fn has_room_or_zone_scoped_scenes(&self) -> bool {
        self.room_or_zone_scoped_count() > 0
    }

    pub fn projects_actions(&self) -> bool {
        self.stateful_action_count > 0
    }

    pub fn has_unprojected_actions(&self) -> bool {
        self.action_count > self.stateful_action_count
    }

    pub fn has_partial_action_projection(&self) -> bool {
        self.scenes_projecting_actions > 0
            && self.scenes_projecting_actions < self.scenes_with_actions
    }

    pub fn scope_family_count(&self) -> usize {
        usize::from(self.room_scoped_scenes > 0)
            + usize::from(self.zone_scoped_scenes > 0)
            + usize::from(self.home_scoped_scenes > 0)
            + usize::from(self.bridge_scoped_scenes > 0)
            + usize::from(self.custom_scoped_scenes > 0)
    }

    pub fn touches_multiple_scope_families(&self) -> bool {
        self.scope_family_count() > 1
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

    pub fn summary(&self) -> HueStateUpdateSummary {
        HueStateUpdateSummary {
            resource: HueResourceRef::new(HueResourceType::Motion, self.id.clone()),
            owner: self
                .owner_device_id
                .as_ref()
                .map(|id| HueResourceRef::new(HueResourceType::Device, id.clone())),
            name: self.name.clone(),
            state_field_count: usize::from(self.motion.is_some())
                + usize::from(self.motion_valid.is_some()),
            delta_count: self.state_deltas().len(),
        }
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

    pub fn summary(&self) -> HueStateUpdateSummary {
        HueStateUpdateSummary {
            resource: HueResourceRef::new(HueResourceType::Button, self.id.clone()),
            owner: self
                .owner_device_id
                .as_ref()
                .map(|id| HueResourceRef::new(HueResourceType::Device, id.clone())),
            name: self.name.clone(),
            state_field_count: usize::from(self.last_event.is_some()),
            delta_count: self.state_deltas().len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueStateUpdateSummary {
    pub resource: HueResourceRef,
    pub owner: Option<HueResourceRef>,
    pub name: Option<String>,
    pub state_field_count: usize,
    pub delta_count: usize,
}

impl HueStateUpdateSummary {
    pub fn has_state(&self) -> bool {
        self.state_field_count > 0
    }

    pub fn projects_deltas(&self) -> bool {
        self.delta_count > 0
    }

    pub fn has_owner(&self) -> bool {
        self.owner.is_some()
    }

    pub fn is_light_surface(&self) -> bool {
        matches!(
            &self.resource.resource_type,
            HueResourceType::Light | HueResourceType::GroupedLight
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HueStateUpdate {
    Light(HueLightStateUpdate),
    GroupedLight(HueGroupedLightStateUpdate),
    Motion(HueMotionStateUpdate),
    Button(HueButtonStateUpdate),
}

impl HueStateUpdate {
    pub fn resource_type(&self) -> HueResourceType {
        match self {
            Self::Light(_) => HueResourceType::Light,
            Self::GroupedLight(_) => HueResourceType::GroupedLight,
            Self::Motion(_) => HueResourceType::Motion,
            Self::Button(_) => HueResourceType::Button,
        }
    }

    pub fn has_state(&self) -> bool {
        match self {
            Self::Light(update) => update.has_state(),
            Self::GroupedLight(update) => update.has_state(),
            Self::Motion(update) => update.has_state(),
            Self::Button(update) => update.has_state(),
        }
    }

    pub fn summary(&self) -> HueStateUpdateSummary {
        match self {
            Self::Light(update) => update.summary(),
            Self::GroupedLight(update) => update.summary(),
            Self::Motion(update) => update.summary(),
            Self::Button(update) => update.summary(),
        }
    }

    pub fn state_deltas(&self) -> Vec<StateDelta> {
        match self {
            Self::Light(update) => update.state_deltas(),
            Self::GroupedLight(update) => update.state_deltas(),
            Self::Motion(update) => update.state_deltas(),
            Self::Button(update) => update.state_deltas(),
        }
    }
}

impl From<HueLightStateUpdate> for HueStateUpdate {
    fn from(update: HueLightStateUpdate) -> Self {
        Self::Light(update)
    }
}

impl From<HueGroupedLightStateUpdate> for HueStateUpdate {
    fn from(update: HueGroupedLightStateUpdate) -> Self {
        Self::GroupedLight(update)
    }
}

impl From<HueMotionStateUpdate> for HueStateUpdate {
    fn from(update: HueMotionStateUpdate) -> Self {
        Self::Motion(update)
    }
}

impl From<HueButtonStateUpdate> for HueStateUpdate {
    fn from(update: HueButtonStateUpdate) -> Self {
        Self::Button(update)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HueStateUpdateSetSummary {
    pub total_updates: usize,
    pub light_updates: usize,
    pub grouped_light_updates: usize,
    pub motion_updates: usize,
    pub button_updates: usize,
    pub updates_with_state: usize,
    pub updates_with_owner: usize,
    pub light_surface_updates: usize,
    pub sensor_or_input_updates: usize,
    pub state_field_count: usize,
    pub delta_count: usize,
}

impl HueStateUpdateSetSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_updates<'a>(updates: impl IntoIterator<Item = &'a HueStateUpdate>) -> Self {
        let mut summary = Self::empty();
        for update in updates {
            summary.record_summary(&update.summary());
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &HueStateUpdateSummary) {
        self.total_updates += 1;
        match summary.resource.resource_type {
            HueResourceType::Light => self.light_updates += 1,
            HueResourceType::GroupedLight => self.grouped_light_updates += 1,
            HueResourceType::Motion => self.motion_updates += 1,
            HueResourceType::Button => self.button_updates += 1,
            _ => {}
        }
        if summary.has_state() {
            self.updates_with_state += 1;
        }
        if summary.has_owner() {
            self.updates_with_owner += 1;
        }
        if summary.is_light_surface() {
            self.light_surface_updates += 1;
        }
        if matches!(
            summary.resource.resource_type,
            HueResourceType::Motion | HueResourceType::Button
        ) {
            self.sensor_or_input_updates += 1;
        }
        self.state_field_count += summary.state_field_count;
        self.delta_count += summary.delta_count;
    }

    pub fn is_empty(&self) -> bool {
        self.total_updates == 0
    }

    pub fn has_light_surfaces(&self) -> bool {
        self.light_surface_updates > 0
    }

    pub fn light_surface_update_count(&self) -> usize {
        self.light_updates + self.grouped_light_updates
    }

    pub fn mixes_direct_and_grouped_light_updates(&self) -> bool {
        self.light_updates > 0 && self.grouped_light_updates > 0
    }

    pub fn has_sensor_or_input_updates(&self) -> bool {
        self.sensor_or_input_updates > 0
    }

    pub fn resource_family_count(&self) -> usize {
        usize::from(self.light_updates > 0)
            + usize::from(self.grouped_light_updates > 0)
            + usize::from(self.motion_updates > 0)
            + usize::from(self.button_updates > 0)
    }

    pub fn touches_multiple_resource_families(&self) -> bool {
        self.resource_family_count() > 1
    }

    pub fn all_updates_have_owner(&self) -> bool {
        self.total_updates > 0 && self.updates_with_owner == self.total_updates
    }

    pub fn has_partial_state_projection(&self) -> bool {
        self.updates_with_state > 0 && self.updates_with_state < self.total_updates
    }

    pub fn projects_deltas(&self) -> bool {
        self.delta_count > 0
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

    pub fn summary(&self) -> HueStateUpdateSummary {
        HueStateUpdateSummary {
            resource: HueResourceRef::new(HueResourceType::Light, self.id.clone()),
            owner: self
                .owner_device_id
                .as_ref()
                .map(|id| HueResourceRef::new(HueResourceType::Device, id.clone())),
            name: self.name.clone(),
            state_field_count: usize::from(self.on.is_some())
                + usize::from(self.brightness.is_some())
                + usize::from(self.color_temperature_mirek.is_some()),
            delta_count: self.state_deltas().len(),
        }
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

pub fn hue_grouped_light_state_deltas(update: &HueGroupedLightStateUpdate) -> Vec<StateDelta> {
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

pub fn hue_entity_id_for_resource_ref(bridge_id: &BridgeId, resource: &HueResourceRef) -> EntityId {
    EntityId::trusted(format!(
        "hue.{}.{}.{}",
        resource.resource_type.as_hue_type(),
        bridge_id,
        resource.id
    ))
}

fn json_string_field<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(serde_json::Value::as_str)
}

fn json_i64_field(value: &serde_json::Value, field: &str) -> Option<i64> {
    value.get(field).and_then(serde_json::Value::as_i64)
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
    use smart_home_discovery::{
        DiscoveryWorkerRunStatus, MdnsScanNetwork, MdnsWorkerScanReport, MdnsWorkerScanRequest,
    };
    use std::time::Duration;

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

        let grouped_color_temperature = HueCommand::SetGroupedLightColorTemperature {
            grouped_light_id: HueResourceId::trusted("grouped-light-1"),
            mirek: 370,
        };

        assert_eq!(
            grouped_color_temperature.to_request(),
            HueRequest {
                method: HueMethod::Put,
                path: "/clip/v2/resource/grouped_light/grouped-light-1".to_string(),
                body: Some(HueRequestBody::SetColorTemperature { mirek: 370 }),
            }
        );
    }

    #[test]
    fn hue_command_summaries_are_payload_free() {
        let command = HueCommand::SetLightOn {
            light_id: HueResourceId::trusted("light-1"),
            on: true,
        };

        let summary = command.summary();

        assert_eq!(
            summary,
            HueCommandSummary {
                target: HueCommandTarget::Light,
                method: HueMethod::Put,
                body_kind: HueRequestBodyKind::SetOn,
            }
        );
        assert_eq!(summary.target.resource_type(), HueResourceType::Light);
        assert!(summary.writes_light_state());
        assert!(summary.targets_direct_light());
        assert!(!summary.targets_grouped_light());
        assert!(!summary.recalls_scene());
        assert_eq!(
            HueRequestBody::SetBrightness { brightness: 42 }.kind(),
            HueRequestBodyKind::SetBrightness
        );
    }

    #[test]
    fn hue_command_plan_summaries_roll_up_write_surfaces() {
        let commands = vec![
            HueCommand::SetLightOn {
                light_id: HueResourceId::trusted("light-1"),
                on: true,
            },
            HueCommand::SetGroupedLightOn {
                grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                on: false,
            },
            HueCommand::SetLightBrightness {
                light_id: HueResourceId::trusted("light-1"),
                brightness: 70,
            },
            HueCommand::SetGroupedLightBrightness {
                grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                brightness: 35,
            },
            HueCommand::SetLightColorTemperature {
                light_id: HueResourceId::trusted("light-1"),
                mirek: 366,
            },
            HueCommand::SetGroupedLightColorTemperature {
                grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                mirek: 370,
            },
            HueCommand::RecallScene {
                scene_id: HueResourceId::trusted("scene-1"),
            },
        ];

        let summary = HueCommandPlanSummary::from_commands(&commands);

        assert_eq!(
            summary,
            HueCommandPlanSummary {
                total_commands: 7,
                light_commands: 3,
                grouped_light_commands: 3,
                scene_commands: 1,
                on_off_commands: 2,
                brightness_commands: 2,
                color_temperature_commands: 2,
                scene_recall_commands: 1,
            }
        );
        assert!(summary.has_lighting_writes());
        assert_eq!(summary.lighting_write_count(), 6);
        assert_eq!(summary.target_surface_count(), 3);
        assert_eq!(summary.light_capability_write_count(), 6);
        assert_eq!(summary.light_capability_kind_count(), 3);
        assert!(summary.has_direct_light_commands());
        assert!(summary.has_group_commands());
        assert!(summary.mixes_direct_and_grouped_light_writes());
        assert!(summary.has_color_temperature_writes());
        assert!(summary.writes_multiple_light_capability_kinds());
        assert!(summary.has_scene_recalls());
        assert!(!summary.has_only_light_surface_writes());
        assert!(!summary.has_only_scene_recalls());
        assert!(summary.touches_multiple_surfaces());

        let light_only = HueCommandPlanSummary::from_commands(commands.iter().take(6));
        assert!(light_only.has_only_light_surface_writes());
        assert!(!light_only.has_only_scene_recalls());

        let scene_only = HueCommandPlanSummary::from_commands(commands.iter().skip(6));
        assert_eq!(scene_only.target_surface_count(), 1);
        assert_eq!(scene_only.light_capability_write_count(), 0);
        assert_eq!(scene_only.light_capability_kind_count(), 0);
        assert!(!scene_only.has_only_light_surface_writes());
        assert!(scene_only.has_only_scene_recalls());

        let empty = HueCommandPlanSummary::empty();
        assert!(empty.is_empty());
        assert!(!empty.has_lighting_writes());
        assert_eq!(empty.lighting_write_count(), 0);
        assert_eq!(empty.target_surface_count(), 0);
        assert_eq!(empty.light_capability_write_count(), 0);
        assert_eq!(empty.light_capability_kind_count(), 0);
        assert!(!empty.has_direct_light_commands());
        assert!(!empty.has_group_commands());
        assert!(!empty.mixes_direct_and_grouped_light_writes());
        assert!(!empty.has_color_temperature_writes());
        assert!(!empty.writes_multiple_light_capability_kinds());
        assert!(!empty.has_scene_recalls());
        assert!(!empty.has_only_light_surface_writes());
        assert!(!empty.has_only_scene_recalls());
        assert!(!empty.touches_multiple_surfaces());
    }

    #[test]
    fn hue_commands_can_be_planned_from_normalized_light_deltas() {
        let light = HueResourceRef::new(HueResourceType::Light, HueResourceId::trusted("light-1"));
        let grouped_light = HueResourceRef::new(
            HueResourceType::GroupedLight,
            HueResourceId::trusted("grouped-light-1"),
        );
        let light_deltas = vec![
            StateDelta {
                capability_id: CapabilityId::trusted("light.on_off"),
                value: Value::Bool(true),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(42),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("light.color_temperature"),
                value: Value::Integer(366),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.occupancy"),
                value: Value::Bool(false),
            },
        ];
        let grouped_deltas = vec![
            StateDelta {
                capability_id: CapabilityId::trusted("light.on_off"),
                value: Value::Bool(false),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(20),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("light.color_temperature"),
                value: Value::Integer(370),
            },
        ];

        let light_commands = hue_commands_from_state_deltas(&light, &light_deltas).unwrap();
        let grouped_commands =
            hue_commands_from_state_deltas(&grouped_light, &grouped_deltas).unwrap();

        assert_eq!(
            light_commands,
            vec![
                HueCommand::SetLightOn {
                    light_id: HueResourceId::trusted("light-1"),
                    on: true,
                },
                HueCommand::SetLightBrightness {
                    light_id: HueResourceId::trusted("light-1"),
                    brightness: 42,
                },
                HueCommand::SetLightColorTemperature {
                    light_id: HueResourceId::trusted("light-1"),
                    mirek: 366,
                },
            ]
        );
        assert_eq!(
            grouped_commands,
            vec![
                HueCommand::SetGroupedLightOn {
                    grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                    on: false,
                },
                HueCommand::SetGroupedLightBrightness {
                    grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                    brightness: 20,
                },
                HueCommand::SetGroupedLightColorTemperature {
                    grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                    mirek: 370,
                },
            ]
        );
    }

    #[test]
    fn hue_command_plans_keep_requests_and_ignored_capabilities() {
        let light = HueResourceRef::new(HueResourceType::Light, HueResourceId::trusted("light-1"));
        let deltas = vec![
            StateDelta {
                capability_id: CapabilityId::trusted("light.on_off"),
                value: Value::Bool(true),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.occupancy"),
                value: Value::Bool(false),
            },
            StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(25),
            },
        ];

        let plan = HueCommandPlan::from_state_deltas(&light, &deltas).unwrap();

        assert_eq!(plan.target, light);
        assert_eq!(
            plan.commands,
            vec![
                HueCommand::SetLightOn {
                    light_id: HueResourceId::trusted("light-1"),
                    on: true,
                },
                HueCommand::SetLightBrightness {
                    light_id: HueResourceId::trusted("light-1"),
                    brightness: 25,
                },
            ]
        );
        assert_eq!(
            plan.ignored_capability_ids,
            vec![CapabilityId::trusted("sensor.occupancy")]
        );
        assert!(plan.has_ignored_deltas());
        assert_eq!(plan.ignored_delta_count(), 1);
        assert_eq!(
            plan.summary(),
            HueCommandPlanSummary {
                total_commands: 2,
                light_commands: 2,
                grouped_light_commands: 0,
                scene_commands: 0,
                on_off_commands: 1,
                brightness_commands: 1,
                color_temperature_commands: 0,
                scene_recall_commands: 0,
            }
        );
        let projection_summary = plan.projection_summary();
        assert_eq!(
            projection_summary,
            HueCommandPlanProjectionSummary {
                target_resource_type: HueResourceType::Light,
                requested_delta_count: 3,
                generated_command_count: 2,
                ignored_delta_count: 1,
                command_summary: plan.summary(),
            }
        );
        assert!(!projection_summary.is_empty());
        assert!(projection_summary.has_generated_commands());
        assert!(projection_summary.has_ignored_deltas());
        assert!(projection_summary.has_partial_projection());
        assert!(!projection_summary.projected_all_requested_deltas());
        assert!(projection_summary.target_is_light_surface());
        assert!(!projection_summary.target_is_scene_surface());
        assert_eq!(
            plan.requests(),
            vec![
                HueRequest {
                    method: HueMethod::Put,
                    path: "/clip/v2/resource/light/light-1".to_string(),
                    body: Some(HueRequestBody::SetOn { on: true }),
                },
                HueRequest {
                    method: HueMethod::Put,
                    path: "/clip/v2/resource/light/light-1".to_string(),
                    body: Some(HueRequestBody::SetBrightness { brightness: 25 }),
                },
            ]
        );

        let grouped_light = HueResourceRef::new(
            HueResourceType::GroupedLight,
            HueResourceId::trusted("grouped-light-1"),
        );
        let projected = HueCommandPlan::from_state_deltas(
            &grouped_light,
            &[StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(55),
            }],
        )
        .unwrap()
        .projection_summary();
        assert_eq!(projected.requested_delta_count, 1);
        assert_eq!(projected.generated_command_count, 1);
        assert_eq!(projected.ignored_delta_count, 0);
        assert!(projected.projected_all_requested_deltas());
        assert!(projected.target_is_light_surface());

        let empty_projection = HueCommandPlan::empty(light.clone()).projection_summary();
        assert!(empty_projection.is_empty());
        assert!(!empty_projection.has_generated_commands());
        assert!(!empty_projection.has_ignored_deltas());
        assert!(!empty_projection.has_partial_projection());
        assert!(!empty_projection.projected_all_requested_deltas());
    }

    #[test]
    fn hue_command_delta_planning_rejects_wrong_shapes_and_targets() {
        let light = HueResourceRef::new(HueResourceType::Light, HueResourceId::trusted("light-1"));
        let grouped_light = HueResourceRef::new(
            HueResourceType::GroupedLight,
            HueResourceId::trusted("grouped-light-1"),
        );
        let room = HueResourceRef::new(HueResourceType::Room, HueResourceId::trusted("room-1"));

        assert!(matches!(
            hue_command_from_state_delta(
                &light,
                &StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Text("true".to_string()),
                },
            ),
            Err(HueError::InvalidCommandValue { .. })
        ));
        assert_eq!(
            hue_command_from_state_delta(
                &room,
                &StateDelta {
                    capability_id: CapabilityId::trusted("light.brightness"),
                    value: Value::Percentage(80),
                },
            ),
            Err(HueError::UnsupportedCommandTarget {
                resource_type: HueResourceType::Room
            })
        );
        assert_eq!(
            hue_command_from_state_delta(
                &grouped_light,
                &StateDelta {
                    capability_id: CapabilityId::trusted("light.color_temperature"),
                    value: Value::Integer(366),
                },
            ),
            Ok(Some(HueCommand::SetGroupedLightColorTemperature {
                grouped_light_id: HueResourceId::trusted("grouped-light-1"),
                mirek: 366,
            }))
        );
        assert!(matches!(
            hue_command_from_state_delta(
                &light,
                &StateDelta {
                    capability_id: CapabilityId::trusted("light.color_temperature"),
                    value: Value::Integer(-1),
                },
            ),
            Err(HueError::InvalidCommandValue { .. })
        ));
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
    fn hue_mdns_discovery_advertisement_projects_to_discovery_record() {
        let advertisement = MdnsAdvertisement::new(
            "_hue._tcp.local.",
            "Philips Hue - ABCDEF",
            "hue-bridge.local",
            443,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.10")
        .unwrap()
        .with_txt("bridgeid", "001788fffeabcdef")
        .unwrap()
        .with_txt("modelid", "BSB002")
        .unwrap()
        .with_txt("swversion", "1.66.1960062030")
        .unwrap();

        let record = hue_discovery_record_from_mdns(&advertisement).unwrap();
        let bridge = record.to_bridge_candidate();

        assert_eq!(record.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(record.native_bridge_id, "001788fffeabcdef");
        assert_eq!(record.source, DiscoverySource::Mdns);
        assert_eq!(record.transport, BridgeTransport::LanHttp);
        assert_eq!(record.address.as_deref(), Some("https://192.0.2.10"));
        assert_eq!(record.hardware_model.as_deref(), Some("BSB002"));
        assert_eq!(record.firmware_version.as_deref(), Some("1.66.1960062030"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(
            record.pairing_requirement,
            PairingRequirement::PhysicalPresence
        );
        assert_eq!(bridge.bridge_id.as_str(), "hue.bridge.001788fffeabcdef");
        assert_eq!(bridge.transport, BridgeTransport::LanHttp);
        assert!(record.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.source" && metadata.value == "mdns"
        }));
    }

    #[test]
    fn hue_mdns_discovery_rejects_non_hue_services() {
        let advertisement =
            MdnsAdvertisement::new("_matter._tcp.local", "Matter", "matter.local", 5540, 1_000)
                .unwrap()
                .with_txt("bridgeid", "001788fffeabcdef")
                .unwrap();

        assert_eq!(
            hue_discovery_record_from_mdns(&advertisement),
            Err(HueError::UnsupportedDiscoveryService {
                service_type: "_matter._tcp.local".to_string()
            })
        );
    }

    #[test]
    fn hue_cloud_fallback_discovery_projects_to_candidate_record() {
        let record = HueCloudDiscoveryBridge::new("001788fffeabcdef", "192.0.2.10", 2_000)
            .unwrap()
            .with_port(8443)
            .with_hardware_model("BSB002")
            .with_firmware_version("1.66")
            .into_record()
            .unwrap();

        assert_eq!(record.source, DiscoverySource::CloudFallback);
        assert_eq!(record.transport, BridgeTransport::LanHttp);
        assert_eq!(record.address.as_deref(), Some("https://192.0.2.10:8443"));
        assert_eq!(record.confidence, DiscoveryConfidence::Candidate);
        assert_eq!(
            record.pairing_requirement,
            PairingRequirement::PhysicalPresence
        );
        assert!(record.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.source" && metadata.value == "cloud_fallback"
        }));
    }

    #[test]
    fn hue_discovery_batch_collects_bridge_candidates() {
        let advertisement = MdnsAdvertisement::new(
            HUE_MDNS_SERVICE_TYPE,
            "Philips Hue - ABCDEF",
            "hue-bridge.local",
            443,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.10")
        .unwrap()
        .with_txt("bridgeid", "001788fffeabcdef")
        .unwrap();
        let batch = HueDiscoveryBatch::from_mdns_advertisements([&advertisement], 1_050).unwrap();

        assert_eq!(batch.generated_at_ms, 1_050);
        assert_eq!(batch.len(), 1);
        assert_eq!(
            batch.bridge_candidates()[0].bridge_id.as_str(),
            "hue.bridge.001788fffeabcdef"
        );
    }

    #[test]
    fn hue_discovery_worker_run_preserves_records_and_partial_failures() {
        let mdns = MdnsAdvertisement::new(
            HUE_MDNS_SERVICE_TYPE,
            "Philips Hue - ABCDEF",
            "hue-bridge.local",
            443,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.10")
        .unwrap()
        .with_txt("bridgeid", "001788fffeabcdef")
        .unwrap();
        let non_hue = MdnsAdvertisement::new(
            "_matter._tcp.local",
            "Matter Bridge",
            "matter.local",
            5540,
            1_005,
        )
        .unwrap();
        let cloud = HueCloudDiscoveryBridge::new("001788fffecloud", "192.0.2.11", 1_010).unwrap();

        let run = hue_discovery_worker_run_from_observations(
            "hue-discovery-worker",
            [&mdns, &non_hue],
            [cloud],
            990,
            1_020,
        )
        .unwrap();
        let summary = run.summary_at(1_100, 500, 2, 0, 0);

        assert_eq!(
            run.worker_id,
            DiscoveryWorkerId::trusted("hue-discovery-worker")
        );
        assert_eq!(run.kind, DiscoveryWorkerKind::Composite);
        assert_eq!(run.status(), DiscoveryWorkerRunStatus::Partial);
        assert_eq!(run.len(), 2);
        assert_eq!(run.failure_count(), 1);
        assert_eq!(run.duration_ms(), 30);
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.worker" && metadata.value == "true"
        }));
        assert_eq!(run.records[0].source, DiscoverySource::Mdns);
        assert_eq!(run.records[1].source, DiscoverySource::CloudFallback);
        assert_eq!(run.failures[0].source, DiscoverySource::Mdns);
        assert!(run.failures[0].message.contains("not a Hue bridge service"));
        assert_eq!(summary.record_count, 2);
        assert_eq!(summary.failure_count, 1);
        assert_eq!(
            summary
                .record_summary
                .count_for_source(DiscoverySource::Mdns),
            1
        );
        assert_eq!(
            summary
                .record_summary
                .count_for_source(DiscoverySource::CloudFallback),
            1
        );
        assert_eq!(summary.signal_summary.fresh, 2);
    }

    #[test]
    fn hue_discovery_worker_run_from_mdns_scan_keeps_scan_failures() {
        let mdns = MdnsAdvertisement::new(
            HUE_MDNS_SERVICE_TYPE,
            "Philips Hue - ABCDEF",
            "hue-bridge.local",
            443,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.10")
        .unwrap()
        .with_txt("bridgeid", "001788fffeabcdef")
        .unwrap();
        let non_hue = MdnsAdvertisement::new(
            "_matter._tcp.local",
            "Matter Bridge",
            "matter.local",
            5540,
            1_005,
        )
        .unwrap();
        let scan = MdnsScanResult {
            service_type: HUE_MDNS_SERVICE_TYPE.to_string(),
            discovered_at_ms: 1_010,
            datagram_count: 3,
            advertisements: vec![mdns, non_hue],
            failures: vec![smart_home_discovery::MdnsScanFailure {
                source: Some("192.0.2.50:5353".to_string()),
                message: "invalid mDNS message: DNS header is shorter than 12 bytes".to_string(),
            }],
        };

        let run =
            hue_discovery_worker_run_from_mdns_scan("hue-mdns-scan", &scan, 990, 1_020).unwrap();
        let summary = run.summary_at(1_100, 500, 1, 0, 0);

        assert_eq!(run.kind, DiscoveryWorkerKind::MdnsScan);
        assert_eq!(run.status(), DiscoveryWorkerRunStatus::Partial);
        assert_eq!(run.len(), 1);
        assert_eq!(run.failure_count(), 2);
        assert_eq!(run.records[0].source, DiscoverySource::Mdns);
        assert!(run
            .failures
            .iter()
            .any(|failure| failure.message.contains("not a Hue bridge service")));
        assert!(run.failures.iter().any(|failure| failure
            .metadata
            .iter()
            .any(|metadata| metadata.key == "hue.discovery.scan_source"
                && metadata.value == "192.0.2.50:5353")));
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.scan_datagram_count" && metadata.value == "3"
        }));
        assert_eq!(summary.record_count, 1);
        assert_eq!(summary.failure_count, 2);
        assert_eq!(summary.accepted_count(), 1);
    }

    #[test]
    fn hue_discovery_worker_run_from_mdns_scan_report_keeps_interface_failures() {
        let mdns = MdnsAdvertisement::new(
            HUE_MDNS_SERVICE_TYPE,
            "Philips Hue - ABCDEF",
            "hue-bridge.local",
            443,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.10")
        .unwrap()
        .with_txt("bridgeid", "001788fffeabcdef")
        .unwrap();
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-scan");
        let integration_id = IntegrationId::trusted(HUE_INTEGRATION_ID);
        let ipv4 = MdnsWorkerScanRequest::new(
            worker_id.clone(),
            integration_id.clone(),
            "en0",
            MdnsScanNetwork::Ipv4,
            HUE_MDNS_SERVICE_TYPE,
            1_010,
            Duration::from_millis(250),
        )
        .unwrap();
        let ipv6 = MdnsWorkerScanRequest::new(
            worker_id.clone(),
            integration_id.clone(),
            "en0",
            MdnsScanNetwork::Ipv6,
            HUE_MDNS_SERVICE_TYPE,
            1_010,
            Duration::from_millis(250),
        )
        .unwrap();
        let mut report = MdnsWorkerScanReport::new(
            worker_id.clone(),
            integration_id,
            HUE_MDNS_SERVICE_TYPE,
            990,
            1_020,
        )
        .unwrap()
        .with_metadata("fixture", "hue_mdns_scan_report");
        report
            .push_success(
                ipv4,
                MdnsScanResult {
                    service_type: HUE_MDNS_SERVICE_TYPE.to_string(),
                    discovered_at_ms: 1_010,
                    datagram_count: 1,
                    advertisements: vec![mdns],
                    failures: Vec::new(),
                },
            )
            .unwrap();
        report
            .push_failure(ipv6, "IPv6 multicast route is unavailable")
            .unwrap();

        let run = hue_discovery_worker_run_from_mdns_scan_report(&report).unwrap();

        assert_eq!(run.worker_id, worker_id);
        assert_eq!(run.kind, DiscoveryWorkerKind::MdnsScan);
        assert_eq!(run.status(), DiscoveryWorkerRunStatus::Partial);
        assert_eq!(run.len(), 1);
        assert_eq!(run.failure_count(), 1);
        assert_eq!(run.failures[0].source, DiscoverySource::Mdns);
        assert_eq!(
            run.failures[0]
                .metadata
                .iter()
                .find(|metadata| metadata.key == "hue.discovery.scan_source")
                .map(|metadata| metadata.value.as_str()),
            Some("en0/ipv6")
        );
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.scan_report" && metadata.value == "true"
        }));
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.scan_request_success_count" && metadata.value == "1"
        }));
        assert!(run
            .metadata
            .iter()
            .any(|metadata| metadata.key == "fixture" && metadata.value == "hue_mdns_scan_report"));
    }

    #[test]
    fn hue_discovery_record_can_seed_pairing_plan() {
        let record = HueCloudDiscoveryBridge::new("001788fffeabcdef", "192.0.2.10", 2_000)
            .unwrap()
            .into_record()
            .unwrap();

        let discovered = discovered_hue_bridge_from_record(&record).unwrap();
        let plan =
            hue_pairing_plan_for_discovery_record(&record, "chief-of-staff", "desktop").unwrap();

        assert_eq!(discovered.bridge_id, "001788fffeabcdef");
        assert_eq!(plan.bridge_id().as_str(), "hue.bridge.001788fffeabcdef");
        assert_eq!(
            plan.registration_request.path,
            HUE_APPLICATION_REGISTRATION_PATH
        );
        assert!(plan.requires_user_presence);
    }

    #[test]
    fn discovered_bridge_builds_pairing_plan_without_application_key() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        assert_eq!(plan.bridge_id().as_str(), "hue.bridge.001788fffeabcdef");
        assert_eq!(plan.bridge.health, Health::Unpaired);
        assert_eq!(plan.bridge.address.as_deref(), Some("https://192.0.2.10"));
        assert_eq!(plan.application_key_header, HUE_APPLICATION_KEY_HEADER);
        assert_eq!(plan.event_stream_path, CLIP_V2_EVENT_STREAM_PATH);
        assert!(plan.requires_user_presence);
        assert_eq!(
            plan.registration_request,
            HueRequest {
                method: HueMethod::Post,
                path: HUE_APPLICATION_REGISTRATION_PATH.to_string(),
                body: Some(HueRequestBody::RegisterApplication {
                    app_name: "chief-of-staff".to_string(),
                    instance_name: "desk".to_string(),
                }),
            }
        );

        let summary = plan.summary();
        assert_eq!(summary.registration_method, HueMethod::Post);
        assert!(summary.has_bridge_address);
        assert!(summary.bridge_is_unpaired);
        assert!(summary.registration_path_is_api);
        assert!(summary.registration_body_is_application);
        assert!(summary.uses_hue_application_key_header);
        assert!(summary.uses_event_stream_path);
        assert!(summary.requires_user_presence);
        assert!(summary.uses_physical_presence());
        assert!(summary.posts_registration_request());
        assert!(summary.ready_for_local_registration());

        let mut incomplete = plan.clone();
        incomplete.bridge.address = None;
        incomplete.registration_request.body = None;
        incomplete.application_key_header = "wrong-header".to_string();
        incomplete.requires_user_presence = false;
        let incomplete_summary = HueBridgePairingPlanSummary::from_plan(&incomplete);
        assert!(!incomplete_summary.has_bridge_address);
        assert!(!incomplete_summary.registration_body_is_application);
        assert!(!incomplete_summary.uses_hue_application_key_header);
        assert!(!incomplete_summary.uses_physical_presence());
        assert!(!incomplete_summary.posts_registration_request());
        assert!(!incomplete_summary.ready_for_local_registration());
    }

    #[test]
    fn hue_pairing_registration_builds_local_http_plan_and_parses_credentials() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        let endpoint = LocalHttpEndpoint::hue_bridge(plan.bridge_id().clone(), "192.0.2.10")
            .unwrap()
            .accept_invalid_certs(true);

        let request_plan = hue_pairing_registration_request_plan(&plan, &endpoint).unwrap();

        assert_eq!(request_plan.method, LocalHttpMethod::Post);
        assert_eq!(request_plan.url, "https://192.0.2.10/api");
        assert_eq!(
            request_plan.header("Content-Type"),
            Some("application/json")
        );
        assert_eq!(request_plan.header("Accept"), Some("application/json"));
        assert!(!request_plan.idempotent);
        assert!(request_plan.required_vault_ref().is_none());
        assert!(request_plan.metadata.iter().any(|metadata| {
            metadata.key == "hue.pairing.phase" && metadata.value == "registration"
        }));
        let body: serde_json::Value = serde_json::from_slice(&request_plan.body).unwrap();
        assert_eq!(body["devicetype"], "chief-of-staff#desk");
        assert_eq!(body["generateclientkey"], true);

        let credentials = hue_application_credentials_from_registration_response(
            br#"[{"success":{"username":"raw-hue-application-key","clientkey":"client-key-1"}}]"#,
        )
        .unwrap();
        assert_eq!(credentials.application_key, "raw-hue-application-key");
        assert_eq!(credentials.client_key.as_deref(), Some("client-key-1"));

        let vault_payload = credentials.vault_secret_json();
        let vault_payload_json: serde_json::Value = serde_json::from_slice(&vault_payload).unwrap();
        assert_eq!(
            vault_payload_json["application_key"],
            "raw-hue-application-key"
        );
        assert_eq!(vault_payload_json["client_key"], "client-key-1");
        let decoded = HueApplicationCredentials::from_vault_secret_json(&vault_payload).unwrap();
        assert_eq!(decoded.application_key, "raw-hue-application-key");
        assert_eq!(decoded.client_key.as_deref(), Some("client-key-1"));
        let debug = format!("{decoded:?}");
        assert!(!debug.contains("raw-hue-application-key"));
        assert!(!debug.contains("client-key-1"));

        let handoff = credentials.vault_handoff(
            &plan,
            VaultRef::trusted("vault://smart-home/hue/001788fffeabcdef/application-key"),
            1_300,
        );
        assert_eq!(
            handoff.bridge_id,
            BridgeId::trusted("hue.bridge.001788fffeabcdef")
        );
        assert_eq!(handoff.application_key_header, HUE_APPLICATION_KEY_HEADER);
        assert!(handoff.metadata.iter().any(|metadata| {
            metadata.key == "hue.pairing.client_key_present" && metadata.value == "true"
        }));
        assert!(handoff.metadata.iter().all(|metadata| {
            !metadata.value.contains("raw-hue-application-key")
                && !metadata.value.contains("client-key-1")
        }));

        let summary = handoff.summary();
        assert_eq!(summary.metadata_count, 6);
        assert_eq!(summary.stored_at_ms, 1_300);
        assert!(summary.has_vault_reference);
        assert!(summary.uses_hue_application_key_header);
        assert!(summary.uses_event_stream_path);
        assert!(summary.has_credential_stored_phase);
        assert!(summary.has_application_key_credential_kind);
        assert!(summary.reports_client_key_presence);
        assert!(summary.has_metadata());
        assert!(summary.was_stored());
        assert!(summary.is_complete());
    }

    #[test]
    fn hue_pairing_vault_handoff_summary_flags_incomplete_handoffs() {
        let handoff = HuePairingVaultHandoff {
            bridge_id: BridgeId::trusted("hue.bridge.001788fffeabcdef"),
            vault_ref: VaultRef::trusted(""),
            stored_at_ms: 0,
            application_key_header: "wrong-header".to_string(),
            event_stream_path: "/wrong/path".to_string(),
            metadata: vec![Metadata::new("hue.pairing.phase", "credential_stored")],
        };

        let summary = HuePairingVaultHandoffSummary::from_handoff(&handoff);

        assert_eq!(summary.metadata_count, 1);
        assert!(!summary.has_vault_reference);
        assert!(!summary.uses_hue_application_key_header);
        assert!(!summary.uses_event_stream_path);
        assert!(summary.has_credential_stored_phase);
        assert!(!summary.has_application_key_credential_kind);
        assert!(!summary.reports_client_key_presence);
        assert!(summary.has_metadata());
        assert!(!summary.was_stored());
        assert!(!summary.is_complete());
    }

    #[test]
    fn hue_pairing_registration_surfaces_link_button_errors() {
        let error = hue_application_credentials_from_registration_response(
            br#"[{"error":{"type":101,"address":"/","description":"link button not pressed"}}]"#,
        )
        .unwrap_err();

        assert_eq!(
            error,
            HueError::PairingRejected {
                error_type: Some(101),
                description: "link button not pressed".to_string(),
            }
        );
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
        assert_eq!(
            grouped.command_set_color_temperature(370).to_request(),
            HueRequest {
                method: HueMethod::Put,
                path: "/clip/v2/resource/grouped_light/grouped-light-1".to_string(),
                body: Some(HueRequestBody::SetColorTemperature { mirek: 370 }),
            }
        );
        assert_eq!(grouped.owner.resource_type, HueResourceType::Room);

        let update = HueGroupedLightStateUpdate::from_grouped_light_resource(&grouped);
        assert!(update.has_state());
        assert_eq!(
            update.owner.as_ref().unwrap().resource_type,
            HueResourceType::Room
        );
        let summary = update.summary();
        assert_eq!(
            summary.resource.resource_type,
            HueResourceType::GroupedLight
        );
        assert_eq!(summary.state_field_count, 2);
        assert_eq!(summary.delta_count, 2);
        assert!(summary.has_owner());
        assert!(summary.has_state());
        assert!(summary.projects_deltas());
        assert!(summary.is_light_surface());
        let deltas = update.state_deltas();
        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0].capability_id.as_str(), "light.on_off");
        assert_eq!(deltas[1].capability_id.as_str(), "light.brightness");
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
        assert_eq!(scene.actions[0].state_field_count(), 3);
        let summary = scene.summary();
        assert_eq!(
            summary.scene,
            HueResourceRef::new(HueResourceType::Scene, HueResourceId::trusted("scene-1"))
        );
        assert_eq!(summary.group.resource_type, HueResourceType::Room);
        assert_eq!(summary.scope, SceneScope::Room);
        assert_eq!(summary.action_count, 1);
        assert_eq!(summary.stateful_action_count, 1);
        assert_eq!(summary.desired_state_field_count, 3);
        assert!(summary.has_actions());
        assert!(summary.projects_actions());
        assert!(summary.is_room_or_zone_scoped());

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
    fn hue_scene_summary_counts_empty_actions_without_projecting_them() {
        let scene = HueSceneResource {
            id: HueResourceId::trusted("scene-2"),
            group: HueResourceRef::new(HueResourceType::Zone, HueResourceId::trusted("zone-1")),
            name: "Evening".to_string(),
            actions: vec![
                HueSceneAction {
                    target: HueResourceRef::new(
                        HueResourceType::Light,
                        HueResourceId::trusted("light-1"),
                    ),
                    on: None,
                    brightness: None,
                    color_temperature_mirek: None,
                },
                HueSceneAction {
                    target: HueResourceRef::new(
                        HueResourceType::GroupedLight,
                        HueResourceId::trusted("grouped-light-1"),
                    ),
                    on: None,
                    brightness: Some(25),
                    color_temperature_mirek: None,
                },
            ],
        };

        let summary = scene.summary();
        assert_eq!(summary.scope, SceneScope::Zone);
        assert_eq!(summary.action_count, 2);
        assert_eq!(summary.stateful_action_count, 1);
        assert_eq!(summary.desired_state_field_count, 1);

        let core_scene = scene.to_core(&BridgeId::trusted("hue.bridge.001788"));
        assert_eq!(core_scene.actions.len(), 1);
        assert_eq!(
            core_scene.actions[0].entity_id.as_str(),
            "hue.grouped_light.hue.bridge.001788.grouped-light-1"
        );
    }

    #[test]
    fn hue_scene_set_summary_rolls_up_scope_and_action_projection() {
        let room_scene = HueSceneResource {
            id: HueResourceId::trusted("scene-room"),
            group: HueResourceRef::new(HueResourceType::Room, HueResourceId::trusted("room-1")),
            name: "Dinner".to_string(),
            actions: vec![HueSceneAction {
                target: HueResourceRef::new(
                    HueResourceType::Light,
                    HueResourceId::trusted("light-1"),
                ),
                on: Some(true),
                brightness: Some(70),
                color_temperature_mirek: None,
            }],
        };
        let zone_scene = HueSceneResource {
            id: HueResourceId::trusted("scene-zone"),
            group: HueResourceRef::new(HueResourceType::Zone, HueResourceId::trusted("zone-1")),
            name: "Evening".to_string(),
            actions: vec![
                HueSceneAction {
                    target: HueResourceRef::new(
                        HueResourceType::GroupedLight,
                        HueResourceId::trusted("grouped-1"),
                    ),
                    on: None,
                    brightness: None,
                    color_temperature_mirek: None,
                },
                HueSceneAction {
                    target: HueResourceRef::new(
                        HueResourceType::Light,
                        HueResourceId::trusted("light-2"),
                    ),
                    on: None,
                    brightness: Some(25),
                    color_temperature_mirek: Some(370),
                },
            ],
        };
        let bridge_scene = HueSceneResource {
            id: HueResourceId::trusted("scene-bridge"),
            group: HueResourceRef::new(HueResourceType::Bridge, HueResourceId::trusted("bridge-1")),
            name: "All off".to_string(),
            actions: Vec::new(),
        };

        let scenes = vec![room_scene, zone_scene, bridge_scene];
        let summary = HueSceneSetSummary::from_scenes(&scenes);

        assert_eq!(
            summary,
            HueSceneSetSummary {
                total_scenes: 3,
                room_scoped_scenes: 1,
                zone_scoped_scenes: 1,
                bridge_scoped_scenes: 1,
                scenes_with_actions: 2,
                scenes_projecting_actions: 2,
                action_count: 3,
                stateful_action_count: 2,
                desired_state_field_count: 4,
                ..HueSceneSetSummary::empty()
            }
        );
        assert_eq!(summary.room_or_zone_scoped_count(), 2);
        assert!(summary.has_room_or_zone_scoped_scenes());
        assert!(summary.projects_actions());
        assert!(summary.has_unprojected_actions());
        assert!(!summary.has_partial_action_projection());
        assert_eq!(summary.scope_family_count(), 3);
        assert!(summary.touches_multiple_scope_families());
    }

    #[test]
    fn hue_scene_set_summary_handles_precomputed_and_empty_summaries() {
        let summaries = vec![
            HueSceneSummary {
                scene: HueResourceRef::new(
                    HueResourceType::Scene,
                    HueResourceId::trusted("home-scene"),
                ),
                group: HueResourceRef::new(HueResourceType::Bridge, HueResourceId::trusted("home")),
                scope: SceneScope::Home,
                action_count: 2,
                stateful_action_count: 1,
                desired_state_field_count: 1,
            },
            HueSceneSummary {
                scene: HueResourceRef::new(
                    HueResourceType::Scene,
                    HueResourceId::trusted("custom-scene"),
                ),
                group: HueResourceRef::new(
                    HueResourceType::Device,
                    HueResourceId::trusted("device-1"),
                ),
                scope: SceneScope::Custom,
                action_count: 1,
                stateful_action_count: 0,
                desired_state_field_count: 0,
            },
        ];

        let summary = HueSceneSetSummary::from_summaries(&summaries);
        assert_eq!(summary.total_scenes, 2);
        assert_eq!(summary.home_scoped_scenes, 1);
        assert_eq!(summary.custom_scoped_scenes, 1);
        assert_eq!(summary.scenes_with_actions, 2);
        assert_eq!(summary.scenes_projecting_actions, 1);
        assert_eq!(summary.action_count, 3);
        assert_eq!(summary.stateful_action_count, 1);
        assert!(summary.has_partial_action_projection());
        assert!(summary.has_unprojected_actions());
        assert_eq!(summary.scope_family_count(), 2);

        let empty = HueSceneSetSummary::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.room_or_zone_scoped_count(), 0);
        assert!(!empty.has_room_or_zone_scoped_scenes());
        assert!(!empty.projects_actions());
        assert!(!empty.has_unprojected_actions());
        assert!(!empty.has_partial_action_projection());
        assert_eq!(empty.scope_family_count(), 0);
        assert!(!empty.touches_multiple_scope_families());
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
        let summary = update.summary();
        assert_eq!(
            summary.resource,
            HueResourceRef::new(HueResourceType::Light, HueResourceId::trusted("light-1"))
        );
        assert_eq!(summary.state_field_count, 3);
        assert_eq!(summary.delta_count, 3);
        assert!(!summary.has_owner());
        assert!(summary.is_light_surface());
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
    fn hue_state_update_enum_preserves_summaries_and_deltas() {
        let light = HueStateUpdate::from(HueLightStateUpdate {
            id: HueResourceId::trusted("light-1"),
            owner_device_id: Some(HueResourceId::trusted("device-1")),
            name: Some("Kitchen".to_string()),
            on: Some(true),
            brightness: Some(50),
            color_temperature_mirek: None,
        });
        let grouped = HueStateUpdate::from(HueGroupedLightStateUpdate {
            id: HueResourceId::trusted("grouped-light-1"),
            owner: Some(HueResourceRef::new(
                HueResourceType::Room,
                HueResourceId::trusted("room-1"),
            )),
            name: None,
            on: None,
            brightness: Some(10),
        });
        let motion = HueStateUpdate::from(HueMotionStateUpdate {
            id: HueResourceId::trusted("motion-1"),
            owner_device_id: None,
            name: None,
            motion: Some(true),
            motion_valid: Some(true),
        });
        let button = HueStateUpdate::from(HueButtonStateUpdate {
            id: HueResourceId::trusted("button-1"),
            owner_device_id: None,
            name: None,
            last_event: Some("short_release".to_string()),
        });
        let updates = vec![light, grouped, motion, button];

        let summary = HueStateUpdateSetSummary::from_updates(&updates);

        assert_eq!(updates[0].resource_type(), HueResourceType::Light);
        assert_eq!(updates[0].state_deltas().len(), 2);
        assert_eq!(
            updates[1].summary().resource.resource_type,
            HueResourceType::GroupedLight
        );
        assert!(updates.iter().all(HueStateUpdate::has_state));
        assert_eq!(
            summary,
            HueStateUpdateSetSummary {
                total_updates: 4,
                light_updates: 1,
                grouped_light_updates: 1,
                motion_updates: 1,
                button_updates: 1,
                updates_with_state: 4,
                updates_with_owner: 2,
                light_surface_updates: 2,
                sensor_or_input_updates: 2,
                state_field_count: 6,
                delta_count: 5,
            }
        );
        assert!(summary.has_light_surfaces());
        assert_eq!(summary.light_surface_update_count(), 2);
        assert!(summary.mixes_direct_and_grouped_light_updates());
        assert!(summary.has_sensor_or_input_updates());
        assert_eq!(summary.resource_family_count(), 4);
        assert!(summary.touches_multiple_resource_families());
        assert!(!summary.all_updates_have_owner());
        assert!(!summary.has_partial_state_projection());
        assert!(summary.projects_deltas());

        let partial = HueStateUpdateSetSummary {
            total_updates: 3,
            light_updates: 1,
            updates_with_state: 1,
            updates_with_owner: 3,
            light_surface_updates: 1,
            ..HueStateUpdateSetSummary::empty()
        };
        assert_eq!(partial.resource_family_count(), 1);
        assert!(partial.all_updates_have_owner());
        assert!(partial.has_partial_state_projection());

        let empty = HueStateUpdateSetSummary::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.light_surface_update_count(), 0);
        assert!(!empty.mixes_direct_and_grouped_light_updates());
        assert_eq!(empty.resource_family_count(), 0);
        assert!(!empty.touches_multiple_resource_families());
        assert!(!empty.all_updates_have_owner());
        assert!(!empty.has_partial_state_projection());
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
        let summary = update.summary();
        assert_eq!(summary.resource.resource_type, HueResourceType::Motion);
        assert_eq!(summary.owner.as_ref().unwrap().id.as_str(), "device-1");
        assert_eq!(summary.state_field_count, 2);
        assert_eq!(summary.delta_count, 1);
        assert!(summary.has_owner());
        assert!(!summary.is_light_surface());
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
        let summary = update.summary();
        assert_eq!(summary.resource.resource_type, HueResourceType::Button);
        assert_eq!(summary.state_field_count, 1);
        assert_eq!(summary.delta_count, 1);
        assert!(summary.projects_deltas());
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

    #[test]
    fn hue_integration_descriptor_summary_reports_runtime_surface() {
        let descriptor = hue_integration_descriptor();
        let summary = HueIntegrationDescriptorSummary::from_descriptor(&descriptor);

        assert_eq!(summary.runtime_kind, RuntimeKind::RustWorkerProcess);
        assert_eq!(summary.capability_count, 3);
        assert_eq!(summary.discovery_role_count, 1);
        assert_eq!(summary.pairing_role_count, 1);
        assert!(summary.integration_id_is_hue);
        assert!(summary.declares_read);
        assert!(summary.declares_light_command);
        assert!(summary.declares_pairing);
        assert!(summary.declares_bridge_discovery);
        assert!(summary.declares_bridge_pairing);
        assert!(summary.runs_as_worker_process());
        assert!(summary.has_canonical_identity());
        assert!(summary.has_agent_facing_capabilities());
        assert!(summary.has_bridge_roles());
        assert!(summary.supports_local_pairing_flow());
        assert!(summary.supports_light_command_flow());
    }

    #[test]
    fn hue_integration_descriptor_summary_helper_uses_default_descriptor() {
        assert_eq!(
            hue_integration_descriptor_summary(),
            HueIntegrationDescriptorSummary::from_descriptor(&hue_integration_descriptor())
        );
    }

    #[test]
    fn hue_integration_package_summary_joins_descriptor_and_pairing_readiness() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        let summary = hue_integration_package_summary(&plan);

        assert_eq!(
            summary.descriptor_summary,
            hue_integration_descriptor_summary()
        );
        assert_eq!(summary.pairing_plan_summary, plan.summary());
        assert!(summary.worker_process_ready);
        assert!(summary.command_flow_declared);
        assert!(summary.local_pairing_declared);
        assert!(summary.local_pairing_ready);
        assert!(summary.package_ready);
        assert!(summary.requires_physical_presence);
        assert!(summary.has_agent_facing_capabilities());
        assert!(summary.has_bridge_roles());
        assert!(summary.uses_local_event_stream());
    }

    #[test]
    fn hue_integration_package_summary_flags_incomplete_pairing_package() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.body = None;
        plan.requires_user_presence = false;

        let summary = HueIntegrationPackageSummary::from_pairing_plan(&plan);

        assert!(summary.worker_process_ready);
        assert!(summary.command_flow_declared);
        assert!(summary.local_pairing_declared);
        assert!(!summary.local_pairing_ready);
        assert!(!summary.package_ready);
        assert!(!summary.requires_physical_presence);
        assert!(!summary.pairing_plan_summary.ready_for_local_registration());
        assert!(summary.has_agent_facing_capabilities());
        assert!(summary.has_bridge_roles());
    }

    #[test]
    fn hue_package_release_readiness_summary_marks_catalog_ready_package() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_readiness_summary(&plan);

        assert_eq!(
            summary.package_summary,
            hue_integration_package_summary(&plan)
        );
        assert_eq!(summary.required_check_count, 5);
        assert_eq!(summary.passed_check_count, 5);
        assert_eq!(summary.failed_check_count, 0);
        assert!(summary.worker_process_ready);
        assert!(summary.command_flow_ready);
        assert!(summary.pairing_flow_ready);
        assert!(summary.event_stream_ready);
        assert!(summary.physical_presence_required);
        assert!(summary.release_ready);
        assert!(summary.is_release_ready());
        assert!(!summary.has_failed_checks());
    }

    #[test]
    fn hue_package_release_readiness_summary_counts_missing_pairing_checks() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseReadinessSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_check_count, 5);
        assert_eq!(summary.passed_check_count, 2);
        assert_eq!(summary.failed_check_count, 3);
        assert!(summary.worker_process_ready);
        assert!(summary.command_flow_ready);
        assert!(!summary.pairing_flow_ready);
        assert!(!summary.event_stream_ready);
        assert!(!summary.physical_presence_required);
        assert!(!summary.release_ready);
        assert!(!summary.is_release_ready());
        assert!(summary.has_failed_checks());
    }

    #[test]
    fn hue_package_spec_summary_marks_spec_ready_package() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_spec_summary(&plan);

        assert_eq!(
            summary.release_readiness,
            hue_package_release_readiness_summary(&plan)
        );
        assert_eq!(summary.required_spec_check_count, 9);
        assert_eq!(summary.passed_spec_check_count, 9);
        assert_eq!(summary.missing_spec_check_count, 0);
        assert!(summary.canonical_integration_id);
        assert!(summary.clip_v2_resource_root);
        assert!(summary.registration_endpoint_ready);
        assert!(summary.application_key_header_ready);
        assert!(summary.event_stream_path_ready);
        assert!(summary.read_model_declared);
        assert!(summary.command_model_declared);
        assert!(summary.pairing_model_declared);
        assert!(summary.spec_ready);
        assert!(summary.is_spec_ready());
        assert!(!summary.has_missing_spec_checks());
        assert!(summary.declares_runtime_model_surface());
    }

    #[test]
    fn hue_package_spec_summary_counts_broken_handoff_surface() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();

        let summary = HuePackageSpecSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_spec_check_count, 9);
        assert_eq!(summary.passed_spec_check_count, 5);
        assert_eq!(summary.missing_spec_check_count, 4);
        assert!(summary.canonical_integration_id);
        assert!(summary.clip_v2_resource_root);
        assert!(!summary.registration_endpoint_ready);
        assert!(!summary.application_key_header_ready);
        assert!(!summary.event_stream_path_ready);
        assert!(summary.read_model_declared);
        assert!(summary.command_model_declared);
        assert!(summary.pairing_model_declared);
        assert!(!summary.release_readiness.is_release_ready());
        assert!(!summary.spec_ready);
        assert!(!summary.is_spec_ready());
        assert!(summary.has_missing_spec_checks());
        assert!(summary.declares_runtime_model_surface());
    }

    #[test]
    fn hue_package_spec_gap_summary_marks_clear_package_spec() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_spec_gap_summary(&plan);

        assert_eq!(summary.spec_summary, hue_package_spec_summary(&plan));
        assert_eq!(summary.blocking_spec_check_count, 0);
        assert!(!summary.release_blocked);
        assert!(!summary.identity_blocked);
        assert!(!summary.clip_v2_root_blocked);
        assert!(!summary.registration_endpoint_blocked);
        assert!(!summary.application_key_header_blocked);
        assert!(!summary.event_stream_path_blocked);
        assert!(!summary.runtime_model_blocked);
        assert!(summary.spec_ready);
        assert!(summary.is_clear());
        assert!(!summary.has_blockers());
        assert!(!summary.needs_release_review());
        assert!(!summary.needs_transport_review());
        assert!(!summary.needs_runtime_model_review());
    }

    #[test]
    fn hue_package_spec_gap_summary_routes_transport_and_release_gaps() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();

        let summary = HuePackageSpecGapSummary::from_pairing_plan(&plan);

        assert_eq!(summary.blocking_spec_check_count, 4);
        assert!(summary.release_blocked);
        assert!(!summary.identity_blocked);
        assert!(!summary.clip_v2_root_blocked);
        assert!(summary.registration_endpoint_blocked);
        assert!(summary.application_key_header_blocked);
        assert!(summary.event_stream_path_blocked);
        assert!(!summary.runtime_model_blocked);
        assert!(!summary.spec_ready);
        assert!(!summary.is_clear());
        assert!(summary.has_blockers());
        assert!(summary.needs_release_review());
        assert!(summary.needs_transport_review());
        assert!(!summary.needs_runtime_model_review());
    }

    #[test]
    fn hue_catalog_package_readiness_summary_marks_catalog_ready_package() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_catalog_package_readiness_summary(&plan);

        assert_eq!(summary.spec_summary, hue_package_spec_summary(&plan));
        assert_eq!(summary.required_catalog_check_count, 6);
        assert_eq!(summary.passed_catalog_check_count, 6);
        assert_eq!(summary.missing_catalog_check_count, 0);
        assert!(summary.package_spec_ready);
        assert!(summary.release_ready);
        assert!(summary.catalog_identity_ready);
        assert!(summary.clip_v2_transport_ready);
        assert!(summary.runtime_model_ready);
        assert!(summary.pairing_handoff_ready);
        assert!(summary.catalog_ready);
        assert!(summary.is_catalog_ready());
        assert!(!summary.has_missing_catalog_checks());
        assert!(!summary.transport_or_runtime_blocked());
    }

    #[test]
    fn hue_catalog_package_readiness_summary_counts_handoff_gaps() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HueCatalogPackageReadinessSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_catalog_check_count, 6);
        assert_eq!(summary.passed_catalog_check_count, 2);
        assert_eq!(summary.missing_catalog_check_count, 4);
        assert!(!summary.package_spec_ready);
        assert!(!summary.release_ready);
        assert!(summary.catalog_identity_ready);
        assert!(!summary.clip_v2_transport_ready);
        assert!(summary.runtime_model_ready);
        assert!(!summary.pairing_handoff_ready);
        assert!(!summary.catalog_ready);
        assert!(!summary.is_catalog_ready());
        assert!(summary.has_missing_catalog_checks());
        assert!(summary.transport_or_runtime_blocked());
    }

    #[test]
    fn hue_catalog_package_gap_summary_marks_clear_catalog_handoff() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_catalog_package_gap_summary(&plan);

        assert_eq!(
            summary.catalog_readiness,
            hue_catalog_package_readiness_summary(&plan)
        );
        assert_eq!(summary.blocking_check_count, 0);
        assert!(!summary.package_spec_blocked);
        assert!(!summary.release_blocked);
        assert!(!summary.identity_blocked);
        assert!(!summary.transport_or_runtime_blocked);
        assert!(!summary.pairing_handoff_blocked);
        assert!(summary.catalog_ready);
        assert!(summary.is_clear());
        assert!(!summary.has_blockers());
        assert!(!summary.needs_spec_review());
        assert!(!summary.needs_runtime_handoff_review());
    }

    #[test]
    fn hue_catalog_package_gap_summary_routes_transport_and_pairing_gaps() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HueCatalogPackageGapSummary::from_pairing_plan(&plan);

        assert_eq!(summary.blocking_check_count, 4);
        assert!(summary.package_spec_blocked);
        assert!(summary.release_blocked);
        assert!(!summary.identity_blocked);
        assert!(summary.transport_or_runtime_blocked);
        assert!(summary.pairing_handoff_blocked);
        assert!(!summary.catalog_ready);
        assert!(!summary.is_clear());
        assert!(summary.has_blockers());
        assert!(summary.needs_spec_review());
        assert!(summary.needs_runtime_handoff_review());
    }

    #[test]
    fn hue_catalog_spec_handoff_summary_accepts_clear_catalog_package() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_catalog_spec_handoff_summary(&plan);

        assert_eq!(summary.gap_summary, hue_catalog_package_gap_summary(&plan));
        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 4);
        assert_eq!(summary.missing_handoff_check_count, 0);
        assert!(summary.catalog_ready);
        assert!(summary.spec_review_clear);
        assert!(summary.release_review_clear);
        assert!(summary.runtime_handoff_clear);
        assert!(summary.handoff_accepted);
        assert!(summary.is_handoff_accepted());
        assert!(!summary.has_missing_handoff_checks());
        assert!(!summary.needs_catalog_spec_review());
        assert!(!summary.needs_release_review());
        assert!(!summary.needs_runtime_pairing_review());
    }

    #[test]
    fn hue_catalog_spec_handoff_summary_routes_blocked_runtime_handoff() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HueCatalogSpecHandoffSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 0);
        assert_eq!(summary.missing_handoff_check_count, 4);
        assert!(!summary.catalog_ready);
        assert!(!summary.spec_review_clear);
        assert!(!summary.release_review_clear);
        assert!(!summary.runtime_handoff_clear);
        assert!(!summary.handoff_accepted);
        assert!(!summary.is_handoff_accepted());
        assert!(summary.has_missing_handoff_checks());
        assert!(summary.needs_catalog_spec_review());
        assert!(summary.needs_release_review());
        assert!(summary.needs_runtime_pairing_review());
    }

    #[test]
    fn hue_package_publish_gate_summary_marks_clear_package_publishable() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_publish_gate_summary(&plan);

        assert_eq!(
            summary.handoff_summary,
            hue_catalog_spec_handoff_summary(&plan)
        );
        assert_eq!(summary.required_publish_check_count, 4);
        assert_eq!(summary.passed_publish_check_count, 4);
        assert_eq!(summary.blocked_publish_check_count, 0);
        assert!(summary.handoff_accepted);
        assert!(summary.catalog_spec_review_clear);
        assert!(summary.release_review_clear);
        assert!(summary.runtime_pairing_review_clear);
        assert!(summary.publish_ready);
        assert!(summary.is_publish_ready());
        assert!(!summary.has_publish_blockers());
        assert!(!summary.needs_catalog_spec_queue());
        assert!(!summary.needs_release_queue());
        assert!(!summary.needs_runtime_pairing_queue());
    }

    #[test]
    fn hue_package_publish_gate_summary_routes_blocked_review_queues() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackagePublishGateSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_publish_check_count, 4);
        assert_eq!(summary.passed_publish_check_count, 0);
        assert_eq!(summary.blocked_publish_check_count, 4);
        assert!(!summary.handoff_accepted);
        assert!(!summary.catalog_spec_review_clear);
        assert!(!summary.release_review_clear);
        assert!(!summary.runtime_pairing_review_clear);
        assert!(!summary.publish_ready);
        assert!(!summary.is_publish_ready());
        assert!(summary.has_publish_blockers());
        assert!(summary.needs_catalog_spec_queue());
        assert!(summary.needs_release_queue());
        assert!(summary.needs_runtime_pairing_queue());
    }

    #[test]
    fn hue_package_lifecycle_summary_marks_complete_package_lifecycle() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_lifecycle_summary(&plan);

        assert_eq!(
            summary.publish_gate,
            hue_package_publish_gate_summary(&plan)
        );
        assert_eq!(summary.required_lifecycle_stage_count, 5);
        assert_eq!(summary.passed_lifecycle_stage_count, 5);
        assert_eq!(summary.blocked_lifecycle_stage_count, 0);
        assert!(summary.release_ready);
        assert!(summary.spec_ready);
        assert!(summary.catalog_ready);
        assert!(summary.handoff_accepted);
        assert!(summary.publish_ready);
        assert!(summary.lifecycle_complete);
        assert!(summary.is_lifecycle_complete());
        assert!(!summary.has_blocked_lifecycle_stages());
        assert!(!summary.needs_release_stage());
        assert!(!summary.needs_spec_stage());
        assert!(!summary.needs_catalog_stage());
        assert!(!summary.needs_handoff_stage());
        assert!(!summary.needs_publish_stage());
    }

    #[test]
    fn hue_package_lifecycle_summary_counts_blocked_lifecycle_stages() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageLifecycleSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_lifecycle_stage_count, 5);
        assert_eq!(summary.passed_lifecycle_stage_count, 0);
        assert_eq!(summary.blocked_lifecycle_stage_count, 5);
        assert!(!summary.release_ready);
        assert!(!summary.spec_ready);
        assert!(!summary.catalog_ready);
        assert!(!summary.handoff_accepted);
        assert!(!summary.publish_ready);
        assert!(!summary.lifecycle_complete);
        assert!(!summary.is_lifecycle_complete());
        assert!(summary.has_blocked_lifecycle_stages());
        assert!(summary.needs_release_stage());
        assert!(summary.needs_spec_stage());
        assert!(summary.needs_catalog_stage());
        assert!(summary.needs_handoff_stage());
        assert!(summary.needs_publish_stage());
    }

    #[test]
    fn hue_package_review_queue_summary_marks_clear_package_acceptance() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_review_queue_summary(&plan);

        assert_eq!(
            summary.lifecycle_summary,
            hue_package_lifecycle_summary(&plan)
        );
        assert_eq!(summary.total_review_queue_count, 5);
        assert_eq!(summary.active_review_queue_count, 0);
        assert_eq!(summary.clear_review_queue_count, 5);
        assert!(!summary.release_queue_active);
        assert!(!summary.spec_queue_active);
        assert!(!summary.catalog_queue_active);
        assert!(!summary.handoff_queue_active);
        assert!(!summary.publish_queue_active);
        assert!(summary.package_acceptance_ready);
        assert!(!summary.has_active_review_queues());
        assert!(summary.is_package_acceptance_ready());
        assert!(!summary.needs_release_queue());
        assert!(!summary.needs_spec_queue());
        assert!(!summary.needs_catalog_queue());
        assert!(!summary.needs_handoff_queue());
        assert!(!summary.needs_publish_queue());
    }

    #[test]
    fn hue_package_review_queue_summary_routes_blocked_lifecycle_queues() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReviewQueueSummary::from_pairing_plan(&plan);

        assert_eq!(summary.total_review_queue_count, 5);
        assert_eq!(summary.active_review_queue_count, 5);
        assert_eq!(summary.clear_review_queue_count, 0);
        assert!(summary.release_queue_active);
        assert!(summary.spec_queue_active);
        assert!(summary.catalog_queue_active);
        assert!(summary.handoff_queue_active);
        assert!(summary.publish_queue_active);
        assert!(!summary.package_acceptance_ready);
        assert!(summary.has_active_review_queues());
        assert!(!summary.is_package_acceptance_ready());
        assert!(summary.needs_release_queue());
        assert!(summary.needs_spec_queue());
        assert!(summary.needs_catalog_queue());
        assert!(summary.needs_handoff_queue());
        assert!(summary.needs_publish_queue());
    }

    #[test]
    fn hue_package_acceptance_summary_marks_accepted_package() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_acceptance_summary(&plan);

        assert_eq!(
            summary.review_queue_summary,
            hue_package_review_queue_summary(&plan)
        );
        assert_eq!(summary.required_acceptance_check_count, 4);
        assert_eq!(summary.passed_acceptance_check_count, 4);
        assert_eq!(summary.failed_acceptance_check_count, 0);
        assert!(summary.lifecycle_complete);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.package_accepted);
        assert!(summary.is_package_accepted());
        assert!(!summary.has_acceptance_failures());
        assert!(!summary.needs_lifecycle_completion());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_acceptance_summary_routes_acceptance_failures() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageAcceptanceSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_acceptance_check_count, 4);
        assert_eq!(summary.passed_acceptance_check_count, 0);
        assert_eq!(summary.failed_acceptance_check_count, 4);
        assert!(!summary.lifecycle_complete);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.package_accepted);
        assert!(!summary.is_package_accepted());
        assert!(summary.has_acceptance_failures());
        assert!(summary.needs_lifecycle_completion());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_handoff_summary_marks_ready_package() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_handoff_summary(&plan);

        assert_eq!(
            summary.acceptance_summary,
            hue_package_acceptance_summary(&plan)
        );
        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 4);
        assert_eq!(summary.blocked_handoff_check_count, 0);
        assert!(summary.package_accepted);
        assert!(summary.lifecycle_complete);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_handoff_ready);
        assert!(summary.is_release_handoff_ready());
        assert!(!summary.has_blocked_handoff_checks());
        assert!(!summary.needs_package_acceptance());
        assert!(!summary.needs_lifecycle_completion());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_handoff_summary_routes_blocked_handoff() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseHandoffSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 0);
        assert_eq!(summary.blocked_handoff_check_count, 4);
        assert!(!summary.package_accepted);
        assert!(!summary.lifecycle_complete);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_handoff_ready);
        assert!(!summary.is_release_handoff_ready());
        assert!(summary.has_blocked_handoff_checks());
        assert!(summary.needs_package_acceptance());
        assert!(summary.needs_lifecycle_completion());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_queue_summary_reports_ready_queue() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = HuePackageReleaseQueueSummary::from_pairing_plan(&plan);

        assert_eq!(
            summary.handoff_summary,
            hue_package_release_handoff_summary(&plan)
        );
        assert_eq!(summary.required_release_queue_check_count, 4);
        assert_eq!(summary.queued_release_check_count, 4);
        assert_eq!(summary.blocked_release_queue_check_count, 0);
        assert!(summary.release_handoff_ready);
        assert!(summary.package_accepted);
        assert!(summary.lifecycle_complete);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_queue_ready);
        assert!(summary.is_release_queue_ready());
        assert!(!summary.has_blocked_release_queue_checks());
        assert!(!summary.needs_release_handoff());
        assert!(!summary.needs_package_acceptance());
        assert!(!summary.needs_lifecycle_completion());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_queue_summary_routes_blocked_queue() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseQueueSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_release_queue_check_count, 4);
        assert_eq!(summary.queued_release_check_count, 0);
        assert_eq!(summary.blocked_release_queue_check_count, 4);
        assert!(!summary.release_handoff_ready);
        assert!(!summary.package_accepted);
        assert!(!summary.lifecycle_complete);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_queue_ready);
        assert!(!summary.is_release_queue_ready());
        assert!(summary.has_blocked_release_queue_checks());
        assert!(summary.needs_release_handoff());
        assert!(summary.needs_package_acceptance());
        assert!(summary.needs_lifecycle_completion());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_coordination_summary_reports_ready_coordination() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_coordination_summary(&plan);

        assert_eq!(
            summary.release_queue_summary,
            hue_package_release_queue_summary(&plan)
        );
        assert_eq!(summary.required_coordination_check_count, 5);
        assert_eq!(summary.passed_coordination_check_count, 5);
        assert_eq!(summary.blocked_coordination_check_count, 0);
        assert!(summary.release_queue_ready);
        assert!(summary.release_handoff_ready);
        assert!(summary.package_accepted);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_coordination_ready);
        assert!(summary.is_release_coordination_ready());
        assert!(!summary.has_blocked_coordination_checks());
        assert!(!summary.needs_release_queue());
        assert!(!summary.needs_release_handoff());
        assert!(!summary.needs_package_acceptance());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_coordination_summary_routes_blocked_coordination() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseCoordinationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_coordination_check_count, 5);
        assert_eq!(summary.passed_coordination_check_count, 0);
        assert_eq!(summary.blocked_coordination_check_count, 5);
        assert!(!summary.release_queue_ready);
        assert!(!summary.release_handoff_ready);
        assert!(!summary.package_accepted);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_coordination_ready);
        assert!(!summary.is_release_coordination_ready());
        assert!(summary.has_blocked_coordination_checks());
        assert!(summary.needs_release_queue());
        assert!(summary.needs_release_handoff());
        assert!(summary.needs_package_acceptance());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_dispatch_summary_reports_ready_dispatch() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_dispatch_summary(&plan);

        assert_eq!(
            summary.coordination_summary,
            hue_package_release_coordination_summary(&plan)
        );
        assert_eq!(summary.required_dispatch_check_count, 5);
        assert_eq!(summary.passed_dispatch_check_count, 5);
        assert_eq!(summary.blocked_dispatch_check_count, 0);
        assert!(summary.coordination_ready);
        assert!(summary.release_queue_ready);
        assert!(summary.package_accepted);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_dispatch_ready);
        assert!(summary.is_release_dispatch_ready());
        assert!(!summary.has_blocked_dispatch_checks());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_release_queue());
        assert!(!summary.needs_package_acceptance());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_dispatch_summary_routes_blocked_dispatch() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseDispatchSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_dispatch_check_count, 5);
        assert_eq!(summary.passed_dispatch_check_count, 0);
        assert_eq!(summary.blocked_dispatch_check_count, 5);
        assert!(!summary.coordination_ready);
        assert!(!summary.release_queue_ready);
        assert!(!summary.package_accepted);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_dispatch_ready);
        assert!(!summary.is_release_dispatch_ready());
        assert!(summary.has_blocked_dispatch_checks());
        assert!(summary.needs_coordination());
        assert!(summary.needs_release_queue());
        assert!(summary.needs_package_acceptance());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_operator_summary_reports_ready_operator() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_operator_summary(&plan);

        assert_eq!(
            summary.dispatch_summary,
            hue_package_release_dispatch_summary(&plan)
        );
        assert_eq!(summary.required_operator_check_count, 5);
        assert_eq!(summary.passed_operator_check_count, 5);
        assert_eq!(summary.blocked_operator_check_count, 0);
        assert!(summary.dispatch_ready);
        assert!(summary.coordination_ready);
        assert!(summary.package_accepted);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_operator_ready);
        assert!(summary.is_release_operator_ready());
        assert!(!summary.has_blocked_operator_checks());
        assert!(!summary.needs_dispatch());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_package_acceptance());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_operator_summary_routes_blocked_operator() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseOperatorSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_operator_check_count, 5);
        assert_eq!(summary.passed_operator_check_count, 0);
        assert_eq!(summary.blocked_operator_check_count, 5);
        assert!(!summary.dispatch_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.package_accepted);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_operator_ready);
        assert!(!summary.is_release_operator_ready());
        assert!(summary.has_blocked_operator_checks());
        assert!(summary.needs_dispatch());
        assert!(summary.needs_coordination());
        assert!(summary.needs_package_acceptance());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_audit_summary_reports_ready_audit() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_audit_summary(&plan);

        assert_eq!(
            summary.operator_summary,
            hue_package_release_operator_summary(&plan)
        );
        assert_eq!(summary.required_audit_check_count, 5);
        assert_eq!(summary.passed_audit_check_count, 5);
        assert_eq!(summary.blocked_audit_check_count, 0);
        assert!(summary.operator_ready);
        assert!(summary.dispatch_ready);
        assert!(summary.coordination_ready);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.is_release_audit_ready());
        assert!(!summary.has_blocked_audit_checks());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_dispatch());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_audit_summary_routes_blocked_audit() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseAuditSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_audit_check_count, 5);
        assert_eq!(summary.passed_audit_check_count, 0);
        assert_eq!(summary.blocked_audit_check_count, 5);
        assert!(!summary.operator_ready);
        assert!(!summary.dispatch_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.is_release_audit_ready());
        assert!(summary.has_blocked_audit_checks());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_dispatch());
        assert!(summary.needs_coordination());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_signoff_summary_reports_ready_signoff() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_signoff_summary(&plan);

        assert_eq!(
            summary.audit_summary,
            hue_package_release_audit_summary(&plan)
        );
        assert_eq!(summary.required_signoff_check_count, 6);
        assert_eq!(summary.passed_signoff_check_count, 6);
        assert_eq!(summary.blocked_signoff_check_count, 0);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.dispatch_ready);
        assert!(summary.coordination_ready);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.is_release_signoff_ready());
        assert!(!summary.has_blocked_signoff_checks());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_dispatch());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_signoff_summary_routes_blocked_signoff() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseSignoffSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_signoff_check_count, 6);
        assert_eq!(summary.passed_signoff_check_count, 0);
        assert_eq!(summary.blocked_signoff_check_count, 6);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.dispatch_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.is_release_signoff_ready());
        assert!(summary.has_blocked_signoff_checks());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_dispatch());
        assert!(summary.needs_coordination());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_closure_summary_reports_ready_closure() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_closure_summary(&plan);

        assert_eq!(
            summary.signoff_summary,
            hue_package_release_signoff_summary(&plan)
        );
        assert_eq!(summary.required_closure_check_count, 7);
        assert_eq!(summary.passed_closure_check_count, 7);
        assert_eq!(summary.blocked_closure_check_count, 0);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.dispatch_ready);
        assert!(summary.coordination_ready);
        assert!(summary.review_queues_clear);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.is_release_closure_ready());
        assert!(!summary.has_blocked_closure_checks());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_dispatch());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_review_queue_clearance());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_closure_summary_routes_blocked_closure() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseClosureSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_closure_check_count, 7);
        assert_eq!(summary.passed_closure_check_count, 0);
        assert_eq!(summary.blocked_closure_check_count, 7);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.dispatch_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.review_queues_clear);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.is_release_closure_ready());
        assert!(summary.has_blocked_closure_checks());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_dispatch());
        assert!(summary.needs_coordination());
        assert!(summary.needs_review_queue_clearance());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_summary_reports_ready_archive() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_summary(&plan);

        assert_eq!(
            summary.closure_summary,
            hue_package_release_closure_summary(&plan)
        );
        assert_eq!(summary.required_archive_check_count, 6);
        assert_eq!(summary.passed_archive_check_count, 6);
        assert_eq!(summary.blocked_archive_check_count, 0);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.is_release_archive_ready());
        assert!(!summary.has_blocked_archive_checks());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_summary_routes_blocked_archive() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_check_count, 6);
        assert_eq!(summary.passed_archive_check_count, 0);
        assert_eq!(summary.blocked_archive_check_count, 6);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.is_release_archive_ready());
        assert!(summary.has_blocked_archive_checks());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_signoff_summary_reports_ready_archive_signoff() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_signoff_summary(&plan);

        assert_eq!(
            summary.archive_summary,
            hue_package_release_archive_summary(&plan)
        );
        assert_eq!(summary.required_archive_signoff_check_count, 7);
        assert_eq!(summary.passed_archive_signoff_check_count, 7);
        assert_eq!(summary.blocked_archive_signoff_check_count, 0);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.is_release_archive_signoff_ready());
        assert!(!summary.has_blocked_archive_signoff_checks());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_signoff_summary_routes_blocked_archive_signoff() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveSignoffSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_signoff_check_count, 7);
        assert_eq!(summary.passed_archive_signoff_check_count, 0);
        assert_eq!(summary.blocked_archive_signoff_check_count, 7);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.is_release_archive_signoff_ready());
        assert!(summary.has_blocked_archive_signoff_checks());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_closure_summary_reports_ready_archive_closure() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_closure_summary(&plan);

        assert_eq!(
            summary.archive_signoff_summary,
            hue_package_release_archive_signoff_summary(&plan)
        );
        assert_eq!(summary.required_archive_closure_check_count, 8);
        assert_eq!(summary.passed_archive_closure_check_count, 8);
        assert_eq!(summary.blocked_archive_closure_check_count, 0);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.is_release_archive_closure_ready());
        assert!(!summary.has_blocked_archive_closure_checks());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_closure_summary_routes_blocked_archive_closure() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveClosureSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_closure_check_count, 8);
        assert_eq!(summary.passed_archive_closure_check_count, 0);
        assert_eq!(summary.blocked_archive_closure_check_count, 8);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.is_release_archive_closure_ready());
        assert!(summary.has_blocked_archive_closure_checks());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_handoff_summary_reports_ready_archive_handoff() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_handoff_summary(&plan);

        assert_eq!(
            summary.archive_closure_summary,
            hue_package_release_archive_closure_summary(&plan)
        );
        assert_eq!(summary.required_archive_handoff_check_count, 9);
        assert_eq!(summary.passed_archive_handoff_check_count, 9);
        assert_eq!(summary.blocked_archive_handoff_check_count, 0);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.is_release_archive_handoff_ready());
        assert!(!summary.has_blocked_archive_handoff_checks());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_handoff_summary_routes_blocked_archive_handoff() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveHandoffSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_handoff_check_count, 9);
        assert_eq!(summary.passed_archive_handoff_check_count, 0);
        assert_eq!(summary.blocked_archive_handoff_check_count, 9);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.is_release_archive_handoff_ready());
        assert!(summary.has_blocked_archive_handoff_checks());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_dispatch_summary_reports_ready_archive_dispatch() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_dispatch_summary(&plan);

        assert_eq!(
            summary.archive_handoff_summary,
            hue_package_release_archive_handoff_summary(&plan)
        );
        assert_eq!(summary.required_archive_dispatch_check_count, 10);
        assert_eq!(summary.passed_archive_dispatch_check_count, 10);
        assert_eq!(summary.blocked_archive_dispatch_check_count, 0);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.is_release_archive_dispatch_ready());
        assert!(!summary.has_blocked_archive_dispatch_checks());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_dispatch_summary_routes_blocked_archive_dispatch() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveDispatchSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_dispatch_check_count, 10);
        assert_eq!(summary.passed_archive_dispatch_check_count, 0);
        assert_eq!(summary.blocked_archive_dispatch_check_count, 10);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.is_release_archive_dispatch_ready());
        assert!(summary.has_blocked_archive_dispatch_checks());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_operator_summary_reports_ready_archive_operator() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_operator_summary(&plan);

        assert_eq!(
            summary.archive_dispatch_summary,
            hue_package_release_archive_dispatch_summary(&plan)
        );
        assert_eq!(summary.required_archive_operator_check_count, 11);
        assert_eq!(summary.passed_archive_operator_check_count, 11);
        assert_eq!(summary.blocked_archive_operator_check_count, 0);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.is_release_archive_operator_ready());
        assert!(!summary.has_blocked_archive_operator_checks());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_operator_summary_routes_blocked_archive_operator() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveOperatorSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_operator_check_count, 11);
        assert_eq!(summary.passed_archive_operator_check_count, 0);
        assert_eq!(summary.blocked_archive_operator_check_count, 11);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.is_release_archive_operator_ready());
        assert!(summary.has_blocked_archive_operator_checks());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_supervisor_summary_reports_ready_archive_supervisor() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_supervisor_summary(&plan);

        assert_eq!(
            summary.archive_operator_summary,
            hue_package_release_archive_operator_summary(&plan)
        );
        assert_eq!(summary.required_archive_supervisor_check_count, 12);
        assert_eq!(summary.passed_archive_supervisor_check_count, 12);
        assert_eq!(summary.blocked_archive_supervisor_check_count, 0);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.is_release_archive_supervisor_ready());
        assert!(!summary.has_blocked_archive_supervisor_checks());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_supervisor_summary_routes_blocked_archive_supervisor() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveSupervisorSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_supervisor_check_count, 12);
        assert_eq!(summary.passed_archive_supervisor_check_count, 0);
        assert_eq!(summary.blocked_archive_supervisor_check_count, 12);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.is_release_archive_supervisor_ready());
        assert!(summary.has_blocked_archive_supervisor_checks());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_completion_summary_reports_ready_archive_completion() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_completion_summary(&plan);

        assert_eq!(
            summary.archive_supervisor_summary,
            hue_package_release_archive_supervisor_summary(&plan)
        );
        assert_eq!(summary.required_archive_completion_check_count, 13);
        assert_eq!(summary.passed_archive_completion_check_count, 13);
        assert_eq!(summary.blocked_archive_completion_check_count, 0);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.is_release_archive_completion_ready());
        assert!(!summary.has_blocked_archive_completion_checks());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_completion_summary_routes_blocked_archive_completion() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveCompletionSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_completion_check_count, 13);
        assert_eq!(summary.passed_archive_completion_check_count, 0);
        assert_eq!(summary.blocked_archive_completion_check_count, 13);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.is_release_archive_completion_ready());
        assert!(summary.has_blocked_archive_completion_checks());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_publication_summary_reports_ready_archive_publication() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_publication_summary(&plan);

        assert_eq!(
            summary.archive_completion_summary,
            hue_package_release_archive_completion_summary(&plan)
        );
        assert_eq!(summary.required_archive_publication_check_count, 14);
        assert_eq!(summary.passed_archive_publication_check_count, 14);
        assert_eq!(summary.blocked_archive_publication_check_count, 0);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.is_release_archive_publication_ready());
        assert!(!summary.has_blocked_archive_publication_checks());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_publication_summary_routes_blocked_archive_publication() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchivePublicationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_publication_check_count, 14);
        assert_eq!(summary.passed_archive_publication_check_count, 0);
        assert_eq!(summary.blocked_archive_publication_check_count, 14);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.is_release_archive_publication_ready());
        assert!(summary.has_blocked_archive_publication_checks());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_verification_summary_reports_ready_archive_verification() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_verification_summary(&plan);

        assert_eq!(
            summary.archive_publication_summary,
            hue_package_release_archive_publication_summary(&plan)
        );
        assert_eq!(summary.required_archive_verification_check_count, 15);
        assert_eq!(summary.passed_archive_verification_check_count, 15);
        assert_eq!(summary.blocked_archive_verification_check_count, 0);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.is_release_archive_verification_ready());
        assert!(!summary.has_blocked_archive_verification_checks());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_verification_summary_routes_blocked_archive_verification() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveVerificationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_verification_check_count, 15);
        assert_eq!(summary.passed_archive_verification_check_count, 0);
        assert_eq!(summary.blocked_archive_verification_check_count, 15);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.is_release_archive_verification_ready());
        assert!(summary.has_blocked_archive_verification_checks());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_validation_summary_reports_ready_archive_validation() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_validation_summary(&plan);

        assert_eq!(
            summary.archive_verification_summary,
            hue_package_release_archive_verification_summary(&plan)
        );
        assert_eq!(summary.required_archive_validation_check_count, 16);
        assert_eq!(summary.passed_archive_validation_check_count, 16);
        assert_eq!(summary.blocked_archive_validation_check_count, 0);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.is_release_archive_validation_ready());
        assert!(!summary.has_blocked_archive_validation_checks());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_validation_summary_routes_blocked_archive_validation() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveValidationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_validation_check_count, 16);
        assert_eq!(summary.passed_archive_validation_check_count, 0);
        assert_eq!(summary.blocked_archive_validation_check_count, 16);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.is_release_archive_validation_ready());
        assert!(summary.has_blocked_archive_validation_checks());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_certification_summary_reports_ready_archive_certification() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_certification_summary(&plan);

        assert_eq!(
            summary.archive_validation_summary,
            hue_package_release_archive_validation_summary(&plan)
        );
        assert_eq!(summary.required_archive_certification_check_count, 17);
        assert_eq!(summary.passed_archive_certification_check_count, 17);
        assert_eq!(summary.blocked_archive_certification_check_count, 0);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.is_release_archive_certification_ready());
        assert!(!summary.has_blocked_archive_certification_checks());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_certification_summary_routes_blocked_archive_certification() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveCertificationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_certification_check_count, 17);
        assert_eq!(summary.passed_archive_certification_check_count, 0);
        assert_eq!(summary.blocked_archive_certification_check_count, 17);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.is_release_archive_certification_ready());
        assert!(summary.has_blocked_archive_certification_checks());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_approval_summary_reports_ready_archive_approval() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_approval_summary(&plan);

        assert_eq!(
            summary.archive_certification_summary,
            hue_package_release_archive_certification_summary(&plan)
        );
        assert_eq!(summary.required_archive_approval_check_count, 18);
        assert_eq!(summary.passed_archive_approval_check_count, 18);
        assert_eq!(summary.blocked_archive_approval_check_count, 0);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.is_release_archive_approval_ready());
        assert!(!summary.has_blocked_archive_approval_checks());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_approval_summary_routes_blocked_archive_approval() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveApprovalSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_approval_check_count, 18);
        assert_eq!(summary.passed_archive_approval_check_count, 0);
        assert_eq!(summary.blocked_archive_approval_check_count, 18);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.is_release_archive_approval_ready());
        assert!(summary.has_blocked_archive_approval_checks());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_activation_summary_reports_ready_archive_activation() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_activation_summary(&plan);

        assert_eq!(
            summary.archive_approval_summary,
            hue_package_release_archive_approval_summary(&plan)
        );
        assert_eq!(summary.required_archive_activation_check_count, 19);
        assert_eq!(summary.passed_archive_activation_check_count, 19);
        assert_eq!(summary.blocked_archive_activation_check_count, 0);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.is_release_archive_activation_ready());
        assert!(!summary.has_blocked_archive_activation_checks());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_activation_summary_routes_blocked_archive_activation() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveActivationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_activation_check_count, 19);
        assert_eq!(summary.passed_archive_activation_check_count, 0);
        assert_eq!(summary.blocked_archive_activation_check_count, 19);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.is_release_archive_activation_ready());
        assert!(summary.has_blocked_archive_activation_checks());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_rollout_summary_reports_ready_archive_rollout() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_rollout_summary(&plan);

        assert_eq!(
            summary.archive_activation_summary,
            hue_package_release_archive_activation_summary(&plan)
        );
        assert_eq!(summary.required_archive_rollout_check_count, 20);
        assert_eq!(summary.passed_archive_rollout_check_count, 20);
        assert_eq!(summary.blocked_archive_rollout_check_count, 0);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.is_release_archive_rollout_ready());
        assert!(!summary.has_blocked_archive_rollout_checks());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_rollout_summary_routes_blocked_archive_rollout() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveRolloutSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_rollout_check_count, 20);
        assert_eq!(summary.passed_archive_rollout_check_count, 0);
        assert_eq!(summary.blocked_archive_rollout_check_count, 20);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.is_release_archive_rollout_ready());
        assert!(summary.has_blocked_archive_rollout_checks());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_adoption_summary_reports_ready_archive_adoption() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_adoption_summary(&plan);

        assert_eq!(
            summary.archive_rollout_summary,
            hue_package_release_archive_rollout_summary(&plan)
        );
        assert_eq!(summary.required_archive_adoption_check_count, 21);
        assert_eq!(summary.passed_archive_adoption_check_count, 21);
        assert_eq!(summary.blocked_archive_adoption_check_count, 0);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.is_release_archive_adoption_ready());
        assert!(!summary.has_blocked_archive_adoption_checks());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_adoption_summary_routes_blocked_archive_adoption() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveAdoptionSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_adoption_check_count, 21);
        assert_eq!(summary.passed_archive_adoption_check_count, 0);
        assert_eq!(summary.blocked_archive_adoption_check_count, 21);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.is_release_archive_adoption_ready());
        assert!(summary.has_blocked_archive_adoption_checks());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_acceptance_summary_reports_ready_archive_acceptance() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_acceptance_summary(&plan);

        assert_eq!(
            summary.archive_adoption_summary,
            hue_package_release_archive_adoption_summary(&plan)
        );
        assert_eq!(summary.required_archive_acceptance_check_count, 22);
        assert_eq!(summary.passed_archive_acceptance_check_count, 22);
        assert_eq!(summary.blocked_archive_acceptance_check_count, 0);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.is_release_archive_acceptance_ready());
        assert!(!summary.has_blocked_archive_acceptance_checks());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_acceptance_summary_routes_blocked_archive_acceptance() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveAcceptanceSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_acceptance_check_count, 22);
        assert_eq!(summary.passed_archive_acceptance_check_count, 0);
        assert_eq!(summary.blocked_archive_acceptance_check_count, 22);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.is_release_archive_acceptance_ready());
        assert!(summary.has_blocked_archive_acceptance_checks());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_distribution_summary_reports_ready_archive_distribution() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_distribution_summary(&plan);

        assert_eq!(
            summary.archive_acceptance_summary,
            hue_package_release_archive_acceptance_summary(&plan)
        );
        assert_eq!(summary.required_archive_distribution_check_count, 23);
        assert_eq!(summary.passed_archive_distribution_check_count, 23);
        assert_eq!(summary.blocked_archive_distribution_check_count, 0);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.is_release_archive_distribution_ready());
        assert!(!summary.has_blocked_archive_distribution_checks());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_distribution_summary_routes_blocked_archive_distribution() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveDistributionSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_distribution_check_count, 23);
        assert_eq!(summary.passed_archive_distribution_check_count, 0);
        assert_eq!(summary.blocked_archive_distribution_check_count, 23);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.is_release_archive_distribution_ready());
        assert!(summary.has_blocked_archive_distribution_checks());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_export_summary_reports_ready_archive_export() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_export_summary(&plan);

        assert_eq!(
            summary.archive_distribution_summary,
            hue_package_release_archive_distribution_summary(&plan)
        );
        assert_eq!(summary.required_archive_export_check_count, 24);
        assert_eq!(summary.passed_archive_export_check_count, 24);
        assert_eq!(summary.blocked_archive_export_check_count, 0);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.is_release_archive_export_ready());
        assert!(!summary.has_blocked_archive_export_checks());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_export_summary_routes_blocked_archive_export() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveExportSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_export_check_count, 24);
        assert_eq!(summary.passed_archive_export_check_count, 0);
        assert_eq!(summary.blocked_archive_export_check_count, 24);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.is_release_archive_export_ready());
        assert!(summary.has_blocked_archive_export_checks());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_import_summary_reports_ready_archive_import() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_import_summary(&plan);

        assert_eq!(
            summary.archive_export_summary,
            hue_package_release_archive_export_summary(&plan)
        );
        assert_eq!(summary.required_archive_import_check_count, 25);
        assert_eq!(summary.passed_archive_import_check_count, 25);
        assert_eq!(summary.blocked_archive_import_check_count, 0);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.is_release_archive_import_ready());
        assert!(!summary.has_blocked_archive_import_checks());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_import_summary_routes_blocked_archive_import() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveImportSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_import_check_count, 25);
        assert_eq!(summary.passed_archive_import_check_count, 0);
        assert_eq!(summary.blocked_archive_import_check_count, 25);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.is_release_archive_import_ready());
        assert!(summary.has_blocked_archive_import_checks());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_ingest_summary_reports_ready_archive_ingest() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_ingest_summary(&plan);

        assert_eq!(
            summary.archive_import_summary,
            hue_package_release_archive_import_summary(&plan)
        );
        assert_eq!(summary.required_archive_ingest_check_count, 26);
        assert_eq!(summary.passed_archive_ingest_check_count, 26);
        assert_eq!(summary.blocked_archive_ingest_check_count, 0);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.is_release_archive_ingest_ready());
        assert!(!summary.has_blocked_archive_ingest_checks());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_ingest_summary_routes_blocked_archive_ingest() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveIngestSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_ingest_check_count, 26);
        assert_eq!(summary.passed_archive_ingest_check_count, 0);
        assert_eq!(summary.blocked_archive_ingest_check_count, 26);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.is_release_archive_ingest_ready());
        assert!(summary.has_blocked_archive_ingest_checks());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_load_summary_reports_ready_archive_load() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_load_summary(&plan);

        assert_eq!(
            summary.archive_ingest_summary,
            hue_package_release_archive_ingest_summary(&plan)
        );
        assert_eq!(summary.required_archive_load_check_count, 27);
        assert_eq!(summary.passed_archive_load_check_count, 27);
        assert_eq!(summary.blocked_archive_load_check_count, 0);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.is_release_archive_load_ready());
        assert!(!summary.has_blocked_archive_load_checks());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_load_summary_routes_blocked_archive_load() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveLoadSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_load_check_count, 27);
        assert_eq!(summary.passed_archive_load_check_count, 0);
        assert_eq!(summary.blocked_archive_load_check_count, 27);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.is_release_archive_load_ready());
        assert!(summary.has_blocked_archive_load_checks());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_restore_summary_reports_ready_archive_restore() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_restore_summary(&plan);

        assert_eq!(
            summary.archive_load_summary,
            hue_package_release_archive_load_summary(&plan)
        );
        assert_eq!(summary.required_archive_restore_check_count, 28);
        assert_eq!(summary.passed_archive_restore_check_count, 28);
        assert_eq!(summary.blocked_archive_restore_check_count, 0);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.is_release_archive_restore_ready());
        assert!(!summary.has_blocked_archive_restore_checks());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_restore_summary_routes_blocked_archive_restore() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveRestoreSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_restore_check_count, 28);
        assert_eq!(summary.passed_archive_restore_check_count, 0);
        assert_eq!(summary.blocked_archive_restore_check_count, 28);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.is_release_archive_restore_ready());
        assert!(summary.has_blocked_archive_restore_checks());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_recovery_summary_reports_ready_archive_recovery() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_recovery_summary(&plan);

        assert_eq!(
            summary.archive_restore_summary,
            hue_package_release_archive_restore_summary(&plan)
        );
        assert_eq!(summary.required_archive_recovery_check_count, 29);
        assert_eq!(summary.passed_archive_recovery_check_count, 29);
        assert_eq!(summary.blocked_archive_recovery_check_count, 0);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.is_release_archive_recovery_ready());
        assert!(!summary.has_blocked_archive_recovery_checks());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_recovery_summary_routes_blocked_archive_recovery() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveRecoverySummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_recovery_check_count, 29);
        assert_eq!(summary.passed_archive_recovery_check_count, 0);
        assert_eq!(summary.blocked_archive_recovery_check_count, 29);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.is_release_archive_recovery_ready());
        assert!(summary.has_blocked_archive_recovery_checks());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_replay_summary_reports_ready_archive_replay() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_replay_summary(&plan);

        assert_eq!(
            summary.archive_recovery_summary,
            hue_package_release_archive_recovery_summary(&plan)
        );
        assert_eq!(summary.required_archive_replay_check_count, 30);
        assert_eq!(summary.passed_archive_replay_check_count, 30);
        assert_eq!(summary.blocked_archive_replay_check_count, 0);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.is_release_archive_replay_ready());
        assert!(!summary.has_blocked_archive_replay_checks());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_replay_summary_routes_blocked_archive_replay() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveReplaySummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_replay_check_count, 30);
        assert_eq!(summary.passed_archive_replay_check_count, 0);
        assert_eq!(summary.blocked_archive_replay_check_count, 30);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.is_release_archive_replay_ready());
        assert!(summary.has_blocked_archive_replay_checks());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_reconciliation_summary_reports_ready_archive_reconciliation() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_reconciliation_summary(&plan);

        assert_eq!(
            summary.archive_replay_summary,
            hue_package_release_archive_replay_summary(&plan)
        );
        assert_eq!(summary.required_archive_reconciliation_check_count, 31);
        assert_eq!(summary.passed_archive_reconciliation_check_count, 31);
        assert_eq!(summary.blocked_archive_reconciliation_check_count, 0);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.is_release_archive_reconciliation_ready());
        assert!(!summary.has_blocked_archive_reconciliation_checks());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_reconciliation_summary_routes_blocked_archive_reconciliation() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveReconciliationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_reconciliation_check_count, 31);
        assert_eq!(summary.passed_archive_reconciliation_check_count, 0);
        assert_eq!(summary.blocked_archive_reconciliation_check_count, 31);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.is_release_archive_reconciliation_ready());
        assert!(summary.has_blocked_archive_reconciliation_checks());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_settlement_summary_reports_ready_archive_settlement() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_settlement_summary(&plan);

        assert_eq!(
            summary.archive_reconciliation_summary,
            hue_package_release_archive_reconciliation_summary(&plan)
        );
        assert_eq!(summary.required_archive_settlement_check_count, 32);
        assert_eq!(summary.passed_archive_settlement_check_count, 32);
        assert_eq!(summary.blocked_archive_settlement_check_count, 0);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.is_release_archive_settlement_ready());
        assert!(!summary.has_blocked_archive_settlement_checks());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_settlement_summary_routes_blocked_archive_settlement() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveSettlementSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_settlement_check_count, 32);
        assert_eq!(summary.passed_archive_settlement_check_count, 0);
        assert_eq!(summary.blocked_archive_settlement_check_count, 32);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.is_release_archive_settlement_ready());
        assert!(summary.has_blocked_archive_settlement_checks());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_finalization_summary_reports_ready_archive_finalization() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_finalization_summary(&plan);

        assert_eq!(
            summary.archive_settlement_summary,
            hue_package_release_archive_settlement_summary(&plan)
        );
        assert_eq!(summary.required_archive_finalization_check_count, 33);
        assert_eq!(summary.passed_archive_finalization_check_count, 33);
        assert_eq!(summary.blocked_archive_finalization_check_count, 0);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_finalization_ready);
        assert!(summary.is_release_archive_finalization_ready());
        assert!(!summary.has_blocked_archive_finalization_checks());
        assert!(!summary.needs_release_archive_settlement());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_finalization_summary_routes_blocked_archive_finalization() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveFinalizationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_finalization_check_count, 33);
        assert_eq!(summary.passed_archive_finalization_check_count, 0);
        assert_eq!(summary.blocked_archive_finalization_check_count, 33);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_finalization_ready);
        assert!(!summary.is_release_archive_finalization_ready());
        assert!(summary.has_blocked_archive_finalization_checks());
        assert!(summary.needs_release_archive_settlement());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_confirmation_summary_reports_ready_archive_confirmation() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_confirmation_summary(&plan);

        assert_eq!(
            summary.archive_finalization_summary,
            hue_package_release_archive_finalization_summary(&plan)
        );
        assert_eq!(summary.required_archive_confirmation_check_count, 34);
        assert_eq!(summary.passed_archive_confirmation_check_count, 34);
        assert_eq!(summary.blocked_archive_confirmation_check_count, 0);
        assert!(summary.release_archive_finalization_ready);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_confirmation_ready);
        assert!(summary.is_release_archive_confirmation_ready());
        assert!(!summary.has_blocked_archive_confirmation_checks());
        assert!(!summary.needs_release_archive_finalization());
        assert!(!summary.needs_release_archive_settlement());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_confirmation_summary_routes_blocked_archive_confirmation() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveConfirmationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_confirmation_check_count, 34);
        assert_eq!(summary.passed_archive_confirmation_check_count, 0);
        assert_eq!(summary.blocked_archive_confirmation_check_count, 34);
        assert!(!summary.release_archive_finalization_ready);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_confirmation_ready);
        assert!(!summary.is_release_archive_confirmation_ready());
        assert!(summary.has_blocked_archive_confirmation_checks());
        assert!(summary.needs_release_archive_finalization());
        assert!(summary.needs_release_archive_settlement());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_attestation_summary_reports_ready_archive_attestation() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_attestation_summary(&plan);

        assert_eq!(
            summary.archive_confirmation_summary,
            hue_package_release_archive_confirmation_summary(&plan)
        );
        assert_eq!(summary.required_archive_attestation_check_count, 35);
        assert_eq!(summary.passed_archive_attestation_check_count, 35);
        assert_eq!(summary.blocked_archive_attestation_check_count, 0);
        assert!(summary.release_archive_confirmation_ready);
        assert!(summary.release_archive_finalization_ready);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_attestation_ready);
        assert!(summary.is_release_archive_attestation_ready());
        assert!(!summary.has_blocked_archive_attestation_checks());
        assert!(!summary.needs_release_archive_confirmation());
        assert!(!summary.needs_release_archive_finalization());
        assert!(!summary.needs_release_archive_settlement());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_attestation_summary_routes_blocked_archive_attestation() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveAttestationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_attestation_check_count, 35);
        assert_eq!(summary.passed_archive_attestation_check_count, 0);
        assert_eq!(summary.blocked_archive_attestation_check_count, 35);
        assert!(!summary.release_archive_confirmation_ready);
        assert!(!summary.release_archive_finalization_ready);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_attestation_ready);
        assert!(!summary.is_release_archive_attestation_ready());
        assert!(summary.has_blocked_archive_attestation_checks());
        assert!(summary.needs_release_archive_confirmation());
        assert!(summary.needs_release_archive_finalization());
        assert!(summary.needs_release_archive_settlement());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_evidence_summary_reports_ready_archive_evidence() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_evidence_summary(&plan);

        assert_eq!(
            summary.archive_attestation_summary,
            hue_package_release_archive_attestation_summary(&plan)
        );
        assert_eq!(summary.required_archive_evidence_check_count, 36);
        assert_eq!(summary.passed_archive_evidence_check_count, 36);
        assert_eq!(summary.blocked_archive_evidence_check_count, 0);
        assert!(summary.release_archive_attestation_ready);
        assert!(summary.release_archive_confirmation_ready);
        assert!(summary.release_archive_finalization_ready);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_evidence_ready);
        assert!(summary.is_release_archive_evidence_ready());
        assert!(!summary.has_blocked_archive_evidence_checks());
        assert!(!summary.needs_release_archive_attestation());
        assert!(!summary.needs_release_archive_confirmation());
        assert!(!summary.needs_release_archive_finalization());
        assert!(!summary.needs_release_archive_settlement());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_evidence_summary_routes_blocked_archive_evidence() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveEvidenceSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_evidence_check_count, 36);
        assert_eq!(summary.passed_archive_evidence_check_count, 0);
        assert_eq!(summary.blocked_archive_evidence_check_count, 36);
        assert!(!summary.release_archive_attestation_ready);
        assert!(!summary.release_archive_confirmation_ready);
        assert!(!summary.release_archive_finalization_ready);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_evidence_ready);
        assert!(!summary.is_release_archive_evidence_ready());
        assert!(summary.has_blocked_archive_evidence_checks());
        assert!(summary.needs_release_archive_attestation());
        assert!(summary.needs_release_archive_confirmation());
        assert!(summary.needs_release_archive_finalization());
        assert!(summary.needs_release_archive_settlement());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_evidence_ledger_summary_reports_ready_archive_evidence_ledger() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_evidence_ledger_summary(&plan);

        assert_eq!(
            summary.archive_evidence_summary,
            hue_package_release_archive_evidence_summary(&plan)
        );
        assert_eq!(summary.required_archive_evidence_ledger_check_count, 37);
        assert_eq!(summary.passed_archive_evidence_ledger_check_count, 37);
        assert_eq!(summary.blocked_archive_evidence_ledger_check_count, 0);
        assert!(summary.release_archive_evidence_ready);
        assert!(summary.release_archive_attestation_ready);
        assert!(summary.release_archive_confirmation_ready);
        assert!(summary.release_archive_finalization_ready);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_archive_evidence_ledger_ready);
        assert!(summary.is_release_archive_evidence_ledger_ready());
        assert!(!summary.has_blocked_archive_evidence_ledger_checks());
        assert!(!summary.needs_release_archive_evidence());
        assert!(!summary.needs_release_archive_attestation());
        assert!(!summary.needs_release_archive_confirmation());
        assert!(!summary.needs_release_archive_finalization());
        assert!(!summary.needs_release_archive_settlement());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_archive_evidence_ledger_summary_routes_blocked_archive_evidence_ledger()
    {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveEvidenceLedgerSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_evidence_ledger_check_count, 37);
        assert_eq!(summary.passed_archive_evidence_ledger_check_count, 0);
        assert_eq!(summary.blocked_archive_evidence_ledger_check_count, 37);
        assert!(!summary.release_archive_evidence_ready);
        assert!(!summary.release_archive_attestation_ready);
        assert!(!summary.release_archive_confirmation_ready);
        assert!(!summary.release_archive_finalization_ready);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_archive_evidence_ledger_ready);
        assert!(!summary.is_release_archive_evidence_ledger_ready());
        assert!(summary.has_blocked_archive_evidence_ledger_checks());
        assert!(summary.needs_release_archive_evidence());
        assert!(summary.needs_release_archive_attestation());
        assert!(summary.needs_release_archive_confirmation());
        assert!(summary.needs_release_archive_finalization());
        assert!(summary.needs_release_archive_settlement());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_readiness_evidence_summary_reports_ready_release_evidence() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_readiness_evidence_summary(&plan);

        assert_eq!(
            summary.release_readiness_summary,
            hue_package_release_readiness_summary(&plan)
        );
        assert_eq!(
            summary.archive_evidence_ledger_summary,
            hue_package_release_archive_evidence_ledger_summary(&plan)
        );
        assert_eq!(summary.required_release_readiness_evidence_check_count, 44);
        assert_eq!(summary.passed_release_readiness_evidence_check_count, 44);
        assert_eq!(summary.blocked_release_readiness_evidence_check_count, 0);
        assert!(summary.worker_process_ready);
        assert!(summary.command_flow_ready);
        assert!(summary.pairing_flow_ready);
        assert!(summary.event_stream_ready);
        assert!(summary.physical_presence_required);
        assert!(summary.package_release_ready);
        assert!(summary.release_archive_evidence_ledger_ready);
        assert!(summary.release_archive_evidence_ready);
        assert!(summary.release_archive_attestation_ready);
        assert!(summary.release_archive_confirmation_ready);
        assert!(summary.release_archive_finalization_ready);
        assert!(summary.release_archive_settlement_ready);
        assert!(summary.release_archive_reconciliation_ready);
        assert!(summary.release_archive_replay_ready);
        assert!(summary.release_archive_recovery_ready);
        assert!(summary.release_archive_restore_ready);
        assert!(summary.release_archive_load_ready);
        assert!(summary.release_archive_ingest_ready);
        assert!(summary.release_archive_import_ready);
        assert!(summary.release_archive_export_ready);
        assert!(summary.release_archive_distribution_ready);
        assert!(summary.release_archive_acceptance_ready);
        assert!(summary.release_archive_adoption_ready);
        assert!(summary.release_archive_rollout_ready);
        assert!(summary.release_archive_activation_ready);
        assert!(summary.release_archive_approval_ready);
        assert!(summary.release_archive_certification_ready);
        assert!(summary.release_archive_validation_ready);
        assert!(summary.release_archive_verification_ready);
        assert!(summary.release_archive_publication_ready);
        assert!(summary.release_archive_completion_ready);
        assert!(summary.release_archive_supervisor_ready);
        assert!(summary.release_archive_operator_ready);
        assert!(summary.release_archive_dispatch_ready);
        assert!(summary.release_archive_handoff_ready);
        assert!(summary.release_archive_closure_ready);
        assert!(summary.release_archive_signoff_ready);
        assert!(summary.release_archive_ready);
        assert!(summary.release_closure_ready);
        assert!(summary.release_signoff_ready);
        assert!(summary.release_audit_ready);
        assert!(summary.operator_ready);
        assert!(summary.coordination_ready);
        assert!(summary.publish_gate_ready);
        assert!(summary.release_readiness_evidence_ready);
        assert!(summary.is_release_readiness_evidence_ready());
        assert!(!summary.has_blocked_release_readiness_evidence_checks());
        assert!(!summary.needs_worker_process());
        assert!(!summary.needs_command_flow());
        assert!(!summary.needs_pairing_flow());
        assert!(!summary.needs_event_stream());
        assert!(!summary.needs_physical_presence_requirement());
        assert!(!summary.needs_package_release());
        assert!(!summary.needs_release_archive_evidence_ledger());
        assert!(!summary.needs_release_archive_evidence());
        assert!(!summary.needs_release_archive_attestation());
        assert!(!summary.needs_release_archive_confirmation());
        assert!(!summary.needs_release_archive_finalization());
        assert!(!summary.needs_release_archive_settlement());
        assert!(!summary.needs_release_archive_reconciliation());
        assert!(!summary.needs_release_archive_replay());
        assert!(!summary.needs_release_archive_recovery());
        assert!(!summary.needs_release_archive_restore());
        assert!(!summary.needs_release_archive_load());
        assert!(!summary.needs_release_archive_ingest());
        assert!(!summary.needs_release_archive_import());
        assert!(!summary.needs_release_archive_export());
        assert!(!summary.needs_release_archive_distribution());
        assert!(!summary.needs_release_archive_acceptance());
        assert!(!summary.needs_release_archive_adoption());
        assert!(!summary.needs_release_archive_rollout());
        assert!(!summary.needs_release_archive_activation());
        assert!(!summary.needs_release_archive_approval());
        assert!(!summary.needs_release_archive_certification());
        assert!(!summary.needs_release_archive_validation());
        assert!(!summary.needs_release_archive_verification());
        assert!(!summary.needs_release_archive_publication());
        assert!(!summary.needs_release_archive_completion());
        assert!(!summary.needs_release_archive_supervisor());
        assert!(!summary.needs_release_archive_operator());
        assert!(!summary.needs_release_archive_dispatch());
        assert!(!summary.needs_release_archive_handoff());
        assert!(!summary.needs_release_archive_closure());
        assert!(!summary.needs_release_archive_signoff());
        assert!(!summary.needs_release_archive());
        assert!(!summary.needs_release_closure());
        assert!(!summary.needs_release_signoff());
        assert!(!summary.needs_release_audit());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_coordination());
        assert!(!summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_readiness_evidence_summary_routes_blocked_release_evidence() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseReadinessEvidenceSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_release_readiness_evidence_check_count, 44);
        assert_eq!(summary.passed_release_readiness_evidence_check_count, 2);
        assert_eq!(summary.blocked_release_readiness_evidence_check_count, 42);
        assert!(summary.worker_process_ready);
        assert!(summary.command_flow_ready);
        assert!(!summary.pairing_flow_ready);
        assert!(!summary.event_stream_ready);
        assert!(!summary.physical_presence_required);
        assert!(!summary.package_release_ready);
        assert!(!summary.release_archive_evidence_ledger_ready);
        assert!(!summary.release_archive_evidence_ready);
        assert!(!summary.release_archive_attestation_ready);
        assert!(!summary.release_archive_confirmation_ready);
        assert!(!summary.release_archive_finalization_ready);
        assert!(!summary.release_archive_settlement_ready);
        assert!(!summary.release_archive_reconciliation_ready);
        assert!(!summary.release_archive_replay_ready);
        assert!(!summary.release_archive_recovery_ready);
        assert!(!summary.release_archive_restore_ready);
        assert!(!summary.release_archive_load_ready);
        assert!(!summary.release_archive_ingest_ready);
        assert!(!summary.release_archive_import_ready);
        assert!(!summary.release_archive_export_ready);
        assert!(!summary.release_archive_distribution_ready);
        assert!(!summary.release_archive_acceptance_ready);
        assert!(!summary.release_archive_adoption_ready);
        assert!(!summary.release_archive_rollout_ready);
        assert!(!summary.release_archive_activation_ready);
        assert!(!summary.release_archive_approval_ready);
        assert!(!summary.release_archive_certification_ready);
        assert!(!summary.release_archive_validation_ready);
        assert!(!summary.release_archive_verification_ready);
        assert!(!summary.release_archive_publication_ready);
        assert!(!summary.release_archive_completion_ready);
        assert!(!summary.release_archive_supervisor_ready);
        assert!(!summary.release_archive_operator_ready);
        assert!(!summary.release_archive_dispatch_ready);
        assert!(!summary.release_archive_handoff_ready);
        assert!(!summary.release_archive_closure_ready);
        assert!(!summary.release_archive_signoff_ready);
        assert!(!summary.release_archive_ready);
        assert!(!summary.release_closure_ready);
        assert!(!summary.release_signoff_ready);
        assert!(!summary.release_audit_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.coordination_ready);
        assert!(!summary.publish_gate_ready);
        assert!(!summary.release_readiness_evidence_ready);
        assert!(!summary.is_release_readiness_evidence_ready());
        assert!(summary.has_blocked_release_readiness_evidence_checks());
        assert!(!summary.needs_worker_process());
        assert!(!summary.needs_command_flow());
        assert!(summary.needs_pairing_flow());
        assert!(summary.needs_event_stream());
        assert!(summary.needs_physical_presence_requirement());
        assert!(summary.needs_package_release());
        assert!(summary.needs_release_archive_evidence_ledger());
        assert!(summary.needs_release_archive_evidence());
        assert!(summary.needs_release_archive_attestation());
        assert!(summary.needs_release_archive_confirmation());
        assert!(summary.needs_release_archive_finalization());
        assert!(summary.needs_release_archive_settlement());
        assert!(summary.needs_release_archive_reconciliation());
        assert!(summary.needs_release_archive_replay());
        assert!(summary.needs_release_archive_recovery());
        assert!(summary.needs_release_archive_restore());
        assert!(summary.needs_release_archive_load());
        assert!(summary.needs_release_archive_ingest());
        assert!(summary.needs_release_archive_import());
        assert!(summary.needs_release_archive_export());
        assert!(summary.needs_release_archive_distribution());
        assert!(summary.needs_release_archive_acceptance());
        assert!(summary.needs_release_archive_adoption());
        assert!(summary.needs_release_archive_rollout());
        assert!(summary.needs_release_archive_activation());
        assert!(summary.needs_release_archive_approval());
        assert!(summary.needs_release_archive_certification());
        assert!(summary.needs_release_archive_validation());
        assert!(summary.needs_release_archive_verification());
        assert!(summary.needs_release_archive_publication());
        assert!(summary.needs_release_archive_completion());
        assert!(summary.needs_release_archive_supervisor());
        assert!(summary.needs_release_archive_operator());
        assert!(summary.needs_release_archive_dispatch());
        assert!(summary.needs_release_archive_handoff());
        assert!(summary.needs_release_archive_closure());
        assert!(summary.needs_release_archive_signoff());
        assert!(summary.needs_release_archive());
        assert!(summary.needs_release_closure());
        assert!(summary.needs_release_signoff());
        assert!(summary.needs_release_audit());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_coordination());
        assert!(summary.needs_publish_gate());
    }

    #[test]
    fn hue_package_release_evidence_index_summary_reports_ready_release_evidence_index() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_evidence_index_summary(&plan);

        assert_eq!(
            summary.release_readiness_evidence_summary,
            hue_package_release_readiness_evidence_summary(&plan)
        );
        assert_eq!(summary.required_release_evidence_index_check_count, 5);
        assert_eq!(summary.passed_release_evidence_index_check_count, 5);
        assert_eq!(summary.blocked_release_evidence_index_check_count, 0);
        assert_eq!(summary.indexed_release_readiness_evidence_check_count, 44);
        assert_eq!(summary.indexed_archive_evidence_ledger_check_count, 37);
        assert_eq!(summary.blocked_indexed_release_evidence_check_count, 0);
        assert_eq!(
            summary.blocked_indexed_archive_evidence_ledger_check_count,
            0
        );
        assert!(summary.release_readiness_evidence_ready);
        assert!(summary.runtime_evidence_ready);
        assert!(summary.archive_evidence_ready);
        assert!(summary.release_closeout_ready);
        assert!(summary.operations_ready);
        assert!(summary.release_evidence_index_ready);
        assert!(summary.is_release_evidence_index_ready());
        assert!(!summary.has_blocked_release_evidence_index_checks());
        assert!(!summary.has_blocked_indexed_release_evidence_checks());
        assert!(!summary.has_blocked_indexed_archive_evidence_ledger_checks());
        assert!(!summary.needs_release_readiness_evidence());
        assert!(!summary.needs_runtime_evidence());
        assert!(!summary.needs_archive_evidence());
        assert!(!summary.needs_release_closeout());
        assert!(!summary.needs_operations());
    }

    #[test]
    fn hue_package_release_evidence_index_summary_routes_blocked_release_evidence_index() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseEvidenceIndexSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_release_evidence_index_check_count, 5);
        assert_eq!(summary.passed_release_evidence_index_check_count, 0);
        assert_eq!(summary.blocked_release_evidence_index_check_count, 5);
        assert_eq!(summary.indexed_release_readiness_evidence_check_count, 44);
        assert_eq!(summary.indexed_archive_evidence_ledger_check_count, 37);
        assert_eq!(summary.blocked_indexed_release_evidence_check_count, 42);
        assert_eq!(
            summary.blocked_indexed_archive_evidence_ledger_check_count,
            37
        );
        assert!(!summary.release_readiness_evidence_ready);
        assert!(!summary.runtime_evidence_ready);
        assert!(!summary.archive_evidence_ready);
        assert!(!summary.release_closeout_ready);
        assert!(!summary.operations_ready);
        assert!(!summary.release_evidence_index_ready);
        assert!(!summary.is_release_evidence_index_ready());
        assert!(summary.has_blocked_release_evidence_index_checks());
        assert!(summary.has_blocked_indexed_release_evidence_checks());
        assert!(summary.has_blocked_indexed_archive_evidence_ledger_checks());
        assert!(summary.needs_release_readiness_evidence());
        assert!(summary.needs_runtime_evidence());
        assert!(summary.needs_archive_evidence());
        assert!(summary.needs_release_closeout());
        assert!(summary.needs_operations());
    }

    #[test]
    fn hue_package_release_archive_notarization_summary_reports_ready_archive_notarization() {
        let plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );

        let summary = hue_package_release_archive_notarization_summary(&plan);

        assert_eq!(
            summary.release_evidence_index_summary,
            hue_package_release_evidence_index_summary(&plan)
        );
        assert_eq!(summary.required_archive_notarization_check_count, 6);
        assert_eq!(summary.passed_archive_notarization_check_count, 6);
        assert_eq!(summary.blocked_archive_notarization_check_count, 0);
        assert!(summary.release_readiness_evidence_ready);
        assert!(summary.runtime_evidence_ready);
        assert!(summary.archive_evidence_ready);
        assert!(summary.release_closeout_ready);
        assert!(summary.operations_ready);
        assert!(summary.release_evidence_index_ready);
        assert!(summary.release_archive_notarization_ready);
        assert!(summary.is_release_archive_notarization_ready());
        assert!(!summary.has_blocked_archive_notarization_checks());
        assert!(!summary.needs_release_readiness_evidence());
        assert!(!summary.needs_runtime_evidence());
        assert!(!summary.needs_archive_evidence());
        assert!(!summary.needs_release_closeout());
        assert!(!summary.needs_operations());
        assert!(!summary.needs_release_evidence_index());
    }

    #[test]
    fn hue_package_release_archive_notarization_summary_routes_blocked_archive_notarization() {
        let mut plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.60".to_string()),
            },
            "chief-of-staff",
            "desk",
        );
        plan.bridge.address = None;
        plan.registration_request.path = "/wrong/api".to_string();
        plan.application_key_header = "x-application-key".to_string();
        plan.event_stream_path = "/wrong/eventstream".to_string();
        plan.requires_user_presence = false;

        let summary = HuePackageReleaseArchiveNotarizationSummary::from_pairing_plan(&plan);

        assert_eq!(summary.required_archive_notarization_check_count, 6);
        assert_eq!(summary.passed_archive_notarization_check_count, 0);
        assert_eq!(summary.blocked_archive_notarization_check_count, 6);
        assert!(!summary.release_readiness_evidence_ready);
        assert!(!summary.runtime_evidence_ready);
        assert!(!summary.archive_evidence_ready);
        assert!(!summary.release_closeout_ready);
        assert!(!summary.operations_ready);
        assert!(!summary.release_evidence_index_ready);
        assert!(!summary.release_archive_notarization_ready);
        assert!(!summary.is_release_archive_notarization_ready());
        assert!(summary.has_blocked_archive_notarization_checks());
        assert!(summary.needs_release_readiness_evidence());
        assert!(summary.needs_runtime_evidence());
        assert!(summary.needs_archive_evidence());
        assert!(summary.needs_release_closeout());
        assert!(summary.needs_operations());
        assert!(summary.needs_release_evidence_index());
    }
}
