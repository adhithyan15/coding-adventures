//! Smart-home worker capability-cage policy projections.
//!
//! The generic `capability-cage` crate owns OS-level categories such as
//! `net:connect`, `fs:read`, and `proc:exec`. This crate keeps the D23
//! smart-home side explicit: integration workers describe their bridge,
//! worker id, OS needs, and D23 command/read capability hints before a runtime
//! host decides whether to start an in-process worker or a capability-caged
//! sidecar.

#![forbid(unsafe_code)]

use capability_cage::{Action, Capability, Category, InvalidCombination, Manifest};
use smart_home_core::{BridgeId, CapabilityId, IntegrationId, Metadata};
use std::fmt;

pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartHomeCageError {
    EmptyField { field: &'static str },
    InvalidCapability(InvalidCombination),
}

impl fmt::Display for SmartHomeCageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(f, "{field} must not be empty"),
            Self::InvalidCapability(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SmartHomeCageError {}

impl From<InvalidCombination> for SmartHomeCageError {
    fn from(error: InvalidCombination) -> Self {
        Self::InvalidCapability(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmartHomeWorkerId(String);

impl SmartHomeWorkerId {
    pub fn new(value: impl Into<String>) -> Result<Self, SmartHomeCageError> {
        Ok(Self(non_empty("worker_id", value)?))
    }

    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn for_bridge(integration_id: &IntegrationId, bridge_id: &BridgeId) -> Self {
        Self(format!(
            "{}:{}",
            integration_id.as_str(),
            bridge_id.as_str()
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SmartHomeWorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartHomeWorkerMode {
    InProcessRust,
    RustProcess,
    SidecarProcess,
}

impl SmartHomeWorkerMode {
    pub fn requires_process_cage(self) -> bool {
        matches!(self, Self::RustProcess | Self::SidecarProcess)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartHomeWorkerNeed {
    DnsLookup {
        host: String,
        justification: String,
    },
    NetConnect {
        target: String,
        justification: String,
    },
    NetListen {
        bind: String,
        justification: String,
    },
    FileRead {
        path: String,
        justification: String,
    },
    FileWrite {
        path: String,
        justification: String,
    },
    ProcessExec {
        binary: String,
        justification: String,
    },
    StdoutWrite {
        target: String,
        justification: String,
    },
    TimeRead {
        target: String,
        justification: String,
    },
    TimeSleep {
        target: String,
        justification: String,
    },
}

impl SmartHomeWorkerNeed {
    pub fn dns(host: impl Into<String>) -> Result<Self, SmartHomeCageError> {
        Ok(Self::DnsLookup {
            host: non_empty("dns_host", host)?,
            justification: "resolve smart-home endpoint host".to_string(),
        })
    }

    pub fn net_connect(
        target: impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(Self::NetConnect {
            target: non_empty("net_connect_target", target)?,
            justification: justification.into(),
        })
    }

    pub fn net_listen(
        bind: impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(Self::NetListen {
            bind: non_empty("net_listen_bind", bind)?,
            justification: justification.into(),
        })
    }

    pub fn file_read(
        path: impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(Self::FileRead {
            path: non_empty("file_read_path", path)?,
            justification: justification.into(),
        })
    }

    pub fn file_write(
        path: impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(Self::FileWrite {
            path: non_empty("file_write_path", path)?,
            justification: justification.into(),
        })
    }

    pub fn process_exec(
        binary: impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(Self::ProcessExec {
            binary: non_empty("process_exec_binary", binary)?,
            justification: justification.into(),
        })
    }

    pub fn stdout_write(target: impl Into<String>) -> Result<Self, SmartHomeCageError> {
        Ok(Self::StdoutWrite {
            target: non_empty("stdout_target", target)?,
            justification: "emit worker diagnostics and audit breadcrumbs".to_string(),
        })
    }

    pub fn time_read(target: impl Into<String>) -> Result<Self, SmartHomeCageError> {
        Ok(Self::TimeRead {
            target: non_empty("time_target", target)?,
            justification: "compute worker freshness and timeout deadlines".to_string(),
        })
    }

    pub fn time_sleep(target: impl Into<String>) -> Result<Self, SmartHomeCageError> {
        Ok(Self::TimeSleep {
            target: non_empty("time_sleep_target", target)?,
            justification: "wait between supervised polling or reconnect attempts".to_string(),
        })
    }

    pub fn category_action_target(&self) -> (Category, Action, &str) {
        match self {
            Self::DnsLookup { host, .. } => (Category::Net, Action::Dns, host),
            Self::NetConnect { target, .. } => (Category::Net, Action::Connect, target),
            Self::NetListen { bind, .. } => (Category::Net, Action::Listen, bind),
            Self::FileRead { path, .. } => (Category::Fs, Action::Read, path),
            Self::FileWrite { path, .. } => (Category::Fs, Action::Write, path),
            Self::ProcessExec { binary, .. } => (Category::Proc, Action::Exec, binary),
            Self::StdoutWrite { target, .. } => (Category::Stdout, Action::Write, target),
            Self::TimeRead { target, .. } => (Category::Time, Action::Read, target),
            Self::TimeSleep { target, .. } => (Category::Time, Action::Sleep, target),
        }
    }

    pub fn justification(&self) -> &str {
        match self {
            Self::DnsLookup { justification, .. }
            | Self::NetConnect { justification, .. }
            | Self::NetListen { justification, .. }
            | Self::FileRead { justification, .. }
            | Self::FileWrite { justification, .. }
            | Self::ProcessExec { justification, .. }
            | Self::StdoutWrite { justification, .. }
            | Self::TimeRead { justification, .. }
            | Self::TimeSleep { justification, .. } => justification,
        }
    }

    pub fn to_capability(&self) -> Result<Capability, SmartHomeCageError> {
        let (category, action, target) = self.category_action_target();
        Ok(Capability::new(
            category,
            action,
            target,
            self.justification(),
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomeCageProfile {
    pub integration_id: IntegrationId,
    pub bridge_id: Option<BridgeId>,
    pub worker_id: SmartHomeWorkerId,
    pub mode: SmartHomeWorkerMode,
    pub needs: Vec<SmartHomeWorkerNeed>,
    pub smart_home_capabilities: Vec<CapabilityId>,
    pub metadata: Vec<Metadata>,
}

impl SmartHomeCageProfile {
    pub fn new(
        integration_id: IntegrationId,
        worker_id: SmartHomeWorkerId,
        mode: SmartHomeWorkerMode,
    ) -> Self {
        Self {
            integration_id,
            bridge_id: None,
            worker_id,
            mode,
            needs: Vec::new(),
            smart_home_capabilities: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn for_bridge(
        integration_id: IntegrationId,
        bridge_id: BridgeId,
        mode: SmartHomeWorkerMode,
    ) -> Self {
        let worker_id = SmartHomeWorkerId::for_bridge(&integration_id, &bridge_id);
        Self::new(integration_id, worker_id, mode).with_bridge(bridge_id)
    }

    pub fn local_http(
        integration_id: IntegrationId,
        bridge_id: BridgeId,
        host: impl Into<String>,
        port: u16,
    ) -> Result<Self, SmartHomeCageError> {
        let host = non_empty("host", host)?;
        Ok(
            Self::for_bridge(integration_id, bridge_id, SmartHomeWorkerMode::RustProcess)
                .with_need(SmartHomeWorkerNeed::dns(host.clone())?)
                .with_need(SmartHomeWorkerNeed::net_connect(
                    endpoint(&host, port),
                    "call local smart-home HTTP API",
                )?)
                .with_need(SmartHomeWorkerNeed::time_read("clock")?)
                .with_need(SmartHomeWorkerNeed::stdout_write("worker-log")?)
                .with_smart_home_capability(CapabilityId::trusted("smart_home.read")),
        )
    }

    pub fn mqtt_subscription(
        integration_id: IntegrationId,
        broker_id: BridgeId,
        host: impl Into<String>,
        port: u16,
    ) -> Result<Self, SmartHomeCageError> {
        let host = non_empty("mqtt_host", host)?;
        Ok(
            Self::for_bridge(integration_id, broker_id, SmartHomeWorkerMode::RustProcess)
                .with_need(SmartHomeWorkerNeed::dns(host.clone())?)
                .with_need(SmartHomeWorkerNeed::net_connect(
                    endpoint(&host, port),
                    "subscribe to MQTT smart-home event topics",
                )?)
                .with_need(SmartHomeWorkerNeed::time_read("clock")?)
                .with_need(SmartHomeWorkerNeed::time_sleep("backoff")?)
                .with_smart_home_capability(CapabilityId::trusted("smart_home.read")),
        )
    }

    pub fn serial_adapter(
        integration_id: IntegrationId,
        bridge_id: BridgeId,
        device_path: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        let device_path = non_empty("serial_device_path", device_path)?;
        Ok(
            Self::for_bridge(integration_id, bridge_id, SmartHomeWorkerMode::RustProcess)
                .with_need(SmartHomeWorkerNeed::file_read(
                    device_path.clone(),
                    "read radio adapter frames",
                )?)
                .with_need(SmartHomeWorkerNeed::file_write(
                    device_path,
                    "write radio adapter commands",
                )?)
                .with_need(SmartHomeWorkerNeed::time_read("clock")?)
                .with_smart_home_capability(CapabilityId::trusted("smart_home.read"))
                .with_smart_home_capability(CapabilityId::trusted("smart_home.command.radio")),
        )
    }

    pub fn cloud_api(
        integration_id: IntegrationId,
        worker_id: SmartHomeWorkerId,
        host: impl Into<String>,
        port: u16,
    ) -> Result<Self, SmartHomeCageError> {
        let host = non_empty("cloud_host", host)?;
        Ok(
            Self::new(integration_id, worker_id, SmartHomeWorkerMode::RustProcess)
                .with_need(SmartHomeWorkerNeed::dns(host.clone())?)
                .with_need(SmartHomeWorkerNeed::net_connect(
                    endpoint(&host, port),
                    "call vendor cloud API",
                )?)
                .with_need(SmartHomeWorkerNeed::time_read("clock")?)
                .with_smart_home_capability(CapabilityId::trusted("smart_home.read")),
        )
    }

    pub fn webhook_receiver(
        integration_id: IntegrationId,
        worker_id: SmartHomeWorkerId,
        bind: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(
            Self::new(integration_id, worker_id, SmartHomeWorkerMode::RustProcess)
                .with_need(SmartHomeWorkerNeed::net_listen(
                    bind,
                    "receive smart-home webhook callbacks",
                )?)
                .with_need(SmartHomeWorkerNeed::time_read("clock")?)
                .with_smart_home_capability(CapabilityId::trusted("smart_home.read")),
        )
    }

    pub fn sidecar_process(
        integration_id: IntegrationId,
        worker_id: SmartHomeWorkerId,
        binary: impl Into<String>,
    ) -> Result<Self, SmartHomeCageError> {
        Ok(Self::new(
            integration_id,
            worker_id,
            SmartHomeWorkerMode::SidecarProcess,
        )
        .with_need(SmartHomeWorkerNeed::process_exec(
            binary,
            "launch declared smart-home sidecar",
        )?)
        .with_need(SmartHomeWorkerNeed::stdout_write("sidecar-log")?)
        .with_need(SmartHomeWorkerNeed::time_read("clock")?))
    }

    pub fn with_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn with_need(mut self, need: SmartHomeWorkerNeed) -> Self {
        self.needs.push(need);
        self
    }

    pub fn with_smart_home_capability(mut self, capability_id: CapabilityId) -> Self {
        if !self
            .smart_home_capabilities
            .iter()
            .any(|existing| existing == &capability_id)
        {
            self.smart_home_capabilities.push(capability_id);
        }
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }

    pub fn os_capabilities(&self) -> Result<Vec<Capability>, SmartHomeCageError> {
        let mut capabilities = Vec::with_capacity(self.needs.len());
        for need in &self.needs {
            let capability = need.to_capability()?;
            if !capabilities.iter().any(|existing| existing == &capability) {
                capabilities.push(capability);
            }
        }
        Ok(capabilities)
    }

    pub fn manifest(&self) -> Result<Manifest, SmartHomeCageError> {
        Ok(Manifest::new(self.os_capabilities()?))
    }

    pub fn allows_need(&self, need: &SmartHomeWorkerNeed) -> Result<bool, SmartHomeCageError> {
        let (category, action, target) = need.category_action_target();
        Ok(self.manifest()?.has(category, action, target))
    }

    pub fn requires_process_cage(&self) -> bool {
        self.mode.requires_process_cage() || !self.needs.is_empty()
    }
}

fn endpoint(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, SmartHomeCageError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(SmartHomeCageError::EmptyField { field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge_id() -> BridgeId {
        BridgeId::trusted("bridge-1")
    }

    #[test]
    fn local_http_profiles_project_to_cage_manifest() {
        let profile = SmartHomeCageProfile::local_http(
            IntegrationId::trusted("hue"),
            bridge_id(),
            "hue.local",
            443,
        )
        .unwrap();
        let manifest = profile.manifest().unwrap();

        assert_eq!(profile.worker_id.as_str(), "hue:bridge-1");
        assert!(profile.requires_process_cage());
        assert!(manifest.has(Category::Net, Action::Dns, "hue.local"));
        assert!(manifest.has(Category::Net, Action::Connect, "hue.local:443"));
        assert!(manifest.has(Category::Time, Action::Read, "clock"));
        assert!(manifest.has(Category::Stdout, Action::Write, "worker-log"));
        assert_eq!(
            profile.smart_home_capabilities,
            vec![CapabilityId::trusted("smart_home.read")]
        );
    }

    #[test]
    fn mqtt_subscription_profiles_include_backoff_sleep() {
        let profile = SmartHomeCageProfile::mqtt_subscription(
            IntegrationId::trusted("zigbee2mqtt"),
            BridgeId::trusted("broker-1"),
            "mqtt.local",
            1883,
        )
        .unwrap();

        assert!(profile
            .manifest()
            .unwrap()
            .has(Category::Time, Action::Sleep, "backoff"));
        assert!(profile
            .allows_need(
                &SmartHomeWorkerNeed::net_connect(
                    "mqtt.local:1883",
                    "subscribe to MQTT smart-home event topics"
                )
                .unwrap()
            )
            .unwrap());
    }

    #[test]
    fn serial_adapter_profiles_project_read_write_device_access() {
        let profile = SmartHomeCageProfile::serial_adapter(
            IntegrationId::trusted("zwave"),
            bridge_id(),
            "/dev/tty.usbmodem1",
        )
        .unwrap();
        let manifest = profile.manifest().unwrap();

        assert!(manifest.has(Category::Fs, Action::Read, "/dev/tty.usbmodem1"));
        assert!(manifest.has(Category::Fs, Action::Write, "/dev/tty.usbmodem1"));
        assert!(profile
            .smart_home_capabilities
            .contains(&CapabilityId::trusted("smart_home.command.radio")));
    }

    #[test]
    fn webhook_profiles_listen_without_connect() {
        let profile = SmartHomeCageProfile::webhook_receiver(
            IntegrationId::trusted("cloud-webhook"),
            SmartHomeWorkerId::trusted("cloud-webhook:listener"),
            "127.0.0.1:8080",
        )
        .unwrap();
        let manifest = profile.manifest().unwrap();

        assert!(manifest.has(Category::Net, Action::Listen, "127.0.0.1:8080"));
        assert!(!manifest.has(Category::Net, Action::Connect, "127.0.0.1:8080"));
    }

    #[test]
    fn sidecar_profiles_require_declared_process_exec() {
        let profile = SmartHomeCageProfile::sidecar_process(
            IntegrationId::trusted("homey-pro"),
            SmartHomeWorkerId::trusted("homey-pro:sidecar"),
            "/opt/smart-home/homey-adapter",
        )
        .unwrap();
        let manifest = profile.manifest().unwrap();

        assert_eq!(profile.mode, SmartHomeWorkerMode::SidecarProcess);
        assert!(manifest.has(
            Category::Proc,
            Action::Exec,
            "/opt/smart-home/homey-adapter"
        ));
        assert!(manifest.has(Category::Stdout, Action::Write, "sidecar-log"));
    }

    #[test]
    fn duplicate_needs_are_collapsed_in_manifest_projection() {
        let need = SmartHomeWorkerNeed::time_read("clock").unwrap();
        let profile = SmartHomeCageProfile::new(
            IntegrationId::trusted("test"),
            SmartHomeWorkerId::trusted("test-worker"),
            SmartHomeWorkerMode::InProcessRust,
        )
        .with_need(need.clone())
        .with_need(need);

        assert_eq!(profile.needs.len(), 2);
        assert_eq!(profile.os_capabilities().unwrap().len(), 1);
    }

    #[test]
    fn empty_worker_fields_are_rejected() {
        assert_eq!(
            SmartHomeWorkerId::new(" ").unwrap_err(),
            SmartHomeCageError::EmptyField { field: "worker_id" }
        );
        assert_eq!(
            SmartHomeWorkerNeed::net_connect(" ", "x").unwrap_err(),
            SmartHomeCageError::EmptyField {
                field: "net_connect_target"
            }
        );
    }
}
