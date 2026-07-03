//! Zigbee Cluster Library frame, attribute, and D23 mapping primitives.
//!
//! ZCL sits above APS and below smart-home modeling. This crate owns cluster
//! ids, foundation command frames, typed attribute reports, and the first
//! capability/state-delta projection into `smart-home-core`.

#![forbid(unsafe_code)]

use smart_home_core::{Capability, CapabilityId, CapabilityMode, StateDelta, Value, ValueKind};
use std::fmt;
use zigbee_nwk::NetworkAddress;

pub const ZCL_READ_ATTRIBUTES_COMMAND_ID: u8 = 0x00;
pub const ZCL_REPORT_ATTRIBUTES_COMMAND_ID: u8 = 0x0a;
pub const ZCL_DEFAULT_RESPONSE_COMMAND_ID: u8 = 0x0b;
pub const ZCL_LEVEL_MOVE_TO_LEVEL_WITH_ON_OFF_COMMAND_ID: u8 = 0x04;
pub const ZCL_COLOR_MOVE_TO_COLOR_TEMPERATURE_COMMAND_ID: u8 = 0x0a;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZclClusterId(pub u16);

impl ZclClusterId {
    pub const BASIC: Self = Self(0x0000);
    pub const IDENTIFY: Self = Self(0x0003);
    pub const GROUPS: Self = Self(0x0004);
    pub const SCENES: Self = Self(0x0005);
    pub const ON_OFF: Self = Self(0x0006);
    pub const LEVEL_CONTROL: Self = Self(0x0008);
    pub const DOOR_LOCK: Self = Self(0x0101);
    pub const THERMOSTAT: Self = Self(0x0201);
    pub const COLOR_CONTROL: Self = Self(0x0300);
    pub const TEMPERATURE_MEASUREMENT: Self = Self(0x0402);
    pub const RELATIVE_HUMIDITY_MEASUREMENT: Self = Self(0x0405);
    pub const ILLUMINANCE_MEASUREMENT: Self = Self(0x0400);
    pub const OCCUPANCY_SENSING: Self = Self(0x0406);

    pub fn name(self) -> &'static str {
        match self {
            Self::BASIC => "basic",
            Self::IDENTIFY => "identify",
            Self::GROUPS => "groups",
            Self::SCENES => "scenes",
            Self::ON_OFF => "on_off",
            Self::LEVEL_CONTROL => "level_control",
            Self::DOOR_LOCK => "door_lock",
            Self::THERMOSTAT => "thermostat",
            Self::COLOR_CONTROL => "color_control",
            Self::TEMPERATURE_MEASUREMENT => "temperature_measurement",
            Self::RELATIVE_HUMIDITY_MEASUREMENT => "relative_humidity_measurement",
            Self::ILLUMINANCE_MEASUREMENT => "illuminance_measurement",
            Self::OCCUPANCY_SENSING => "occupancy_sensing",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZclAttributeId(pub u16);

impl ZclAttributeId {
    pub const ON_OFF: Self = Self(0x0000);
    pub const CURRENT_LEVEL: Self = Self(0x0000);
    pub const LOCK_STATE: Self = Self(0x0000);
    pub const LOCAL_TEMPERATURE: Self = Self(0x0000);
    pub const MEASURED_VALUE: Self = Self(0x0000);
    pub const OCCUPANCY: Self = Self(0x0000);
    pub const COLOR_TEMPERATURE_MIREK: Self = Self(0x0007);
    pub const MANUFACTURER_NAME: Self = Self(0x0004);
    pub const MODEL_IDENTIFIER: Self = Self(0x0005);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclFrameType {
    Foundation,
    ClusterSpecific,
    Reserved(u8),
}

impl ZclFrameType {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Foundation,
            1 => Self::ClusterSpecific,
            other => Self::Reserved(other),
        }
    }

    fn bits(self) -> u8 {
        match self {
            Self::Foundation => 0,
            Self::ClusterSpecific => 1,
            Self::Reserved(bits) => bits & 0b11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclDirection {
    ClientToServer,
    ServerToClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclFrameControl {
    pub frame_type: ZclFrameType,
    pub manufacturer_specific: bool,
    pub direction: ZclDirection,
    pub disable_default_response: bool,
}

impl ZclFrameControl {
    pub fn parse(raw: u8) -> Self {
        Self {
            frame_type: ZclFrameType::from_bits(raw),
            manufacturer_specific: raw & (1 << 2) != 0,
            direction: if raw & (1 << 3) == 0 {
                ZclDirection::ClientToServer
            } else {
                ZclDirection::ServerToClient
            },
            disable_default_response: raw & (1 << 4) != 0,
        }
    }

    pub fn encode(self) -> u8 {
        let mut raw = self.frame_type.bits();
        raw |= (self.manufacturer_specific as u8) << 2;
        raw |= (matches!(self.direction, ZclDirection::ServerToClient) as u8) << 3;
        raw |= (self.disable_default_response as u8) << 4;
        raw
    }

    pub fn foundation_client_to_server() -> Self {
        Self {
            frame_type: ZclFrameType::Foundation,
            manufacturer_specific: false,
            direction: ZclDirection::ClientToServer,
            disable_default_response: true,
        }
    }

    pub fn cluster_client_to_server() -> Self {
        Self {
            frame_type: ZclFrameType::ClusterSpecific,
            manufacturer_specific: false,
            direction: ZclDirection::ClientToServer,
            disable_default_response: true,
        }
    }

    pub fn foundation_server_to_client() -> Self {
        Self {
            frame_type: ZclFrameType::Foundation,
            manufacturer_specific: false,
            direction: ZclDirection::ServerToClient,
            disable_default_response: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclFrame {
    pub frame_control: ZclFrameControl,
    pub manufacturer_code: Option<u16>,
    pub transaction_sequence_number: u8,
    pub command_id: u8,
    pub payload: Vec<u8>,
}

impl ZclFrame {
    pub fn parse(bytes: &[u8]) -> Result<Self, ZclError> {
        let mut cursor = Cursor::new(bytes);
        let frame_control = ZclFrameControl::parse(cursor.read_u8()?);
        let manufacturer_code = if frame_control.manufacturer_specific {
            Some(cursor.read_u16_le()?)
        } else {
            None
        };
        let transaction_sequence_number = cursor.read_u8()?;
        let command_id = cursor.read_u8()?;
        let payload = cursor.remaining_bytes().to_vec();
        Ok(Self {
            frame_control,
            manufacturer_code,
            transaction_sequence_number,
            command_id,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ZclError> {
        if self.frame_control.manufacturer_specific != self.manufacturer_code.is_some() {
            return Err(ZclError::ManufacturerCodeMismatch);
        }
        let mut out = Vec::with_capacity(3 + self.payload.len());
        out.push(self.frame_control.encode());
        if let Some(code) = self.manufacturer_code {
            out.extend_from_slice(&code.to_le_bytes());
        }
        out.push(self.transaction_sequence_number);
        out.push(self.command_id);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn foundation_command(
        transaction_sequence_number: u8,
        command_id: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: ZclFrameControl::foundation_client_to_server(),
            manufacturer_code: None,
            transaction_sequence_number,
            command_id,
            payload,
        }
    }

    pub fn cluster_command(
        transaction_sequence_number: u8,
        command_id: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: ZclFrameControl::cluster_client_to_server(),
            manufacturer_code: None,
            transaction_sequence_number,
            command_id,
            payload,
        }
    }

    pub fn foundation_response(
        transaction_sequence_number: u8,
        command_id: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: ZclFrameControl::foundation_server_to_client(),
            manufacturer_code: None,
            transaction_sequence_number,
            command_id,
            payload,
        }
    }

    pub fn summary(&self) -> ZclFrameSummary {
        ZclFrameSummary::from_frame(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclFrameSummary {
    pub frame_type: ZclFrameType,
    pub direction: ZclDirection,
    pub manufacturer_specific: bool,
    pub has_manufacturer_code: bool,
    pub disables_default_response: bool,
    pub transaction_sequence_number: u8,
    pub command_id: u8,
    pub payload_len: usize,
}

impl ZclFrameSummary {
    pub fn from_frame(frame: &ZclFrame) -> Self {
        Self {
            frame_type: frame.frame_control.frame_type,
            direction: frame.frame_control.direction,
            manufacturer_specific: frame.frame_control.manufacturer_specific,
            has_manufacturer_code: frame.manufacturer_code.is_some(),
            disables_default_response: frame.frame_control.disable_default_response,
            transaction_sequence_number: frame.transaction_sequence_number,
            command_id: frame.command_id,
            payload_len: frame.payload.len(),
        }
    }

    pub fn is_foundation_frame(&self) -> bool {
        self.frame_type == ZclFrameType::Foundation
    }

    pub fn is_cluster_specific_frame(&self) -> bool {
        self.frame_type == ZclFrameType::ClusterSpecific
    }

    pub fn is_client_to_server(&self) -> bool {
        self.direction == ZclDirection::ClientToServer
    }

    pub fn is_server_to_client(&self) -> bool {
        self.direction == ZclDirection::ServerToClient
    }

    pub fn has_payload(&self) -> bool {
        self.payload_len > 0
    }

    pub fn has_manufacturer_context(&self) -> bool {
        self.manufacturer_specific || self.has_manufacturer_code
    }

    pub fn expects_default_response(&self) -> bool {
        !self.disables_default_response && self.is_client_to_server()
    }

    pub fn is_read_attributes(&self) -> bool {
        self.is_foundation_frame() && self.command_id == ZCL_READ_ATTRIBUTES_COMMAND_ID
    }

    pub fn is_report_attributes(&self) -> bool {
        self.is_foundation_frame() && self.command_id == ZCL_REPORT_ATTRIBUTES_COMMAND_ID
    }

    pub fn is_default_response(&self) -> bool {
        self.is_foundation_frame()
            && self.is_server_to_client()
            && self.command_id == ZCL_DEFAULT_RESPONSE_COMMAND_ID
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ZclFrameBatchSummary {
    pub frame_count: usize,
    pub foundation_frames: usize,
    pub cluster_specific_frames: usize,
    pub reserved_frames: usize,
    pub client_to_server_frames: usize,
    pub server_to_client_frames: usize,
    pub manufacturer_context_frames: usize,
    pub default_response_expected_frames: usize,
    pub read_attributes_frames: usize,
    pub report_attributes_frames: usize,
    pub default_response_frames: usize,
    pub payload_frames: usize,
    pub total_payload_bytes: usize,
    pub max_payload_bytes: usize,
}

impl ZclFrameBatchSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_frames<'a>(frames: impl IntoIterator<Item = &'a ZclFrame>) -> Self {
        let mut summary = Self::empty();
        for frame in frames {
            summary.record_summary(&frame.summary());
        }
        summary
    }

    pub fn from_summaries<'a>(summaries: impl IntoIterator<Item = &'a ZclFrameSummary>) -> Self {
        let mut summary = Self::empty();
        for frame_summary in summaries {
            summary.record_summary(frame_summary);
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &ZclFrameSummary) {
        self.frame_count += 1;

        match summary.frame_type {
            ZclFrameType::Foundation => self.foundation_frames += 1,
            ZclFrameType::ClusterSpecific => self.cluster_specific_frames += 1,
            ZclFrameType::Reserved(_) => self.reserved_frames += 1,
        }

        match summary.direction {
            ZclDirection::ClientToServer => self.client_to_server_frames += 1,
            ZclDirection::ServerToClient => self.server_to_client_frames += 1,
        }

        if summary.has_manufacturer_context() {
            self.manufacturer_context_frames += 1;
        }
        if summary.expects_default_response() {
            self.default_response_expected_frames += 1;
        }
        if summary.is_read_attributes() {
            self.read_attributes_frames += 1;
        }
        if summary.is_report_attributes() {
            self.report_attributes_frames += 1;
        }
        if summary.is_default_response() {
            self.default_response_frames += 1;
        }
        if summary.has_payload() {
            self.payload_frames += 1;
        }

        self.total_payload_bytes += summary.payload_len;
        self.max_payload_bytes = self.max_payload_bytes.max(summary.payload_len);
    }

    pub fn is_empty(self) -> bool {
        self.frame_count == 0
    }

    pub fn has_foundation_frames(self) -> bool {
        self.foundation_frames > 0
    }

    pub fn has_cluster_specific_frames(self) -> bool {
        self.cluster_specific_frames > 0
    }

    pub fn has_server_to_client_frames(self) -> bool {
        self.server_to_client_frames > 0
    }

    pub fn has_manufacturer_context(self) -> bool {
        self.manufacturer_context_frames > 0
    }

    pub fn expects_default_responses(self) -> bool {
        self.default_response_expected_frames > 0
    }

    pub fn has_payloads(self) -> bool {
        self.payload_frames > 0
    }
}

pub fn summarize_zcl_frames<'a>(
    frames: impl IntoIterator<Item = &'a ZclFrame>,
) -> ZclFrameBatchSummary {
    ZclFrameBatchSummary::from_frames(frames)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnOffCommand {
    Off,
    On,
    Toggle,
}

impl OnOffCommand {
    pub fn command_id(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::On => 0x01,
            Self::Toggle => 0x02,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclStatusCode {
    Success,
    Failure,
    UnsupportedAttribute,
    InvalidValue,
    UnsupportedCommand,
    Unknown(u8),
}

impl ZclStatusCode {
    pub fn parse(raw: u8) -> Self {
        match raw {
            0x00 => Self::Success,
            0x01 => Self::Failure,
            0x86 => Self::UnsupportedAttribute,
            0x87 => Self::InvalidValue,
            0x81 => Self::UnsupportedCommand,
            other => Self::Unknown(other),
        }
    }

    pub fn encode(self) -> u8 {
        match self {
            Self::Success => 0x00,
            Self::Failure => 0x01,
            Self::UnsupportedAttribute => 0x86,
            Self::InvalidValue => 0x87,
            Self::UnsupportedCommand => 0x81,
            Self::Unknown(raw) => raw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZigbeeEndpointRef {
    pub network_address: NetworkAddress,
    pub endpoint: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclDataType {
    Bool,
    Bitmap8,
    U8,
    U16,
    U32,
    I16,
    Enum8,
    CharacterString,
    Unknown(u8),
}

impl ZclDataType {
    pub fn parse(raw: u8) -> Self {
        match raw {
            0x10 => Self::Bool,
            0x18 => Self::Bitmap8,
            0x20 => Self::U8,
            0x21 => Self::U16,
            0x23 => Self::U32,
            0x29 => Self::I16,
            0x30 => Self::Enum8,
            0x42 => Self::CharacterString,
            other => Self::Unknown(other),
        }
    }

    pub fn encode(self) -> u8 {
        match self {
            Self::Bool => 0x10,
            Self::Bitmap8 => 0x18,
            Self::U8 => 0x20,
            Self::U16 => 0x21,
            Self::U32 => 0x23,
            Self::I16 => 0x29,
            Self::Enum8 => 0x30,
            Self::CharacterString => 0x42,
            Self::Unknown(raw) => raw,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZclValue {
    Bool(bool),
    Bitmap8(u8),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    Enum8(u8),
    CharacterString(String),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclAttributeReport {
    pub cluster_id: ZclClusterId,
    pub attribute_id: ZclAttributeId,
    pub data_type: ZclDataType,
    pub value: ZclValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclAttributeReportSummary {
    pub cluster_id: ZclClusterId,
    pub report_count: usize,
    pub state_delta_count: usize,
    pub bool_reports: usize,
    pub numeric_reports: usize,
    pub text_reports: usize,
    pub raw_reports: usize,
    pub unknown_type_reports: usize,
}

impl ZclAttributeReportSummary {
    pub fn from_reports<'a, I>(cluster_id: ZclClusterId, reports: I) -> Self
    where
        I: IntoIterator<Item = &'a ZclAttributeReport>,
    {
        let mut summary = Self {
            cluster_id,
            report_count: 0,
            state_delta_count: 0,
            bool_reports: 0,
            numeric_reports: 0,
            text_reports: 0,
            raw_reports: 0,
            unknown_type_reports: 0,
        };
        for report in reports {
            summary.report_count += 1;
            if state_delta_for_report(report).is_some() {
                summary.state_delta_count += 1;
            }
            match &report.value {
                ZclValue::Bool(_) | ZclValue::Bitmap8(_) => summary.bool_reports += 1,
                ZclValue::U8(_) | ZclValue::U16(_) | ZclValue::U32(_) | ZclValue::I16(_) => {
                    summary.numeric_reports += 1;
                }
                ZclValue::Enum8(_) => summary.numeric_reports += 1,
                ZclValue::CharacterString(_) => summary.text_reports += 1,
                ZclValue::Raw(_) => summary.raw_reports += 1,
            }
            if matches!(report.data_type, ZclDataType::Unknown(_)) {
                summary.unknown_type_reports += 1;
            }
        }
        summary
    }

    pub fn has_state_deltas(&self) -> bool {
        self.state_delta_count > 0
    }

    pub fn has_reports(&self) -> bool {
        self.report_count > 0
    }

    pub fn typed_report_count(&self) -> usize {
        self.bool_reports + self.numeric_reports + self.text_reports
    }

    pub fn has_typed_reports(&self) -> bool {
        self.typed_report_count() > 0
    }

    pub fn has_raw_reports(&self) -> bool {
        self.raw_reports > 0
    }

    pub fn has_unknown_type_reports(&self) -> bool {
        self.unknown_type_reports > 0
    }

    pub fn readiness(self) -> ZclAttributeReportReadinessSummary {
        ZclAttributeReportReadinessSummary::from_summary(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclAttributeReportReadinessSummary {
    pub report_summary: ZclAttributeReportSummary,
    pub required_check_count: usize,
    pub passed_check_count: usize,
    pub missing_check_count: usize,
    pub reports_present: bool,
    pub state_delta_coverage_ready: bool,
    pub typed_value_coverage_ready: bool,
    pub raw_reports_absent: bool,
    pub unknown_types_absent: bool,
    pub report_ready: bool,
}

impl ZclAttributeReportReadinessSummary {
    pub fn from_summary(report_summary: ZclAttributeReportSummary) -> Self {
        let reports_present = report_summary.has_reports();
        let state_delta_coverage_ready = report_summary.has_state_deltas();
        let typed_value_coverage_ready = report_summary.has_typed_reports();
        let raw_reports_absent = !report_summary.has_raw_reports();
        let unknown_types_absent = !report_summary.has_unknown_type_reports();
        let checks = [
            reports_present,
            state_delta_coverage_ready,
            typed_value_coverage_ready,
            raw_reports_absent,
            unknown_types_absent,
        ];
        let passed_check_count = checks.iter().filter(|ready| **ready).count();
        let required_check_count = checks.len();
        let missing_check_count = required_check_count - passed_check_count;
        let report_ready = missing_check_count == 0;

        Self {
            report_summary,
            required_check_count,
            passed_check_count,
            missing_check_count,
            reports_present,
            state_delta_coverage_ready,
            typed_value_coverage_ready,
            raw_reports_absent,
            unknown_types_absent,
            report_ready,
        }
    }

    pub fn is_report_ready(self) -> bool {
        self.report_ready
    }

    pub fn has_missing_checks(self) -> bool {
        self.missing_check_count > 0
    }

    pub fn needs_report_discovery(self) -> bool {
        !self.reports_present
    }

    pub fn needs_state_delta_mapping(self) -> bool {
        !self.state_delta_coverage_ready
    }

    pub fn needs_typed_value_mapping(self) -> bool {
        !self.typed_value_coverage_ready
    }

    pub fn has_raw_or_unknown_reports(self) -> bool {
        !self.raw_reports_absent || !self.unknown_types_absent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclReportOperatorSummary {
    pub frame_batch: ZclFrameBatchSummary,
    pub report_readiness: ZclAttributeReportReadinessSummary,
    pub required_check_count: usize,
    pub passed_check_count: usize,
    pub missing_check_count: usize,
    pub report_frame_observed: bool,
    pub report_payloads_present: bool,
    pub no_default_response_backlog: bool,
    pub report_ready: bool,
    pub operator_ready: bool,
}

impl ZclReportOperatorSummary {
    pub fn from_parts(
        frame_batch: ZclFrameBatchSummary,
        report_readiness: ZclAttributeReportReadinessSummary,
    ) -> Self {
        let report_frame_observed = frame_batch.report_attributes_frames > 0;
        let report_payloads_present = frame_batch.has_payloads();
        let no_default_response_backlog = !frame_batch.expects_default_responses();
        let report_ready = report_readiness.is_report_ready();
        let checks = [
            report_frame_observed,
            report_payloads_present,
            no_default_response_backlog,
            report_ready,
        ];
        let passed_check_count = checks.iter().filter(|ready| **ready).count();
        let required_check_count = checks.len();
        let missing_check_count = required_check_count - passed_check_count;
        let operator_ready = missing_check_count == 0;

        Self {
            frame_batch,
            report_readiness,
            required_check_count,
            passed_check_count,
            missing_check_count,
            report_frame_observed,
            report_payloads_present,
            no_default_response_backlog,
            report_ready,
            operator_ready,
        }
    }

    pub fn is_operator_ready(self) -> bool {
        self.operator_ready
    }

    pub fn has_missing_checks(self) -> bool {
        self.missing_check_count > 0
    }

    pub fn needs_report_frame_capture(self) -> bool {
        !self.report_frame_observed
    }

    pub fn needs_report_payloads(self) -> bool {
        !self.report_payloads_present
    }

    pub fn has_default_response_backlog(self) -> bool {
        !self.no_default_response_backlog
    }

    pub fn needs_report_readiness_work(self) -> bool {
        !self.report_ready
    }
}

pub fn zcl_report_operator_summary<'a, 'b>(
    frames: impl IntoIterator<Item = &'a ZclFrame>,
    cluster_id: ZclClusterId,
    reports: impl IntoIterator<Item = &'b ZclAttributeReport>,
) -> ZclReportOperatorSummary {
    let frame_batch = ZclFrameBatchSummary::from_frames(frames);
    let report_readiness = ZclAttributeReportSummary::from_reports(cluster_id, reports).readiness();
    ZclReportOperatorSummary::from_parts(frame_batch, report_readiness)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclReportSignoffSummary {
    pub operator_summary: ZclReportOperatorSummary,
    pub required_signoff_check_count: usize,
    pub passed_signoff_check_count: usize,
    pub missing_signoff_check_count: usize,
    pub operator_ready: bool,
    pub report_frame_observed: bool,
    pub report_payloads_present: bool,
    pub default_response_backlog_clear: bool,
    pub report_readiness_ready: bool,
    pub signoff_ready: bool,
}

impl ZclReportSignoffSummary {
    pub fn from_operator_summary(operator_summary: ZclReportOperatorSummary) -> Self {
        let operator_ready = operator_summary.is_operator_ready();
        let report_frame_observed = !operator_summary.needs_report_frame_capture();
        let report_payloads_present = !operator_summary.needs_report_payloads();
        let default_response_backlog_clear = !operator_summary.has_default_response_backlog();
        let report_readiness_ready = !operator_summary.needs_report_readiness_work();
        let checks = [
            operator_ready,
            report_frame_observed,
            report_payloads_present,
            default_response_backlog_clear,
            report_readiness_ready,
        ];
        let passed_signoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_signoff_check_count = checks.len();
        let missing_signoff_check_count = required_signoff_check_count - passed_signoff_check_count;
        let signoff_ready = missing_signoff_check_count == 0;

        Self {
            operator_summary,
            required_signoff_check_count,
            passed_signoff_check_count,
            missing_signoff_check_count,
            operator_ready,
            report_frame_observed,
            report_payloads_present,
            default_response_backlog_clear,
            report_readiness_ready,
            signoff_ready,
        }
    }

    pub fn is_signoff_ready(self) -> bool {
        self.signoff_ready
    }

    pub fn has_missing_signoff_checks(self) -> bool {
        self.missing_signoff_check_count > 0
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_report_frame_capture(self) -> bool {
        !self.report_frame_observed
    }

    pub fn needs_report_payloads(self) -> bool {
        !self.report_payloads_present
    }

    pub fn has_default_response_backlog(self) -> bool {
        !self.default_response_backlog_clear
    }

    pub fn needs_report_readiness_work(self) -> bool {
        !self.report_readiness_ready
    }
}

pub fn zcl_report_signoff_summary<'a, 'b>(
    frames: impl IntoIterator<Item = &'a ZclFrame>,
    cluster_id: ZclClusterId,
    reports: impl IntoIterator<Item = &'b ZclAttributeReport>,
) -> ZclReportSignoffSummary {
    ZclReportSignoffSummary::from_operator_summary(zcl_report_operator_summary(
        frames, cluster_id, reports,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclReportClosureSummary {
    pub signoff_summary: ZclReportSignoffSummary,
    pub required_closure_check_count: usize,
    pub passed_closure_check_count: usize,
    pub missing_closure_check_count: usize,
    pub signoff_ready: bool,
    pub operator_ready: bool,
    pub report_frame_observed: bool,
    pub report_payloads_present: bool,
    pub default_response_backlog_clear: bool,
    pub report_readiness_ready: bool,
    pub closure_ready: bool,
}

impl ZclReportClosureSummary {
    pub fn from_signoff_summary(signoff_summary: ZclReportSignoffSummary) -> Self {
        let signoff_ready = signoff_summary.is_signoff_ready();
        let operator_ready = !signoff_summary.needs_operator_readiness();
        let report_frame_observed = !signoff_summary.needs_report_frame_capture();
        let report_payloads_present = !signoff_summary.needs_report_payloads();
        let default_response_backlog_clear = !signoff_summary.has_default_response_backlog();
        let report_readiness_ready = !signoff_summary.needs_report_readiness_work();
        let checks = [
            signoff_ready,
            operator_ready,
            report_frame_observed,
            report_payloads_present,
            default_response_backlog_clear,
            report_readiness_ready,
        ];
        let passed_closure_check_count = checks.iter().filter(|ready| **ready).count();
        let required_closure_check_count = checks.len();
        let missing_closure_check_count = required_closure_check_count - passed_closure_check_count;
        let closure_ready = missing_closure_check_count == 0;

        Self {
            signoff_summary,
            required_closure_check_count,
            passed_closure_check_count,
            missing_closure_check_count,
            signoff_ready,
            operator_ready,
            report_frame_observed,
            report_payloads_present,
            default_response_backlog_clear,
            report_readiness_ready,
            closure_ready,
        }
    }

    pub fn is_closure_ready(self) -> bool {
        self.closure_ready
    }

    pub fn has_missing_closure_checks(self) -> bool {
        self.missing_closure_check_count > 0
    }

    pub fn needs_signoff(self) -> bool {
        !self.signoff_ready
    }

    pub fn needs_operator_readiness(self) -> bool {
        !self.operator_ready
    }

    pub fn needs_report_frame_capture(self) -> bool {
        !self.report_frame_observed
    }

    pub fn needs_report_payloads(self) -> bool {
        !self.report_payloads_present
    }

    pub fn has_default_response_backlog(self) -> bool {
        !self.default_response_backlog_clear
    }

    pub fn needs_report_readiness_work(self) -> bool {
        !self.report_readiness_ready
    }
}

pub fn zcl_report_closure_summary<'a, 'b>(
    frames: impl IntoIterator<Item = &'a ZclFrame>,
    cluster_id: ZclClusterId,
    reports: impl IntoIterator<Item = &'b ZclAttributeReport>,
) -> ZclReportClosureSummary {
    ZclReportClosureSummary::from_signoff_summary(zcl_report_signoff_summary(
        frames, cluster_id, reports,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZclError {
    Truncated { needed: usize, remaining: usize },
    ManufacturerCodeMismatch,
    InvalidString,
    ValueTooLong { len: usize },
    ValueTypeMismatch { data_type: ZclDataType },
}

impl fmt::Display for ZclError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Zigbee ZCL frame: needed {needed} bytes, had {remaining}"
            ),
            Self::ManufacturerCodeMismatch => {
                write!(f, "manufacturer-specific flag does not match code field")
            }
            Self::InvalidString => write!(f, "ZCL character string is not valid UTF-8"),
            Self::ValueTooLong { len } => {
                write!(
                    f,
                    "ZCL value length {len} exceeds the one-byte length field"
                )
            }
            Self::ValueTypeMismatch { data_type } => {
                write!(f, "ZCL value does not match data type {data_type:?}")
            }
        }
    }
}

impl std::error::Error for ZclError {}

pub fn read_attributes_frame(
    transaction_sequence_number: u8,
    attribute_ids: &[ZclAttributeId],
) -> ZclFrame {
    let mut payload = Vec::with_capacity(attribute_ids.len() * 2);
    for attribute_id in attribute_ids {
        payload.extend_from_slice(&attribute_id.0.to_le_bytes());
    }
    ZclFrame::foundation_command(
        transaction_sequence_number,
        ZCL_READ_ATTRIBUTES_COMMAND_ID,
        payload,
    )
}

pub fn on_off_command_frame(transaction_sequence_number: u8, command: OnOffCommand) -> ZclFrame {
    ZclFrame::cluster_command(
        transaction_sequence_number,
        command.command_id(),
        Vec::new(),
    )
}

pub fn move_to_level_with_on_off_frame(
    transaction_sequence_number: u8,
    percent: u8,
    transition_time_ds: u16,
) -> ZclFrame {
    let mut payload = Vec::with_capacity(3);
    payload.push(percentage_to_level(percent));
    payload.extend_from_slice(&transition_time_ds.to_le_bytes());
    ZclFrame::cluster_command(
        transaction_sequence_number,
        ZCL_LEVEL_MOVE_TO_LEVEL_WITH_ON_OFF_COMMAND_ID,
        payload,
    )
}

pub fn move_to_color_temperature_frame(
    transaction_sequence_number: u8,
    mirek: u16,
    transition_time_ds: u16,
) -> ZclFrame {
    let mut payload = Vec::with_capacity(4);
    payload.extend_from_slice(&mirek.to_le_bytes());
    payload.extend_from_slice(&transition_time_ds.to_le_bytes());
    ZclFrame::cluster_command(
        transaction_sequence_number,
        ZCL_COLOR_MOVE_TO_COLOR_TEMPERATURE_COMMAND_ID,
        payload,
    )
}

pub fn report_attributes_frame(
    transaction_sequence_number: u8,
    reports: &[ZclAttributeReport],
) -> Result<ZclFrame, ZclError> {
    let payload = encode_attribute_reports(reports)?;
    Ok(ZclFrame::foundation_command(
        transaction_sequence_number,
        ZCL_REPORT_ATTRIBUTES_COMMAND_ID,
        payload,
    ))
}

pub fn default_response_frame(
    transaction_sequence_number: u8,
    original_command_id: u8,
    status: ZclStatusCode,
) -> ZclFrame {
    ZclFrame::foundation_response(
        transaction_sequence_number,
        ZCL_DEFAULT_RESPONSE_COMMAND_ID,
        vec![original_command_id, status.encode()],
    )
}

pub fn encode_attribute_reports(reports: &[ZclAttributeReport]) -> Result<Vec<u8>, ZclError> {
    let mut out = Vec::new();
    for report in reports {
        out.extend_from_slice(&report.attribute_id.0.to_le_bytes());
        out.push(report.data_type.encode());
        encode_zcl_value(report.data_type, &report.value, &mut out)?;
    }
    Ok(out)
}

pub fn parse_attribute_reports(
    cluster_id: ZclClusterId,
    payload: &[u8],
) -> Result<Vec<ZclAttributeReport>, ZclError> {
    let mut cursor = Cursor::new(payload);
    let mut reports = Vec::new();
    while cursor.remaining_len() > 0 {
        let attribute_id = ZclAttributeId(cursor.read_u16_le()?);
        let data_type = ZclDataType::parse(cursor.read_u8()?);
        let value = read_zcl_value(&mut cursor, data_type)?;
        reports.push(ZclAttributeReport {
            cluster_id,
            attribute_id,
            data_type,
            value,
        });
    }
    Ok(reports)
}

pub fn attribute_report_summary(
    cluster_id: ZclClusterId,
    payload: &[u8],
) -> Result<ZclAttributeReportSummary, ZclError> {
    let reports = parse_attribute_reports(cluster_id, payload)?;
    Ok(ZclAttributeReportSummary::from_reports(
        cluster_id, &reports,
    ))
}

pub fn capabilities_for_cluster(cluster_id: ZclClusterId) -> Vec<Capability> {
    match cluster_id {
        ZclClusterId::ON_OFF => vec![Capability::light_on_off()],
        ZclClusterId::LEVEL_CONTROL => vec![Capability::light_brightness()],
        ZclClusterId::COLOR_CONTROL => vec![Capability::light_color_temperature()],
        ZclClusterId::OCCUPANCY_SENSING => vec![Capability::sensor_occupancy()],
        ZclClusterId::TEMPERATURE_MEASUREMENT => vec![Capability::sensor_temperature()],
        ZclClusterId::RELATIVE_HUMIDITY_MEASUREMENT => vec![Capability::sensor_humidity()],
        ZclClusterId::ILLUMINANCE_MEASUREMENT => vec![Capability::sensor_illuminance()],
        ZclClusterId::DOOR_LOCK => vec![Capability::new(
            CapabilityId::trusted("lock.state"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Text,
        )],
        ZclClusterId::THERMOSTAT => vec![Capability::new(
            CapabilityId::trusted("climate.setpoint"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Number,
        )],
        _ => Vec::new(),
    }
}

pub fn state_delta_for_report(report: &ZclAttributeReport) -> Option<StateDelta> {
    match (report.cluster_id, report.attribute_id, &report.value) {
        (ZclClusterId::ON_OFF, ZclAttributeId::ON_OFF, ZclValue::Bool(on)) => Some(StateDelta {
            capability_id: CapabilityId::trusted("light.on_off"),
            value: Value::Bool(*on),
        }),
        (ZclClusterId::LEVEL_CONTROL, ZclAttributeId::CURRENT_LEVEL, ZclValue::U8(level)) => {
            Some(StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(level_to_percentage(*level)),
            })
        }
        (
            ZclClusterId::COLOR_CONTROL,
            ZclAttributeId::COLOR_TEMPERATURE_MIREK,
            ZclValue::U16(mirek),
        ) => Some(StateDelta {
            capability_id: CapabilityId::trusted("light.color_temperature"),
            value: Value::Integer(i64::from(*mirek)),
        }),
        (ZclClusterId::OCCUPANCY_SENSING, ZclAttributeId::OCCUPANCY, ZclValue::Bitmap8(bits)) => {
            Some(StateDelta {
                capability_id: CapabilityId::trusted("sensor.occupancy"),
                value: Value::Bool(bits & 0x01 != 0),
            })
        }
        (ZclClusterId::DOOR_LOCK, ZclAttributeId::LOCK_STATE, ZclValue::Enum8(state)) => {
            Some(StateDelta {
                capability_id: CapabilityId::trusted("lock.state"),
                value: Value::Text(lock_state_name(*state).to_string()),
            })
        }
        (
            ZclClusterId::TEMPERATURE_MEASUREMENT,
            ZclAttributeId::MEASURED_VALUE,
            ZclValue::I16(centi_celsius),
        ) => Some(StateDelta {
            capability_id: CapabilityId::trusted("sensor.temperature"),
            value: Value::Number(centi_celsius_to_celsius(*centi_celsius)),
        }),
        (
            ZclClusterId::RELATIVE_HUMIDITY_MEASUREMENT,
            ZclAttributeId::MEASURED_VALUE,
            ZclValue::U16(centi_percent),
        ) => Some(StateDelta {
            capability_id: CapabilityId::trusted("sensor.humidity"),
            value: Value::Number(centi_percent_to_percent(*centi_percent)),
        }),
        (
            ZclClusterId::ILLUMINANCE_MEASUREMENT,
            ZclAttributeId::MEASURED_VALUE,
            ZclValue::U16(measured_value),
        ) => illuminance_measured_value_to_lux(*measured_value).map(|lux| StateDelta {
            capability_id: CapabilityId::trusted("sensor.illuminance"),
            value: Value::Number(lux),
        }),
        _ => None,
    }
}

pub fn level_to_percentage(level: u8) -> u8 {
    ((u16::from(level) * 100 + 127) / 254).min(100) as u8
}

pub fn percentage_to_level(percent: u8) -> u8 {
    let percent = percent.min(100);
    ((u16::from(percent) * 254 + 50) / 100).min(254) as u8
}

pub fn lock_state_name(value: u8) -> &'static str {
    match value {
        0x00 => "not_fully_locked",
        0x01 => "locked",
        0x02 => "unlocked",
        _ => "unknown",
    }
}

pub fn centi_celsius_to_celsius(value: i16) -> f64 {
    f64::from(value) / 100.0
}

pub fn centi_percent_to_percent(value: u16) -> f64 {
    f64::from(value) / 100.0
}

pub fn illuminance_measured_value_to_lux(value: u16) -> Option<f64> {
    match value {
        0xffff => None,
        0 => Some(0.0),
        measured_value => Some(10_f64.powf((f64::from(measured_value) - 1.0) / 10_000.0)),
    }
}

fn read_zcl_value(cursor: &mut Cursor<'_>, data_type: ZclDataType) -> Result<ZclValue, ZclError> {
    match data_type {
        ZclDataType::Bool => Ok(ZclValue::Bool(cursor.read_u8()? != 0)),
        ZclDataType::Bitmap8 => Ok(ZclValue::Bitmap8(cursor.read_u8()?)),
        ZclDataType::U8 => Ok(ZclValue::U8(cursor.read_u8()?)),
        ZclDataType::U16 => Ok(ZclValue::U16(cursor.read_u16_le()?)),
        ZclDataType::U32 => Ok(ZclValue::U32(cursor.read_u32_le()?)),
        ZclDataType::I16 => Ok(ZclValue::I16(cursor.read_i16_le()?)),
        ZclDataType::Enum8 => Ok(ZclValue::Enum8(cursor.read_u8()?)),
        ZclDataType::CharacterString => {
            let len = cursor.read_u8()? as usize;
            let bytes = cursor.read_bytes(len)?;
            let value = std::str::from_utf8(bytes).map_err(|_| ZclError::InvalidString)?;
            Ok(ZclValue::CharacterString(value.to_string()))
        }
        ZclDataType::Unknown(_) => Ok(ZclValue::Raw(cursor.read_remaining_bytes().to_vec())),
    }
}

fn encode_zcl_value(
    data_type: ZclDataType,
    value: &ZclValue,
    out: &mut Vec<u8>,
) -> Result<(), ZclError> {
    match (data_type, value) {
        (ZclDataType::Bool, ZclValue::Bool(value)) => out.push(u8::from(*value)),
        (ZclDataType::Bitmap8, ZclValue::Bitmap8(value)) => out.push(*value),
        (ZclDataType::U8, ZclValue::U8(value)) => out.push(*value),
        (ZclDataType::U16, ZclValue::U16(value)) => out.extend_from_slice(&value.to_le_bytes()),
        (ZclDataType::U32, ZclValue::U32(value)) => out.extend_from_slice(&value.to_le_bytes()),
        (ZclDataType::I16, ZclValue::I16(value)) => out.extend_from_slice(&value.to_le_bytes()),
        (ZclDataType::Enum8, ZclValue::Enum8(value)) => out.push(*value),
        (ZclDataType::CharacterString, ZclValue::CharacterString(value)) => {
            if value.len() > u8::MAX as usize {
                return Err(ZclError::ValueTooLong { len: value.len() });
            }
            out.push(value.len() as u8);
            out.extend_from_slice(value.as_bytes());
        }
        (ZclDataType::Unknown(_), ZclValue::Raw(value)) => out.extend_from_slice(value),
        _ => return Err(ZclError::ValueTypeMismatch { data_type }),
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn remaining_bytes(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }

    fn read_remaining_bytes(&mut self) -> &'a [u8] {
        let start = self.pos;
        self.pos = self.bytes.len();
        &self.bytes[start..]
    }

    fn read_u8(&mut self) -> Result<u8, ZclError> {
        if self.remaining_len() < 1 {
            return Err(ZclError::Truncated {
                needed: 1,
                remaining: self.remaining_len(),
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_u16_le(&mut self) -> Result<u16, ZclError> {
        let bytes = self.read_array::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_i16_le(&mut self) -> Result<i16, ZclError> {
        let bytes = self.read_array::<2>()?;
        Ok(i16::from_le_bytes(bytes))
    }

    fn read_u32_le(&mut self) -> Result<u32, ZclError> {
        let bytes = self.read_array::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ZclError> {
        if self.remaining_len() < len {
            return Err(ZclError::Truncated {
                needed: len,
                remaining: self.remaining_len(),
            });
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.bytes[start..self.pos])
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ZclError> {
        let bytes = self.read_bytes(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_control_round_trips_foundation_flags() {
        let control = ZclFrameControl {
            frame_type: ZclFrameType::Foundation,
            manufacturer_specific: true,
            direction: ZclDirection::ServerToClient,
            disable_default_response: true,
        };

        assert_eq!(ZclFrameControl::parse(control.encode()), control);
    }

    #[test]
    fn read_attributes_frame_encodes_foundation_command() {
        let frame = read_attributes_frame(
            0x22,
            &[
                ZclAttributeId::MANUFACTURER_NAME,
                ZclAttributeId::MODEL_IDENTIFIER,
            ],
        );

        assert_eq!(frame.command_id, ZCL_READ_ATTRIBUTES_COMMAND_ID);
        assert_eq!(
            frame.encode().unwrap(),
            vec![0x10, 0x22, 0x00, 0x04, 0x00, 0x05, 0x00]
        );
        let summary = frame.summary();
        assert_eq!(
            summary,
            ZclFrameSummary {
                frame_type: ZclFrameType::Foundation,
                direction: ZclDirection::ClientToServer,
                manufacturer_specific: false,
                has_manufacturer_code: false,
                disables_default_response: true,
                transaction_sequence_number: 0x22,
                command_id: ZCL_READ_ATTRIBUTES_COMMAND_ID,
                payload_len: 4,
            }
        );
        assert!(summary.is_foundation_frame());
        assert!(summary.is_client_to_server());
        assert!(summary.is_read_attributes());
        assert!(!summary.expects_default_response());
        assert!(summary.has_payload());
    }

    #[test]
    fn default_response_frame_encodes_server_to_client_status() {
        let frame = default_response_frame(
            0x23,
            ZCL_READ_ATTRIBUTES_COMMAND_ID,
            ZclStatusCode::UnsupportedAttribute,
        );

        assert_eq!(frame.command_id, ZCL_DEFAULT_RESPONSE_COMMAND_ID);
        assert_eq!(
            frame.frame_control,
            ZclFrameControl::foundation_server_to_client()
        );
        assert_eq!(frame.payload, vec![ZCL_READ_ATTRIBUTES_COMMAND_ID, 0x86]);
        assert_eq!(frame.encode().unwrap(), vec![0x18, 0x23, 0x0b, 0x00, 0x86]);
        assert_eq!(ZclFrame::parse(&frame.encode().unwrap()).unwrap(), frame);
        assert_eq!(
            ZclStatusCode::parse(0x81),
            ZclStatusCode::UnsupportedCommand
        );
        assert_eq!(ZclStatusCode::Unknown(0xfe).encode(), 0xfe);
        let summary = frame.summary();
        assert!(summary.is_default_response());
        assert!(summary.is_server_to_client());
        assert!(summary.has_payload());
        assert!(!summary.expects_default_response());
    }

    #[test]
    fn on_command_encodes_cluster_specific_frame() {
        let frame = on_off_command_frame(0x33, OnOffCommand::On);

        assert_eq!(frame.command_id, 0x01);
        assert_eq!(frame.encode().unwrap(), vec![0x11, 0x33, 0x01]);
        let summary = frame.summary();
        assert!(summary.is_cluster_specific_frame());
        assert!(summary.is_client_to_server());
        assert!(!summary.has_payload());
        assert!(!summary.is_report_attributes());
        assert!(!summary.has_manufacturer_context());
    }

    #[test]
    fn frame_summary_reports_manufacturer_and_default_response_context() {
        let frame = ZclFrame::parse(&[0x04, 0x34, 0x12, 0x51, 0x02, 0xaa]).unwrap();
        let summary = frame.summary();

        assert_eq!(frame.manufacturer_code, Some(0x1234));
        assert_eq!(summary.frame_type, ZclFrameType::Foundation);
        assert_eq!(summary.direction, ZclDirection::ClientToServer);
        assert_eq!(summary.transaction_sequence_number, 0x51);
        assert_eq!(summary.command_id, 0x02);
        assert_eq!(summary.payload_len, 1);
        assert!(summary.manufacturer_specific);
        assert!(summary.has_manufacturer_code);
        assert!(summary.has_manufacturer_context());
        assert!(summary.expects_default_response());
        assert!(summary.has_payload());
    }

    #[test]
    fn frame_batch_summary_rolls_up_shape_and_payload_context() {
        let read = read_attributes_frame(
            0x22,
            &[
                ZclAttributeId::MANUFACTURER_NAME,
                ZclAttributeId::MODEL_IDENTIFIER,
            ],
        );
        let report = report_attributes_frame(
            0x23,
            &[ZclAttributeReport {
                cluster_id: ZclClusterId::ON_OFF,
                attribute_id: ZclAttributeId::ON_OFF,
                data_type: ZclDataType::Bool,
                value: ZclValue::Bool(true),
            }],
        )
        .unwrap();
        let on = on_off_command_frame(0x24, OnOffCommand::On);
        let manufacturer = ZclFrame::parse(&[0x04, 0x34, 0x12, 0x25, 0x02, 0xaa]).unwrap();
        let default_response =
            default_response_frame(0x26, ZCL_READ_ATTRIBUTES_COMMAND_ID, ZclStatusCode::Success);

        let summary = summarize_zcl_frames([&read, &report, &on, &manufacturer, &default_response]);

        assert_eq!(
            summary,
            ZclFrameBatchSummary {
                frame_count: 5,
                foundation_frames: 4,
                cluster_specific_frames: 1,
                reserved_frames: 0,
                client_to_server_frames: 4,
                server_to_client_frames: 1,
                manufacturer_context_frames: 1,
                default_response_expected_frames: 1,
                read_attributes_frames: 1,
                report_attributes_frames: 1,
                default_response_frames: 1,
                payload_frames: 4,
                total_payload_bytes: 11,
                max_payload_bytes: 4,
            }
        );
        assert!(summary.has_foundation_frames());
        assert!(summary.has_cluster_specific_frames());
        assert!(summary.has_server_to_client_frames());
        assert!(summary.has_manufacturer_context());
        assert!(summary.expects_default_responses());
        assert!(summary.has_payloads());
        assert!(!summary.is_empty());
    }

    #[test]
    fn frame_batch_summary_can_record_precomputed_summaries() {
        let summaries = [
            read_attributes_frame(0x01, &[ZclAttributeId::MODEL_IDENTIFIER]).summary(),
            on_off_command_frame(0x02, OnOffCommand::Toggle).summary(),
        ];
        let summary = ZclFrameBatchSummary::from_summaries(&summaries);

        assert_eq!(summary.frame_count, 2);
        assert_eq!(summary.foundation_frames, 1);
        assert_eq!(summary.cluster_specific_frames, 1);
        assert_eq!(summary.total_payload_bytes, 2);
        assert_eq!(summary.max_payload_bytes, 2);
        assert_eq!(summary.payload_frames, 1);
        assert_eq!(
            ZclFrameBatchSummary::empty(),
            ZclFrameBatchSummary::default()
        );
        assert!(ZclFrameBatchSummary::empty().is_empty());
    }

    #[test]
    fn light_command_frames_encode_level_and_color_temperature() {
        let level = move_to_level_with_on_off_frame(0x44, 50, 25);
        let color_temperature = move_to_color_temperature_frame(0x45, 366, 10);

        assert_eq!(
            level.command_id,
            ZCL_LEVEL_MOVE_TO_LEVEL_WITH_ON_OFF_COMMAND_ID
        );
        assert_eq!(level.encode().unwrap(), vec![0x11, 0x44, 0x04, 127, 25, 0]);
        assert_eq!(
            color_temperature.command_id,
            ZCL_COLOR_MOVE_TO_COLOR_TEMPERATURE_COMMAND_ID
        );
        assert_eq!(
            color_temperature.encode().unwrap(),
            vec![0x11, 0x45, 0x0a, 0x6e, 0x01, 10, 0]
        );
        assert_eq!(percentage_to_level(0), 0);
        assert_eq!(percentage_to_level(100), 254);
        assert_eq!(percentage_to_level(250), 254);
    }

    #[test]
    fn parses_on_off_attribute_report_and_maps_to_state_delta() {
        let reports = parse_attribute_reports(
            ZclClusterId::ON_OFF,
            &[0x00, 0x00, ZclDataType::Bool.encode(), 0x01],
        )
        .unwrap();
        let delta = state_delta_for_report(&reports[0]).unwrap();

        assert_eq!(reports[0].value, ZclValue::Bool(true));
        assert_eq!(delta.capability_id, CapabilityId::trusted("light.on_off"));
        assert_eq!(delta.value, Value::Bool(true));
    }

    #[test]
    fn parses_character_string_attribute_report() {
        let reports = parse_attribute_reports(
            ZclClusterId::BASIC,
            &[
                0x04,
                0x00,
                ZclDataType::CharacterString.encode(),
                0x07,
                b'S',
                b'i',
                b'g',
                b'n',
                b'i',
                b'f',
                b'y',
            ],
        )
        .unwrap();

        assert_eq!(
            reports[0],
            ZclAttributeReport {
                cluster_id: ZclClusterId::BASIC,
                attribute_id: ZclAttributeId::MANUFACTURER_NAME,
                data_type: ZclDataType::CharacterString,
                value: ZclValue::CharacterString("Signify".to_string()),
            }
        );
    }

    #[test]
    fn report_attributes_frame_encodes_and_round_trips_reports() {
        let reports = vec![
            ZclAttributeReport {
                cluster_id: ZclClusterId::ON_OFF,
                attribute_id: ZclAttributeId::ON_OFF,
                data_type: ZclDataType::Bool,
                value: ZclValue::Bool(true),
            },
            ZclAttributeReport {
                cluster_id: ZclClusterId::ON_OFF,
                attribute_id: ZclAttributeId(0x0001),
                data_type: ZclDataType::U16,
                value: ZclValue::U16(0x1234),
            },
            ZclAttributeReport {
                cluster_id: ZclClusterId::ON_OFF,
                attribute_id: ZclAttributeId::MANUFACTURER_NAME,
                data_type: ZclDataType::CharacterString,
                value: ZclValue::CharacterString("Acme".to_string()),
            },
        ];

        let frame = report_attributes_frame(0x56, &reports).unwrap();

        assert_eq!(frame.command_id, ZCL_REPORT_ATTRIBUTES_COMMAND_ID);
        assert_eq!(
            frame.encode().unwrap(),
            vec![
                0x10, 0x56, 0x0a, 0x00, 0x00, 0x10, 0x01, 0x01, 0x00, 0x21, 0x34, 0x12, 0x04, 0x00,
                0x42, 0x04, b'A', b'c', b'm', b'e',
            ]
        );
        assert_eq!(
            parse_attribute_reports(ZclClusterId::ON_OFF, &frame.payload).unwrap(),
            reports
        );
    }

    #[test]
    fn attribute_report_encoder_preserves_unknown_raw_values() {
        let reports = vec![ZclAttributeReport {
            cluster_id: ZclClusterId::BASIC,
            attribute_id: ZclAttributeId(0x0099),
            data_type: ZclDataType::Unknown(0xff),
            value: ZclValue::Raw(vec![0xde, 0xad]),
        }];

        let payload = encode_attribute_reports(&reports).unwrap();

        assert_eq!(payload, vec![0x99, 0x00, 0xff, 0xde, 0xad]);
        assert_eq!(
            parse_attribute_reports(ZclClusterId::BASIC, &payload).unwrap(),
            reports
        );
    }

    #[test]
    fn attribute_report_encoder_rejects_wrong_value_shapes() {
        let wrong_shape = [ZclAttributeReport {
            cluster_id: ZclClusterId::ON_OFF,
            attribute_id: ZclAttributeId::ON_OFF,
            data_type: ZclDataType::Bool,
            value: ZclValue::U8(1),
        }];
        let long_string = [ZclAttributeReport {
            cluster_id: ZclClusterId::BASIC,
            attribute_id: ZclAttributeId::MANUFACTURER_NAME,
            data_type: ZclDataType::CharacterString,
            value: ZclValue::CharacterString("x".repeat(256)),
        }];

        assert_eq!(
            encode_attribute_reports(&wrong_shape),
            Err(ZclError::ValueTypeMismatch {
                data_type: ZclDataType::Bool,
            })
        );
        assert_eq!(
            encode_attribute_reports(&long_string),
            Err(ZclError::ValueTooLong { len: 256 })
        );
    }

    #[test]
    fn attribute_report_summary_counts_report_shape() {
        let payload = [
            0x00,
            0x00,
            ZclDataType::Bool.encode(),
            0x01,
            0x07,
            0x00,
            ZclDataType::U16.encode(),
            0x2c,
            0x01,
            0x04,
            0x00,
            ZclDataType::CharacterString.encode(),
            0x04,
            b'T',
            b'e',
            b's',
            b't',
        ];

        let summary = attribute_report_summary(ZclClusterId::ON_OFF, &payload).unwrap();

        assert_eq!(summary.cluster_id, ZclClusterId::ON_OFF);
        assert_eq!(summary.report_count, 3);
        assert_eq!(summary.state_delta_count, 1);
        assert_eq!(summary.bool_reports, 1);
        assert_eq!(summary.numeric_reports, 1);
        assert_eq!(summary.text_reports, 1);
        assert_eq!(summary.raw_reports, 0);
        assert_eq!(summary.unknown_type_reports, 0);
        assert!(summary.has_state_deltas());
        assert!(summary.has_reports());
        assert_eq!(summary.typed_report_count(), 3);
        assert!(summary.has_typed_reports());
        assert!(!summary.has_raw_reports());
        assert!(!summary.has_unknown_type_reports());

        let readiness = summary.readiness();
        assert_eq!(readiness.report_summary, summary);
        assert_eq!(readiness.required_check_count, 5);
        assert_eq!(readiness.passed_check_count, 5);
        assert_eq!(readiness.missing_check_count, 0);
        assert!(readiness.reports_present);
        assert!(readiness.state_delta_coverage_ready);
        assert!(readiness.typed_value_coverage_ready);
        assert!(readiness.raw_reports_absent);
        assert!(readiness.unknown_types_absent);
        assert!(readiness.report_ready);
        assert!(readiness.is_report_ready());
        assert!(!readiness.has_missing_checks());
        assert!(!readiness.needs_report_discovery());
        assert!(!readiness.needs_state_delta_mapping());
        assert!(!readiness.needs_typed_value_mapping());
        assert!(!readiness.has_raw_or_unknown_reports());
    }

    #[test]
    fn attribute_report_summary_counts_unknown_raw_reports() {
        let reports =
            parse_attribute_reports(ZclClusterId::BASIC, &[0x99, 0x00, 0xff, 0xde, 0xad]).unwrap();

        let summary = ZclAttributeReportSummary::from_reports(ZclClusterId::BASIC, &reports);

        assert_eq!(summary.report_count, 1);
        assert_eq!(summary.state_delta_count, 0);
        assert_eq!(summary.raw_reports, 1);
        assert_eq!(summary.unknown_type_reports, 1);
        assert!(!summary.has_state_deltas());
        assert!(summary.has_reports());
        assert_eq!(summary.typed_report_count(), 0);
        assert!(!summary.has_typed_reports());
        assert!(summary.has_raw_reports());
        assert!(summary.has_unknown_type_reports());

        let readiness = ZclAttributeReportReadinessSummary::from_summary(summary);
        assert_eq!(readiness.required_check_count, 5);
        assert_eq!(readiness.passed_check_count, 1);
        assert_eq!(readiness.missing_check_count, 4);
        assert!(readiness.reports_present);
        assert!(!readiness.state_delta_coverage_ready);
        assert!(!readiness.typed_value_coverage_ready);
        assert!(!readiness.raw_reports_absent);
        assert!(!readiness.unknown_types_absent);
        assert!(!readiness.report_ready);
        assert!(!readiness.is_report_ready());
        assert!(readiness.has_missing_checks());
        assert!(!readiness.needs_report_discovery());
        assert!(readiness.needs_state_delta_mapping());
        assert!(readiness.needs_typed_value_mapping());
        assert!(readiness.has_raw_or_unknown_reports());
    }

    #[test]
    fn zcl_report_operator_summary_reports_ready_path() {
        let reports = vec![ZclAttributeReport {
            cluster_id: ZclClusterId::ON_OFF,
            attribute_id: ZclAttributeId::ON_OFF,
            data_type: ZclDataType::Bool,
            value: ZclValue::Bool(true),
        }];
        let frame = report_attributes_frame(0x61, &reports).unwrap();

        let summary = zcl_report_operator_summary([&frame], ZclClusterId::ON_OFF, &reports);

        assert_eq!(summary.required_check_count, 4);
        assert_eq!(summary.passed_check_count, 4);
        assert_eq!(summary.missing_check_count, 0);
        assert_eq!(summary.frame_batch.report_attributes_frames, 1);
        assert_eq!(summary.report_readiness.report_summary.report_count, 1);
        assert!(summary.report_frame_observed);
        assert!(summary.report_payloads_present);
        assert!(summary.no_default_response_backlog);
        assert!(summary.report_ready);
        assert!(summary.operator_ready);
        assert!(summary.is_operator_ready());
        assert!(!summary.has_missing_checks());
        assert!(!summary.needs_report_frame_capture());
        assert!(!summary.needs_report_payloads());
        assert!(!summary.has_default_response_backlog());
        assert!(!summary.needs_report_readiness_work());
    }

    #[test]
    fn zcl_report_operator_summary_routes_blocked_path() {
        let read = ZclFrame::parse(&[0x00, 0x62, ZCL_READ_ATTRIBUTES_COMMAND_ID]).unwrap();
        let reports =
            parse_attribute_reports(ZclClusterId::BASIC, &[0x99, 0x00, 0xff, 0xde, 0xad]).unwrap();

        let summary = zcl_report_operator_summary([&read], ZclClusterId::BASIC, &reports);

        assert_eq!(summary.required_check_count, 4);
        assert_eq!(summary.passed_check_count, 0);
        assert_eq!(summary.missing_check_count, 4);
        assert_eq!(summary.frame_batch.read_attributes_frames, 1);
        assert_eq!(summary.frame_batch.default_response_expected_frames, 1);
        assert_eq!(summary.report_readiness.report_summary.raw_reports, 1);
        assert!(!summary.report_frame_observed);
        assert!(!summary.report_payloads_present);
        assert!(!summary.no_default_response_backlog);
        assert!(!summary.report_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.is_operator_ready());
        assert!(summary.has_missing_checks());
        assert!(summary.needs_report_frame_capture());
        assert!(summary.needs_report_payloads());
        assert!(summary.has_default_response_backlog());
        assert!(summary.needs_report_readiness_work());
    }

    #[test]
    fn zcl_report_signoff_summary_reports_ready_path() {
        let reports = vec![ZclAttributeReport {
            cluster_id: ZclClusterId::ON_OFF,
            attribute_id: ZclAttributeId::ON_OFF,
            data_type: ZclDataType::Bool,
            value: ZclValue::Bool(true),
        }];
        let frame = report_attributes_frame(0x61, &reports).unwrap();

        let summary = zcl_report_signoff_summary([&frame], ZclClusterId::ON_OFF, &reports);

        assert_eq!(
            summary.operator_summary,
            zcl_report_operator_summary([&frame], ZclClusterId::ON_OFF, &reports)
        );
        assert_eq!(summary.required_signoff_check_count, 5);
        assert_eq!(summary.passed_signoff_check_count, 5);
        assert_eq!(summary.missing_signoff_check_count, 0);
        assert!(summary.operator_ready);
        assert!(summary.report_frame_observed);
        assert!(summary.report_payloads_present);
        assert!(summary.default_response_backlog_clear);
        assert!(summary.report_readiness_ready);
        assert!(summary.signoff_ready);
        assert!(summary.is_signoff_ready());
        assert!(!summary.has_missing_signoff_checks());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_report_frame_capture());
        assert!(!summary.needs_report_payloads());
        assert!(!summary.has_default_response_backlog());
        assert!(!summary.needs_report_readiness_work());
    }

    #[test]
    fn zcl_report_signoff_summary_routes_blocked_path() {
        let read = ZclFrame::parse(&[0x00, 0x62, ZCL_READ_ATTRIBUTES_COMMAND_ID]).unwrap();
        let reports =
            parse_attribute_reports(ZclClusterId::BASIC, &[0x99, 0x00, 0xff, 0xde, 0xad]).unwrap();

        let summary = zcl_report_signoff_summary([&read], ZclClusterId::BASIC, &reports);

        assert_eq!(summary.required_signoff_check_count, 5);
        assert_eq!(summary.passed_signoff_check_count, 0);
        assert_eq!(summary.missing_signoff_check_count, 5);
        assert!(!summary.operator_ready);
        assert!(!summary.report_frame_observed);
        assert!(!summary.report_payloads_present);
        assert!(!summary.default_response_backlog_clear);
        assert!(!summary.report_readiness_ready);
        assert!(!summary.signoff_ready);
        assert!(!summary.is_signoff_ready());
        assert!(summary.has_missing_signoff_checks());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_report_frame_capture());
        assert!(summary.needs_report_payloads());
        assert!(summary.has_default_response_backlog());
        assert!(summary.needs_report_readiness_work());
    }

    #[test]
    fn zcl_report_closure_summary_reports_ready_path() {
        let reports = vec![ZclAttributeReport {
            cluster_id: ZclClusterId::ON_OFF,
            attribute_id: ZclAttributeId::ON_OFF,
            data_type: ZclDataType::Bool,
            value: ZclValue::Bool(true),
        }];
        let frame = report_attributes_frame(0x61, &reports).unwrap();

        let summary = zcl_report_closure_summary([&frame], ZclClusterId::ON_OFF, &reports);

        assert_eq!(
            summary.signoff_summary,
            zcl_report_signoff_summary([&frame], ZclClusterId::ON_OFF, &reports)
        );
        assert_eq!(summary.required_closure_check_count, 6);
        assert_eq!(summary.passed_closure_check_count, 6);
        assert_eq!(summary.missing_closure_check_count, 0);
        assert!(summary.signoff_ready);
        assert!(summary.operator_ready);
        assert!(summary.report_frame_observed);
        assert!(summary.report_payloads_present);
        assert!(summary.default_response_backlog_clear);
        assert!(summary.report_readiness_ready);
        assert!(summary.closure_ready);
        assert!(summary.is_closure_ready());
        assert!(!summary.has_missing_closure_checks());
        assert!(!summary.needs_signoff());
        assert!(!summary.needs_operator_readiness());
        assert!(!summary.needs_report_frame_capture());
        assert!(!summary.needs_report_payloads());
        assert!(!summary.has_default_response_backlog());
        assert!(!summary.needs_report_readiness_work());
    }

    #[test]
    fn zcl_report_closure_summary_routes_blocked_path() {
        let read = ZclFrame::parse(&[0x00, 0x62, ZCL_READ_ATTRIBUTES_COMMAND_ID]).unwrap();
        let reports =
            parse_attribute_reports(ZclClusterId::BASIC, &[0x99, 0x00, 0xff, 0xde, 0xad]).unwrap();

        let summary = zcl_report_closure_summary([&read], ZclClusterId::BASIC, &reports);

        assert_eq!(summary.required_closure_check_count, 6);
        assert_eq!(summary.passed_closure_check_count, 0);
        assert_eq!(summary.missing_closure_check_count, 6);
        assert!(!summary.signoff_ready);
        assert!(!summary.operator_ready);
        assert!(!summary.report_frame_observed);
        assert!(!summary.report_payloads_present);
        assert!(!summary.default_response_backlog_clear);
        assert!(!summary.report_readiness_ready);
        assert!(!summary.closure_ready);
        assert!(!summary.is_closure_ready());
        assert!(summary.has_missing_closure_checks());
        assert!(summary.needs_signoff());
        assert!(summary.needs_operator_readiness());
        assert!(summary.needs_report_frame_capture());
        assert!(summary.needs_report_payloads());
        assert!(summary.has_default_response_backlog());
        assert!(summary.needs_report_readiness_work());
    }

    #[test]
    fn maps_level_and_occupancy_reports_to_d23_deltas() {
        let level = ZclAttributeReport {
            cluster_id: ZclClusterId::LEVEL_CONTROL,
            attribute_id: ZclAttributeId::CURRENT_LEVEL,
            data_type: ZclDataType::U8,
            value: ZclValue::U8(254),
        };
        let occupancy = ZclAttributeReport {
            cluster_id: ZclClusterId::OCCUPANCY_SENSING,
            attribute_id: ZclAttributeId::OCCUPANCY,
            data_type: ZclDataType::Bitmap8,
            value: ZclValue::Bitmap8(0x01),
        };

        assert_eq!(
            state_delta_for_report(&level).unwrap().value,
            Value::Percentage(100)
        );
        assert_eq!(
            state_delta_for_report(&occupancy).unwrap().value,
            Value::Bool(true)
        );
    }

    #[test]
    fn maps_environment_measurement_reports_to_d23_deltas() {
        let temperature = ZclAttributeReport {
            cluster_id: ZclClusterId::TEMPERATURE_MEASUREMENT,
            attribute_id: ZclAttributeId::MEASURED_VALUE,
            data_type: ZclDataType::I16,
            value: ZclValue::I16(2312),
        };
        let illuminance = ZclAttributeReport {
            cluster_id: ZclClusterId::ILLUMINANCE_MEASUREMENT,
            attribute_id: ZclAttributeId::MEASURED_VALUE,
            data_type: ZclDataType::U16,
            value: ZclValue::U16(10001),
        };
        let humidity = ZclAttributeReport {
            cluster_id: ZclClusterId::RELATIVE_HUMIDITY_MEASUREMENT,
            attribute_id: ZclAttributeId::MEASURED_VALUE,
            data_type: ZclDataType::U16,
            value: ZclValue::U16(4532),
        };
        let invalid_illuminance = ZclAttributeReport {
            value: ZclValue::U16(0xffff),
            ..illuminance.clone()
        };

        assert_eq!(
            state_delta_for_report(&temperature).unwrap(),
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.temperature"),
                value: Value::Number(23.12),
            }
        );
        assert_eq!(
            state_delta_for_report(&illuminance).unwrap(),
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.illuminance"),
                value: Value::Number(10.0),
            }
        );
        assert_eq!(
            state_delta_for_report(&humidity).unwrap(),
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.humidity"),
                value: Value::Number(45.32),
            }
        );
        assert!(state_delta_for_report(&invalid_illuminance).is_none());
        assert_eq!(centi_celsius_to_celsius(-550), -5.5);
        assert_eq!(centi_percent_to_percent(4532), 45.32);
        assert_eq!(illuminance_measured_value_to_lux(0), Some(0.0));
        assert_eq!(illuminance_measured_value_to_lux(0xffff), None);
    }

    #[test]
    fn common_clusters_project_capabilities() {
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::ON_OFF)[0].capability_id,
            CapabilityId::trusted("light.on_off")
        );
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::DOOR_LOCK)[0].capability_id,
            CapabilityId::trusted("lock.state")
        );
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::TEMPERATURE_MEASUREMENT)[0].capability_id,
            CapabilityId::trusted("sensor.temperature")
        );
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::ILLUMINANCE_MEASUREMENT)[0].capability_id,
            CapabilityId::trusted("sensor.illuminance")
        );
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::RELATIVE_HUMIDITY_MEASUREMENT)[0].capability_id,
            CapabilityId::trusted("sensor.humidity")
        );
        assert!(capabilities_for_cluster(ZclClusterId::BASIC).is_empty());
    }

    #[test]
    fn endpoint_refs_use_nwk_addresses() {
        let endpoint = ZigbeeEndpointRef {
            network_address: NetworkAddress(0x1234),
            endpoint: 11,
        };

        assert_eq!(endpoint.network_address, NetworkAddress(0x1234));
        assert_eq!(endpoint.endpoint, 11);
    }
}
