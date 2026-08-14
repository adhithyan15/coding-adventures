use serde_json::json;
use smart_home_core::{BridgeId, VaultRef};
use smart_home_snmp_integration::{
    SnmpClient, SnmpCommunity, SnmpDeviceConfig, SnmpPoint, SnmpScalar, SnmpValueCodec,
    UdpSnmpTransport,
};
use snmp_protocol::ObjectIdentifier;
use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-snmp-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, endpoint, device_key, point_id, oid, codec, unit] if command == "inspect" => {
            inspect(endpoint, device_key, point_id, oid, codec, unit)
        }
        _ => Err("usage: SMART_HOME_SNMP_VAULT_REF=<opaque-ref> SMART_HOME_SNMP_COMMUNITY=<leased-secret> smart-home-snmp-integration inspect <ip:port> <device-key> <point-id> <oid> <integer|utf8-text|object-identifier|ip-address|counter32|gauge32|timeticks-seconds|counter64-decimal> <unit>".to_string()),
    }
}

fn inspect(
    endpoint: &str,
    device_key: &str,
    point_id: &str,
    oid: &str,
    codec: &str,
    unit: &str,
) -> Result<(), String> {
    let endpoint = endpoint
        .parse::<SocketAddr>()
        .map_err(|_| "endpoint must be an explicit IP address and port".to_string())?;
    let community = env::var("SMART_HOME_SNMP_COMMUNITY")
        .map_err(|_| "SMART_HOME_SNMP_COMMUNITY must be set".to_string())?;
    env::remove_var("SMART_HOME_SNMP_COMMUNITY");
    let community = SnmpCommunity::from_text(community).map_err(|error| error.to_string())?;
    let auth_ref = env::var("SMART_HOME_SNMP_VAULT_REF")
        .map_err(|_| "SMART_HOME_SNMP_VAULT_REF must be set".to_string())?;
    let auth_ref = VaultRef::new(auth_ref).map_err(|error| error.to_string())?;
    let point = SnmpPoint::new(
        point_id,
        point_id.replace('-', " "),
        ObjectIdentifier::parse(oid).map_err(|error| error.to_string())?,
        parse_codec(codec)?,
        unit,
    )
    .map_err(|error| error.to_string())?;
    let config = SnmpDeviceConfig::new(
        BridgeId::trusted("snmp.cli"),
        auth_ref,
        endpoint,
        device_key,
        vec![point],
    )
    .map_err(|error| error.to_string())?;
    let mut client = SnmpClient::new(config, community, UdpSnmpTransport);
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    let measurement = &snapshot.measurements[0];
    let value = match &measurement.value {
        SnmpScalar::Number(value) => json!(value),
        SnmpScalar::Boolean(value) => json!(value),
        SnmpScalar::Text(value) => json!(value),
    };
    println!(
        "{}",
        json!({
            "endpoint": snapshot.endpoint.to_string(),
            "point_id": measurement.point_id,
            "oid": measurement.oid.to_string(),
            "codec": measurement.codec.as_str(),
            "value": value,
            "unit": measurement.unit,
        })
    );
    Ok(())
}

fn parse_codec(value: &str) -> Result<SnmpValueCodec, String> {
    match value {
        "integer" => Ok(SnmpValueCodec::Integer),
        "utf8-text" => Ok(SnmpValueCodec::Utf8Text),
        "object-identifier" => Ok(SnmpValueCodec::ObjectIdentifier),
        "ip-address" => Ok(SnmpValueCodec::IpAddress),
        "counter32" => Ok(SnmpValueCodec::Counter32),
        "gauge32" => Ok(SnmpValueCodec::Gauge32),
        "timeticks-seconds" => Ok(SnmpValueCodec::TimeTicksSeconds),
        "counter64-decimal" => Ok(SnmpValueCodec::Counter64Decimal),
        _ => Err("unsupported SNMP codec".to_string()),
    }
}
