//! Secure RIFF/WAVE serializer and deserializer for uncompressed PCM audio.
//!
//! # Architecture & Security Specifications
//!
//! - **CWE-400 (Allocation Bombs)**: A hostile WAV header can declare a `data` chunk
//!   size of 4 GB (`0xFFFF_FFFF`) while supplying only 44 bytes of actual data. The
//!   parser **strictly verifies** that declared chunk sizes do not exceed actual
//!   slice lengths before reserving memory, and caps total sample counts at [`MAX_SAMPLES`].
//! - **CWE-190 (Integer Overflow)**: All buffer sizing and byte arithmetic use
//!   checked additions and multiplications.
//! - **CWE-20 (Improper Input Validation)**: Headers are thoroughly validated:
//!   `RIFF` and `WAVE` magic bytes, `fmt ` sub-chunk dimensions, valid channel counts,
//!   Nyquist/sample-rate constraints, and block-alignment divisibility.

use crate::format::{
    PcmError, PcmFormat, MAX_CHANNELS, MAX_SAMPLES, MAX_SAMPLE_RATE_HZ, MAX_WAV_BYTES,
    MIN_WAV_HEADER_SIZE,
};
use crate::pcm::{safe_round_sample_f32, PcmAudio};

const RIFF_HEADER_SIZE: usize = 12;
const MIN_FMT_CHUNK_SIZE: usize = 16;

const FORMAT_PCM: u16 = 1;
const FORMAT_IEEE_FLOAT: u16 = 3;

/// Validated uncompressed PCM `data` chunk payload size in bytes (`MUS01` Stage 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataSize {
    /// Total byte length of the PCM `data` chunk payload.
    pub bytes: u64,
}

/// Validated canonical RIFF/WAVE total file size in bytes (`44 + data_size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavFileSize {
    /// Total byte length of the complete RIFF/WAVE container.
    pub bytes: u64,
}

/// Compute the checked WAV `data` chunk payload size in bytes (`sample_count * channels * bytes_per_sample`).
///
/// Enforces:
/// - **CWE-20**: `channels` in `1..=MAX_CHANNELS` (`1..=8`) and `bytes_per_sample` in `{2, 4}`.
/// - **CWE-400**: `sample_count <= MAX_SAMPLES` (`16_777_216`) and byte length `<= MAX_WAV_BYTES` (`64 MiB`).
/// - **CWE-190**: Checked `u64` multiplications throughout.
pub fn compute_data_size(
    sample_count: u64,
    channels: u16,
    bytes_per_sample: u16,
) -> Result<DataSize, PcmError> {
    if channels == 0 || channels > MAX_CHANNELS {
        return Err(PcmError::UnsupportedChannelCount { channels });
    }
    if bytes_per_sample != 2 && bytes_per_sample != 4 {
        return Err(PcmError::UnsupportedBitDepth {
            bit_depth: bytes_per_sample.saturating_mul(8),
        });
    }
    if sample_count > MAX_SAMPLES as u64 {
        return Err(PcmError::SampleCountTooLarge {
            count: usize::try_from(sample_count).unwrap_or(usize::MAX),
            max: MAX_SAMPLES,
        });
    }
    let per_frame = u64::from(channels)
        .checked_mul(u64::from(bytes_per_sample))
        .ok_or(PcmError::IntegerOverflow("WAV frame size"))?;
    let bytes = sample_count
        .checked_mul(per_frame)
        .ok_or(PcmError::IntegerOverflow("WAV data size"))?;
    if bytes > MAX_WAV_BYTES as u64 {
        return Err(PcmError::OversizedWav {
            size: usize::try_from(bytes).unwrap_or(usize::MAX),
            max: MAX_WAV_BYTES,
        });
    }
    Ok(DataSize { bytes })
}

/// Compute the checked total RIFF/WAVE file size (`44 + data_size`) in bytes.
///
/// Enforces checked `u64` addition (CWE-190) and caps the total container
/// size at [`MAX_WAV_BYTES`] (CWE-400).
pub fn compute_wav_file_size(data_size: u64) -> Result<WavFileSize, PcmError> {
    let bytes = data_size
        .checked_add(MIN_WAV_HEADER_SIZE as u64)
        .ok_or(PcmError::IntegerOverflow("WAV total file size"))?;
    if bytes > MAX_WAV_BYTES as u64 {
        return Err(PcmError::OversizedWav {
            size: usize::try_from(bytes).unwrap_or(usize::MAX),
            max: MAX_WAV_BYTES,
        });
    }
    Ok(WavFileSize { bytes })
}

/// Serialize a `PcmAudio` buffer into canonical 16-bit PCM RIFF/WAVE file bytes.
pub fn to_wav_bytes(audio: &PcmAudio) -> Result<Vec<u8>, PcmError> {
    let sample_rate = audio.format().integer_sample_rate()?;
    let channels = audio.format().channel_count();
    let bit_depth = 16u16;
    let bytes_per_sample = 2u16;

    let sample_count = audio.samples().len();
    if sample_count > MAX_SAMPLES {
        return Err(PcmError::SampleCountTooLarge {
            count: sample_count,
            max: MAX_SAMPLES,
        });
    }

    let data_len = sample_count
        .checked_mul(bytes_per_sample as usize)
        .ok_or(PcmError::IntegerOverflow("WAV data length"))?;

    let total_len_u64 = compute_wav_file_size(data_len as u64)?.bytes;
    let total_len = usize::try_from(total_len_u64)
        .map_err(|_| PcmError::IntegerOverflow("WAV total length"))?;

    let riff_size = (total_len - 8) as u32;
    let block_align = channels
        .checked_mul(bytes_per_sample)
        .ok_or(PcmError::IntegerOverflow("block align"))?;
    let byte_rate = (sample_rate as u64)
        .checked_mul(block_align as u64)
        .ok_or(PcmError::IntegerOverflow("byte rate"))? as u32;

    // Checked capacity pre-allocation
    let mut out = Vec::with_capacity(total_len);

    // 1. RIFF Header
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // 2. 'fmt ' Chunk
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(MIN_FMT_CHUNK_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&FORMAT_PCM.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bit_depth.to_le_bytes());

    // 3. 'data' Chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &sample in audio.samples() {
        out.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(out)
}

/// Parse raw RIFF/WAVE file bytes into a `PcmAudio` buffer.
///
/// Supports 16-bit signed integer PCM and 32-bit IEEE floating-point PCM.
pub fn from_wav_bytes(bytes: &[u8]) -> Result<PcmAudio, PcmError> {
    if bytes.len() > MAX_WAV_BYTES {
        return Err(PcmError::OversizedWav {
            size: bytes.len(),
            max: MAX_WAV_BYTES,
        });
    }
    if bytes.len() < MIN_WAV_HEADER_SIZE {
        return Err(PcmError::MalformedWav(format!(
            "input length {} is less than minimum WAV header size {}",
            bytes.len(),
            MIN_WAV_HEADER_SIZE
        )));
    }

    // Magic checks
    if &bytes[0..4] != b"RIFF" {
        return Err(PcmError::MalformedWav("missing 'RIFF' chunk header".into()));
    }
    if &bytes[8..12] != b"WAVE" {
        return Err(PcmError::MalformedWav("missing 'WAVE' format marker".into()));
    }

    let declared_riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if declared_riff_size < 4 {
        return Err(PcmError::MalformedWav("RIFF chunk size too small".into()));
    }

    // Chunk iteration
    let mut offset = RIFF_HEADER_SIZE;
    let mut fmt_parsed = false;
    let mut audio_format = 0u16;
    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut block_align = 0u16;
    let mut bits_per_sample = 0u16;

    while offset.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;

        let chunk_data_start = offset + 8;
        let chunk_data_end = chunk_data_start
            .checked_add(chunk_size)
            .ok_or(PcmError::IntegerOverflow("chunk offset"))?;

        // CWE-400 guard: chunk must physically fit within the provided slice
        if chunk_data_end > bytes.len() {
            return Err(PcmError::MalformedWav(format!(
                "chunk '{:?}' declared size {} exceeds available buffer",
                std::str::from_utf8(chunk_id).unwrap_or("????"),
                chunk_size
            )));
        }

        if chunk_id == b"fmt " {
            if chunk_size < MIN_FMT_CHUNK_SIZE {
                return Err(PcmError::MalformedWav("fmt chunk too small".into()));
            }

            audio_format = u16::from_le_bytes([bytes[chunk_data_start], bytes[chunk_data_start + 1]]);
            channels = u16::from_le_bytes([bytes[chunk_data_start + 2], bytes[chunk_data_start + 3]]);
            sample_rate = u32::from_le_bytes([
                bytes[chunk_data_start + 4],
                bytes[chunk_data_start + 5],
                bytes[chunk_data_start + 6],
                bytes[chunk_data_start + 7],
            ]);
            let byte_rate = u32::from_le_bytes([
                bytes[chunk_data_start + 8],
                bytes[chunk_data_start + 9],
                bytes[chunk_data_start + 10],
                bytes[chunk_data_start + 11],
            ]);
            block_align = u16::from_le_bytes([
                bytes[chunk_data_start + 12],
                bytes[chunk_data_start + 13],
            ]);
            bits_per_sample = u16::from_le_bytes([
                bytes[chunk_data_start + 14],
                bytes[chunk_data_start + 15],
            ]);

            if audio_format != FORMAT_PCM && audio_format != FORMAT_IEEE_FLOAT {
                return Err(PcmError::UnsupportedFormat {
                    format_code: audio_format,
                });
            }

            if channels == 0 || channels > MAX_CHANNELS {
                return Err(PcmError::UnsupportedChannelCount { channels });
            }

            if sample_rate == 0 || (sample_rate as f64) > MAX_SAMPLE_RATE_HZ {
                return Err(PcmError::SampleRateTooHigh {
                    rate: sample_rate as f64,
                    max: MAX_SAMPLE_RATE_HZ,
                });
            }

            if audio_format == FORMAT_PCM && bits_per_sample != 16 {
                return Err(PcmError::UnsupportedBitDepth {
                    bit_depth: bits_per_sample,
                });
            }
            if audio_format == FORMAT_IEEE_FLOAT && bits_per_sample != 32 {
                return Err(PcmError::UnsupportedBitDepth {
                    bit_depth: bits_per_sample,
                });
            }

            let expected_block_align = channels
                .checked_mul(bits_per_sample / 8)
                .ok_or(PcmError::IntegerOverflow("block align validation"))?;
            if block_align != expected_block_align {
                return Err(PcmError::MalformedWav(format!(
                    "inconsistent block align: declared {block_align}, expected {expected_block_align}"
                )));
            }

            let expected_byte_rate = (sample_rate as u64)
                .checked_mul(expected_block_align as u64)
                .ok_or(PcmError::IntegerOverflow("byte rate validation"))? as u32;
            if byte_rate != expected_byte_rate {
                return Err(PcmError::MalformedWav(format!(
                    "inconsistent byte rate: declared {byte_rate}, expected {expected_byte_rate}"
                )));
            }

            fmt_parsed = true;
        } else if chunk_id == b"data" {
            if !fmt_parsed {
                return Err(PcmError::MalformedWav(
                    "data chunk encountered before fmt chunk".into(),
                ));
            }

            if !chunk_size.is_multiple_of(block_align as usize) {
                return Err(PcmError::MalformedWav(
                    "data chunk size is not an integer multiple of block align".into(),
                ));
            }

            let bytes_per_sample_val = (bits_per_sample / 8) as usize;
            let sample_count = chunk_size / bytes_per_sample_val;

            if sample_count > MAX_SAMPLES {
                return Err(PcmError::SampleCountTooLarge {
                    count: sample_count,
                    max: MAX_SAMPLES,
                });
            }

            let data_bytes = &bytes[chunk_data_start..chunk_data_end];
            let format = PcmFormat::new(
                sample_rate as f64,
                channels,
                16, // PcmAudio represents 16-bit PCM samples internally
                crate::format::DEFAULT_FULL_SCALE_VOLTAGE,
            )?;

            if audio_format == FORMAT_PCM {
                // 16-bit signed PCM
                let mut samples = Vec::with_capacity(sample_count);
                for chunk in data_bytes.as_chunks::<2>().0 {
                    let s = i16::from_le_bytes(*chunk);
                    samples.push(s);
                }
                return PcmAudio::from_i16(samples, format, 0.0);
            } else {
                // 32-bit IEEE float PCM -> quantize to 16-bit PCM
                let mut samples = Vec::with_capacity(sample_count);
                let mut clipped_count = 0;
                for chunk in data_bytes.as_chunks::<4>().0 {
                    let float_val = f32::from_le_bytes(*chunk);
                    let (pcm, clipped) = safe_round_sample_f32(float_val)?;
                    samples.push(pcm);
                    if clipped {
                        clipped_count += 1;
                    }
                }
                let mut audio = PcmAudio::from_i16(samples, format, 0.0)?;
                audio.clipped_sample_count = clipped_count;
                return Ok(audio);
            }
        }

        // RIFF chunk alignment padding to 2-byte boundary
        let pad = chunk_size % 2;
        offset = chunk_data_end
            .checked_add(pad)
            .ok_or(PcmError::IntegerOverflow("next chunk offset"))?;
    }

    Err(PcmError::MalformedWav("missing 'data' chunk in WAV file".into()))
}
