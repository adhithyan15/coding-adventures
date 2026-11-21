//! Deterministic, zero-allocation Flash event sound mixer, streaming FIFO, and clock sync (`SWF06`).
//!
//! Integrates with:
//! - [`audio_mixer_realtime`] (`MUS08`): [`PcmRingBuffer`], [`RealtimeMixerState`], [`clamp_sample`],
//!   [`UNITY_GAIN_Q16`], and [`MAX_GAIN_Q16`].
//! - [`game_clock`] (`GAME10`): [`GameClockConfig`], [`GameClockState`], [`advance_frame`].
//! - [`pcm_audio`] (`MUS01`/`MUS02`): [`pcm_audio::PcmAudio`] export bridge.
//! - [`audio_device_sink`] (`MUS02`): [`audio_device_sink::PcmPlaybackBuffer`] export bridge.
//!
//! ## Hot-Path Zero-Allocation Contract (`VER07`)
//!
//! Once event sounds are registered into [`FlashAudioState`], every runtime playback operation
//! operates with **zero heap allocations**:
//! - [`FlashAudioState::apply_start_sound`] / [`FlashAudioState::apply_start_sound2`]
//! - [`FlashAudioState::as_sound_start`] / [`FlashAudioState::as_stop_all_sounds`]
//! - [`FlashAudioState::push_stream_block`] / [`FlashAudioState::push_stream_pcm_into_fifo`]
//! - [`FlashAudioState::advance_frame_clock`]
//! - [`render_flash_audio_into`]
//! - [`bridge_render_to_ring_buffer`]

use audio_mixer_realtime::{
    clamp_sample, export_to_pcm_audio, export_to_sink_buffer, validate_sample_rate, PcmRingBuffer,
    RealtimeMixerState, MAX_GAIN_Q16, MAX_MIXER_CHANNELS, UNITY_GAIN_Q16,
};
use game_clock::{advance_frame, new_clock, GameClockConfig, GameClockState};

use crate::decoder::{
    decode_adpcm_into, decode_define_sound_into, decode_pcm16_le_into, decode_pcm16_ne_into,
    decode_pcm8_into, mix_stereo_to_mono_into,
};
use crate::error::AudioBridgeError;
use crate::tags::{
    DefineSoundTag, SoundEnvelopePoint, SoundInfo, SoundStreamBlockTag, SoundStreamHead,
    StartSound2Tag, StartSoundTag, SwfAudioChannels, SwfSampleWidth, SwfSoundFormat,
    MAX_ENVELOPE_POINTS, MAX_LOOP_COUNT, MAX_SOUND_SAMPLES, MAX_TAG_PAYLOAD_BYTES,
};

/// Maximum number of `DefineSound` event assets cached in the sound bank (CWE-400).
pub const MAX_EVENT_SOUNDS: usize = 32;

/// Maximum simultaneous active Flash event voices (matches `audio-mixer-realtime`).
pub const MAX_ACTIVE_VOICES: usize = MAX_MIXER_CHANNELS;

/// Fixed capacity in mono `i16` samples for the streaming sound FIFO ring buffer (CWE-400).
pub const STREAM_FIFO_CAPACITY: usize = 8_192;

/// Default output audio sampling rate in Hertz (44.1 kHz).
pub const DEFAULT_OUTPUT_SAMPLE_RATE_HZ: u32 = 44_100;

/// Default Flash movie frame rate in Hz used when initializing the audio sync clock.
pub const DEFAULT_FRAME_RATE_HZ: u32 = 24;

/// Stack scratch capacity for zero-alloc streaming block decoding.
const STREAM_SCRATCH_SAMPLES: usize = 4_096;

/// Pre-decoded event sound asset stored in [`FlashAudioState`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct EventSoundEntry {
    sound_id: u16,
    sample_rate_hz: u32,
    channels: SwfAudioChannels,
    sample_count_per_channel: u32,
    pcm_samples: Vec<i16>,
    class_name: Option<String>,
}

/// Active event voice slot (fixed-size `Copy`, zero heap allocation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FlashVoice {
    active: bool,
    sound_id: u16,
    bank_slot: usize,
    phase_q16: u64,
    step_q16: u32,
    start_frame: usize,
    end_frame: usize,
    remaining_loops: u16,
    envelope_points: [SoundEnvelopePoint; MAX_ENVELOPE_POINTS],
    envelope_len: usize,
    env_pos44_q16: u64,
    env_step44_q16: u64,
}

impl FlashVoice {
    const fn empty() -> Self {
        Self {
            active: false,
            sound_id: 0,
            bank_slot: 0,
            phase_q16: 0,
            step_q16: UNITY_GAIN_Q16,
            start_frame: 0,
            end_frame: 0,
            remaining_loops: 0,
            envelope_points: [SoundEnvelopePoint {
                pos44: 0,
                left_level: 32_768,
                right_level: 32_768,
            }; MAX_ENVELOPE_POINTS],
            envelope_len: 0,
            env_pos44_q16: 0,
            env_step44_q16: UNITY_GAIN_Q16 as u64,
        }
    }

    /// Evaluates the Q16.16 stereo-averaged envelope gain at `pos44` (CWE-369 safe).
    fn eval_envelope_q16(&self) -> u32 {
        if self.envelope_len == 0 {
            return UNITY_GAIN_Q16;
        }
        let pos44 = (self.env_pos44_q16 >> 16) as u32;
        let pts = &self.envelope_points[..self.envelope_len];

        let point_gain = |p: &SoundEnvelopePoint| -> u32 {
            // SWF level 0..=32768 maps to 0..=65536 (UNITY_GAIN_Q16) via * 2.
            (p.left_level as u32) + (p.right_level as u32)
        };

        if pos44 <= pts[0].pos44 {
            return point_gain(&pts[0]);
        }
        let last_idx = pts.len() - 1;
        if pos44 >= pts[last_idx].pos44 {
            return point_gain(&pts[last_idx]);
        }

        for idx in 0..last_idx {
            let a = &pts[idx];
            let b = &pts[idx + 1];
            if pos44 >= a.pos44 && pos44 <= b.pos44 {
                let span = b.pos44 - a.pos44;
                if span == 0 {
                    return point_gain(b);
                }
                let g_a = point_gain(a) as i64;
                let g_b = point_gain(b) as i64;
                let t = (pos44 - a.pos44) as i64;
                let interp = g_a + ((g_b - g_a) * t) / (span as i64);
                return interp.clamp(0, UNITY_GAIN_Q16 as i64) as u32;
            }
        }
        UNITY_GAIN_Q16
    }
}

/// Complete deterministic runtime state for Flash event & streaming audio synchronization.
#[derive(Debug, Clone)]
pub struct FlashAudioState {
    /// Target output PCM sample rate in Hertz (`1..=192_000`).
    output_sample_rate_hz: u32,
    /// Visual timeline frame rate in Hertz.
    frame_rate_hz: u32,
    /// Master output gain in Q16.16 fixed point (`0..=MAX_GAIN_Q16`).
    master_gain_q16: u32,
    /// Streaming track gain in Q16.16 fixed point (`0..=MAX_GAIN_Q16`).
    stream_gain_q16: u32,
    bank: [Option<EventSoundEntry>; MAX_EVENT_SOUNDS],
    voices: [FlashVoice; MAX_ACTIVE_VOICES],
    stream_fifo: PcmRingBuffer<STREAM_FIFO_CAPACITY>,
    stream_head: Option<SoundStreamHead>,
    mixer_telemetry: RealtimeMixerState,
    clock: GameClockState,
    samples_per_frame_q16: u64,
    frame_sample_accum_q16: u64,
}

impl FlashAudioState {
    /// Initializes a new Flash audio runtime engine with validated sample rate and frame rate.
    pub fn create_engine(
        output_sample_rate_hz: u32,
        frame_rate_hz: u32,
    ) -> Result<Self, AudioBridgeError> {
        validate_sample_rate(output_sample_rate_hz).map_err(|_| AudioBridgeError::MixerFailure)?;
        if frame_rate_hz == 0 || frame_rate_hz > 1_000 {
            return Err(AudioBridgeError::ClockFailure);
        }

        let mixer_telemetry = RealtimeMixerState::new(output_sample_rate_hz)
            .map_err(|_| AudioBridgeError::MixerFailure)?;
        let clock_cfg = GameClockConfig::from_hz(frame_rate_hz, 8)
            .map_err(|_| AudioBridgeError::ClockFailure)?;
        let clock = new_clock(clock_cfg).map_err(|_| AudioBridgeError::ClockFailure)?;

        let samples_per_frame_q16 = ((output_sample_rate_hz as u64) << 16) / (frame_rate_hz as u64);

        Ok(Self {
            output_sample_rate_hz,
            frame_rate_hz,
            master_gain_q16: UNITY_GAIN_Q16,
            stream_gain_q16: UNITY_GAIN_Q16,
            bank: std::array::from_fn(|_| None),
            voices: [FlashVoice::empty(); MAX_ACTIVE_VOICES],
            stream_fifo: PcmRingBuffer::empty(),
            stream_head: None,
            mixer_telemetry,
            clock,
            samples_per_frame_q16,
            frame_sample_accum_q16: 0,
        })
    }

    /// Returns the configured output PCM sample rate in Hertz.
    #[inline]
    pub const fn engine_output_sample_rate_hz(&self) -> u32 {
        self.output_sample_rate_hz
    }

    /// Returns the configured visual timeline frame rate in Hertz.
    #[inline]
    pub const fn engine_frame_rate_hz(&self) -> u32 {
        self.frame_rate_hz
    }

    /// Returns the master output gain in Q16.16 fixed-point.
    #[inline]
    pub const fn engine_master_gain_q16(&self) -> u32 {
        self.master_gain_q16
    }

    /// Returns the streaming track gain in Q16.16 fixed-point.
    #[inline]
    pub const fn engine_stream_gain_q16(&self) -> u32 {
        self.stream_gain_q16
    }

    /// Sets master output gain in Q16.16 fixed-point (clamped to `[0, MAX_GAIN_Q16]`).
    pub fn set_master_gain_q16(&mut self, gain_q16: u32) {
        self.master_gain_q16 = gain_q16.min(MAX_GAIN_Q16);
        self.mixer_telemetry.master_gain_q16 = self.master_gain_q16;
    }

    /// Sets streaming track gain in Q16.16 fixed-point (clamped to `[0, MAX_GAIN_Q16]`).
    pub fn set_stream_gain_q16(&mut self, gain_q16: u32) {
        self.stream_gain_q16 = gain_q16.min(MAX_GAIN_Q16);
    }

    /// Decodes a `DefineSound` (Tag 14) PCM or ADPCM asset and caches it in the event sound bank.
    ///
    /// Returns the assigned bank slot index. This is a setup-time operation; playback and
    /// mixing of registered sounds execute with zero heap allocations.
    pub fn register_define_sound(
        &mut self,
        tag: &DefineSoundTag<'_>,
    ) -> Result<usize, AudioBridgeError> {
        if tag.sound_id == 0 {
            return Err(AudioBridgeError::InvalidSoundId);
        }
        let total_scalars = tag
            .properties
            .props_total_scalar_samples(tag.sample_count)?;
        let mut pcm = vec![0i16; total_scalars];
        let written = decode_define_sound_into(tag, &mut pcm)?;
        pcm.truncate(written);

        self.insert_bank_entry(EventSoundEntry {
            sound_id: tag.sound_id,
            sample_rate_hz: tag.properties.rate.rate_to_hz(),
            channels: tag.properties.channels,
            sample_count_per_channel: tag.sample_count,
            pcm_samples: pcm,
            class_name: None,
        })
    }

    /// Registers an already-decoded PCM sound asset (with optional AS3 linkage class name).
    pub fn register_pcm_sound(
        &mut self,
        sound_id: u16,
        sample_rate_hz: u32,
        channels: SwfAudioChannels,
        samples: &[i16],
        class_name: Option<&str>,
    ) -> Result<usize, AudioBridgeError> {
        if sound_id == 0 {
            return Err(AudioBridgeError::InvalidSoundId);
        }
        validate_sample_rate(sample_rate_hz).map_err(|_| AudioBridgeError::MixerFailure)?;
        let ch = channels.channel_count();
        if !samples.len().is_multiple_of(ch) {
            return Err(AudioBridgeError::MalformedTagPayload {
                reason: "PCM sample count is not aligned to channel count",
            });
        }
        let per_ch = samples.len() / ch;
        if per_ch > MAX_SOUND_SAMPLES as usize {
            return Err(AudioBridgeError::SampleCountExceeded {
                count: per_ch as u32,
                max: MAX_SOUND_SAMPLES,
            });
        }

        self.insert_bank_entry(EventSoundEntry {
            sound_id,
            sample_rate_hz,
            channels,
            sample_count_per_channel: per_ch as u32,
            pcm_samples: samples.to_vec(),
            class_name: class_name.map(str::to_owned),
        })
    }

    fn insert_bank_entry(&mut self, entry: EventSoundEntry) -> Result<usize, AudioBridgeError> {
        // Replace existing slot if sound_id already registered.
        for (idx, slot) in self.bank.iter_mut().enumerate() {
            if let Some(existing) = slot {
                if existing.sound_id == entry.sound_id {
                    *slot = Some(entry);
                    return Ok(idx);
                }
            }
        }
        for (idx, slot) in self.bank.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(entry);
                return Ok(idx);
            }
        }
        Err(AudioBridgeError::SoundBankFull {
            max: MAX_EVENT_SOUNDS,
        })
    }

    fn find_bank_by_id(&self, sound_id: u16) -> Option<usize> {
        self.bank
            .iter()
            .position(|slot| matches!(slot, Some(e) if e.sound_id == sound_id))
    }

    fn find_bank_by_class(&self, class_name: &str) -> Option<usize> {
        self.bank
            .iter()
            .position(|slot| matches!(slot, Some(e) if e.class_name.as_deref() == Some(class_name)))
    }

    fn stop_voices_for_sound_id(&mut self, sound_id: u16) {
        for v in &mut self.voices {
            if v.active && v.sound_id == sound_id {
                v.active = false;
            }
        }
    }

    fn is_sound_playing(&self, sound_id: u16) -> bool {
        self.voices
            .iter()
            .any(|v| v.active && v.sound_id == sound_id)
    }

    fn start_bank_slot_with_info(
        &mut self,
        bank_slot: usize,
        info: &SoundInfo,
    ) -> Result<(), AudioBridgeError> {
        let entry = self.bank[bank_slot]
            .as_ref()
            .ok_or(AudioBridgeError::SoundNotFound { sound_id: 0 })?;
        let sound_id = entry.sound_id;

        if info.sync_stop {
            self.stop_voices_for_sound_id(sound_id);
            return Ok(());
        }
        if info.loop_count == 0 || info.loop_count > MAX_LOOP_COUNT {
            return Err(AudioBridgeError::InvalidLoopCount {
                count: info.loop_count,
            });
        }
        if let (Some(inp), Some(outp)) = (info.in_point, info.out_point) {
            if inp >= outp {
                return Err(AudioBridgeError::InvalidInOutPoints {
                    in_point: inp,
                    out_point: outp,
                });
            }
        }
        if info.sync_no_multiple && self.is_sound_playing(sound_id) {
            return Ok(());
        }

        let total_frames = entry.sample_count_per_channel as usize;
        if total_frames == 0 {
            return Ok(());
        }

        let src_rate = entry.sample_rate_hz as u64;
        let out_rate = self.output_sample_rate_hz as u64;

        // Convert 44.1 kHz in_point/out_point into source sample frame indices.
        // Clamp to total_frames in u64 before narrowing to usize (32-bit safety, CWE-190).
        let start_frame = match info.in_point {
            Some(p44) => (((p44 as u64) * src_rate) / 44_100).min(total_frames as u64) as usize,
            None => 0,
        };
        let end_frame = match info.out_point {
            Some(p44) => (((p44 as u64) * src_rate) / 44_100).min(total_frames as u64) as usize,
            None => total_frames,
        };
        if start_frame >= end_frame {
            return Ok(());
        }

        let step_q16 = ((src_rate << 16) / out_rate).clamp(1, u32::MAX as u64) as u32;
        let env_step44_q16 = ((44_100u64) << 16) / out_rate;

        let slot_idx = self
            .voices
            .iter()
            .position(|v| !v.active)
            .ok_or(AudioBridgeError::VoiceLimitExceeded)?;

        let mut env_arr = [SoundEnvelopePoint::default(); MAX_ENVELOPE_POINTS];
        let env_slice = info.info_envelope_slice();
        let env_len = env_slice.len().min(MAX_ENVELOPE_POINTS);
        env_arr[..env_len].copy_from_slice(&env_slice[..env_len]);

        self.voices[slot_idx] = FlashVoice {
            active: true,
            sound_id,
            bank_slot,
            phase_q16: (start_frame as u64) << 16,
            step_q16,
            start_frame,
            end_frame,
            remaining_loops: info.loop_count,
            envelope_points: env_arr,
            envelope_len: env_len,
            env_pos44_q16: ((info.in_point.unwrap_or(0) as u64) << 16),
            env_step44_q16,
        };

        Ok(())
    }

    /// Applies a `StartSound` (Tag 15) command with zero heap allocations.
    pub fn apply_start_sound(&mut self, tag: &StartSoundTag) -> Result<(), AudioBridgeError> {
        if tag.sound_id == 0 {
            return Err(AudioBridgeError::InvalidSoundId);
        }
        if tag.info.sync_stop {
            self.stop_voices_for_sound_id(tag.sound_id);
            return Ok(());
        }
        let bank_slot =
            self.find_bank_by_id(tag.sound_id)
                .ok_or(AudioBridgeError::SoundNotFound {
                    sound_id: tag.sound_id,
                })?;
        self.start_bank_slot_with_info(bank_slot, &tag.info)
    }

    /// Applies a `StartSound2` (Tag 89) command targeting an AS3 linkage class name (zero-alloc).
    pub fn apply_start_sound2(&mut self, tag: &StartSound2Tag<'_>) -> Result<(), AudioBridgeError> {
        let bank_slot = self
            .find_bank_by_class(tag.class_name)
            .ok_or(AudioBridgeError::SoundNotFound { sound_id: 0 })?;
        self.start_bank_slot_with_info(bank_slot, &tag.info)
    }

    /// ActionScript `Sound.start(secondOffsetMs, loops)` trigger with zero heap allocations.
    pub fn as_sound_start(
        &mut self,
        sound_id: u16,
        second_offset_ms: u32,
        loops: u16,
    ) -> Result<usize, AudioBridgeError> {
        if sound_id == 0 {
            return Err(AudioBridgeError::InvalidSoundId);
        }
        if loops > MAX_LOOP_COUNT {
            return Err(AudioBridgeError::InvalidLoopCount { count: loops });
        }
        let bank_slot = self
            .find_bank_by_id(sound_id)
            .ok_or(AudioBridgeError::SoundNotFound { sound_id })?;
        let entry = self.bank[bank_slot]
            .as_ref()
            .ok_or(AudioBridgeError::SoundNotFound { sound_id })?;

        let in_point_44 = ((second_offset_ms as u64) * 44_100) / 1_000;
        let mut info = SoundInfo::default();
        info.in_point = if second_offset_ms > 0 {
            Some(in_point_44.min(u32::MAX as u64) as u32)
        } else {
            None
        };
        info.loop_count = loops.max(1);

        let _ = entry;
        self.start_bank_slot_with_info(bank_slot, &info)?;
        Ok(self.active_voice_count())
    }

    /// ActionScript `stopAllSounds()` — immediately silences all event voices and clears the stream FIFO.
    pub fn as_stop_all_sounds(&mut self) {
        for v in &mut self.voices {
            v.active = false;
        }
        self.stream_fifo.clear();
    }

    /// Configures the active timeline streaming audio format from `SoundStreamHead*`.
    pub fn configure_sound_stream(
        &mut self,
        head: SoundStreamHead,
    ) -> Result<(), AudioBridgeError> {
        self.stream_head = Some(head);
        self.stream_fifo.clear();
        Ok(())
    }

    /// Decodes a `SoundStreamBlock` (Tag 19) directly into the streaming FIFO without heap allocation.
    ///
    /// Supports uncompressed 8-bit/16-bit PCM and Flash ADPCM streams. MP3 stream blocks require
    /// external frame decoding via `Mp3FrameIter` and return `UnsupportedSoundFormat { code: 2 }`.
    pub fn push_stream_block(
        &mut self,
        block: &SoundStreamBlockTag<'_>,
    ) -> Result<usize, AudioBridgeError> {
        if block.audio_data.len() > MAX_TAG_PAYLOAD_BYTES {
            return Err(AudioBridgeError::MalformedTagPayload {
                reason: "SoundStreamBlock payload exceeds maximum tag size",
            });
        }
        let head = self
            .stream_head
            .ok_or(AudioBridgeError::StreamFormatMismatch)?;
        let props = head.stream_properties;

        if props.format == SwfSoundFormat::Mp3 {
            return Err(AudioBridgeError::UnsupportedSoundFormat { code: 2 });
        }

        let mut scratch = [0i16; STREAM_SCRATCH_SAMPLES];
        let mut mono_scratch = [0i16; STREAM_SCRATCH_SAMPLES / 2];
        let mut total_pushed = 0usize;

        match props.format {
            SwfSoundFormat::UncompressedNe | SwfSoundFormat::UncompressedLe => {
                let bytes_per_sample = props.width.width_bytes();
                let ch_count = props.channels.channel_count();
                let frame_bytes = bytes_per_sample * ch_count;
                if frame_bytes == 0 || !block.audio_data.len().is_multiple_of(frame_bytes) {
                    return Err(AudioBridgeError::MalformedTagPayload {
                        reason: "stream PCM block size is not frame-aligned",
                    });
                }

                let max_chunk_bytes = (STREAM_SCRATCH_SAMPLES / ch_count) * frame_bytes;
                let mut offset = 0usize;
                while offset < block.audio_data.len() {
                    let end = (offset + max_chunk_bytes).min(block.audio_data.len());
                    let chunk = &block.audio_data[offset..end];
                    let scalar_written = match props.width {
                        SwfSampleWidth::Bits8 => decode_pcm8_into(chunk, &mut scratch)?,
                        SwfSampleWidth::Bits16 => {
                            if props.format == SwfSoundFormat::UncompressedLe {
                                decode_pcm16_le_into(chunk, &mut scratch)?
                            } else {
                                decode_pcm16_ne_into(chunk, &mut scratch)?
                            }
                        }
                    };
                    let pushed = if props.channels == SwfAudioChannels::Stereo {
                        let mono_len =
                            mix_stereo_to_mono_into(&scratch[..scalar_written], &mut mono_scratch)?;
                        self.push_stream_pcm_into_fifo(&mono_scratch[..mono_len])?
                    } else {
                        self.push_stream_pcm_into_fifo(&scratch[..scalar_written])?
                    };
                    total_pushed += pushed;
                    offset = end;
                }
            }
            SwfSoundFormat::Adpcm => {
                let ch_count = props.channels.channel_count();
                let est_samples = estimate_adpcm_stream_samples(
                    block.audio_data,
                    props.channels,
                    head.avg_sample_count as u32,
                )?;
                if est_samples == 0 {
                    return Ok(0);
                }
                let needed_scalars = (est_samples as usize).checked_mul(ch_count).ok_or(
                    AudioBridgeError::MalformedTagPayload {
                        reason: "stream ADPCM sample count overflow",
                    },
                )?;
                if needed_scalars > STREAM_SCRATCH_SAMPLES {
                    return Err(AudioBridgeError::SampleCountExceeded {
                        count: est_samples,
                        max: (STREAM_SCRATCH_SAMPLES / ch_count) as u32,
                    });
                }
                let scalar_written =
                    decode_adpcm_into(block.audio_data, props.channels, est_samples, &mut scratch)?;
                if props.channels == SwfAudioChannels::Stereo {
                    let mono_len =
                        mix_stereo_to_mono_into(&scratch[..scalar_written], &mut mono_scratch)?;
                    total_pushed = self.push_stream_pcm_into_fifo(&mono_scratch[..mono_len])?;
                } else {
                    total_pushed = self.push_stream_pcm_into_fifo(&scratch[..scalar_written])?;
                }
            }
            SwfSoundFormat::Mp3 => {
                return Err(AudioBridgeError::UnsupportedSoundFormat { code: 2 });
            }
        }

        Ok(total_pushed)
    }

    /// Pushes mono `i16` samples directly into the streaming FIFO [`PcmRingBuffer`] (zero-alloc).
    pub fn push_stream_pcm_into_fifo(
        &mut self,
        samples: &[i16],
    ) -> Result<usize, AudioBridgeError> {
        self.stream_fifo
            .push_slice(samples)
            .map_err(|_| AudioBridgeError::RingBufferOverrun)
    }

    /// Advances the `game-clock` fixed-timestep accumulator and returns the exact number of
    /// mono output audio samples corresponding to elapsed simulation frames (zero-alloc).
    pub fn advance_frame_clock(
        &mut self,
        raw_frame_delta_ns: u64,
    ) -> Result<usize, AudioBridgeError> {
        let step = advance_frame(&self.clock, raw_frame_delta_ns)
            .map_err(|_| AudioBridgeError::ClockFailure)?;
        self.clock = step.state;

        let added_q16 = (step.steps_to_run as u64)
            .checked_mul(self.samples_per_frame_q16)
            .ok_or(AudioBridgeError::ClockFailure)?;
        self.frame_sample_accum_q16 = self
            .frame_sample_accum_q16
            .checked_add(added_q16)
            .ok_or(AudioBridgeError::ClockFailure)?;

        let whole_samples = (self.frame_sample_accum_q16 >> 16) as usize;
        self.frame_sample_accum_q16 &= 0xFFFF;
        Ok(whole_samples)
    }

    /// Returns the number of currently active event voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Returns the number of buffered samples currently queued in the streaming FIFO.
    pub const fn stream_fifo_len(&self) -> usize {
        self.stream_fifo.len()
    }

    /// Returns total clipped samples observed across all `render_flash_audio_into` calls.
    pub const fn engine_clipped_samples(&self) -> u64 {
        self.mixer_telemetry.clipped_samples
    }

    /// Returns total mixed output frames rendered by this engine.
    pub const fn engine_mixed_frames(&self) -> u64 {
        self.mixer_telemetry.mixed_frames
    }
}

/// Estimates the per-channel sample count encoded in an ADPCM `SoundStreamBlock` payload.
fn estimate_adpcm_stream_samples(
    payload: &[u8],
    channels: SwfAudioChannels,
    avg_hint: u32,
) -> Result<u32, AudioBridgeError> {
    if payload.is_empty() {
        return Ok(0);
    }
    let bits_per_sample = ((payload[0] >> 6) & 0x03) as usize + 2;
    let total_bits = payload.len().saturating_mul(8);
    let ch = channels.channel_count();
    let header_bits = 2 + 22 * ch;
    if total_bits < header_bits {
        return Err(AudioBridgeError::UnexpectedEof);
    }
    let code_bits = total_bits - header_bits;
    let bits_per_frame = bits_per_sample * ch;
    let max_in_payload = 1usize
        .saturating_add(code_bits / bits_per_frame)
        .min(u32::MAX as usize) as u32;
    if avg_hint > 0 {
        Ok(avg_hint.min(max_in_payload))
    } else {
        Ok(max_in_payload)
    }
}

/// Renders `out_i16.len()` mono PCM samples from active Flash event voices and the streaming FIFO.
///
/// Strictly zero heap allocation (`max_alloc_bytes == 0`), deterministic Q16.16 fixed-point
/// resampling and mixing, with `i64`/`i128` accumulation and CWE-190 saturation clamping.
pub fn render_flash_audio_into(
    state: &mut FlashAudioState,
    out_i16: &mut [i16],
) -> Result<usize, AudioBridgeError> {
    if out_i16.is_empty() {
        return Ok(0);
    }

    for out_sample in out_i16.iter_mut() {
        let mut acc: i64 = 0;

        // 1. Mix all active event voices
        for voice in &mut state.voices {
            if !voice.active {
                continue;
            }
            let Some(entry) = state.bank.get(voice.bank_slot).and_then(|s| s.as_ref()) else {
                voice.active = false;
                continue;
            };

            let mut frame_idx = (voice.phase_q16 >> 16) as usize;
            if frame_idx >= voice.end_frame {
                if voice.remaining_loops > 1 {
                    voice.remaining_loops -= 1;
                    voice.phase_q16 = (voice.start_frame as u64) << 16;
                    frame_idx = voice.start_frame;
                } else {
                    voice.active = false;
                    continue;
                }
            }

            let raw_mono: i64 = match entry.channels {
                SwfAudioChannels::Mono => {
                    if frame_idx >= entry.pcm_samples.len() {
                        voice.active = false;
                        continue;
                    }
                    entry.pcm_samples[frame_idx] as i64
                }
                SwfAudioChannels::Stereo => {
                    let base = frame_idx.saturating_mul(2);
                    if base + 1 >= entry.pcm_samples.len() {
                        voice.active = false;
                        continue;
                    }
                    let l = entry.pcm_samples[base] as i64;
                    let r = entry.pcm_samples[base + 1] as i64;
                    (l + r) >> 1
                }
            };

            let env_gain_q16 = voice.eval_envelope_q16() as i64;
            acc += (raw_mono * env_gain_q16) >> 16;

            voice.phase_q16 = voice.phase_q16.saturating_add(voice.step_q16 as u64);
            voice.env_pos44_q16 = voice.env_pos44_q16.saturating_add(voice.env_step44_q16);
            if voice.remaining_loops <= 1 && ((voice.phase_q16 >> 16) as usize) >= voice.end_frame {
                voice.active = false;
            }
        }

        // 2. Mix streaming audio FIFO sample if queued
        if let Some(stream_sample) = state.stream_fifo.pop() {
            acc += ((stream_sample as i64) * (state.stream_gain_q16 as i64)) >> 16;
        }

        // 3. Apply master gain in i128 and saturate (CWE-190 guard)
        let scaled = ((acc as i128) * (state.master_gain_q16 as i128)) >> 16;
        let clamped = clamp_sample(scaled.clamp(i64::MIN as i128, i64::MAX as i128) as i64);
        if scaled > i16::MAX as i128 || scaled < i16::MIN as i128 {
            state.mixer_telemetry.clipped_samples =
                state.mixer_telemetry.clipped_samples.saturating_add(1);
        }
        *out_sample = clamped;
        state.mixer_telemetry.mixed_frames = state.mixer_telemetry.mixed_frames.saturating_add(1);
    }

    Ok(out_i16.len())
}

/// Zero-allocation bridge: renders `frame_samples` directly into an external [`PcmRingBuffer`].
pub fn bridge_render_to_ring_buffer<const N: usize>(
    state: &mut FlashAudioState,
    ring: &mut PcmRingBuffer<N>,
    frame_samples: usize,
) -> Result<usize, AudioBridgeError> {
    if frame_samples == 0 {
        return Ok(0);
    }
    let mut chunk = [0i16; 256];
    let mut remaining = frame_samples;
    let mut total_written = 0usize;

    while remaining > 0 {
        let take = remaining.min(chunk.len());
        let rendered = render_flash_audio_into(state, &mut chunk[..take])?;
        ring.push_slice(&chunk[..rendered])
            .map_err(|_| AudioBridgeError::RingBufferOverrun)?;
        total_written += rendered;
        remaining -= take;
    }

    Ok(total_written)
}

/// Offline bridge: renders `sample_count` mono samples into a [`pcm_audio::PcmAudio`] container.
pub fn export_flash_audio_to_pcm_audio(
    state: &mut FlashAudioState,
    sample_count: usize,
) -> Result<pcm_audio::PcmAudio, AudioBridgeError> {
    if sample_count > MAX_SOUND_SAMPLES as usize {
        return Err(AudioBridgeError::SampleCountExceeded {
            count: sample_count as u32,
            max: MAX_SOUND_SAMPLES,
        });
    }
    let mut buf = vec![0i16; sample_count];
    render_flash_audio_into(state, &mut buf)?;
    export_to_pcm_audio(&buf, state.output_sample_rate_hz)
        .map_err(|_| AudioBridgeError::PcmExportFailure)
}

/// Offline bridge: renders `sample_count` mono samples into an [`audio_device_sink::PcmPlaybackBuffer`].
pub fn export_flash_audio_to_sink_buffer(
    state: &mut FlashAudioState,
    sample_count: usize,
) -> Result<audio_device_sink::PcmPlaybackBuffer, AudioBridgeError> {
    if sample_count > MAX_SOUND_SAMPLES as usize {
        return Err(AudioBridgeError::SampleCountExceeded {
            count: sample_count as u32,
            max: MAX_SOUND_SAMPLES,
        });
    }
    let mut buf = vec![0i16; sample_count];
    render_flash_audio_into(state, &mut buf)?;
    export_to_sink_buffer(&buf, state.output_sample_rate_hz)
        .map_err(|_| AudioBridgeError::PcmExportFailure)
}
