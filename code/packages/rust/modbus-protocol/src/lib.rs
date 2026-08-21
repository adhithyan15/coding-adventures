//! Bounded Modbus TCP request and response framing.

#![forbid(unsafe_code)]

use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const MODBUS_PROTOCOL_ID: u16 = 0;
pub const MAX_READ_REGISTERS: u16 = 125;
pub const MAX_ADU_BYTES: usize = 260;
const MBAP_PREFIX_BYTES: usize = 6;
const MBAP_HEADER_BYTES: usize = 7;

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
}
