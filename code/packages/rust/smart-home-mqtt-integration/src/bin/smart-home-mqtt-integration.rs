use smart_home_core::{BridgeId, VaultRef};
use smart_home_mqtt_integration::{
    MqttBrokerConfig, MqttCredentials, MqttHostOutcome, MqttRuntimeHost,
};
use smart_home_runtime::SmartHomeRuntime;
use std::env;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.is_empty() || arguments.iter().any(|argument| argument == "--help") {
        print_help();
        return Ok(());
    }
    if arguments.len() < 3 {
        print_help();
        return Err("expected HOST PORT CLIENT_ID".to_string());
    }
    let host = arguments[0].clone();
    let port = arguments[1]
        .parse::<u16>()
        .map_err(|error| format!("invalid MQTT port: {error}"))?;
    let client_id = arguments[2].clone();
    let event_limit = option_value(&arguments[3..], "--events")
        .unwrap_or("1")
        .parse::<usize>()
        .map_err(|error| format!("invalid --events value: {error}"))?;
    let timeout_ms = option_value(&arguments[3..], "--timeout-ms")
        .unwrap_or("5000")
        .parse::<u64>()
        .map_err(|error| format!("invalid --timeout-ms value: {error}"))?;
    let mut config = MqttBrokerConfig::new(
        BridgeId::trusted(format!("mqtt-broker:{host}:{port}")),
        host,
        port,
        client_id,
    );
    if let Ok(vault_ref) = env::var("SMART_HOME_MQTT_VAULT_REF") {
        config = config.with_auth_ref(VaultRef::trusted(vault_ref));
    }
    let credentials = match (
        env::var("SMART_HOME_MQTT_USERNAME").ok(),
        env::var("SMART_HOME_MQTT_PASSWORD").ok(),
    ) {
        (Some(username), Some(password)) => Some(MqttCredentials::new(username, password)),
        (None, None) => None,
        _ => {
            return Err(
                "SMART_HOME_MQTT_USERNAME and SMART_HOME_MQTT_PASSWORD must be set together"
                    .to_string(),
            );
        }
    };
    let mut runtime_host =
        MqttRuntimeHost::open(config, credentials, SmartHomeRuntime::new(), now_ms()?)
            .map_err(|error| error.to_string())?;
    let mut observed = 0;
    while observed < event_limit {
        match runtime_host
            .poll_once(Duration::from_millis(timeout_ms), now_ms()?)
            .map_err(|error| error.to_string())?
        {
            MqttHostOutcome::Idle => {}
            outcome => {
                observed += 1;
                println!("{outcome:?}");
            }
        }
    }
    Ok(())
}

fn option_value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn now_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))
}

fn print_help() {
    println!(
        "smart-home-mqtt-integration HOST PORT CLIENT_ID [--events N] [--timeout-ms MS]\n\
         Credentials: SMART_HOME_MQTT_USERNAME and SMART_HOME_MQTT_PASSWORD\n\
         Durable credential reference: SMART_HOME_MQTT_VAULT_REF"
    );
}
