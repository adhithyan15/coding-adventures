use modbus_protocol::RegisterTable;
use serde_json::json;
use smart_home_core::BridgeId;
use smart_home_modbus_tcp_integration::{
    ModbusClient, ModbusPoint, ModbusTcpConfig, ModbusTcpTransport, RegisterEncoding, DEFAULT_PORT,
    MAX_PROFILE_POINTS,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("smart-home-modbus-tcp-integration: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, host, unit, table, address, quantity] if command == "inspect" => inspect(
            host,
            parse_u8(unit, "unit id")?,
            parse_table(table)?,
            parse_u16(address, "starting address")?,
            parse_quantity(quantity)?,
            DEFAULT_PORT,
        ),
        [command, host, unit, table, address, quantity, port] if command == "inspect" => inspect(
            host,
            parse_u8(unit, "unit id")?,
            parse_table(table)?,
            parse_u16(address, "starting address")?,
            parse_quantity(quantity)?,
            parse_u16(port, "port")?,
        ),
        _ => Err("usage: smart-home-modbus-tcp-integration inspect <host> <unit-id> <holding|input> <address> <quantity> [port]".to_string()),
    }
}

fn inspect(
    host: &str,
    unit_id: u8,
    table: RegisterTable,
    address: u16,
    quantity: u16,
    port: u16,
) -> Result<(), String> {
    let points = (0..quantity)
        .map(|offset| {
            let register = address
                .checked_add(offset)
                .ok_or_else(|| "register range exceeds u16".to_string())?;
            ModbusPoint::new(
                format!("register-{register}"),
                format!("Register {register}"),
                table,
                register,
                RegisterEncoding::Unsigned16,
                "raw",
            )
            .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let config = ModbusTcpConfig::new(BridgeId::trusted("modbus.cli"), host, unit_id, points)
        .map_err(|error| error.to_string())?
        .with_port(port);
    let mut client = ModbusClient::new(config, ModbusTcpTransport);
    let snapshot = client.inspect().map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "host": host,
            "port": port,
            "unit_id": snapshot.unit_id,
            "table": table.as_str(),
            "measurements": snapshot.measurements.iter().map(|measurement| json!({
                "address": measurement.address,
                "value": measurement.value,
                "unit": measurement.unit,
            })).collect::<Vec<_>>(),
        })
    );
    Ok(())
}

fn parse_table(value: &str) -> Result<RegisterTable, String> {
    match value {
        "holding" => Ok(RegisterTable::Holding),
        "input" => Ok(RegisterTable::Input),
        _ => Err("table must be `holding` or `input`".to_string()),
    }
}

fn parse_quantity(value: &str) -> Result<u16, String> {
    let quantity = parse_u16(value, "quantity")?;
    if quantity == 0 || usize::from(quantity) > MAX_PROFILE_POINTS {
        return Err(format!(
            "quantity must be between 1 and {MAX_PROFILE_POINTS}"
        ));
    }
    Ok(quantity)
}

fn parse_u8(value: &str, name: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("{name} must be an unsigned 8-bit integer"))
}

fn parse_u16(value: &str, name: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| format!("{name} must be an unsigned 16-bit integer"))
}
