//! Zero-allocation audio decoders and MP3 frame validators (`SWF06`).
//!
//! Decodes SWF audio payloads directly into caller-provided `&mut [i16]` slices without
//! heap allocation (`max_alloc_bytes == 0`):
//!
//! - **Uncompressed PCM**: 8-bit unsigned (`0..=255` centered at `128`) and 16-bit signed
//!   little-endian / native-endian (`UncompressedLe`, `UncompressedNe`).
//! - **Flash ADPCM**: Custom 2-bit, 3-bit, 4-bit, and 5-bit Adaptive Differential Pulse-Code
//!   Modulation in 4,096-sample blocks with IMA step-size and bit-width-specific delta-index
//!   tables, powered by `swf_container::BitReader`.
//! - **MP3 Frame Validation & Bounded Extraction**: Validates ISO/IEC 11172-3 / 13818-3 Layer III
//!   frame headers (sync word, MPEG version, bitrate, sampling rate, padding, channel mode)
//!   and yields bounded frame slices from `DefineSound` and `SoundStreamBlock` payloads.

use swf_container::BitReader;

use crate::error::AudioBridgeError;
use crate::tags::{
    DefineSoundTag, SwfAudioChannels, SwfSampleWidth, SwfSoundFormat, MAX_SOUND_SAMPLES,
};

/// Number of per-channel samples in one Flash ADPCM packet (1 initial sample + up to 4095 deltas).
pub const ADPCM_SAMPLES_PER_BLOCK: u32 = 4096;

/// Maximum number of MP3 frames yielded by [`Mp3FrameIter`] (CWE-400 guard).
pub const MAX_MP3_FRAMES: usize = 65_536;

/// Standard 89-entry IMA / Flash ADPCM quantizer step-size table (`0..=88`).
const ADPCM_STEP_SIZE_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

const ADPCM_INDEX_TABLE_2: [i32; 2] = [-1, 2];
const ADPCM_INDEX_TABLE_3: [i32; 4] = [-1, -1, 2, 4];
const ADPCM_INDEX_TABLE_4: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
const ADPCM_INDEX_TABLE_5: [i32; 16] = [-1, -1, -1, -1, -1, -1, -1, -1, 1, 2, 4, 6, 8, 10, 13, 16];

const MPEG1_L3_BITRATES_KBPS: [u16; 15] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];

const MPEG2_L3_BITRATES_KBPS: [u16; 15] =
    [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

const MPEG1_SAMPLE_RATES_HZ: [u32; 3] = [44_100, 48_000, 32_000];
const MPEG2_SAMPLE_RATES_HZ: [u32; 3] = [22_050, 24_000, 16_000];
const MPEG25_SAMPLE_RATES_HZ: [u32; 3] = [11_025, 12_000, 8_000];

/// Decodes 8-bit unsigned PCM bytes (`0..=255`, center `128`) into signed `i16` samples.
///
/// Mapping: `sample_i16 = ((byte as i32) - 128) << 8`.
/// Returns the number of scalar `i16` samples written.
pub fn decode_pcm8_into(payload: &[u8], out: &mut [i16]) -> Result<usize, AudioBridgeError> {
    let required = payload.len();
    if out.len() < required {
        return Err(AudioBridgeError::BufferTooSmall {
            required,
            provided: out.len(),
        });
    }
    for (dst, &byte) in out[..required].iter_mut().zip(payload.iter()) {
        *dst = (((byte as i32) - 128) << 8) as i16;
    }
    Ok(required)
}

/// Decodes 16-bit signed little-endian PCM bytes (`UncompressedLe`) into `i16` samples.
///
/// Uses `.as_chunks::<2>()` and rejects unaligned trailing bytes (CWE-20).
pub fn decode_pcm16_le_into(payload: &[u8], out: &mut [i16]) -> Result<usize, AudioBridgeError> {
    let (pairs, rem) = payload.as_chunks::<2>();
    if !rem.is_empty() {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "16-bit LE PCM payload has odd byte count",
        });
    }
    let required = pairs.len();
    if out.len() < required {
        return Err(AudioBridgeError::BufferTooSmall {
            required,
            provided: out.len(),
        });
    }
    for (dst, pair) in out[..required].iter_mut().zip(pairs.iter()) {
        *dst = i16::from_le_bytes(*pair);
    }
    Ok(required)
}

/// Decodes 16-bit signed native-endian PCM bytes (`UncompressedNe`) into `i16` samples.
///
/// Uses `.as_chunks::<2>()` and rejects unaligned trailing bytes (CWE-20).
pub fn decode_pcm16_ne_into(payload: &[u8], out: &mut [i16]) -> Result<usize, AudioBridgeError> {
    let (pairs, rem) = payload.as_chunks::<2>();
    if !rem.is_empty() {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "16-bit NE PCM payload has odd byte count",
        });
    }
    let required = pairs.len();
    if out.len() < required {
        return Err(AudioBridgeError::BufferTooSmall {
            required,
            provided: out.len(),
        });
    }
    for (dst, pair) in out[..required].iter_mut().zip(pairs.iter()) {
        *dst = i16::from_ne_bytes(*pair);
    }
    Ok(required)
}

/// Downmixes interleaved stereo `i16` pairs `[L, R, L, R, ...]` into mono `i16` samples.
///
/// Uses exact `i32` midpoint `(L + R) / 2` without overflow (CWE-190).
pub fn mix_stereo_to_mono_into(
    interleaved_stereo: &[i16],
    out_mono: &mut [i16],
) -> Result<usize, AudioBridgeError> {
    let (pairs, rem) = interleaved_stereo.as_chunks::<2>();
    if !rem.is_empty() {
        return Err(AudioBridgeError::MalformedTagPayload {
            reason: "interleaved stereo slice has odd sample count",
        });
    }
    let required = pairs.len();
    if out_mono.len() < required {
        return Err(AudioBridgeError::BufferTooSmall {
            required,
            provided: out_mono.len(),
        });
    }
    for (dst, &[left, right]) in out_mono[..required].iter_mut().zip(pairs.iter()) {
        *dst = (((left as i32) + (right as i32)) >> 1) as i16;
    }
    Ok(required)
}

/// Steps one channel of Flash ADPCM predictor + step-index state for an `n`-bit code (`n in 2..=5`).
#[inline]
fn adpcm_step_channel(
    bits_per_sample: usize,
    code: u32,
    predictor: &mut i32,
    step_index: &mut usize,
) -> Result<i16, AudioBridgeError> {
    if !(2..=5).contains(&bits_per_sample) {
        return Err(AudioBridgeError::InvalidAdpcmBitWidth {
            bits: bits_per_sample,
        });
    }
    if *step_index >= ADPCM_STEP_SIZE_TABLE.len() {
        return Err(AudioBridgeError::InvalidAdpcmStepIndex { index: *step_index });
    }

    let step = ADPCM_STEP_SIZE_TABLE[*step_index];
    let sign_bit = 1u32 << (bits_per_sample - 1);
    let mag_mask = sign_bit - 1;
    let mag = (code & mag_mask) as usize;

    // Flash SWF ADPCM shift-sum difference:
    // delta = step >> (n - 1) + sum_{k=0..n-2} (if bit k set: step >> (n - 2 - k))
    let mut delta = step >> (bits_per_sample - 1);
    let mut bit_idx = 0usize;
    while bit_idx < bits_per_sample - 1 {
        if (code & (1u32 << bit_idx)) != 0 {
            delta += step >> (bits_per_sample - 2 - bit_idx);
        }
        bit_idx += 1;
    }

    if (code & sign_bit) != 0 {
        *predictor = predictor.saturating_sub(delta);
    } else {
        *predictor = predictor.saturating_add(delta);
    }
    *predictor = (*predictor).clamp(i16::MIN as i32, i16::MAX as i32);

    let index_delta = match bits_per_sample {
        2 => ADPCM_INDEX_TABLE_2[mag],
        3 => ADPCM_INDEX_TABLE_3[mag],
        4 => ADPCM_INDEX_TABLE_4[mag],
        5 => ADPCM_INDEX_TABLE_5[mag],
        _ => {
            return Err(AudioBridgeError::InvalidAdpcmBitWidth {
                bits: bits_per_sample,
            });
        }
    };
    *step_index = ((*step_index as i32) + index_delta).clamp(0, 88) as usize;

    Ok(*predictor as i16)
}

/// Decodes a Flash ADPCM stream (`2..=5` bits per sample, mono or stereo) into `out`.
///
/// Zero heap allocations. Returns the total scalar `i16` sample count written
/// (`sample_count_per_channel * channels.channel_count()`).
pub fn decode_adpcm_into(
    payload: &[u8],
    channels: SwfAudioChannels,
    sample_count_per_channel: u32,
    out: &mut [i16],
) -> Result<usize, AudioBridgeError> {
    if sample_count_per_channel > MAX_SOUND_SAMPLES {
        return Err(AudioBridgeError::SampleCountExceeded {
            count: sample_count_per_channel,
            max: MAX_SOUND_SAMPLES,
        });
    }
    if sample_count_per_channel == 0 {
        return Ok(0);
    }

    let channel_count = channels.channel_count();
    let total_scalars = (sample_count_per_channel as usize)
        .checked_mul(channel_count)
        .ok_or(AudioBridgeError::MalformedTagPayload {
            reason: "ADPCM output sample count overflow",
        })?;
    if out.len() < total_scalars {
        return Err(AudioBridgeError::BufferTooSmall {
            required: total_scalars,
            provided: out.len(),
        });
    }

    let mut reader = BitReader::from_bits(payload);
    let code_size = reader
        .read_ub(2)
        .map_err(|_| AudioBridgeError::UnexpectedEof)? as usize;
    let bits_per_sample = code_size + 2;
    if !(2..=5).contains(&bits_per_sample) {
        return Err(AudioBridgeError::InvalidAdpcmBitWidth {
            bits: bits_per_sample,
        });
    }

    let mut decoded_per_ch: u32 = 0;
    let mut out_idx: usize = 0;

    while decoded_per_ch < sample_count_per_channel {
        let packet_samples =
            (sample_count_per_channel - decoded_per_ch).min(ADPCM_SAMPLES_PER_BLOCK);

        let mut pred_l = reader
            .read_sb(16)
            .map_err(|_| AudioBridgeError::UnexpectedEof)?;
        let mut idx_l = reader
            .read_ub(6)
            .map_err(|_| AudioBridgeError::UnexpectedEof)? as usize;
        if idx_l >= ADPCM_STEP_SIZE_TABLE.len() {
            return Err(AudioBridgeError::InvalidAdpcmStepIndex { index: idx_l });
        }

        let (mut pred_r, mut idx_r) = if channels == SwfAudioChannels::Stereo {
            let pr = reader
                .read_sb(16)
                .map_err(|_| AudioBridgeError::UnexpectedEof)?;
            let ir = reader
                .read_ub(6)
                .map_err(|_| AudioBridgeError::UnexpectedEof)? as usize;
            if ir >= ADPCM_STEP_SIZE_TABLE.len() {
                return Err(AudioBridgeError::InvalidAdpcmStepIndex { index: ir });
            }
            (pr, ir)
        } else {
            (0, 0)
        };

        // Sample 0 of the packet is the explicit 16-bit InitialSample.
        out[out_idx] = pred_l.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        out_idx += 1;
        if channels == SwfAudioChannels::Stereo {
            out[out_idx] = pred_r.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            out_idx += 1;
        }

        for _ in 1..packet_samples {
            let code_l = reader
                .read_ub(bits_per_sample)
                .map_err(|_| AudioBridgeError::UnexpectedEof)?;
            let sample_l = adpcm_step_channel(bits_per_sample, code_l, &mut pred_l, &mut idx_l)?;
            out[out_idx] = sample_l;
            out_idx += 1;

            if channels == SwfAudioChannels::Stereo {
                let code_r = reader
                    .read_ub(bits_per_sample)
                    .map_err(|_| AudioBridgeError::UnexpectedEof)?;
                let sample_r =
                    adpcm_step_channel(bits_per_sample, code_r, &mut pred_r, &mut idx_r)?;
                out[out_idx] = sample_r;
                out_idx += 1;
            }
        }

        decoded_per_ch += packet_samples;
    }

    Ok(out_idx)
}

/// Decodes an uncompressed PCM or Flash ADPCM [`DefineSoundTag`] directly into `out`.
///
/// Zero heap allocations. Returns the number of scalar `i16` samples written.
/// MP3 `DefineSound` tags require frame extraction via [`extract_define_sound_mp3_info`] and
/// [`Mp3FrameIter`] and return `Err(AudioBridgeError::UnsupportedSoundFormat { code: 2 })`.
pub fn decode_define_sound_into(
    tag: &DefineSoundTag<'_>,
    out: &mut [i16],
) -> Result<usize, AudioBridgeError> {
    if tag.sound_id == 0 {
        return Err(AudioBridgeError::InvalidSoundId);
    }
    let props = tag.properties;
    match props.format {
        SwfSoundFormat::UncompressedNe => match props.width {
            SwfSampleWidth::Bits8 => {
                let expected = props.props_expected_pcm_bytes(tag.sample_count)?;
                let pcm_slice = tag
                    .payload
                    .get(..expected)
                    .ok_or(AudioBridgeError::UnexpectedEof)?;
                decode_pcm8_into(pcm_slice, out)
            }
            SwfSampleWidth::Bits16 => {
                let expected = props.props_expected_pcm_bytes(tag.sample_count)?;
                let pcm_slice = tag
                    .payload
                    .get(..expected)
                    .ok_or(AudioBridgeError::UnexpectedEof)?;
                decode_pcm16_ne_into(pcm_slice, out)
            }
        },
        SwfSoundFormat::UncompressedLe => match props.width {
            SwfSampleWidth::Bits8 => {
                let expected = props.props_expected_pcm_bytes(tag.sample_count)?;
                let pcm_slice = tag
                    .payload
                    .get(..expected)
                    .ok_or(AudioBridgeError::UnexpectedEof)?;
                decode_pcm8_into(pcm_slice, out)
            }
            SwfSampleWidth::Bits16 => {
                let expected = props.props_expected_pcm_bytes(tag.sample_count)?;
                let pcm_slice = tag
                    .payload
                    .get(..expected)
                    .ok_or(AudioBridgeError::UnexpectedEof)?;
                decode_pcm16_le_into(pcm_slice, out)
            }
        },
        SwfSoundFormat::Adpcm => {
            decode_adpcm_into(tag.payload, props.channels, tag.sample_count, out)
        }
        SwfSoundFormat::Mp3 => Err(AudioBridgeError::UnsupportedSoundFormat { code: 2 }),
    }
}

/// MPEG audio standard version parsed from an MP3 frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpegVersion {
    /// MPEG Version 1 (ISO/IEC 11172-3).
    Mpeg1,
    /// MPEG Version 2 (ISO/IEC 13818-3).
    Mpeg2,
    /// MPEG Version 2.5 (unofficial low-bitrate extension).
    Mpeg2_5,
}

impl MpegVersion {
    /// Returns PCM samples per Layer III frame (`1152` for MPEG-1, `576` for MPEG-2/2.5).
    #[inline]
    pub const fn mpeg_version_samples_per_frame(self) -> u16 {
        match self {
            Self::Mpeg1 => 1152,
            Self::Mpeg2 | Self::Mpeg2_5 => 576,
        }
    }
}

/// MP3 channel configuration parsed from bits 7..6 of the frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp3ChannelMode {
    /// Independent stereo channels.
    Stereo,
    /// Joint stereo (mid/side or intensity).
    JointStereo,
    /// Dual independent mono channels.
    DualChannel,
    /// Single channel (mono).
    Mono,
}

impl Mp3ChannelMode {
    /// Returns the number of audio channels (`1` for Mono, `2` otherwise).
    #[inline]
    pub const fn mp3_mode_channel_count(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo | Self::JointStereo | Self::DualChannel => 2,
        }
    }
}

/// Validated 4-byte MPEG Layer III frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp3FrameHeader {
    /// MPEG version (`Mpeg1`, `Mpeg2`, or `Mpeg2_5`).
    pub version: MpegVersion,
    /// `true` if a 16-bit CRC follows the 4-byte frame header.
    pub has_crc: bool,
    /// Bitrate in kilobits per second.
    pub bitrate_kbps: u16,
    /// Sampling frequency in Hertz (`8000..=48000`).
    pub sample_rate_hz: u32,
    /// Whether this frame carries a 1-byte padding slot.
    pub padding: bool,
    /// Channel encoding mode.
    pub channel_mode: Mp3ChannelMode,
    /// Exact total frame length in bytes (including the 4-byte header).
    pub frame_byte_len: usize,
    /// PCM samples per channel decoded from this frame (`1152` or `576`).
    pub samples_per_frame: u16,
}

impl Mp3FrameHeader {
    /// Returns the total frame byte length including header.
    #[inline]
    pub const fn mp3_header_frame_len(&self) -> usize {
        self.frame_byte_len
    }
}

/// A validated, borrowed MP3 frame slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp3Frame<'a> {
    /// Validated frame header metadata.
    pub header: Mp3FrameHeader,
    /// Exact bounded slice covering `[0..header.frame_byte_len]`.
    pub bytes: &'a [u8],
}

/// Parses and validates a 4-byte MPEG Layer III frame header.
pub fn parse_mp3_frame_header(bytes: &[u8]) -> Result<Mp3FrameHeader, AudioBridgeError> {
    if bytes.len() < 4 {
        return Err(AudioBridgeError::UnexpectedEof);
    }
    let hdr = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);

    // 11-bit frame sync must be all ones (0x7FF).
    if ((hdr >> 21) & 0x7FF) != 0x7FF {
        return Err(AudioBridgeError::InvalidMp3Frame {
            reason: "missing 11-bit MP3 frame sync",
        });
    }

    let version = match (hdr >> 19) & 0x03 {
        0 => MpegVersion::Mpeg2_5,
        2 => MpegVersion::Mpeg2,
        3 => MpegVersion::Mpeg1,
        _ => {
            return Err(AudioBridgeError::InvalidMp3Frame {
                reason: "reserved MPEG version ID",
            });
        }
    };

    // Layer bits: 01 = Layer III (MP3).
    if ((hdr >> 17) & 0x03) != 1 {
        return Err(AudioBridgeError::InvalidMp3Frame {
            reason: "expected MPEG Layer III frame",
        });
    }

    let has_crc = ((hdr >> 16) & 0x01) == 0;
    let bitrate_idx = ((hdr >> 12) & 0x0F) as usize;
    if bitrate_idx == 0 || bitrate_idx == 15 {
        return Err(AudioBridgeError::InvalidMp3Frame {
            reason: "free or invalid MP3 bitrate index",
        });
    }

    let bitrate_kbps = match version {
        MpegVersion::Mpeg1 => MPEG1_L3_BITRATES_KBPS[bitrate_idx],
        MpegVersion::Mpeg2 | MpegVersion::Mpeg2_5 => MPEG2_L3_BITRATES_KBPS[bitrate_idx],
    };

    let sr_idx = ((hdr >> 10) & 0x03) as usize;
    if sr_idx >= 3 {
        return Err(AudioBridgeError::InvalidMp3Frame {
            reason: "reserved MP3 sample rate index",
        });
    }
    let sample_rate_hz = match version {
        MpegVersion::Mpeg1 => MPEG1_SAMPLE_RATES_HZ[sr_idx],
        MpegVersion::Mpeg2 => MPEG2_SAMPLE_RATES_HZ[sr_idx],
        MpegVersion::Mpeg2_5 => MPEG25_SAMPLE_RATES_HZ[sr_idx],
    };

    let padding = ((hdr >> 9) & 0x01) != 0;
    let channel_mode = match (hdr >> 6) & 0x03 {
        0 => Mp3ChannelMode::Stereo,
        1 => Mp3ChannelMode::JointStereo,
        2 => Mp3ChannelMode::DualChannel,
        _ => Mp3ChannelMode::Mono,
    };

    let samples_per_frame = version.mpeg_version_samples_per_frame();
    let coeff: usize = match version {
        MpegVersion::Mpeg1 => 144_000,
        MpegVersion::Mpeg2 | MpegVersion::Mpeg2_5 => 72_000,
    };
    let pad_bytes = usize::from(padding);

    // sample_rate_hz is guaranteed >= 8000 (CWE-369 safe).
    let frame_byte_len = ((bitrate_kbps as usize) * coeff) / (sample_rate_hz as usize) + pad_bytes;
    if frame_byte_len < 4 {
        return Err(AudioBridgeError::InvalidMp3Frame {
            reason: "computed MP3 frame size smaller than 4-byte header",
        });
    }

    Ok(Mp3FrameHeader {
        version,
        has_crc,
        bitrate_kbps,
        sample_rate_hz,
        padding,
        channel_mode,
        frame_byte_len,
        samples_per_frame,
    })
}

/// Extracts `(seek_samples, mp3_frame_bytes)` from a `DefineSound` MP3 payload.
pub fn extract_define_sound_mp3_info(payload: &[u8]) -> Result<(i16, &[u8]), AudioBridgeError> {
    if payload.len() < 2 {
        return Err(AudioBridgeError::UnexpectedEof);
    }
    let seek_samples = i16::from_le_bytes([payload[0], payload[1]]);
    Ok((seek_samples, &payload[2..]))
}

/// Zero-allocation bounded iterator over MP3 frames in an SWF MP3 payload.
#[derive(Debug, Clone)]
pub struct Mp3FrameIter<'a> {
    data: &'a [u8],
    offset: usize,
    frames_yielded: usize,
    failed: bool,
}

impl<'a> Mp3FrameIter<'a> {
    /// Creates an iterator positioned at the first MP3 frame of `data`.
    #[inline]
    pub const fn mp3_iter_from_bytes(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            frames_yielded: 0,
            failed: false,
        }
    }

    /// Returns the number of unparsed bytes remaining.
    #[inline]
    pub const fn mp3_iter_remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }
}

impl<'a> Iterator for Mp3FrameIter<'a> {
    type Item = Result<Mp3Frame<'a>, AudioBridgeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.offset >= self.data.len() {
            return None;
        }
        if self.frames_yielded >= MAX_MP3_FRAMES {
            self.failed = true;
            return Some(Err(AudioBridgeError::Mp3FrameLimitExceeded {
                max: MAX_MP3_FRAMES,
            }));
        }

        let remaining = &self.data[self.offset..];
        let header = match parse_mp3_frame_header(remaining) {
            Ok(h) => h,
            Err(e) => {
                self.failed = true;
                return Some(Err(e));
            }
        };

        let end = match self.offset.checked_add(header.frame_byte_len) {
            Some(e) if e <= self.data.len() => e,
            _ => {
                self.failed = true;
                return Some(Err(AudioBridgeError::UnexpectedEof));
            }
        };

        let bytes = &self.data[self.offset..end];
        self.offset = end;
        self.frames_yielded += 1;

        Some(Ok(Mp3Frame { header, bytes }))
    }
}
