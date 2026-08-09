//! Typed, bounded launch bindings delivered before child readiness.

use std::collections::BTreeSet;

use crate::ControlError;

/// Maximum authorized channel bindings delivered to one child.
pub const MAX_LAUNCH_CHANNEL_BINDINGS: usize = 128;
/// Maximum UTF-8 bytes in one signed channel name.
pub const MAX_LAUNCH_CHANNEL_NAME_BYTES: usize = 128;
/// Maximum UTF-8 bytes in one provider-specific model selector.
pub const MAX_LAUNCH_MODEL_BYTES: usize = 200;
/// Maximum output-token cap accepted by the Level 1 launch contract.
pub const MAX_LAUNCH_COMPLETION_TOKENS: u32 = 1_000_000;

/// Authorized operation for one signed channel name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelBindingAccess {
    /// The child may receive and acknowledge messages from this channel.
    Read,
    /// The child may publish messages to this channel.
    Write,
}

/// One signed channel name resolved to an authorized durable UUID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelBinding {
    name: String,
    access: ChannelBindingAccess,
    channel_id: [u8; 16],
}

impl ChannelBinding {
    /// Validate a signed channel name, access direction, and canonical UUID-v7 identity.
    pub fn new(
        name: impl Into<String>,
        access: ChannelBindingAccess,
        channel_id: [u8; 16],
    ) -> Result<Self, ControlError> {
        let name = name.into();
        if !valid_channel_name(&name) || !valid_uuid_v7(&channel_id) {
            return Err(ControlError::InvalidLaunchBindings);
        }
        Ok(Self {
            name,
            access,
            channel_id,
        })
    }

    /// Return the signed channel name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the authorized access direction.
    pub fn access(&self) -> ChannelBindingAccess {
        self.access
    }

    /// Return the canonical durable channel UUID.
    pub fn channel_id(&self) -> [u8; 16] {
        self.channel_id
    }
}

/// Bounded model settings selected for one Level 1 child launch.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelOneModelBinding {
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl LevelOneModelBinding {
    /// Validate one provider-specific model selector and bounded generation settings.
    pub fn new(
        model: impl Into<String>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<Self, ControlError> {
        let model = model.into();
        if model.trim().is_empty()
            || model.len() > MAX_LAUNCH_MODEL_BYTES
            || !temperature.is_finite()
            || !(0.0..=2.0).contains(&temperature)
            || max_tokens == 0
            || max_tokens > MAX_LAUNCH_COMPLETION_TOKENS
        {
            return Err(ControlError::InvalidLaunchBindings);
        }
        Ok(Self {
            model,
            temperature,
            max_tokens,
        })
    }

    /// Return the provider-specific model selector.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the finite sampling temperature.
    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Return the non-zero output-token cap.
    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }
}

/// Complete pipeline-authorized launch bindings for one child.
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchBindings {
    channels: Vec<ChannelBinding>,
    level_one_model: Option<LevelOneModelBinding>,
}

impl LaunchBindings {
    /// Validate, canonicalize, and own one launch's channel and model bindings.
    pub fn new(
        mut channels: Vec<ChannelBinding>,
        level_one_model: Option<LevelOneModelBinding>,
    ) -> Result<Self, ControlError> {
        if channels.len() > MAX_LAUNCH_CHANNEL_BINDINGS {
            return Err(ControlError::InvalidLaunchBindings);
        }
        channels.sort_by(|left, right| left.name.cmp(&right.name));
        let mut names = BTreeSet::new();
        let mut channel_ids = BTreeSet::new();
        for binding in &channels {
            if !names.insert(binding.name.clone()) || !channel_ids.insert(binding.channel_id) {
                return Err(ControlError::InvalidLaunchBindings);
            }
        }
        Ok(Self {
            channels,
            level_one_model,
        })
    }

    /// Borrow canonical channel bindings sorted by signed name.
    pub fn channels(&self) -> &[ChannelBinding] {
        &self.channels
    }

    /// Borrow Level 1 model settings when this package runtime requires them.
    pub fn level_one_model(&self) -> Option<&LevelOneModelBinding> {
        self.level_one_model.as_ref()
    }
}

pub(crate) fn encode_launch_bindings(bindings: &LaunchBindings) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&(bindings.channels.len() as u16).to_be_bytes());
    for binding in &bindings.channels {
        output.push(match binding.access {
            ChannelBindingAccess::Read => 1,
            ChannelBindingAccess::Write => 2,
        });
        output.push(binding.name.len() as u8);
        output.extend_from_slice(binding.name.as_bytes());
        output.extend_from_slice(&binding.channel_id);
    }
    match &bindings.level_one_model {
        None => output.push(0),
        Some(model) => {
            output.push(1);
            output.extend_from_slice(&(model.model.len() as u16).to_be_bytes());
            output.extend_from_slice(model.model.as_bytes());
            output.extend_from_slice(&model.temperature.to_bits().to_be_bytes());
            output.extend_from_slice(&model.max_tokens.to_be_bytes());
        }
    }
    output
}

pub(crate) fn decode_launch_bindings(body: &[u8]) -> Result<LaunchBindings, ControlError> {
    let mut decoder = Decoder::new(body);
    let count = decoder.u16()? as usize;
    if count > MAX_LAUNCH_CHANNEL_BINDINGS {
        return Err(ControlError::InvalidLaunchBindings);
    }
    let mut channels = Vec::with_capacity(count);
    for _ in 0..count {
        let access = match decoder.u8()? {
            1 => ChannelBindingAccess::Read,
            2 => ChannelBindingAccess::Write,
            _ => return Err(ControlError::InvalidLaunchBindings),
        };
        let name_length = decoder.u8()? as usize;
        if name_length == 0 || name_length > MAX_LAUNCH_CHANNEL_NAME_BYTES {
            return Err(ControlError::InvalidLaunchBindings);
        }
        let name = std::str::from_utf8(decoder.take(name_length)?)
            .map_err(|_| ControlError::InvalidLaunchBindings)?;
        let channel_id = decoder
            .take(16)?
            .try_into()
            .map_err(|_| ControlError::InvalidLaunchBindings)?;
        channels.push(ChannelBinding::new(name, access, channel_id)?);
    }
    let level_one_model = match decoder.u8()? {
        0 => None,
        1 => {
            let model_length = decoder.u16()? as usize;
            if model_length == 0 || model_length > MAX_LAUNCH_MODEL_BYTES {
                return Err(ControlError::InvalidLaunchBindings);
            }
            let model = std::str::from_utf8(decoder.take(model_length)?)
                .map_err(|_| ControlError::InvalidLaunchBindings)?;
            let temperature = f32::from_bits(decoder.u32()?);
            let max_tokens = decoder.u32()?;
            Some(LevelOneModelBinding::new(model, temperature, max_tokens)?)
        }
        _ => return Err(ControlError::InvalidLaunchBindings),
    };
    if !decoder.remaining().is_empty() {
        return Err(ControlError::InvalidLaunchBindings);
    }
    LaunchBindings::new(channels, level_one_model)
}

fn valid_channel_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_LAUNCH_CHANNEL_NAME_BYTES
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_uuid_v7(bytes: &[u8; 16]) -> bool {
    bytes[6] >> 4 == 7 && bytes[8] & 0xc0 == 0x80
}

struct Decoder<'a> {
    remaining: &'a [u8],
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn remaining(&self) -> &'a [u8] {
        self.remaining
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ControlError> {
        let (value, remaining) = self
            .remaining
            .split_at_checked(length)
            .ok_or(ControlError::InvalidLaunchBindings)?;
        self.remaining = remaining;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ControlError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ControlError> {
        Ok(u16::from_be_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ControlError::InvalidLaunchBindings)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, ControlError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ControlError::InvalidLaunchBindings)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid_v7(byte: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = byte;
        bytes
    }

    fn bindings() -> LaunchBindings {
        LaunchBindings::new(
            vec![
                ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, uuid_v7(2))
                    .unwrap(),
                ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                    .unwrap(),
            ],
            Some(LevelOneModelBinding::new("test-model", 0.25, 256).unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn exact_codec_round_trips_canonical_bindings() {
        let bindings = bindings();
        assert_eq!(bindings.channels()[0].name(), "weather-reports");
        assert_eq!(bindings.channels()[1].name(), "weather-requests");
        let bytes = encode_launch_bindings(&bindings);
        assert_eq!(decode_launch_bindings(&bytes), Ok(bindings));
    }

    #[test]
    fn constructors_reject_invalid_names_ids_models_and_duplicates() {
        for name in ["", "Bad", "bad_name", &"x".repeat(129)] {
            assert_eq!(
                ChannelBinding::new(name, ChannelBindingAccess::Read, uuid_v7(1)),
                Err(ControlError::InvalidLaunchBindings)
            );
        }
        assert_eq!(
            ChannelBinding::new("valid", ChannelBindingAccess::Read, [0; 16]),
            Err(ControlError::InvalidLaunchBindings)
        );
        for model in ["", "   ", &"x".repeat(201)] {
            assert_eq!(
                LevelOneModelBinding::new(model, 0.0, 1),
                Err(ControlError::InvalidLaunchBindings)
            );
        }
        for temperature in [f32::NAN, -0.1, 2.1] {
            assert_eq!(
                LevelOneModelBinding::new("model", temperature, 1),
                Err(ControlError::InvalidLaunchBindings)
            );
        }
        assert_eq!(
            LevelOneModelBinding::new("model", 0.0, 0),
            Err(ControlError::InvalidLaunchBindings)
        );
        let first = ChannelBinding::new("first", ChannelBindingAccess::Read, uuid_v7(1)).unwrap();
        let duplicate_name =
            ChannelBinding::new("first", ChannelBindingAccess::Write, uuid_v7(2)).unwrap();
        assert_eq!(
            LaunchBindings::new(vec![first.clone(), duplicate_name], None),
            Err(ControlError::InvalidLaunchBindings)
        );
        let duplicate_id =
            ChannelBinding::new("second", ChannelBindingAccess::Write, uuid_v7(1)).unwrap();
        assert_eq!(
            LaunchBindings::new(vec![first, duplicate_id], None),
            Err(ControlError::InvalidLaunchBindings)
        );
    }

    #[test]
    fn decoder_rejects_every_truncation_trailing_bytes_and_discriminants() {
        let bytes = encode_launch_bindings(&bindings());
        for length in 0..bytes.len() {
            assert_eq!(
                decode_launch_bindings(&bytes[..length]),
                Err(ControlError::InvalidLaunchBindings),
                "accepted truncation at {length}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            decode_launch_bindings(&trailing),
            Err(ControlError::InvalidLaunchBindings)
        );
        let mut invalid_access = bytes.clone();
        invalid_access[2] = 99;
        assert_eq!(
            decode_launch_bindings(&invalid_access),
            Err(ControlError::InvalidLaunchBindings)
        );
        let mut invalid_model_presence =
            encode_launch_bindings(&LaunchBindings::new(Vec::new(), None).expect("empty bindings"));
        invalid_model_presence[2] = 2;
        assert_eq!(
            decode_launch_bindings(&invalid_model_presence),
            Err(ControlError::InvalidLaunchBindings)
        );
    }
}
