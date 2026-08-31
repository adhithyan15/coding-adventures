//! Bounded Modbus TCP request and response framing.

#![forbid(unsafe_code)]

use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const MODBUS_PROTOCOL_ID: u16 = 0;
pub const MAX_READ_REGISTERS: u16 = 125;
pub const MAX_ADU_BYTES: usize = 260;
pub const READ_DEVICE_IDENTIFICATION_FUNCTION: u8 = 0x2b;
pub const DEVICE_IDENTIFICATION_MEI_TYPE: u8 = 0x0e;
pub const BASIC_DEVICE_IDENTIFICATION_CODE: u8 = 0x01;
pub const MAX_BASIC_DEVICE_IDENTIFICATION_PAGES: usize = 3;
pub const MAX_DEVICE_IDENTIFICATION_VALUE_BYTES: usize = 128;
const MBAP_PREFIX_BYTES: usize = 6;
const MBAP_HEADER_BYTES: usize = 7;
const LAST_BASIC_DEVICE_IDENTIFICATION_OBJECT: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterTable {
    Holding,
    Input,
}

impl RegisterTable {
    pub const fn function_code(self) -> u8 {
        match self {
            Self::Holding => 0x03,
            Self::Input => 0x04,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Holding => "holding",
            Self::Input => "input",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadRegistersRequest {
    pub table: RegisterTable,
    pub starting_address: u16,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentificationObject {
    pub object_id: u8,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentificationPage {
    pub conformity_level: u8,
    pub more_follows: bool,
    pub next_object_id: u8,
    pub objects: Vec<DeviceIdentificationObject>,
}

impl ReadRegistersRequest {
    pub fn new(
        table: RegisterTable,
        starting_address: u16,
        quantity: u16,
    ) -> Result<Self, ModbusError> {
        if quantity == 0 || quantity > MAX_READ_REGISTERS {
            return Err(ModbusError::InvalidQuantity(quantity));
        }
        starting_address
            .checked_add(quantity - 1)
            .ok_or(ModbusError::AddressRangeOverflow {
                starting_address,
                quantity,
            })?;
        Ok(Self {
            table,
            starting_address,
            quantity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModbusError {
    InvalidQuantity(u16),
    AddressRangeOverflow {
        starting_address: u16,
        quantity: u16,
    },
    AduTooShort {
        actual: usize,
    },
    AduTooLarge {
        actual: usize,
        maximum: usize,
    },
    ProtocolId(u16),
    LengthMismatch {
        declared: usize,
        actual: usize,
    },
    TransactionId {
        expected: u16,
        actual: u16,
    },
    UnitId {
        expected: u8,
        actual: u8,
    },
    FunctionCode {
        expected: u8,
        actual: u8,
    },
    Exception {
        function: u8,
        code: u8,
    },
    ByteCount {
        expected: usize,
        actual: usize,
    },
    InvalidDeviceIdentificationObject(u8),
    MeiType(u8),
    ReadDeviceIdentificationCode(u8),
    ConformityLevel(u8),
    MoreFollows(u8),
    NextObjectId(u8),
    DeviceIdentificationObjectCount(u8),
    DeviceIdentificationObjectId {
        expected: u8,
        actual: u8,
    },
    DeviceIdentificationObjectValue {
        object_id: u8,
        reason: &'static str,
    },
    DeviceIdentificationTrailingBytes(usize),
}

impl fmt::Display for ModbusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity(quantity) => write!(
                formatter,
                "Modbus read quantity must be between 1 and {MAX_READ_REGISTERS}, got {quantity}"
            ),
            Self::AddressRangeOverflow {
                starting_address,
                quantity,
            } => write!(
                formatter,
                "Modbus address range {starting_address} + {quantity} registers exceeds u16"
            ),
            Self::AduTooShort { actual } => {
                write!(formatter, "Modbus TCP ADU is too short: {actual} bytes")
            }
            Self::AduTooLarge { actual, maximum } => write!(
                formatter,
                "Modbus TCP ADU is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::ProtocolId(actual) => {
                write!(formatter, "Modbus TCP protocol id must be 0, got {actual}")
            }
            Self::LengthMismatch { declared, actual } => write!(
                formatter,
                "Modbus TCP length declares {declared} bytes but ADU contains {actual}"
            ),
            Self::TransactionId { expected, actual } => write!(
                formatter,
                "Modbus transaction id mismatch: expected {expected}, got {actual}"
            ),
            Self::UnitId { expected, actual } => write!(
                formatter,
                "Modbus unit id mismatch: expected {expected}, got {actual}"
            ),
            Self::FunctionCode { expected, actual } => write!(
                formatter,
                "Modbus function mismatch: expected 0x{expected:02x}, got 0x{actual:02x}"
            ),
            Self::Exception { function, code } => write!(
                formatter,
                "Modbus function 0x{function:02x} returned exception 0x{code:02x}"
            ),
            Self::ByteCount { expected, actual } => write!(
                formatter,
                "Modbus register payload must contain {expected} bytes, got {actual}"
            ),
            Self::InvalidDeviceIdentificationObject(object_id) => write!(
                formatter,
                "Modbus basic device identification object must be between 0x00 and 0x02, got 0x{object_id:02x}"
            ),
            Self::MeiType(actual) => write!(
                formatter,
                "Modbus device identification MEI type must be 0x{DEVICE_IDENTIFICATION_MEI_TYPE:02x}, got 0x{actual:02x}"
            ),
            Self::ReadDeviceIdentificationCode(actual) => write!(
                formatter,
                "Modbus Read Device ID code must be 0x{BASIC_DEVICE_IDENTIFICATION_CODE:02x}, got 0x{actual:02x}"
            ),
            Self::ConformityLevel(actual) => write!(
                formatter,
                "Modbus device identification conformity level is invalid: 0x{actual:02x}"
            ),
            Self::MoreFollows(actual) => write!(
                formatter,
                "Modbus device identification More Follows must be 0x00 or 0xff, got 0x{actual:02x}"
            ),
            Self::NextObjectId(actual) => write!(
                formatter,
                "Modbus device identification Next Object ID is invalid: 0x{actual:02x}"
            ),
            Self::DeviceIdentificationObjectCount(actual) => write!(
                formatter,
                "Modbus basic device identification page must contain between 1 and 3 objects, got {actual}"
            ),
            Self::DeviceIdentificationObjectId { expected, actual } => write!(
                formatter,
                "Modbus device identification object mismatch: expected 0x{expected:02x}, got 0x{actual:02x}"
            ),
            Self::DeviceIdentificationObjectValue { object_id, reason } => write!(
                formatter,
                "Modbus device identification object 0x{object_id:02x} has invalid value: {reason}"
            ),
            Self::DeviceIdentificationTrailingBytes(actual) => write!(
                formatter,
                "Modbus device identification response contains {actual} trailing bytes"
            ),
        }
    }
}

impl std::error::Error for ModbusError {}

pub fn encode_read_request(
    transaction_id: u16,
    unit_id: u8,
    request: ReadRegistersRequest,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&transaction_id.to_be_bytes());
    bytes.extend_from_slice(&MODBUS_PROTOCOL_ID.to_be_bytes());
    bytes.extend_from_slice(&6u16.to_be_bytes());
    bytes.push(unit_id);
    bytes.push(request.table.function_code());
    bytes.extend_from_slice(&request.starting_address.to_be_bytes());
    bytes.extend_from_slice(&request.quantity.to_be_bytes());
    bytes
}

pub fn encode_read_device_identification_request(
    transaction_id: u16,
    unit_id: u8,
    starting_object_id: u8,
) -> Result<Vec<u8>, ModbusError> {
    if starting_object_id > LAST_BASIC_DEVICE_IDENTIFICATION_OBJECT {
        return Err(ModbusError::InvalidDeviceIdentificationObject(
            starting_object_id,
        ));
    }
    let mut bytes = Vec::with_capacity(11);
    bytes.extend_from_slice(&transaction_id.to_be_bytes());
    bytes.extend_from_slice(&MODBUS_PROTOCOL_ID.to_be_bytes());
    bytes.extend_from_slice(&5u16.to_be_bytes());
    bytes.push(unit_id);
    bytes.push(READ_DEVICE_IDENTIFICATION_FUNCTION);
    bytes.push(DEVICE_IDENTIFICATION_MEI_TYPE);
    bytes.push(BASIC_DEVICE_IDENTIFICATION_CODE);
    bytes.push(starting_object_id);
    Ok(bytes)
}

pub fn decode_read_response(
    bytes: &[u8],
    expected_transaction_id: u16,
    expected_unit_id: u8,
    request: ReadRegistersRequest,
) -> Result<Vec<u16>, ModbusError> {
    if bytes.len() < MBAP_HEADER_BYTES + 2 {
        return Err(ModbusError::AduTooShort {
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_ADU_BYTES {
        return Err(ModbusError::AduTooLarge {
            actual: bytes.len(),
            maximum: MAX_ADU_BYTES,
        });
    }

    let transaction_id = u16::from_be_bytes([bytes[0], bytes[1]]);
    if transaction_id != expected_transaction_id {
        return Err(ModbusError::TransactionId {
            expected: expected_transaction_id,
            actual: transaction_id,
        });
    }
    let protocol_id = u16::from_be_bytes([bytes[2], bytes[3]]);
    if protocol_id != MODBUS_PROTOCOL_ID {
        return Err(ModbusError::ProtocolId(protocol_id));
    }
    let declared = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let actual = bytes.len() - MBAP_PREFIX_BYTES;
    if declared != actual {
        return Err(ModbusError::LengthMismatch { declared, actual });
    }
    let unit_id = bytes[6];
    if unit_id != expected_unit_id {
        return Err(ModbusError::UnitId {
            expected: expected_unit_id,
            actual: unit_id,
        });
    }

    let expected_function = request.table.function_code();
    let function = bytes[7];
    if function == expected_function | 0x80 {
        if bytes.len() != MBAP_HEADER_BYTES + 2 {
            return Err(ModbusError::LengthMismatch {
                declared: 3,
                actual: bytes.len() - MBAP_PREFIX_BYTES,
            });
        }
        return Err(ModbusError::Exception {
            function: expected_function,
            code: bytes[8],
        });
    }
    if function != expected_function {
        return Err(ModbusError::FunctionCode {
            expected: expected_function,
            actual: function,
        });
    }

    let expected_bytes = usize::from(request.quantity) * 2;
    let byte_count = usize::from(bytes[8]);
    if byte_count != expected_bytes || bytes.len() != MBAP_HEADER_BYTES + 2 + byte_count {
        return Err(ModbusError::ByteCount {
            expected: expected_bytes,
            actual: byte_count.min(bytes.len().saturating_sub(MBAP_HEADER_BYTES + 2)),
        });
    }
    Ok(bytes[9..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|register| u16::from_be_bytes([register[0], register[1]]))
        .collect())
}

pub fn decode_read_device_identification_response(
    bytes: &[u8],
    expected_transaction_id: u16,
    expected_unit_id: u8,
    expected_starting_object_id: u8,
) -> Result<DeviceIdentificationPage, ModbusError> {
    if expected_starting_object_id > LAST_BASIC_DEVICE_IDENTIFICATION_OBJECT {
        return Err(ModbusError::InvalidDeviceIdentificationObject(
            expected_starting_object_id,
        ));
    }
    if bytes.len() < MBAP_HEADER_BYTES + 2 {
        return Err(ModbusError::AduTooShort {
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_ADU_BYTES {
        return Err(ModbusError::AduTooLarge {
            actual: bytes.len(),
            maximum: MAX_ADU_BYTES,
        });
    }

    let transaction_id = u16::from_be_bytes([bytes[0], bytes[1]]);
    if transaction_id != expected_transaction_id {
        return Err(ModbusError::TransactionId {
            expected: expected_transaction_id,
            actual: transaction_id,
        });
    }
    let protocol_id = u16::from_be_bytes([bytes[2], bytes[3]]);
    if protocol_id != MODBUS_PROTOCOL_ID {
        return Err(ModbusError::ProtocolId(protocol_id));
    }
    let declared = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let actual = bytes.len() - MBAP_PREFIX_BYTES;
    if declared != actual {
        return Err(ModbusError::LengthMismatch { declared, actual });
    }
    let unit_id = bytes[6];
    if unit_id != expected_unit_id {
        return Err(ModbusError::UnitId {
            expected: expected_unit_id,
            actual: unit_id,
        });
    }

    let function = bytes[7];
    if function == READ_DEVICE_IDENTIFICATION_FUNCTION | 0x80 {
        if bytes.len() != MBAP_HEADER_BYTES + 2 {
            return Err(ModbusError::LengthMismatch {
                declared: 3,
                actual: bytes.len() - MBAP_PREFIX_BYTES,
            });
        }
        return Err(ModbusError::Exception {
            function: READ_DEVICE_IDENTIFICATION_FUNCTION,
            code: bytes[8],
        });
    }
    if function != READ_DEVICE_IDENTIFICATION_FUNCTION {
        return Err(ModbusError::FunctionCode {
            expected: READ_DEVICE_IDENTIFICATION_FUNCTION,
            actual: function,
        });
    }
    if bytes.len() < MBAP_HEADER_BYTES + 7 {
        return Err(ModbusError::AduTooShort {
            actual: bytes.len(),
        });
    }
    if bytes[8] != DEVICE_IDENTIFICATION_MEI_TYPE {
        return Err(ModbusError::MeiType(bytes[8]));
    }
    if bytes[9] != BASIC_DEVICE_IDENTIFICATION_CODE {
        return Err(ModbusError::ReadDeviceIdentificationCode(bytes[9]));
    }
    let conformity_level = bytes[10];
    if !matches!(conformity_level, 0x01..=0x03 | 0x81..=0x83) {
        return Err(ModbusError::ConformityLevel(conformity_level));
    }
    let more_follows = match bytes[11] {
        0x00 => false,
        0xff => true,
        actual => return Err(ModbusError::MoreFollows(actual)),
    };
    let next_object_id = bytes[12];
    let object_count = bytes[13];
    if object_count == 0 || object_count > 3 {
        return Err(ModbusError::DeviceIdentificationObjectCount(object_count));
    }

    let mut cursor = MBAP_HEADER_BYTES + 7;
    let mut expected_object_id = expected_starting_object_id;
    let mut objects = Vec::with_capacity(usize::from(object_count));
    for _ in 0..object_count {
        if cursor + 2 > bytes.len() {
            return Err(ModbusError::AduTooShort {
                actual: bytes.len(),
            });
        }
        let object_id = bytes[cursor];
        let value_length = usize::from(bytes[cursor + 1]);
        cursor += 2;
        if object_id != expected_object_id {
            return Err(ModbusError::DeviceIdentificationObjectId {
                expected: expected_object_id,
                actual: object_id,
            });
        }
        if value_length == 0 || value_length > MAX_DEVICE_IDENTIFICATION_VALUE_BYTES {
            return Err(ModbusError::DeviceIdentificationObjectValue {
                object_id,
                reason: "length must be between 1 and 128 bytes",
            });
        }
        let Some(value_bytes) = bytes.get(cursor..cursor + value_length) else {
            return Err(ModbusError::AduTooShort {
                actual: bytes.len(),
            });
        };
        if !value_bytes.iter().all(|byte| matches!(byte, 0x20..=0x7e)) {
            return Err(ModbusError::DeviceIdentificationObjectValue {
                object_id,
                reason: "value must contain printable ASCII bytes",
            });
        }
        objects.push(DeviceIdentificationObject {
            object_id,
            value: String::from_utf8(value_bytes.to_vec()).expect("printable ASCII is UTF-8"),
        });
        cursor += value_length;
        expected_object_id = expected_object_id
            .checked_add(1)
            .ok_or(ModbusError::InvalidDeviceIdentificationObject(object_id))?;
    }
    if cursor != bytes.len() {
        return Err(ModbusError::DeviceIdentificationTrailingBytes(
            bytes.len() - cursor,
        ));
    }
    let expected_next = if more_follows {
        if expected_object_id > LAST_BASIC_DEVICE_IDENTIFICATION_OBJECT {
            return Err(ModbusError::NextObjectId(next_object_id));
        }
        expected_object_id
    } else {
        0
    };
    if next_object_id != expected_next {
        return Err(ModbusError::NextObjectId(next_object_id));
    }

    Ok(DeviceIdentificationPage {
        conformity_level,
        more_follows,
        next_object_id,
        objects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ReadRegistersRequest {
        ReadRegistersRequest::new(RegisterTable::Holding, 0x006b, 3).unwrap()
    }

    #[test]
    fn encodes_read_holding_register_request() {
        assert_eq!(
            encode_read_request(1, 0x11, request()),
            [0, 1, 0, 0, 0, 6, 0x11, 0x03, 0, 0x6b, 0, 3]
        );
    }

    #[test]
    fn decodes_matching_register_response() {
        let bytes = [
            0, 1, 0, 0, 0, 9, 0x11, 0x03, 6, 0x02, 0x2b, 0x00, 0x00, 0x00, 0x64,
        ];
        assert_eq!(
            decode_read_response(&bytes, 1, 0x11, request()).unwrap(),
            [0x022b, 0, 100]
        );
    }

    #[test]
    fn rejects_invalid_ranges_before_encoding() {
        assert!(matches!(
            ReadRegistersRequest::new(RegisterTable::Input, 0, 0),
            Err(ModbusError::InvalidQuantity(0))
        ));
        assert!(matches!(
            ReadRegistersRequest::new(RegisterTable::Input, u16::MAX, 2),
            Err(ModbusError::AddressRangeOverflow { .. })
        ));
    }

    #[test]
    fn reports_exception_response() {
        let bytes = [0, 7, 0, 0, 0, 3, 1, 0x84, 2];
        let request = ReadRegistersRequest::new(RegisterTable::Input, 0, 1).unwrap();
        assert_eq!(
            decode_read_response(&bytes, 7, 1, request),
            Err(ModbusError::Exception {
                function: 0x04,
                code: 2
            })
        );

        let malformed = [0, 7, 0, 0, 0, 4, 1, 0x84, 2, 0];
        assert_eq!(
            decode_read_response(&malformed, 7, 1, request),
            Err(ModbusError::LengthMismatch {
                declared: 3,
                actual: 4,
            })
        );
    }

    #[test]
    fn rejects_cross_request_and_malformed_responses() {
        let valid = [0, 1, 0, 0, 0, 5, 0x11, 0x03, 2, 0, 9];
        let one = ReadRegistersRequest::new(RegisterTable::Holding, 0, 1).unwrap();
        assert!(matches!(
            decode_read_response(&valid, 2, 0x11, one),
            Err(ModbusError::TransactionId { .. })
        ));
        let mut bad_protocol = valid;
        bad_protocol[3] = 1;
        assert!(matches!(
            decode_read_response(&bad_protocol, 1, 0x11, one),
            Err(ModbusError::ProtocolId(1))
        ));
        let mut bad_length = valid;
        bad_length[5] = 4;
        assert!(matches!(
            decode_read_response(&bad_length, 1, 0x11, one),
            Err(ModbusError::LengthMismatch { .. })
        ));
        let mut bad_count = valid;
        bad_count[8] = 4;
        assert!(matches!(
            decode_read_response(&bad_count, 1, 0x11, one),
            Err(ModbusError::ByteCount { .. })
        ));
    }

    #[test]
    fn encodes_basic_device_identification_request() {
        assert_eq!(
            encode_read_device_identification_request(9, 0x11, 0).unwrap(),
            [0, 9, 0, 0, 0, 5, 0x11, 0x2b, 0x0e, 0x01, 0]
        );
        assert_eq!(
            encode_read_device_identification_request(9, 0x11, 3),
            Err(ModbusError::InvalidDeviceIdentificationObject(3))
        );
    }

    #[test]
    fn decodes_correlated_device_identification_pages() {
        let first = [
            0, 9, 0, 0, 0, 20, 0x11, 0x2b, 0x0e, 0x01, 0x81, 0xff, 2, 2, 0, 4, b'A', b'c', b'm',
            b'e', 1, 4, b'P', b'M', b'-', b'1',
        ];
        assert_eq!(
            decode_read_device_identification_response(&first, 9, 0x11, 0).unwrap(),
            DeviceIdentificationPage {
                conformity_level: 0x81,
                more_follows: true,
                next_object_id: 2,
                objects: vec![
                    DeviceIdentificationObject {
                        object_id: 0,
                        value: "Acme".to_string(),
                    },
                    DeviceIdentificationObject {
                        object_id: 1,
                        value: "PM-1".to_string(),
                    },
                ],
            }
        );

        let final_page = [
            0, 10, 0, 0, 0, 15, 0x11, 0x2b, 0x0e, 0x01, 0x01, 0, 0, 1, 2, 5, b'1', b'.', b'2',
            b'.', b'3',
        ];
        assert_eq!(
            decode_read_device_identification_response(&final_page, 10, 0x11, 2)
                .unwrap()
                .objects,
            [DeviceIdentificationObject {
                object_id: 2,
                value: "1.2.3".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_uncorrelated_or_malformed_device_identification() {
        let valid = [
            0, 9, 0, 0, 0, 14, 0x11, 0x2b, 0x0e, 0x01, 0x01, 0, 0, 1, 0, 4, b'A', b'c', b'm', b'e',
        ];
        let mut bad_mei = valid;
        bad_mei[8] = 0x0d;
        assert_eq!(
            decode_read_device_identification_response(&bad_mei, 9, 0x11, 0),
            Err(ModbusError::MeiType(0x0d))
        );

        let mut bad_next = valid;
        bad_next[11] = 0xff;
        bad_next[12] = 2;
        assert_eq!(
            decode_read_device_identification_response(&bad_next, 9, 0x11, 0),
            Err(ModbusError::NextObjectId(2))
        );

        let mut bad_object = valid;
        bad_object[14] = 1;
        assert!(matches!(
            decode_read_device_identification_response(&bad_object, 9, 0x11, 0),
            Err(ModbusError::DeviceIdentificationObjectId { .. })
        ));

        let mut bad_ascii = valid;
        bad_ascii[16] = b'\n';
        assert!(matches!(
            decode_read_device_identification_response(&bad_ascii, 9, 0x11, 0),
            Err(ModbusError::DeviceIdentificationObjectValue { .. })
        ));

        let exception = [0, 9, 0, 0, 0, 3, 0x11, 0xab, 1];
        assert_eq!(
            decode_read_device_identification_response(&exception, 9, 0x11, 0),
            Err(ModbusError::Exception {
                function: 0x2b,
                code: 1,
            })
        );
    }
}
