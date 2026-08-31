//! Strict DSMR 5.0.2 P1 telegram framing and telemetry decoding.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const DSMR_OUTPUT_VERSION: &str = "50";
pub const DEFAULT_MAX_TELEGRAM_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct DsmrP1Telegram {
    pub header: String,
    pub version: String,
    pub timestamp: String,
    pub equipment_id: String,
    pub electricity_import_tariff_1_kwh: f64,
    pub electricity_import_tariff_2_kwh: f64,
    pub electricity_export_tariff_1_kwh: f64,
    pub electricity_export_tariff_2_kwh: f64,
    pub active_tariff: String,
    pub electricity_import_kw: f64,
    pub electricity_export_kw: f64,
    pub phase_voltage_v: [Option<f64>; 3],
    pub phase_current_a: [Option<f64>; 3],
    pub phase_import_kw: [Option<f64>; 3],
    pub phase_export_kw: [Option<f64>; 3],
    pub gas: Option<DsmrGasReading>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DsmrGasReading {
    pub timestamp: String,
    pub cubic_metres: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DsmrP1Error {
    Empty,
    TooLarge { limit: usize },
    NonAscii,
    InvalidLineEnding,
    InvalidHeader,
    InvalidTerminator,
    InvalidChecksumText,
    ChecksumMismatch { expected: u16, actual: u16 },
    InvalidLine(String),
    DuplicateField(&'static str),
    MissingField(&'static str),
    InvalidField { field: &'static str, value: String },
    WrongUnit { field: &'static str, unit: String },
}

impl fmt::Display for DsmrP1Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("DSMR P1 telegram is empty"),
            Self::TooLarge { limit } => write!(formatter, "DSMR P1 telegram exceeds {limit} bytes"),
            Self::NonAscii => formatter.write_str("DSMR P1 telegram must contain ASCII bytes only"),
            Self::InvalidLineEnding => {
                formatter.write_str("DSMR P1 telegram must use CRLF line endings")
            }
            Self::InvalidHeader => formatter.write_str("DSMR P1 telegram has an invalid header"),
            Self::InvalidTerminator => {
                formatter.write_str("DSMR P1 telegram has an invalid terminator")
            }
            Self::InvalidChecksumText => {
                formatter.write_str("DSMR P1 checksum must be four hexadecimal characters")
            }
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "DSMR P1 checksum mismatch: expected {expected:04X}, got {actual:04X}"
            ),
            Self::InvalidLine(line) => write!(formatter, "invalid DSMR P1 data line `{line}`"),
            Self::DuplicateField(field) => write!(formatter, "duplicate DSMR P1 field `{field}`"),
            Self::MissingField(field) => write!(formatter, "DSMR P1 telegram is missing `{field}`"),
            Self::InvalidField { field, value } => {
                write!(formatter, "invalid DSMR P1 {field} value `{value}`")
            }
            Self::WrongUnit { field, unit } => {
                write!(formatter, "DSMR P1 {field} has unexpected unit `{unit}`")
            }
        }
    }
}

impl std::error::Error for DsmrP1Error {}

pub fn parse_telegram(bytes: &[u8]) -> Result<DsmrP1Telegram, DsmrP1Error> {
    parse_telegram_with_limit(bytes, DEFAULT_MAX_TELEGRAM_BYTES)
}

pub fn parse_telegram_with_limit(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<DsmrP1Telegram, DsmrP1Error> {
    if bytes.is_empty() {
        return Err(DsmrP1Error::Empty);
    }
    if bytes.len() > max_bytes {
        return Err(DsmrP1Error::TooLarge { limit: max_bytes });
    }
    if !bytes.is_ascii()
        || bytes
            .iter()
            .any(|byte| !matches!(*byte, b'\r' | b'\n' | 0x20..=0x7e))
    {
        return Err(DsmrP1Error::NonAscii);
    }
    validate_crlf(bytes)?;

    let bang = bytes
        .iter()
        .rposition(|byte| *byte == b'!')
        .ok_or(DsmrP1Error::InvalidTerminator)?;
    if bang + 7 != bytes.len() || &bytes[bang + 5..] != b"\r\n" {
        return Err(DsmrP1Error::InvalidTerminator);
    }
    let checksum_text = std::str::from_utf8(&bytes[bang + 1..bang + 5])
        .map_err(|_| DsmrP1Error::InvalidChecksumText)?;
    if checksum_text.len() != 4 || !checksum_text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DsmrP1Error::InvalidChecksumText);
    }
    let actual =
        u16::from_str_radix(checksum_text, 16).map_err(|_| DsmrP1Error::InvalidChecksumText)?;
    let expected = crc16(&bytes[..=bang]);
    if expected != actual {
        return Err(DsmrP1Error::ChecksumMismatch { expected, actual });
    }

    let content = std::str::from_utf8(&bytes[..bang]).map_err(|_| DsmrP1Error::NonAscii)?;
    let mut lines = content.split("\r\n");
    let header = lines.next().ok_or(DsmrP1Error::InvalidHeader)?;
    if !header.starts_with('/')
        || header.len() < 5
        || header.len() > 96
        || header.bytes().any(|byte| !(0x20..=0x7e).contains(&byte))
    {
        return Err(DsmrP1Error::InvalidHeader);
    }
    if lines.next() != Some("") {
        return Err(DsmrP1Error::InvalidHeader);
    }

    let mut builder = TelegramBuilder::new(header[1..].to_string());
    let mut data_lines = lines.collect::<Vec<_>>();
    if data_lines.last() == Some(&"") {
        data_lines.pop();
    }
    for line in data_lines {
        if line.is_empty() {
            return Err(DsmrP1Error::InvalidLine(line.to_string()));
        }
        let open = line
            .find('(')
            .ok_or_else(|| DsmrP1Error::InvalidLine(line.to_string()))?;
        if !line.ends_with(')') || open == 0 || !line[..open].bytes().all(is_obis_byte) {
            return Err(DsmrP1Error::InvalidLine(line.to_string()));
        }
        builder.parse_line(&line[..open], &line[open..])?;
    }
    builder.finish()
}

pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0_u16;
    for byte in bytes {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

fn validate_crlf(bytes: &[u8]) -> Result<(), DsmrP1Error> {
    for (index, byte) in bytes.iter().enumerate() {
        if (*byte == b'\n' && (index == 0 || bytes[index - 1] != b'\r'))
            || (*byte == b'\r' && bytes.get(index + 1) != Some(&b'\n'))
        {
            return Err(DsmrP1Error::InvalidLineEnding);
        }
    }
    Ok(())
}

fn is_obis_byte(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'-' | b':' | b'.')
}

#[derive(Default)]
struct TelegramBuilder {
    header: String,
    seen: BTreeSet<&'static str>,
    version: Option<String>,
    timestamp: Option<String>,
    equipment_id: Option<String>,
    import_1: Option<f64>,
    import_2: Option<f64>,
    export_1: Option<f64>,
    export_2: Option<f64>,
    active_tariff: Option<String>,
    import_kw: Option<f64>,
    export_kw: Option<f64>,
    voltage: [Option<f64>; 3],
    current: [Option<f64>; 3],
    phase_import: [Option<f64>; 3],
    phase_export: [Option<f64>; 3],
    gas: Option<DsmrGasReading>,
}

impl TelegramBuilder {
    fn new(header: String) -> Self {
        Self {
            header,
            ..Self::default()
        }
    }

    fn parse_line(&mut self, obis: &str, groups: &str) -> Result<(), DsmrP1Error> {
        let _ = parse_groups(groups)?;
        match obis {
            "1-3:0.2.8" => {
                self.version = Some(self.unique_text("version", groups, validate_version)?)
            }
            "0-0:1.0.0" => {
                self.timestamp = Some(self.unique_text("timestamp", groups, validate_timestamp)?)
            }
            "0-0:96.1.1" => {
                self.equipment_id =
                    Some(self.unique_text("equipment_id", groups, validate_equipment_id)?)
            }
            "1-0:1.8.1" => {
                self.import_1 = Some(self.unique_number("import_tariff_1", groups, "kWh")?)
            }
            "1-0:1.8.2" => {
                self.import_2 = Some(self.unique_number("import_tariff_2", groups, "kWh")?)
            }
            "1-0:2.8.1" => {
                self.export_1 = Some(self.unique_number("export_tariff_1", groups, "kWh")?)
            }
            "1-0:2.8.2" => {
                self.export_2 = Some(self.unique_number("export_tariff_2", groups, "kWh")?)
            }
            "0-0:96.14.0" => {
                self.active_tariff =
                    Some(self.unique_text("active_tariff", groups, validate_tariff)?)
            }
            "1-0:1.7.0" => {
                self.import_kw = Some(self.unique_number("import_power", groups, "kW")?)
            }
            "1-0:2.7.0" => {
                self.export_kw = Some(self.unique_number("export_power", groups, "kW")?)
            }
            "1-0:32.7.0" => {
                self.voltage[0] = Some(self.unique_number("voltage_l1", groups, "V")?)
            }
            "1-0:52.7.0" => {
                self.voltage[1] = Some(self.unique_number("voltage_l2", groups, "V")?)
            }
            "1-0:72.7.0" => {
                self.voltage[2] = Some(self.unique_number("voltage_l3", groups, "V")?)
            }
            "1-0:31.7.0" => {
                self.current[0] = Some(self.unique_number("current_l1", groups, "A")?)
            }
            "1-0:51.7.0" => {
                self.current[1] = Some(self.unique_number("current_l2", groups, "A")?)
            }
            "1-0:71.7.0" => {
                self.current[2] = Some(self.unique_number("current_l3", groups, "A")?)
            }
            "1-0:21.7.0" => {
                self.phase_import[0] = Some(self.unique_number("import_power_l1", groups, "kW")?)
            }
            "1-0:41.7.0" => {
                self.phase_import[1] = Some(self.unique_number("import_power_l2", groups, "kW")?)
            }
            "1-0:61.7.0" => {
                self.phase_import[2] = Some(self.unique_number("import_power_l3", groups, "kW")?)
            }
            "1-0:22.7.0" => {
                self.phase_export[0] = Some(self.unique_number("export_power_l1", groups, "kW")?)
            }
            "1-0:42.7.0" => {
                self.phase_export[1] = Some(self.unique_number("export_power_l2", groups, "kW")?)
            }
            "1-0:62.7.0" => {
                self.phase_export[2] = Some(self.unique_number("export_power_l3", groups, "kW")?)
            }
            value if is_gas_reading(value) => self.gas = Some(self.unique_gas(groups)?),
            _ => {}
        }
        Ok(())
    }

    fn unique_text(
        &mut self,
        field: &'static str,
        groups: &str,
        validate: fn(&str) -> bool,
    ) -> Result<String, DsmrP1Error> {
        self.mark(field)?;
        let value = single_group(groups)?;
        if !validate(value) {
            return Err(DsmrP1Error::InvalidField {
                field,
                value: value.to_string(),
            });
        }
        Ok(value.to_string())
    }

    fn unique_number(
        &mut self,
        field: &'static str,
        groups: &str,
        expected_unit: &'static str,
    ) -> Result<f64, DsmrP1Error> {
        self.mark(field)?;
        let value = single_group(groups)?;
        let (number, unit) = value
            .split_once('*')
            .ok_or_else(|| DsmrP1Error::InvalidField {
                field,
                value: value.to_string(),
            })?;
        if unit != expected_unit {
            return Err(DsmrP1Error::WrongUnit {
                field,
                unit: unit.to_string(),
            });
        }
        parse_non_negative(field, number)
    }

    fn unique_gas(&mut self, groups: &str) -> Result<DsmrGasReading, DsmrP1Error> {
        self.mark("gas")?;
        let values = parse_groups(groups)?;
        if values.len() != 2 || !validate_timestamp(values[0]) {
            return Err(DsmrP1Error::InvalidField {
                field: "gas",
                value: groups.to_string(),
            });
        }
        let (number, unit) =
            values[1]
                .split_once('*')
                .ok_or_else(|| DsmrP1Error::InvalidField {
                    field: "gas",
                    value: groups.to_string(),
                })?;
        if unit != "m3" {
            return Err(DsmrP1Error::WrongUnit {
                field: "gas",
                unit: unit.to_string(),
            });
        }
        Ok(DsmrGasReading {
            timestamp: values[0].to_string(),
            cubic_metres: parse_non_negative("gas", number)?,
        })
    }

    fn mark(&mut self, field: &'static str) -> Result<(), DsmrP1Error> {
        if !self.seen.insert(field) {
            return Err(DsmrP1Error::DuplicateField(field));
        }
        Ok(())
    }

    fn finish(self) -> Result<DsmrP1Telegram, DsmrP1Error> {
        Ok(DsmrP1Telegram {
            header: self.header,
            version: required(self.version, "version")?,
            timestamp: required(self.timestamp, "timestamp")?,
            equipment_id: required(self.equipment_id, "equipment_id")?,
            electricity_import_tariff_1_kwh: required(self.import_1, "import_tariff_1")?,
            electricity_import_tariff_2_kwh: required(self.import_2, "import_tariff_2")?,
            electricity_export_tariff_1_kwh: required(self.export_1, "export_tariff_1")?,
            electricity_export_tariff_2_kwh: required(self.export_2, "export_tariff_2")?,
            active_tariff: required(self.active_tariff, "active_tariff")?,
            electricity_import_kw: required(self.import_kw, "import_power")?,
            electricity_export_kw: required(self.export_kw, "export_power")?,
            phase_voltage_v: self.voltage,
            phase_current_a: self.current,
            phase_import_kw: self.phase_import,
            phase_export_kw: self.phase_export,
            gas: self.gas,
        })
    }
}

fn required<T>(value: Option<T>, field: &'static str) -> Result<T, DsmrP1Error> {
    value.ok_or(DsmrP1Error::MissingField(field))
}

fn single_group(groups: &str) -> Result<&str, DsmrP1Error> {
    let values = parse_groups(groups)?;
    if values.len() != 1 {
        return Err(DsmrP1Error::InvalidLine(groups.to_string()));
    }
    Ok(values[0])
}

fn parse_groups(groups: &str) -> Result<Vec<&str>, DsmrP1Error> {
    let mut values = Vec::new();
    let mut remaining = groups;
    while let Some(after_open) = remaining.strip_prefix('(') {
        let close = after_open
            .find(')')
            .ok_or_else(|| DsmrP1Error::InvalidLine(groups.to_string()))?;
        let value = &after_open[..close];
        if value.is_empty() {
            return Err(DsmrP1Error::InvalidLine(groups.to_string()));
        }
        values.push(value);
        remaining = &after_open[close + 1..];
    }
    if !remaining.is_empty() || values.is_empty() {
        return Err(DsmrP1Error::InvalidLine(groups.to_string()));
    }
    Ok(values)
}

fn validate_version(value: &str) -> bool {
    value == DSMR_OUTPUT_VERSION
}

fn validate_timestamp(value: &str) -> bool {
    value.len() == 13
        && value[..12].bytes().all(|byte| byte.is_ascii_digit())
        && matches!(value.as_bytes()[12], b'S' | b'W')
}

fn validate_equipment_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.len().is_multiple_of(2)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_tariff(value: &str) -> bool {
    value.len() == 4 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn parse_non_negative(field: &'static str, value: &str) -> Result<f64, DsmrP1Error> {
    let parsed = value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| DsmrP1Error::InvalidField {
            field,
            value: value.to_string(),
        })?;
    Ok(parsed)
}

fn is_gas_reading(obis: &str) -> bool {
    let Some(channel) = obis
        .strip_prefix("0-")
        .and_then(|value| value.strip_suffix(":24.2.1"))
    else {
        return false;
    };
    channel.len() == 1 && channel.as_bytes()[0].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(body: &str) -> Vec<u8> {
        let mut bytes = body.as_bytes().to_vec();
        bytes.push(b'!');
        let checksum = crc16(&bytes);
        bytes.extend_from_slice(format!("{checksum:04X}\r\n").as_bytes());
        bytes
    }

    fn body() -> &'static str {
        "/ISK5\\2MT382-1000\r\n\r\n1-3:0.2.8(50)\r\n0-0:1.0.0(101209113020W)\r\n0-0:96.1.1(4B384547303034303436333935353037)\r\n1-0:1.8.1(123456.789*kWh)\r\n1-0:1.8.2(123456.790*kWh)\r\n1-0:2.8.1(000001.001*kWh)\r\n1-0:2.8.2(000002.002*kWh)\r\n0-0:96.14.0(0002)\r\n1-0:1.7.0(01.193*kW)\r\n1-0:2.7.0(00.000*kW)\r\n0-0:96.7.21(00004)\r\n1-0:32.7.0(220.1*V)\r\n1-0:31.7.0(001*A)\r\n0-1:24.2.1(101209112500W)(12785.123*m3)\r\n"
    }

    #[test]
    fn parses_fixed_dsmr_telemetry_and_ignores_other_standard_fields() {
        let telegram = parse_telegram(&frame(body())).unwrap();
        assert_eq!(telegram.version, "50");
        assert_eq!(telegram.electricity_import_tariff_1_kwh, 123456.789);
        assert_eq!(telegram.phase_voltage_v, [Some(220.1), None, None]);
        assert_eq!(telegram.phase_current_a, [Some(1.0), None, None]);
        assert_eq!(telegram.gas.unwrap().cubic_metres, 12785.123);
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let mut telegram = frame(body());
        telegram[12] ^= 1;
        assert!(matches!(
            parse_telegram(&telegram),
            Err(DsmrP1Error::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_allowlisted_field() {
        let duplicated = body().replace(
            "1-0:1.7.0(01.193*kW)\r\n",
            "1-0:1.7.0(01.193*kW)\r\n1-0:1.7.0(01.194*kW)\r\n",
        );
        assert_eq!(
            parse_telegram(&frame(&duplicated)),
            Err(DsmrP1Error::DuplicateField("import_power"))
        );
    }

    #[test]
    fn rejects_wrong_units_and_unsupported_version() {
        let wrong_unit = body().replace("01.193*kW", "01.193*W");
        assert!(matches!(
            parse_telegram(&frame(&wrong_unit)),
            Err(DsmrP1Error::WrongUnit {
                field: "import_power",
                ..
            })
        ));
        let old_version = body().replace("0.2.8(50)", "0.2.8(42)");
        assert!(matches!(
            parse_telegram(&frame(&old_version)),
            Err(DsmrP1Error::InvalidField {
                field: "version",
                ..
            })
        ));
    }

    #[test]
    fn rejects_lf_only_and_oversized_telegrams() {
        let lf_only = frame(&body().replace("\r\n", "\n"));
        assert_eq!(
            parse_telegram(&lf_only),
            Err(DsmrP1Error::InvalidLineEnding)
        );
        assert_eq!(
            parse_telegram_with_limit(&frame(body()), 32),
            Err(DsmrP1Error::TooLarge { limit: 32 })
        );
    }

    #[test]
    fn rejects_control_bytes_and_malformed_unknown_groups() {
        let control = body().replace("00004", "00\t04");
        assert_eq!(parse_telegram(&frame(&control)), Err(DsmrP1Error::NonAscii));

        let malformed = body().replace("0-0:96.7.21(00004)", "0-0:96.7.21(00004)(broken");
        assert!(matches!(
            parse_telegram(&frame(&malformed)),
            Err(DsmrP1Error::InvalidLine(_))
        ));
    }

    #[test]
    fn matches_the_standard_crc_polynomial_example() {
        assert_eq!(crc16(b"123456789"), 0xbb3d);
    }
}
