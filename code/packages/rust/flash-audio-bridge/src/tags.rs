//! Zero-allocation SWF audio tag parsers (`SWF01` -> `SWF06`).
//!
//! Implements bounded, validated binary readers for every audio tag in the Adobe SWF
//! File Format Specification Version 10:
//!
//! - `DefineSound` (Tag 14): Defines an event sound asset with format, rate, width, channels,
//!   sample count, and encoded audio payload.
//! - `StartSound` (Tag 15) and `StartSound2` (Tag 89): Triggers or stops playback of an
//!   event sound by numeric ID or ActionScript 3 class name, with optional `inPoint`,
//!   `outPoint`, `loopCount`, and up to `MAX_ENVELOPE_POINTS` stereo volume envelope points.
//! - `SoundStreamHead` (Tag 18) and `SoundStreamHead2` (Tag 45): Configures timeline-synced
//!   streaming audio format, average samples per frame, and MP3 latency seek.
//! - `SoundStreamBlock` (Tag 19): Delivers one frame-aligned chunk of streaming audio data.
//!
//! ## Wire Layout Summary
//!
//! ```text
//! DefineSound (Tag 14):
//! +----------+--------+--------+--------+--------+-------------+-------------+
//! | SoundId  | Format |  Rate  |  Size  |  Type  | SampleCount | SoundData   |
//! | UI16 LE  | UB[4]  | UB[2]  | UB[1]  | UB[1]  | UI32 LE     | [u8]        |
//! +----------+--------+--------+--------+--------+-------------+-------------+
//!
//! StartSound (Tag 15) / StartSound2 (Tag 89):
//! +-------------------+------------------------------------------------------+
//! | SoundId (UI16) or | SoundInfo:                                           |
//! | ClassName (STR)   | [Flags: UB[2] rsv | Stop | NoMult | Env | Loop | O | I]
//! +-------------------+------------------------------------------------------+
//!
//! SoundStreamHead (Tag 18) / SoundStreamHead2 (Tag 45):
//! +----------------------+----------------------+-----------------+----------+
//! | PlaybackFlags (UI8)  | StreamFlags (UI8)    | AvgSampleCount  | Seek?    |
//! | rsv2|rate2|sz1|ch1   | fmt4|rate2|sz1|ch1   | UI16 LE         | SI16 LE  |
//! +----------------------+----------------------+-----------------+----------+
//! ```

use swf_container::{SwfTag, SwfTagKind};

use crate::error::AudioBridgeError;

/// Maximum per-channel sample count permitted in a single SWF audio definition (CWE-400).
pub const MAX_SOUND_SAMPLES: u32 = 8_388_608;

/// Maximum raw SWF audio tag payload size in bytes (32 MiB, CWE-400).
pub const MAX_TAG_PAYLOAD_BYTES: usize = 33_554_432;

/// Maximum number of `SOUNDENVELOPE` records allowed in a `SoundInfo` header (CWE-400).
pub const MAX_ENVELOPE_POINTS: usize = 16;

/// Maximum volume level in a `SOUNDENVELOPE` record (`32768` = unity 1.0 in SWF spec).
pub const MAX_ENVELOPE_LEVEL: u16 = 32_768;

/// Maximum loop count permitted in `StartSound` / `StartSound2` (CWE-400).
pub const MAX_LOOP_COUNT: u16 = 32_768;

/// Maximum byte length of a `StartSound2` `SoundClassName` (excluding NUL byte).
pub const MAX_CLASS_NAME_BYTES: usize = 256;

/// Numeric SWF tag code for `StartSound2` (Tag 89).
pub const SWF_TAG_START_SOUND2: u16 = 89;

/// SWF audio codec formats supported by `DefineSound` and `SoundStreamHead*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwfSoundFormat {
    /// Code `0`: Uncompressed PCM, native-endian (8-bit unsigned or 16-bit signed).
    UncompressedNe,
    /// Code `1`: Flash ADPCM (2-, 3-, 4-, or 5-bit adaptive delta PCM).
    Adpcm,
    /// Code `2`: MPEG-1/2/2.5 Layer III (MP3).
    Mp3,
    /// Code `3`: Uncompressed PCM, little-endian (8-bit unsigned or 16-bit signed LE).
    UncompressedLe,
}

impl SwfSoundFormat {
    /// Parses a 4-bit SWF `SoundFormat` code (`0..=3`).
    pub const fn format_from_code(code: u8) -> Result<Self, AudioBridgeError> {
        match code {
            0 => Ok(Self::UncompressedNe),
            1 => Ok(Self::Adpcm),
            2 => Ok(Self::Mp3),
            3 => Ok(Self::UncompressedLe),
            _ => Err(AudioBridgeError::UnsupportedSoundFormat { code }),
        }
    }

    /// Returns the 4-bit SWF wire format code.
    pub const fn format_to_code(self) -> u8 {
        match self {
            Self::UncompressedNe => 0,
            Self::Adpcm => 1,
            Self::Mp3 => 2,
            Self::UncompressedLe => 3,
        }
    }

    /// Returns `true` if this format is uncompressed PCM (`UncompressedNe` or `UncompressedLe`).
    pub const fn format_is_pcm(self) -> bool {
        matches!(self, Self::UncompressedNe | Self::UncompressedLe)
    }
}

/// SWF discrete audio sample rates encoded by 2-bit `SoundRate` (`0..=3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwfSoundRate {
    /// Code `0`: 5,512 Hz (nominally 5.5 kHz).
    Rate5512,
    /// Code `1`: 11,025 Hz.
    Rate11025,
    /// Code `2`: 22,050 Hz.
    Rate22050,
    /// Code `3`: 44,100 Hz.
    Rate44100,
}

impl SwfSoundRate {
    /// Parses a 2-bit SWF `SoundRate` code (`0..=3`).
    pub const fn rate_from_code(code: u8) -> Result<Self, AudioBridgeError> {
        match code {
            0 => Ok(Self::Rate5512),
            1 => Ok(Self::Rate11025),
            2 => Ok(Self::Rate22050),
            3 => Ok(Self::Rate44100),
            _ => Err(AudioBridgeError::InvalidSoundRateCode { code }),
        }
    }

    /// Converts the SWF rate enum into integer Hertz (`5512`, `11025`, `22050`, or `44100`).
    pub const fn rate_to_hz(self) -> u32 {
        match self {
            Self::Rate5512 => 5_512,
            Self::Rate11025 => 11_025,
            Self::Rate22050 => 22_050,
            Self::Rate44100 => 44_100,
        }
    }

    /// Returns the 2-bit SWF wire code (`0..=3`).
    pub const fn rate_to_code(self) -> u8 {
        match self {
            Self::Rate5512 => 0,
            Self::Rate11025 => 1,
            Self::Rate22050 => 2,
            Self::Rate44100 => 3,
        }
    }
}

/// Sample width encoded by the 1-bit `SoundSize` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwfSampleWidth {
    /// `SoundSize = 0`: 8-bit unsigned samples (`0..=255`).
    Bits8,
    /// `SoundSize = 1`: 16-bit signed samples (`-32768..=32767`).
    Bits16,
}

impl SwfSampleWidth {
    /// Decodes `SoundSize` bit (`false` = 8-bit, `true` = 16-bit).
    pub const fn width_from_bit(is_16bit: bool) -> Self {
        if is_16bit {
            Self::Bits16
        } else {
            Self::Bits8
        }
    }

    /// Bit depth (`8` or `16`).
    pub const fn width_bits(self) -> u8 {
        match self {
            Self::Bits8 => 8,
            Self::Bits16 => 16,
        }
    }

    /// Byte count per scalar sample (`1` or `2`).
    pub const fn width_bytes(self) -> usize {
        match self {
            Self::Bits8 => 1,
            Self::Bits16 => 2,
        }
    }
}

/// Channel layout encoded by the 1-bit `SoundType` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwfAudioChannels {
    /// `SoundType = 0`: 1 channel (mono).
    Mono,
    /// `SoundType = 1`: 2 interleaved channels (left, right).
    Stereo,
}

impl SwfAudioChannels {
    /// Decodes `SoundType` bit (`false` = Mono, `true` = Stereo).
    pub const fn channels_from_bit(is_stereo: bool) -> Self {
        if is_stereo {
            Self::Stereo
        } else {
            Self::Mono
        }
    }

    /// Number of audio channels (`1` or `2`).
    pub const fn channel_count(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

/// Packed SWF sound stream/definition header properties (1 byte on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwfSoundProperties {
    /// Audio codec format.
    pub format: SwfSoundFormat,
    /// Sampling rate.
    pub rate: SwfSoundRate,
    /// Uncompressed sample width (8-bit or 16-bit).
    pub width: SwfSampleWidth,
    /// Channel count (Mono or Stereo).
    pub channels: SwfAudioChannels,
}

impl SwfSoundProperties {
    /// Decodes the 1-byte bitfield `[Format:4 | Rate:2 | Size:1 | Type:1]`.
    pub const fn props_from_flags_byte(byte: u8) -> Result<Self, AudioBridgeError> {
        let fmt_code = (byte >> 4) & 0x0F;
        let rate_code = (byte >> 2) & 0x03;
        let is_16bit = ((byte >> 1) & 0x01) != 0;
        let is_stereo = (byte & 0x01) != 0;

        let format = match SwfSoundFormat::format_from_code(fmt_code) {
            Ok(f) => f,
            Err(e) => return Err(e),
        };
        let rate = match SwfSoundRate::rate_from_code(rate_code) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        Ok(Self {
            format,
            rate,
            width: SwfSampleWidth::width_from_bit(is_16bit),
            channels: SwfAudioChannels::channels_from_bit(is_stereo),
        })
    }

    /// Computes total expected decoded scalar sample count (`samples_per_channel * channels`)
    /// with checked integer arithmetic (CWE-190).
    pub fn props_total_scalar_samples(
        self,
        samples_per_channel: u32,
    ) -> Result<usize, AudioBridgeError> {
        if samples_per_channel > MAX_SOUND_SAMPLES {
            return Err(AudioBridgeError::SampleCountExceeded {
                count: samples_per_channel,
                max: MAX_SOUND_SAMPLES,
            });
        }
        let ch = self.channels.channel_count();
        (samples_per_channel as usize).checked_mul(ch).ok_or(
            AudioBridgeError::MalformedTagPayload {
                reason: "scalar sample count overflow",
            },
        )
    }

    /// Computes expected byte size for uncompressed PCM payloads (CWE-190).
    pub fn props_expected_pcm_bytes(
        self,
        samples_per_channel: u32,
    ) -> Result<usize, AudioBridgeError> {
        let scalars = self.props_total_scalar_samples(samples_per_channel)?;
        scalars
            .checked_mul(self.width.width_bytes())
            .ok_or(AudioBridgeError::MalformedTagPayload {
                reason: "PCM byte length overflow",
            })
    }
}

/// Parsed `DefineSound` (Tag 14) borrowing its bounded audio payload slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefineSoundTag<'a> {
    /// Non-zero SWF character identifier (`1..=65535`).
    pub sound_id: u16,
    /// Sound codec, sample rate, sample width, and channel configuration.
    pub properties: SwfSoundProperties,
    /// Number of samples *per channel* declared by the tag.
    pub sample_count: u32,
    /// Bounded payload slice containing encoded sound data.
    pub payload: &'a [u8],
}

impl<'a> DefineSoundTag<'a> {
    /// Returns the payload byte length.
    #[inline]
    pub const fn define_sound_payload_len(&self) -> usize {
        self.payload.len()
    }

    /// Returns the declared per-channel sample count.
    #[inline]
    pub const fn define_sound_sample_count(&self) -> u32 {
        self.sample_count
    }
}

/// Single volume envelope control point (`SOUNDENVELOPE`, 8 bytes on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SoundEnvelopePoint {
    /// Position from sound start in 44.1 kHz sample units.
    pub pos44: u32,
    /// Left channel gain in `[0, 32768]` (`32768` = 1.0).
    pub left_level: u16,
    /// Right channel gain in `[0, 32768]` (`32768` = 1.0).
    pub right_level: u16,
}

/// Playback modifiers (`SOUNDINFO`) shared by `StartSound` (Tag 15) and `StartSound2` (Tag 89).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundInfo {
    /// If `true`, stops any active voice playing this sound instead of starting one.
    pub sync_stop: bool,
    /// If `true`, suppresses starting a new voice if this sound is already playing.
    pub sync_no_multiple: bool,
    /// Optional start offset in 44.1 kHz sample units (`HasInPoint`).
    pub in_point: Option<u32>,
    /// Optional stop offset in 44.1 kHz sample units (`HasOutPoint`).
    pub out_point: Option<u32>,
    /// Number of repetitions (`1..=MAX_LOOP_COUNT`). Defaults to `1` when `HasLoops` is clear.
    pub loop_count: u16,
    /// Fixed-capacity array of stereo envelope points (zero-allocation).
    envelope_points: [SoundEnvelopePoint; MAX_ENVELOPE_POINTS],
    /// Number of valid entries in `envelope_points`.
    envelope_len: usize,
}

impl Default for SoundInfo {
    fn default() -> Self {
        Self {
            sync_stop: false,
            sync_no_multiple: false,
            in_point: None,
            out_point: None,
            loop_count: 1,
            envelope_points: [SoundEnvelopePoint {
                pos44: 0,
                left_level: MAX_ENVELOPE_LEVEL,
                right_level: MAX_ENVELOPE_LEVEL,
            }; MAX_ENVELOPE_POINTS],
            envelope_len: 0,
        }
    }
}

impl SoundInfo {
    /// Returns the active envelope points as a bounded slice.
    #[inline]
    pub fn info_envelope_slice(&self) -> &[SoundEnvelopePoint] {
        &self.envelope_points[..self.envelope_len]
    }

    /// Returns `true` if one or more envelope control points are present.
    #[inline]
    pub const fn info_has_envelope(&self) -> bool {
        self.envelope_len > 0
    }
}

/// Parsed `StartSound` (Tag 15) record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartSoundTag {
    /// Non-zero sound character ID (`1..=65535`).
    pub sound_id: u16,
    /// Playback modifiers, loop count, and volume envelope.
    pub info: SoundInfo,
}

impl StartSoundTag {
    /// Returns the target `sound_id`.
    #[inline]
    pub const fn start_sound_id(&self) -> u16 {
        self.sound_id
    }
}

/// Parsed `StartSound2` (Tag 89) record targeting an ActionScript 3 sound class name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartSound2Tag<'a> {
    /// Non-empty UTF-8 ActionScript linkage class name.
    pub class_name: &'a str,
    /// Playback modifiers, loop count, and volume envelope.
    pub info: SoundInfo,
}

impl<'a> StartSound2Tag<'a> {
    /// Returns the borrowed ActionScript linkage class name.
    #[inline]
    pub const fn start_sound2_class(&self) -> &'a str {
        self.class_name
    }
}

/// Parsed `SoundStreamHead` (Tag 18) or `SoundStreamHead2` (Tag 45).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundStreamHead {
    /// `false` for `SoundStreamHead` (Tag 18), `true` for `SoundStreamHead2` (Tag 45).
    pub is_v2: bool,
    /// Recommended playback sample rate.
    pub playback_rate: SwfSoundRate,
    /// Recommended playback sample width.
    pub playback_width: SwfSampleWidth,
    /// Recommended playback channel count.
    pub playback_channels: SwfAudioChannels,
    /// Encoded stream properties (format, sample rate, sample width, channels).
    pub stream_properties: SwfSoundProperties,
    /// Average number of samples per `SoundStreamBlock`.
    pub avg_sample_count: u16,
    /// Number of samples to seek forward/skip (MP3 streams only; `0` otherwise).
    pub latency_seek: i16,
}

impl SoundStreamHead {
    /// Returns the stream's encoded sampling rate in Hertz.
    #[inline]
    pub const fn stream_head_rate_hz(&self) -> u32 {
        self.stream_properties.rate.rate_to_hz()
    }
}

/// Parsed `SoundStreamBlock` (Tag 19) borrowing its bounded stream chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundStreamBlockTag<'a> {
    /// MP3-only sample count declared at the head of the block.
    pub mp3_sample_count: Option<u16>,
    /// MP3-only sample seek offset at the head of the block.
    pub mp3_seek_samples: Option<i16>,
    /// Borrowed audio payload bytes for this frame.
    pub audio_data: &'a [u8],
}

impl<'a> SoundStreamBlockTag<'a> {
    /// Returns the byte length of `audio_data`.
    #[inline]
    pub const fn stream_block_len(&self) -> usize {
        self.audio_data.len()
    }
}

/// Unified parsed SWF audio tag dispatched from [`swf_container::SwfTag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwfAudioTag<'a> {
    /// Tag 14: Event sound definition.
    DefineSound(DefineSoundTag<'a>),
    /// Tag 15: Event sound playback trigger/stop by ID.
    StartSound(StartSoundTag),
    /// Tag 89: Event sound playback trigger/stop by AS3 class name.
    StartSound2(StartSound2Tag<'a>),
    /// Tag 18 or Tag 45: Streaming audio track configuration.
    SoundStreamHead(SoundStreamHead),
    /// Tag 19: Frame-synchronized streaming audio block.
    SoundStreamBlock(SoundStreamBlockTag<'a>),
}

/// Parses a `DefineSound` (Tag 14) payload into a bounded [`DefineSoundTag`].
pub fn parse_define_sound(data: &[u8]) -> Result<DefineSoundTag<'_>, AudioBridgeError> {
    if data.len() > MAX_TAG_PAYLOAD_BYTES {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "DefineSound payload exceeds maximum tag size",
        });
    }
    if data.len() < 7 {
        return Err(AudioBridgeError::UnexpectedEof);
    }

    let sound_id = u16::from_le_bytes([data[0], data[1]]);
    if sound_id == 0 {
        return Err(AudioBridgeError::InvalidSoundId);
    }

    let properties = SwfSoundProperties::props_from_flags_byte(data[2])?;
    let sample_count = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    if sample_count > MAX_SOUND_SAMPLES {
        return Err(AudioBridgeError::SampleCountExceeded {
            count: sample_count,
            max: MAX_SOUND_SAMPLES,
        });
    }

    let payload = &data[7..];
    match properties.format {
        SwfSoundFormat::UncompressedNe | SwfSoundFormat::UncompressedLe => {
            let expected_bytes = properties.props_expected_pcm_bytes(sample_count)?;
            if payload.len() < expected_bytes {
                return Err(AudioBridgeError::UnexpectedEof);
            }
        }
        SwfSoundFormat::Adpcm => {
            if sample_count > 0 && payload.is_empty() {
                return Err(AudioBridgeError::UnexpectedEof);
            }
        }
        SwfSoundFormat::Mp3 => {
            if sample_count > 0 && payload.len() < 2 {
                return Err(AudioBridgeError::UnexpectedEof);
            }
        }
    }

    Ok(DefineSoundTag {
        sound_id,
        properties,
        sample_count,
        payload,
    })
}

/// Parses a SWF `SOUNDINFO` record starting at `data[0]`, returning `(SoundInfo, bytes_consumed)`.
pub fn parse_sound_info(data: &[u8]) -> Result<(SoundInfo, usize), AudioBridgeError> {
    if data.is_empty() {
        return Err(AudioBridgeError::UnexpectedEof);
    }
    let flags = data[0];
    if (flags & 0xC0) != 0 {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "SOUNDINFO reserved upper bits must be zero",
        });
    }

    let sync_stop = (flags & 0x20) != 0;
    let sync_no_multiple = (flags & 0x10) != 0;
    let has_envelope = (flags & 0x08) != 0;
    let has_loops = (flags & 0x04) != 0;
    let has_out_point = (flags & 0x02) != 0;
    let has_in_point = (flags & 0x01) != 0;

    let mut offset = 1usize;

    let in_point = if has_in_point {
        if offset.checked_add(4).is_none_or(|end| end > data.len()) {
            return Err(AudioBridgeError::UnexpectedEof);
        }
        let v = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        Some(v)
    } else {
        None
    };

    let out_point = if has_out_point {
        if offset.checked_add(4).is_none_or(|end| end > data.len()) {
            return Err(AudioBridgeError::UnexpectedEof);
        }
        let v = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        Some(v)
    } else {
        None
    };

    if let (Some(inp), Some(outp)) = (in_point, out_point) {
        if inp >= outp {
            return Err(AudioBridgeError::InvalidInOutPoints {
                in_point: inp,
                out_point: outp,
            });
        }
    }

    let loop_count = if has_loops {
        if offset.checked_add(2).is_none_or(|end| end > data.len()) {
            return Err(AudioBridgeError::UnexpectedEof);
        }
        let raw_loops = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        if raw_loops == 0 || raw_loops > MAX_LOOP_COUNT {
            return Err(AudioBridgeError::InvalidLoopCount { count: raw_loops });
        }
        raw_loops
    } else {
        1
    };

    let mut info = SoundInfo {
        sync_stop,
        sync_no_multiple,
        in_point,
        out_point,
        loop_count,
        ..SoundInfo::default()
    };

    if has_envelope {
        if offset >= data.len() {
            return Err(AudioBridgeError::UnexpectedEof);
        }
        let env_count = data[offset] as usize;
        offset += 1;
        if env_count > MAX_ENVELOPE_POINTS {
            return Err(AudioBridgeError::InvalidEnvelopeCount {
                count: env_count,
                max: MAX_ENVELOPE_POINTS,
            });
        }
        let needed = env_count
            .checked_mul(8)
            .ok_or(AudioBridgeError::MalformedTagPayload {
                reason: "envelope byte size overflow",
            })?;
        if offset
            .checked_add(needed)
            .is_none_or(|end| end > data.len())
        {
            return Err(AudioBridgeError::UnexpectedEof);
        }

        let mut prev_pos: Option<u32> = None;
        for idx in 0..env_count {
            let base = offset + idx * 8;
            let pos44 =
                u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
            let left_level = u16::from_le_bytes([data[base + 4], data[base + 5]]);
            let right_level = u16::from_le_bytes([data[base + 6], data[base + 7]]);

            if left_level > MAX_ENVELOPE_LEVEL || right_level > MAX_ENVELOPE_LEVEL {
                return Err(AudioBridgeError::InvalidEnvelopePoint {
                    reason: "envelope level exceeds 32768",
                });
            }
            if let Some(prev) = prev_pos {
                if pos44 < prev {
                    return Err(AudioBridgeError::InvalidEnvelopePoint {
                        reason: "envelope pos44 is not monotonic",
                    });
                }
            }
            prev_pos = Some(pos44);
            info.envelope_points[idx] = SoundEnvelopePoint {
                pos44,
                left_level,
                right_level,
            };
        }
        info.envelope_len = env_count;
        offset += needed;
    }

    Ok((info, offset))
}

/// Parses a `StartSound` (Tag 15) payload.
pub fn parse_start_sound(data: &[u8]) -> Result<StartSoundTag, AudioBridgeError> {
    if data.len() < 3 {
        return Err(AudioBridgeError::UnexpectedEof);
    }
    let sound_id = u16::from_le_bytes([data[0], data[1]]);
    if sound_id == 0 {
        return Err(AudioBridgeError::InvalidSoundId);
    }
    let (info, _) = parse_sound_info(&data[2..])?;
    Ok(StartSoundTag { sound_id, info })
}

/// Parses a `StartSound2` (Tag 89) payload (`SoundClassName` + `SOUNDINFO`).
pub fn parse_start_sound2(data: &[u8]) -> Result<StartSound2Tag<'_>, AudioBridgeError> {
    let nul_pos = data
        .iter()
        .position(|&b| b == 0)
        .ok_or(AudioBridgeError::InvalidClassName)?;
    if nul_pos == 0 || nul_pos > MAX_CLASS_NAME_BYTES {
        return Err(AudioBridgeError::InvalidClassName);
    }
    let class_name =
        std::str::from_utf8(&data[..nul_pos]).map_err(|_| AudioBridgeError::InvalidClassName)?;
    let (info, _) = parse_sound_info(&data[nul_pos + 1..])?;
    Ok(StartSound2Tag { class_name, info })
}

/// Parses a `SoundStreamHead` (Tag 18) or `SoundStreamHead2` (Tag 45) payload.
pub fn parse_sound_stream_head(
    data: &[u8],
    is_v2: bool,
) -> Result<SoundStreamHead, AudioBridgeError> {
    if data.len() < 4 {
        return Err(AudioBridgeError::UnexpectedEof);
    }

    let playback_byte = data[0];
    if (playback_byte & 0xC0) != 0 {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "SoundStreamHead playback reserved bits must be zero",
        });
    }
    let playback_rate = SwfSoundRate::rate_from_code((playback_byte >> 4) & 0x03)?;
    let playback_width = SwfSampleWidth::width_from_bit(((playback_byte >> 3) & 0x01) != 0);
    let playback_channels = SwfAudioChannels::channels_from_bit(((playback_byte >> 2) & 0x01) != 0);

    let stream_properties = SwfSoundProperties::props_from_flags_byte(data[1])?;
    if !is_v2
        && !matches!(
            stream_properties.format,
            SwfSoundFormat::UncompressedNe | SwfSoundFormat::Adpcm | SwfSoundFormat::Mp3
        )
    {
        return Err(AudioBridgeError::UnsupportedSoundFormat {
            code: stream_properties.format.format_to_code(),
        });
    }

    let avg_sample_count = u16::from_le_bytes([data[2], data[3]]);

    let latency_seek = if stream_properties.format == SwfSoundFormat::Mp3 {
        if data.len() < 6 {
            return Err(AudioBridgeError::UnexpectedEof);
        }
        i16::from_le_bytes([data[4], data[5]])
    } else {
        0
    };

    Ok(SoundStreamHead {
        is_v2,
        playback_rate,
        playback_width,
        playback_channels,
        stream_properties,
        avg_sample_count,
        latency_seek,
    })
}

/// Parses a `SoundStreamBlock` (Tag 19) payload given the active stream codec format.
pub fn parse_sound_stream_block(
    data: &[u8],
    stream_format: SwfSoundFormat,
) -> Result<SoundStreamBlockTag<'_>, AudioBridgeError> {
    if data.len() > MAX_TAG_PAYLOAD_BYTES {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "SoundStreamBlock payload exceeds maximum tag size",
        });
    }
    if stream_format == SwfSoundFormat::Mp3 {
        if data.len() < 4 {
            return Err(AudioBridgeError::UnexpectedEof);
        }
        let sample_count = u16::from_le_bytes([data[0], data[1]]);
        let seek_samples = i16::from_le_bytes([data[2], data[3]]);
        Ok(SoundStreamBlockTag {
            mp3_sample_count: Some(sample_count),
            mp3_seek_samples: Some(seek_samples),
            audio_data: &data[4..],
        })
    } else {
        Ok(SoundStreamBlockTag {
            mp3_sample_count: None,
            mp3_seek_samples: None,
            audio_data: data,
        })
    }
}

/// Dispatches a borrowed [`swf_container::SwfTag`] into [`SwfAudioTag`] when it is an audio tag.
///
/// Returns `Ok(None)` for non-audio SWF tags (shapes, display list, scripts, etc.).
pub fn parse_swf_audio_tag<'a>(
    tag: &SwfTag<'a>,
    active_stream_format: Option<SwfSoundFormat>,
) -> Result<Option<SwfAudioTag<'a>>, AudioBridgeError> {
    let payload = tag.tag_payload_slice();
    match tag.header.kind {
        SwfTagKind::DefineSound => Ok(Some(SwfAudioTag::DefineSound(parse_define_sound(payload)?))),
        SwfTagKind::StartSound => Ok(Some(SwfAudioTag::StartSound(parse_start_sound(payload)?))),
        SwfTagKind::Unknown(SWF_TAG_START_SOUND2) => {
            Ok(Some(SwfAudioTag::StartSound2(parse_start_sound2(payload)?)))
        }
        SwfTagKind::SoundStreamHead => Ok(Some(SwfAudioTag::SoundStreamHead(
            parse_sound_stream_head(payload, false)?,
        ))),
        SwfTagKind::SoundStreamHead2 => Ok(Some(SwfAudioTag::SoundStreamHead(
            parse_sound_stream_head(payload, true)?,
        ))),
        SwfTagKind::SoundStreamBlock => {
            let fmt = active_stream_format.ok_or(AudioBridgeError::StreamFormatMismatch)?;
            Ok(Some(SwfAudioTag::SoundStreamBlock(
                parse_sound_stream_block(payload, fmt)?,
            )))
        }
        _ => Ok(None),
    }
}
