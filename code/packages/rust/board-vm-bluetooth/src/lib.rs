#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothEndpoint {
    BleGatt(BleGattEndpoint),
    Rfcomm(RfcommEndpoint),
}

impl BluetoothEndpoint {
    pub const fn transport(&self) -> BluetoothEndpointTransport {
        match self {
            Self::BleGatt(_) => BluetoothEndpointTransport::BleGatt,
            Self::Rfcomm(_) => BluetoothEndpointTransport::Rfcomm,
        }
    }

    pub fn device(&self) -> &str {
        match self {
            Self::BleGatt(endpoint) => &endpoint.device,
            Self::Rfcomm(endpoint) => &endpoint.device,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothEndpointTransport {
    BleGatt,
    Rfcomm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleGattEndpoint {
    pub endpoint: String,
    pub device: String,
    pub service_uuid: String,
    pub write_characteristic_uuid: String,
    pub notify_characteristic_uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfcommEndpoint {
    pub endpoint: String,
    pub device: String,
    pub channel: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothEndpointError {
    UnsupportedScheme(String),
    MissingDevice,
    MissingServiceUuid,
    MissingWriteCharacteristicUuid,
    MissingNotifyCharacteristicUuid,
    MissingChannel,
    InvalidChannel(String),
    InvalidUuid { field: &'static str, value: String },
}

pub fn parse_bluetooth_endpoint(
    endpoint: &str,
) -> Result<BluetoothEndpoint, BluetoothEndpointError> {
    let endpoint = endpoint.trim();
    if endpoint.starts_with("ble://") {
        return parse_ble_gatt_endpoint(endpoint).map(BluetoothEndpoint::BleGatt);
    }
    if endpoint.starts_with("btspp://") || endpoint.starts_with("rfcomm://") {
        return parse_rfcomm_endpoint(endpoint).map(BluetoothEndpoint::Rfcomm);
    }

    let scheme = endpoint
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .unwrap_or("");
    Err(BluetoothEndpointError::UnsupportedScheme(scheme.to_owned()))
}

pub fn parse_ble_gatt_endpoint(endpoint: &str) -> Result<BleGattEndpoint, BluetoothEndpointError> {
    let body = endpoint.trim().strip_prefix("ble://").ok_or_else(|| {
        BluetoothEndpointError::UnsupportedScheme(endpoint_scheme(endpoint).to_owned())
    })?;
    let (path, query) = body.split_once('?').unwrap_or((body, ""));
    let mut path_parts = path.split('/').filter(|part| !part.is_empty());
    let device = path_parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or(BluetoothEndpointError::MissingDevice)?;
    let service_uuid = query_value(query, "service")
        .or_else(|| path_parts.next())
        .ok_or(BluetoothEndpointError::MissingServiceUuid)?;
    let write_characteristic_uuid = query_value(query, "write")
        .or_else(|| query_value(query, "tx"))
        .or_else(|| path_parts.next())
        .ok_or(BluetoothEndpointError::MissingWriteCharacteristicUuid)?;
    let notify_characteristic_uuid = query_value(query, "notify")
        .or_else(|| query_value(query, "rx"))
        .or_else(|| path_parts.next())
        .ok_or(BluetoothEndpointError::MissingNotifyCharacteristicUuid)?;

    validate_uuid("service", service_uuid)?;
    validate_uuid("write", write_characteristic_uuid)?;
    validate_uuid("notify", notify_characteristic_uuid)?;

    Ok(BleGattEndpoint {
        endpoint: endpoint.trim().to_owned(),
        device: device.to_owned(),
        service_uuid: service_uuid.to_owned(),
        write_characteristic_uuid: write_characteristic_uuid.to_owned(),
        notify_characteristic_uuid: notify_characteristic_uuid.to_owned(),
    })
}

pub fn parse_rfcomm_endpoint(endpoint: &str) -> Result<RfcommEndpoint, BluetoothEndpointError> {
    let body = endpoint
        .trim()
        .strip_prefix("btspp://")
        .or_else(|| endpoint.trim().strip_prefix("rfcomm://"))
        .ok_or_else(|| {
            BluetoothEndpointError::UnsupportedScheme(endpoint_scheme(endpoint).to_owned())
        })?;
    let (device, channel) = body
        .rsplit_once(':')
        .ok_or(BluetoothEndpointError::MissingChannel)?;
    if device.trim().is_empty() {
        return Err(BluetoothEndpointError::MissingDevice);
    }
    let channel_text = channel;
    let channel = channel_text
        .parse::<u8>()
        .map_err(|_| BluetoothEndpointError::InvalidChannel(channel_text.to_owned()))?;
    if !(1..=30).contains(&channel) {
        return Err(BluetoothEndpointError::InvalidChannel(
            channel_text.to_owned(),
        ));
    }

    Ok(RfcommEndpoint {
        endpoint: endpoint.trim().to_owned(),
        device: device.to_owned(),
        channel,
    })
}

fn endpoint_scheme(endpoint: &str) -> &str {
    endpoint
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .unwrap_or("")
}

fn query_value<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key && !value.is_empty()).then_some(value)
    })
}

fn validate_uuid(field: &'static str, value: &str) -> Result<(), BluetoothEndpointError> {
    if is_short_uuid(value) || is_canonical_uuid(value) {
        return Ok(());
    }

    Err(BluetoothEndpointError::InvalidUuid {
        field,
        value: value.to_owned(),
    })
}

fn is_short_uuid(value: &str) -> bool {
    matches!(value.len(), 4 | 8) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_canonical_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| {
        matches!(index, 8 | 13 | 18 | 23) && byte == b'-'
            || !matches!(index, 8 | 13 | 18 | 23) && byte.is_ascii_hexdigit()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE_UUID: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
    const WRITE_UUID: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
    const NOTIFY_UUID: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

    #[test]
    fn parses_ble_gatt_query_endpoint() {
        let endpoint = parse_ble_gatt_endpoint(&format!(
            "ble://AA:BB:CC:DD:EE:FF?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
        ))
        .unwrap();

        assert_eq!(endpoint.device, "AA:BB:CC:DD:EE:FF");
        assert_eq!(endpoint.service_uuid, SERVICE_UUID);
        assert_eq!(endpoint.write_characteristic_uuid, WRITE_UUID);
        assert_eq!(endpoint.notify_characteristic_uuid, NOTIFY_UUID);
    }

    #[test]
    fn parses_ble_gatt_path_endpoint_and_short_uuids() {
        let endpoint = parse_ble_gatt_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a").unwrap();

        assert_eq!(endpoint.device, "uno-r4-wifi");
        assert_eq!(endpoint.service_uuid, "180f");
        assert_eq!(endpoint.write_characteristic_uuid, "2a19");
        assert_eq!(endpoint.notify_characteristic_uuid, "2a1a");
    }

    #[test]
    fn rejects_incomplete_ble_gatt_endpoint() {
        assert_eq!(
            parse_ble_gatt_endpoint("ble://esp32?service=180f").unwrap_err(),
            BluetoothEndpointError::MissingWriteCharacteristicUuid
        );
        assert_eq!(
            parse_ble_gatt_endpoint(&format!(
                "ble://esp32?service={SERVICE_UUID}&write=not-a-uuid&notify={NOTIFY_UUID}"
            ))
            .unwrap_err(),
            BluetoothEndpointError::InvalidUuid {
                field: "write",
                value: "not-a-uuid".to_owned()
            }
        );
    }

    #[test]
    fn parses_bluetooth_classic_rfcomm_endpoints() {
        let btspp = parse_rfcomm_endpoint("btspp://ESP32-BoardVM:1").unwrap();
        let rfcomm = parse_bluetooth_endpoint("rfcomm://AA:BB:CC:DD:EE:FF:12").unwrap();

        assert_eq!(btspp.device, "ESP32-BoardVM");
        assert_eq!(btspp.channel, 1);
        assert_eq!(
            rfcomm,
            BluetoothEndpoint::Rfcomm(RfcommEndpoint {
                endpoint: "rfcomm://AA:BB:CC:DD:EE:FF:12".to_owned(),
                device: "AA:BB:CC:DD:EE:FF".to_owned(),
                channel: 12
            })
        );
    }

    #[test]
    fn rejects_bad_rfcomm_channels() {
        assert_eq!(
            parse_rfcomm_endpoint("btspp://ESP32-BoardVM:0").unwrap_err(),
            BluetoothEndpointError::InvalidChannel("0".to_owned())
        );
        assert_eq!(
            parse_rfcomm_endpoint("btspp://ESP32-BoardVM:31").unwrap_err(),
            BluetoothEndpointError::InvalidChannel("31".to_owned())
        );
        assert_eq!(
            parse_rfcomm_endpoint("btspp://ESP32-BoardVM").unwrap_err(),
            BluetoothEndpointError::MissingChannel
        );
    }

    #[test]
    fn dispatches_supported_bluetooth_endpoint_schemes() {
        assert_eq!(
            parse_bluetooth_endpoint(&format!(
                "ble://esp32?service={SERVICE_UUID}&write={WRITE_UUID}&notify={NOTIFY_UUID}"
            ))
            .unwrap()
            .transport(),
            BluetoothEndpointTransport::BleGatt
        );
        assert_eq!(
            parse_bluetooth_endpoint("btspp://ESP32-BoardVM:3")
                .unwrap()
                .transport(),
            BluetoothEndpointTransport::Rfcomm
        );
        assert_eq!(
            parse_bluetooth_endpoint("tcp://board-vm.local:4170").unwrap_err(),
            BluetoothEndpointError::UnsupportedScheme("tcp".to_owned())
        );
    }
}
