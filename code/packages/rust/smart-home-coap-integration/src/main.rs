use serde_json::json;
use smart_home_coap_integration::{
    CoapClient, CoapDeviceConfig, CoapPoint, CoapScalar, CoapValueCodec, UdpCoapTransport,
};
use smart_home_core::BridgeId;
use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-coap-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, endpoint, device_key, point_id, path, codec, unit]
            if command == "inspect" =>
        {
            inspect(endpoint, device_key, point_id, path, codec, unit)
        }
        _ => Err("usage: smart-home-coap-integration inspect <ip:port> <device-key> <point-id> <path> <text-number|text|json-number:key|json-boolean:key|json-text:key> <unit>".to_string()),
    }
}

fn inspect(
    endpoint: &str,
    device_key: &str,
    point_id: &str,
    path: &str,
    codec: &str,
    unit: &str,
) -> Result<(), String> {
    let endpoint = endpoint
        .parse::<SocketAddr>()
        .map_err(|_| "endpoint must be an explicit IP address and port".to_string())?;
    let point = CoapPoint::new(
        point_id,
        point_id.replace('-', " "),
        path,
        parse_codec(codec)?,
        unit,
    )
    .map_err(|error| error.to_string())?;
    let config = CoapDeviceConfig::new(
        BridgeId::trusted("coap.cli"),
        endpoint,
        device_key,
        vec![point],
    )
    .map_err(|error| error.to_string())?;
    let mut client = CoapClient::new(config, UdpCoapTransport);
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    let measurement = &snapshot.measurements[0];
    let value = match &measurement.value {
        CoapScalar::Number(value) => json!(value),
        CoapScalar::Boolean(value) => json!(value),
        CoapScalar::Text(value) => json!(value),
    };
    println!(
        "{}",
        json!({
            "endpoint": snapshot.endpoint.to_string(),
            "point_id": measurement.point_id,
            "path": measurement.path,
            "codec": measurement.codec.as_str(),
            "value": value,
            "unit": measurement.unit,
        })
    );
    Ok(())
}

fn parse_codec(value: &str) -> Result<CoapValueCodec, String> {
    match value {
        "text-number" => Ok(CoapValueCodec::TextNumber),
        "text" => Ok(CoapValueCodec::Text),
        _ => {
            let (kind, key) = value.split_once(':').ok_or_else(|| {
                "codec must be text-number, text, json-number:key, json-boolean:key, or json-text:key"
                    .to_string()
            })?;
            match kind {
                "json-number" => Ok(CoapValueCodec::JsonNumber {
                    key: key.to_string(),
                }),
                "json-boolean" => Ok(CoapValueCodec::JsonBoolean {
                    key: key.to_string(),
                }),
                "json-text" => Ok(CoapValueCodec::JsonText {
                    key: key.to_string(),
                }),
                _ => Err("unsupported codec".to_string()),
            }
        }
    }
}
