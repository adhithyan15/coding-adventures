use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_nut_ups_integration::{
    NutClient, NutDeviceConfig, NutPoint, NutScalar, NutValueCodec, TcpNutTransport,
};
use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-nut-ups-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, endpoint, ups_name, device_key, point_id, variable, codec, unit]
            if command == "inspect" =>
        {
            inspect(
                endpoint, ups_name, device_key, point_id, variable, codec, unit,
            )
        }
        _ => Err("usage: smart-home-nut-ups-integration inspect <ip:port> <ups-name> <device-key> <point-id> <variable> <decimal|text> <unit>".to_string()),
    }
}

fn inspect(
    endpoint: &str,
    ups_name: &str,
    device_key: &str,
    point_id: &str,
    variable: &str,
    codec: &str,
    unit: &str,
) -> Result<(), String> {
    let endpoint = endpoint
        .parse::<SocketAddr>()
        .map_err(|_| "endpoint must be an explicit IP address and port".to_string())?;
    let codec = match codec {
        "decimal" => NutValueCodec::Decimal,
        "text" => NutValueCodec::Text,
        _ => return Err("unsupported NUT codec".to_string()),
    };
    let point = NutPoint::new(point_id, point_id.replace('-', " "), variable, codec, unit)
        .map_err(|error| error.to_string())?;
    let config = NutDeviceConfig::new(
        BridgeId::trusted("nut.cli"),
        endpoint,
        ups_name,
        device_key,
        vec![point],
    )
    .map_err(|error| error.to_string())?;
    let mut client = NutClient::new(config, TcpNutTransport);
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    let measurement = &snapshot.measurements[0];
    let value = match &measurement.value {
        NutScalar::Number(value) => json!(value),
        NutScalar::Boolean(value) => json!(value),
        NutScalar::Text(value) => json!(value),
    };
    println!(
        "{}",
        json!({
            "endpoint": snapshot.endpoint.to_string(),
            "ups_name": snapshot.ups_name,
            "point_id": measurement.point_id,
            "variable": measurement.variable,
            "codec": measurement.codec.as_str(),
            "value": value,
            "unit": measurement.unit,
        })
    );
    Ok(())
}
