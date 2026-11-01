//! Bounded allocation-free `application/x-www-form-urlencoded` serialization.
//!
//! This crate validates ordered borrowed UTF-8 name/value pairs under explicit
//! ceilings and serializes them into caller-owned storage without allocation.

#![forbid(unsafe_code)]

use core::fmt;

/// Largest number of fields admitted by this deliberately narrow encoder.
pub const MAX_FORM_URLENCODED_FIELDS: usize = 64;

/// Largest UTF-8 byte length admitted for one name or value.
pub const MAX_FORM_URLENCODED_COMPONENT_BYTES: usize = 16 * 1024;

/// Largest complete encoded form body admitted by this encoder.
pub const MAX_FORM_URLENCODED_OUTPUT_BYTES: usize = 64 * 1024;

/// One borrowed form field. Debug output redacts both components.
pub struct FormUrlEncodedField<'input> {
    name: &'input str,
    value: &'input str,
}

impl<'input> FormUrlEncodedField<'input> {
    #[must_use]
    pub const fn new(name: &'input str, value: &'input str) -> Self {
        Self { name, value }
    }

    #[must_use]
    pub const fn name(&self) -> &'input str {
        self.name
    }

    #[must_use]
    pub const fn value(&self) -> &'input str {
        self.value
    }
}

impl fmt::Debug for FormUrlEncodedField<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormUrlEncodedField")
            .field("name", &"[REDACTED]")
            .field("name_length", &self.name.len())
            .field("value", &"[REDACTED]")
            .field("value_length", &self.value.len())
            .finish()
    }
}

/// Caller-owned resource limits for one encoded form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormUrlEncodedLimits {
    maximum_fields: u8,
    maximum_component_bytes: u16,
    maximum_output_bytes: u32,
}

impl FormUrlEncodedLimits {
    /// Validate field-count, component, and output ceilings. Zero values are
    /// valid and admit only inputs whose corresponding size is zero.
    pub fn new(
        maximum_fields: usize,
        maximum_component_bytes: usize,
        maximum_output_bytes: usize,
    ) -> Result<Self, FormUrlEncodedError> {
        if maximum_fields > MAX_FORM_URLENCODED_FIELDS
            || maximum_component_bytes > MAX_FORM_URLENCODED_COMPONENT_BYTES
            || maximum_output_bytes > MAX_FORM_URLENCODED_OUTPUT_BYTES
        {
            return Err(FormUrlEncodedError::InvalidLimits);
        }
        Ok(Self {
            maximum_fields: maximum_fields as u8,
            maximum_component_bytes: maximum_component_bytes as u16,
            maximum_output_bytes: maximum_output_bytes as u32,
        })
    }

    #[must_use]
    pub const fn maximum_fields(self) -> usize {
        self.maximum_fields as usize
    }

    #[must_use]
    pub const fn maximum_component_bytes(self) -> usize {
        self.maximum_component_bytes as usize
    }

    #[must_use]
    pub const fn maximum_output_bytes(self) -> usize {
        self.maximum_output_bytes as usize
    }
}

/// Validated ordered form evidence.
pub struct FormUrlEncoded<'input> {
    fields: &'input [FormUrlEncodedField<'input>],
    encoded_length: u32,
}

impl<'input> FormUrlEncoded<'input> {
    #[must_use]
    pub const fn fields(&self) -> &'input [FormUrlEncodedField<'input>] {
        self.fields
    }

    #[must_use]
    pub const fn encoded_length(&self) -> usize {
        self.encoded_length as usize
    }
}

impl fmt::Debug for FormUrlEncoded<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormUrlEncoded")
            .field("fields", &"[REDACTED]")
            .field("field_count", &self.fields.len())
            .field("encoded_length", &self.encoded_length)
            .finish()
    }
}

/// Stable payload-free form serialization failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormUrlEncodedError {
    InvalidLimits,
    TooManyFields,
    NameTooLong,
    ValueTooLong,
    OutputTooLong,
    OutputTooSmall,
}

impl fmt::Display for FormUrlEncodedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid form-urlencoded limits",
            Self::TooManyFields => "form-urlencoded field count exceeds its limit",
            Self::NameTooLong => "form-urlencoded field name exceeds its byte limit",
            Self::ValueTooLong => "form-urlencoded field value exceeds its byte limit",
            Self::OutputTooLong => "form-urlencoded output exceeds its byte limit",
            Self::OutputTooSmall => "form-urlencoded output buffer is too small",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FormUrlEncodedError {}

/// Validate an ordered borrowed sequence and calculate its exact encoded size.
pub fn validate_form_urlencoded<'input>(
    fields: &'input [FormUrlEncodedField<'input>],
    limits: FormUrlEncodedLimits,
) -> Result<FormUrlEncoded<'input>, FormUrlEncodedError> {
    if fields.len() > limits.maximum_fields() {
        return Err(FormUrlEncodedError::TooManyFields);
    }

    let mut encoded_length = 0usize;
    for (index, field) in fields.iter().enumerate() {
        if field.name.len() > limits.maximum_component_bytes() {
            return Err(FormUrlEncodedError::NameTooLong);
        }
        if field.value.len() > limits.maximum_component_bytes() {
            return Err(FormUrlEncodedError::ValueTooLong);
        }
        if index != 0 {
            encoded_length = encoded_length
                .checked_add(1)
                .ok_or(FormUrlEncodedError::OutputTooLong)?;
        }
        let name_length = encoded_component_length(field.name.as_bytes())?;
        let value_length = encoded_component_length(field.value.as_bytes())?;
        encoded_length = encoded_length
            .checked_add(name_length)
            .and_then(|length| length.checked_add(1))
            .and_then(|length| length.checked_add(value_length))
            .ok_or(FormUrlEncodedError::OutputTooLong)?;
        if encoded_length > limits.maximum_output_bytes() {
            return Err(FormUrlEncodedError::OutputTooLong);
        }
    }

    Ok(FormUrlEncoded {
        fields,
        encoded_length: encoded_length as u32,
    })
}

/// Serialize validated form evidence into caller-owned storage.
///
/// The output remains unchanged when its capacity is insufficient.
pub fn encode_form_urlencoded<'output>(
    form: &FormUrlEncoded<'_>,
    output: &'output mut [u8],
) -> Result<&'output mut [u8], FormUrlEncodedError> {
    let encoded_length = form.encoded_length();
    if output.len() < encoded_length {
        return Err(FormUrlEncodedError::OutputTooSmall);
    }

    let mut offset = 0usize;
    for (index, field) in form.fields.iter().enumerate() {
        if index != 0 {
            output[offset] = b'&';
            offset += 1;
        }
        encode_component(field.name.as_bytes(), output, &mut offset);
        output[offset] = b'=';
        offset += 1;
        encode_component(field.value.as_bytes(), output, &mut offset);
    }
    debug_assert_eq!(offset, encoded_length);
    Ok(&mut output[..encoded_length])
}

fn encoded_component_length(bytes: &[u8]) -> Result<usize, FormUrlEncodedError> {
    let mut length = 0usize;
    for byte in bytes {
        let width = if is_unescaped(*byte) || *byte == b' ' {
            1
        } else {
            3
        };
        length = length
            .checked_add(width)
            .ok_or(FormUrlEncodedError::OutputTooLong)?;
    }
    Ok(length)
}

fn encode_component(bytes: &[u8], output: &mut [u8], offset: &mut usize) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in bytes {
        if is_unescaped(*byte) {
            output[*offset] = *byte;
            *offset += 1;
        } else if *byte == b' ' {
            output[*offset] = b'+';
            *offset += 1;
        } else {
            output[*offset] = b'%';
            output[*offset + 1] = HEX[(byte >> 4) as usize];
            output[*offset + 2] = HEX[(byte & 0x0f) as usize];
            *offset += 3;
        }
    }
}

const fn is_unescaped(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'*' | b'-' | b'.' | b'_')
}
