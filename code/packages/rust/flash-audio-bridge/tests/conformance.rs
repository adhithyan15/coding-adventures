//! Conformance test suite for `flash-audio-bridge` (`SWF06`).
//!
//! Covers:
//! - Top-level warrant validation primitives
//! - `DefineSound` (Tag 14), `StartSound` (Tag 15), `StartSound2` (Tag 89),
//!   `SoundStreamHead` (Tag 18), `SoundStreamHead2` (Tag 45), `SoundStreamBlock` (Tag 19)
//! - End-to-end `swf_container::SwfTagIter` integration
//! - Uncompressed PCM (8-bit unsigned, 16-bit LE/NE, stereo downmix)
//! - Flash ADPCM (2-bit, 3-bit, 4-bit, 5-bit mono & stereo, multi-packet > 4096 samples)
//! - MP3 frame header validation & bounded frame extraction
//! - Event sound bank, active voice mixer, `Sound.start()`, `stopAllSounds()`,
//!   envelopes, looping, resampling, streaming FIFO, `game-clock` sync,
//!   saturation clamping, `PcmRingBuffer`, `pcm-audio`, and `audio-device-sink` bridges.

use flash_audio_bridge::{
    bridge_render_to_ring_buffer, decode_adpcm_into, decode_define_sound_into,
    decode_pcm16_le_into, decode_pcm16_ne_into, decode_pcm8_into, export_flash_audio_to_pcm_audio,
    export_flash_audio_to_sink_buffer, extract_define_sound_mp3_info, mix_stereo_to_mono_into,
    parse_define_sound, parse_mp3_frame_header, parse_sound_info, parse_sound_stream_block,
    parse_sound_stream_head, parse_start_sound, parse_start_sound2, parse_swf_audio_tag,
    render_flash_audio_into, validate_adpcm_bit_width, validate_envelope_point_count,
    validate_sound_rate_code, validate_sound_sample_count, AudioBridgeError, FlashAudioState,
    Mp3ChannelMode, Mp3FrameIter, MpegVersion, PcmRingBuffer, PureCapabilities, SwfAudioChannels,
    SwfAudioTag, SwfSampleWidth, SwfSoundFormat, SwfSoundRate, MAX_ACTIVE_VOICES,
    MAX_ENVELOPE_POINTS, MAX_LOOP_COUNT, MAX_SOUND_SAMPLES, SWF_TAG_START_SOUND2,
};
use swf_container::{SwfTagIter, SwfTagKind};

struct BitWriter {
    bytes: Vec<u8>,
    cur: u8,
    bit_pos: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            cur: 0,
            bit_pos: 0,
        }
    }

    fn write_ub(&mut self, n: usize, val: u32) {
        for i in (0..n).rev() {
            let bit = ((val >> i) & 1) as u8;
            self.cur = (self.cur << 1) | bit;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bytes.push(self.cur);
                self.cur = 0;
                self.bit_pos = 0;
            }
        }
    }

    fn write_sb(&mut self, n: usize, val: i32) {
        let mask = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
        self.write_ub(n, (val as u32) & mask);
    }

    fn into_bytes(mut self) -> Vec<u8> {
        if self.bit_pos > 0 {
            self.cur <<= 8 - self.bit_pos;
            self.bytes.push(self.cur);
        }
        self.bytes
    }
}

fn make_swf_tag(code: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if payload.len() < 0x3F {
        let hdr = (code << 6) | (payload.len() as u16);
        out.extend_from_slice(&hdr.to_le_bytes());
    } else {
        let hdr = (code << 6) | 0x3F;
        out.extend_from_slice(&hdr.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(payload);
    out
}

fn sound_flags_byte(
    format: SwfSoundFormat,
    rate: SwfSoundRate,
    width: SwfSampleWidth,
    channels: SwfAudioChannels,
) -> u8 {
    let fmt = format.format_to_code() & 0x0F;
    let r = rate.rate_to_code() & 0x03;
    let sz = if width == SwfSampleWidth::Bits16 {
        1
    } else {
        0
    };
    let ch = if channels == SwfAudioChannels::Stereo {
        1
    } else {
        0
    };
    (fmt << 4) | (r << 2) | (sz << 1) | ch
}

/// Synthetic Flash ADPCM encoder for conformance round-trip verification.
fn encode_flash_adpcm(bits_per_sample: usize, channels: usize, samples: &[i16]) -> Vec<u8> {
    const STEP_TABLE: [i32; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60,
        66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371,
        408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878,
        2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845,
        8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086,
        29794, 32767,
    ];
    const IDX2: [i32; 2] = [-1, 2];
    const IDX3: [i32; 4] = [-1, -1, 2, 4];
    const IDX4: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
    const IDX5: [i32; 16] = [-1, -1, -1, -1, -1, -1, -1, -1, 1, 2, 4, 6, 8, 10, 13, 16];

    let mut bw = BitWriter::new();
    bw.write_ub(2, (bits_per_sample - 2) as u32);
    let total_frames = samples.len() / channels;
    let mut frame = 0usize;

    while frame < total_frames {
        let block_frames = (total_frames - frame).min(4096);
        let mut pred = [0i32; 2];
        let mut idx = [0usize; 2];

        for c in 0..channels {
            pred[c] = samples[frame * channels + c] as i32;
            idx[c] = 10;
            bw.write_sb(16, pred[c]);
            bw.write_ub(6, idx[c] as u32);
        }

        for f in 1..block_frames {
            for c in 0..channels {
                let target = samples[(frame + f) * channels + c] as i32;
                let step = STEP_TABLE[idx[c]];
                let mut diff = target - pred[c];
                let sign_bit = 1u32 << (bits_per_sample - 1);
                let mut code = 0u32;
                if diff < 0 {
                    code |= sign_bit;
                    diff = -diff;
                }
                let mut delta = step >> (bits_per_sample - 1);
                for bit in (0..(bits_per_sample - 1)).rev() {
                    let contrib = step >> (bits_per_sample - 2 - bit);
                    if diff >= contrib {
                        code |= 1u32 << bit;
                        diff -= contrib;
                        delta += contrib;
                    }
                }
                if (code & sign_bit) != 0 {
                    pred[c] = (pred[c] - delta).clamp(-32768, 32767);
                } else {
                    pred[c] = (pred[c] + delta).clamp(-32768, 32767);
                }
                let mag = (code & (sign_bit - 1)) as usize;
                let delta_idx = match bits_per_sample {
                    2 => IDX2[mag],
                    3 => IDX3[mag],
                    4 => IDX4[mag],
                    5 => IDX5[mag],
                    _ => unreachable!(),
                };
                idx[c] = ((idx[c] as i32) + delta_idx).clamp(0, 88) as usize;
                bw.write_ub(bits_per_sample, code);
            }
        }
        frame += block_frames;
    }

    bw.into_bytes()
}

#[test]
fn test_warrant_validators_and_capabilities() {
    let _cap = PureCapabilities;
    assert_eq!(validate_sound_rate_code(0), Ok(true));
    assert_eq!(validate_sound_rate_code(3), Ok(true));
    assert_eq!(
        validate_sound_rate_code(4),
        Err(AudioBridgeError::InvalidSoundRateCode { code: 4 })
    );

    assert_eq!(validate_adpcm_bit_width(2), Ok(true));
    assert_eq!(validate_adpcm_bit_width(5), Ok(true));
    assert_eq!(
        validate_adpcm_bit_width(1),
        Err(AudioBridgeError::InvalidAdpcmBitWidth { bits: 1 })
    );
    assert_eq!(
        validate_adpcm_bit_width(6),
        Err(AudioBridgeError::InvalidAdpcmBitWidth { bits: 6 })
    );

    assert_eq!(validate_sound_sample_count(0), Ok(true));
    assert_eq!(
        validate_sound_sample_count(MAX_SOUND_SAMPLES as usize),
        Ok(true)
    );
    assert_eq!(
        validate_sound_sample_count(MAX_SOUND_SAMPLES as usize + 1),
        Err(AudioBridgeError::SampleCountExceeded {
            count: MAX_SOUND_SAMPLES + 1,
            max: MAX_SOUND_SAMPLES,
        })
    );

    assert_eq!(validate_envelope_point_count(0), Ok(true));
    assert_eq!(validate_envelope_point_count(MAX_ENVELOPE_POINTS), Ok(true));
    assert_eq!(
        validate_envelope_point_count(MAX_ENVELOPE_POINTS + 1),
        Err(AudioBridgeError::InvalidEnvelopeCount {
            count: MAX_ENVELOPE_POINTS + 1,
            max: MAX_ENVELOPE_POINTS,
        })
    );
}

#[test]
fn test_define_sound_parsing_and_bounds() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&42u16.to_le_bytes());
    payload.push(sound_flags_byte(
        SwfSoundFormat::UncompressedLe,
        SwfSoundRate::Rate44100,
        SwfSampleWidth::Bits16,
        SwfAudioChannels::Stereo,
    ));
    payload.extend_from_slice(&2u32.to_le_bytes()); // 2 stereo samples = 8 bytes
    for s in [1000i16, -1000, 2000, -2000] {
        payload.extend_from_slice(&s.to_le_bytes());
    }

    let tag = parse_define_sound(&payload).expect("valid DefineSound");
    assert_eq!(tag.sound_id, 42);
    assert_eq!(tag.properties.format, SwfSoundFormat::UncompressedLe);
    assert_eq!(tag.properties.rate.rate_to_hz(), 44_100);
    assert_eq!(tag.properties.width, SwfSampleWidth::Bits16);
    assert_eq!(tag.properties.channels, SwfAudioChannels::Stereo);
    assert_eq!(tag.define_sound_sample_count(), 2);
    assert_eq!(tag.define_sound_payload_len(), 8);

    // Sound ID 0 rejected (CWE-20)
    let mut bad_id = payload.clone();
    bad_id[0] = 0;
    bad_id[1] = 0;
    assert_eq!(
        parse_define_sound(&bad_id),
        Err(AudioBridgeError::InvalidSoundId)
    );

    // Unsupported format code 6 (Nellymoser) rejected
    let mut bad_fmt = payload.clone();
    bad_fmt[2] = 0x6F;
    assert_eq!(
        parse_define_sound(&bad_fmt),
        Err(AudioBridgeError::UnsupportedSoundFormat { code: 6 })
    );

    // Sample count exceeding MAX_SOUND_SAMPLES rejected (CWE-400)
    let mut huge_count = payload.clone();
    huge_count[3..7].copy_from_slice(&(MAX_SOUND_SAMPLES + 1).to_le_bytes());
    assert_eq!(
        parse_define_sound(&huge_count),
        Err(AudioBridgeError::SampleCountExceeded {
            count: MAX_SOUND_SAMPLES + 1,
            max: MAX_SOUND_SAMPLES
        })
    );

    // Truncated PCM payload rejected (CWE-125)
    assert_eq!(
        parse_define_sound(&payload[..payload.len() - 1]),
        Err(AudioBridgeError::UnexpectedEof)
    );
}

#[test]
fn test_start_sound_and_start_sound2_and_envelope_guards() {
    // StartSound with in_point=100, out_point=5000, loops=3, 2 envelope points
    let mut raw = Vec::new();
    raw.extend_from_slice(&7u16.to_le_bytes());
    raw.push(0x1F); // SyncNoMultiple | HasEnvelope | HasLoops | HasOutPoint | HasInPoint
    raw.extend_from_slice(&100u32.to_le_bytes()); // in_point
    raw.extend_from_slice(&5000u32.to_le_bytes()); // out_point
    raw.extend_from_slice(&3u16.to_le_bytes()); // loops
    raw.push(2); // 2 envelope points
    raw.extend_from_slice(&0u32.to_le_bytes());
    raw.extend_from_slice(&16384u16.to_le_bytes());
    raw.extend_from_slice(&16384u16.to_le_bytes());
    raw.extend_from_slice(&4410u32.to_le_bytes());
    raw.extend_from_slice(&32768u16.to_le_bytes());
    raw.extend_from_slice(&32768u16.to_le_bytes());

    let parsed = parse_start_sound(&raw).expect("valid StartSound");
    assert_eq!(parsed.start_sound_id(), 7);
    assert!(!parsed.info.sync_stop);
    assert!(parsed.info.sync_no_multiple);
    assert_eq!(parsed.info.in_point, Some(100));
    assert_eq!(parsed.info.out_point, Some(5000));
    assert_eq!(parsed.info.loop_count, 3);
    assert!(parsed.info.info_has_envelope());
    assert_eq!(parsed.info.info_envelope_slice().len(), 2);

    // StartSound2 with AS3 class name
    let mut raw2 = Vec::new();
    raw2.extend_from_slice(b"ExplosionSfx\0");
    raw2.push(0x00); // default SoundInfo
    let parsed2 = parse_start_sound2(&raw2).expect("valid StartSound2");
    assert_eq!(parsed2.start_sound2_class(), "ExplosionSfx");
    assert_eq!(parsed2.info.loop_count, 1);

    // Reject in_point >= out_point
    let mut bad_pts = vec![0x03]; // HasOutPoint | HasInPoint
    bad_pts.extend_from_slice(&1000u32.to_le_bytes());
    bad_pts.extend_from_slice(&1000u32.to_le_bytes());
    assert_eq!(
        parse_sound_info(&bad_pts),
        Err(AudioBridgeError::InvalidInOutPoints {
            in_point: 1000,
            out_point: 1000
        })
    );

    // Reject loop_count == 0 or > MAX_LOOP_COUNT
    let mut bad_loop = vec![0x04];
    bad_loop.extend_from_slice(&0u16.to_le_bytes());
    assert_eq!(
        parse_sound_info(&bad_loop),
        Err(AudioBridgeError::InvalidLoopCount { count: 0 })
    );
    bad_loop[1..3].copy_from_slice(&(MAX_LOOP_COUNT + 1).to_le_bytes());
    assert_eq!(
        parse_sound_info(&bad_loop),
        Err(AudioBridgeError::InvalidLoopCount {
            count: MAX_LOOP_COUNT + 1
        })
    );

    // Reject envelope level > 32768 and non-monotonic pos44
    let mut bad_env = vec![0x08, 1];
    bad_env.extend_from_slice(&0u32.to_le_bytes());
    bad_env.extend_from_slice(&32769u16.to_le_bytes());
    bad_env.extend_from_slice(&0u16.to_le_bytes());
    assert!(matches!(
        parse_sound_info(&bad_env),
        Err(AudioBridgeError::InvalidEnvelopePoint { .. })
    ));

    let mut non_mono = vec![0x08, 2];
    non_mono.extend_from_slice(&500u32.to_le_bytes());
    non_mono.extend_from_slice(&1000u16.to_le_bytes());
    non_mono.extend_from_slice(&1000u16.to_le_bytes());
    non_mono.extend_from_slice(&100u32.to_le_bytes());
    non_mono.extend_from_slice(&1000u16.to_le_bytes());
    non_mono.extend_from_slice(&1000u16.to_le_bytes());
    assert!(matches!(
        parse_sound_info(&non_mono),
        Err(AudioBridgeError::InvalidEnvelopePoint { .. })
    ));
}

#[test]
fn test_pcm_decoders_and_downmix() {
    let pcm8 = [0u8, 128, 255];
    let mut out8 = [0i16; 3];
    assert_eq!(decode_pcm8_into(&pcm8, &mut out8), Ok(3));
    assert_eq!(out8, [-32768, 0, 32512]);

    let pcm16_src = [-32768i16, -1234, 0, 1234, 32767];
    let mut le_bytes = Vec::new();
    let mut ne_bytes = Vec::new();
    for s in pcm16_src {
        le_bytes.extend_from_slice(&s.to_le_bytes());
        ne_bytes.extend_from_slice(&s.to_ne_bytes());
    }
    let mut out16 = [0i16; 5];
    assert_eq!(decode_pcm16_le_into(&le_bytes, &mut out16), Ok(5));
    assert_eq!(out16, pcm16_src);

    let mut out16_ne = [0i16; 5];
    assert_eq!(decode_pcm16_ne_into(&ne_bytes, &mut out16_ne), Ok(5));
    assert_eq!(out16_ne, pcm16_src);

    // Odd byte rejection
    assert!(matches!(
        decode_pcm16_le_into(&[1, 2, 3], &mut out16),
        Err(AudioBridgeError::MalformedTagPayload { .. })
    ));

    // Buffer too small (CWE-787)
    let mut tiny = [0i16; 2];
    assert_eq!(
        decode_pcm8_into(&pcm8, &mut tiny),
        Err(AudioBridgeError::BufferTooSmall {
            required: 3,
            provided: 2
        })
    );

    // Stereo to mono downmix
    let stereo = [10000i16, -10000, 30000, 10000];
    let mut mono = [0i16; 2];
    assert_eq!(mix_stereo_to_mono_into(&stereo, &mut mono), Ok(2));
    assert_eq!(mono, [0, 20000]);
}

#[test]
fn test_flash_adpcm_2_3_4_5_bit_mono_and_stereo_and_multipacket() {
    // 1. Test 2, 3, 4, 5-bit mono smooth ramp within [-16384, +16128]
    let mono_signal: Vec<i16> = (0..128).map(|i| ((i * 256) - 16384) as i16).collect();
    for bits in 2..=5 {
        let encoded = encode_flash_adpcm(bits, 1, &mono_signal);
        let mut decoded = vec![0i16; mono_signal.len()];
        let n = decode_adpcm_into(
            &encoded,
            SwfAudioChannels::Mono,
            mono_signal.len() as u32,
            &mut decoded,
        )
        .expect("ADPCM mono decode");
        assert_eq!(n, mono_signal.len());
        assert_eq!(decoded[0], mono_signal[0]);
        let max_err: i32 = decoded
            .iter()
            .zip(mono_signal.iter())
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .max()
            .unwrap();
        assert!(max_err < 4096, "bits={bits} max_err={max_err} too large");
    }

    // 2. Test 4-bit stereo across multiple packets (> 4096 frames = 5000 frames)
    let mut stereo_signal = Vec::with_capacity(5000 * 2);
    for i in 0..5000 {
        let l = (((i % 64) - 32) * 500) as i16;
        let r = ((32 - (i % 64)) * 500) as i16;
        stereo_signal.push(l);
        stereo_signal.push(r);
    }
    let encoded_stereo = encode_flash_adpcm(4, 2, &stereo_signal);
    let mut decoded_stereo = vec![0i16; stereo_signal.len()];
    let written = decode_adpcm_into(
        &encoded_stereo,
        SwfAudioChannels::Stereo,
        5000,
        &mut decoded_stereo,
    )
    .expect("ADPCM stereo multipacket");
    assert_eq!(written, 10000);
    // Packet boundaries (0 and 4096) carry exact initial samples
    assert_eq!(decoded_stereo[0], stereo_signal[0]);
    assert_eq!(decoded_stereo[1], stereo_signal[1]);
    assert_eq!(decoded_stereo[4096 * 2], stereo_signal[4096 * 2]);
    assert_eq!(decoded_stereo[4096 * 2 + 1], stereo_signal[4096 * 2 + 1]);

    // 3. Reject out-of-range initial step index (> 88)
    let mut bw = BitWriter::new();
    bw.write_ub(2, 2); // 4-bit
    bw.write_sb(16, 0);
    bw.write_ub(6, 63); // wait: 6 bits max is 63, which is <= 88; wait: 6 bits max is 63!
                        // Good observation: UB[6] has range 0..=63 which is always <= 88! Still, verify EOF works.
    let bad_eof = bw.into_bytes();
    let mut small = [0i16; 10];
    assert_eq!(
        decode_adpcm_into(&bad_eof, SwfAudioChannels::Mono, 10, &mut small),
        Err(AudioBridgeError::UnexpectedEof)
    );
}

#[test]
fn test_mp3_frame_header_validation_and_iter() {
    // MPEG-1 Layer III, no CRC, 128 kbps (idx 9), 44100 Hz (idx 0), padding=0, Stereo (0)
    // Bytes: FF FB 90 00 -> frame_len = (128 * 144000) / 44100 = 417 bytes, 1152 samples
    let hdr1 = [0xFF, 0xFB, 0x90, 0x00];
    let h1 = parse_mp3_frame_header(&hdr1).expect("valid MPEG1 L3 header");
    assert_eq!(h1.version, MpegVersion::Mpeg1);
    assert!(!h1.has_crc);
    assert_eq!(h1.bitrate_kbps, 128);
    assert_eq!(h1.sample_rate_hz, 44_100);
    assert!(!h1.padding);
    assert_eq!(h1.channel_mode, Mp3ChannelMode::Stereo);
    assert_eq!(h1.samples_per_frame, 1152);
    assert_eq!(h1.mp3_header_frame_len(), 417);

    // MPEG-2 Layer III, CRC=1 (bit16=0 -> 0xF2), 64 kbps (idx 8), 22050 Hz (idx 0), padding=1, Mono (3)
    // Header: 0xFFF2_82C0 -> frame_len = (64 * 72000) / 22050 + 1 = 208 + 1 = 209 bytes, 576 samples
    let hdr2 = [0xFF, 0xF2, 0x82, 0xC0];
    let h2 = parse_mp3_frame_header(&hdr2).expect("valid MPEG2 L3 header");
    assert_eq!(h2.version, MpegVersion::Mpeg2);
    assert!(h2.has_crc);
    assert_eq!(h2.bitrate_kbps, 64);
    assert_eq!(h2.sample_rate_hz, 22_050);
    assert!(h2.padding);
    assert_eq!(h2.channel_mode, Mp3ChannelMode::Mono);
    assert_eq!(h2.samples_per_frame, 576);
    assert_eq!(h2.mp3_header_frame_len(), 209);

    // Build DefineSound MP3 payload: SeekSamples (529) + 2 frames
    let mut mp3_payload = Vec::new();
    mp3_payload.extend_from_slice(&529i16.to_le_bytes());
    let mut frame1 = vec![0u8; 417];
    frame1[0..4].copy_from_slice(&hdr1);
    let mut frame2 = vec![0u8; 209];
    frame2[0..4].copy_from_slice(&hdr2);
    mp3_payload.extend_from_slice(&frame1);
    mp3_payload.extend_from_slice(&frame2);

    let (seek, stream) = extract_define_sound_mp3_info(&mp3_payload).expect("extract MP3 info");
    assert_eq!(seek, 529);
    let mut iter = Mp3FrameIter::mp3_iter_from_bytes(stream);
    let f1 = iter.next().unwrap().expect("frame 1");
    assert_eq!(f1.bytes.len(), 417);
    assert_eq!(iter.mp3_iter_remaining(), 209);
    let f2 = iter.next().unwrap().expect("frame 2");
    assert_eq!(f2.bytes.len(), 209);
    assert!(iter.next().is_none());

    // Invalid sync rejected
    assert!(matches!(
        parse_mp3_frame_header(&[0x00, 0xFB, 0x90, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    // Reserved bitrate (idx 15) rejected
    assert!(matches!(
        parse_mp3_frame_header(&[0xFF, 0xFB, 0xF0, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
}

#[test]
fn test_swf_tag_iter_stream_and_audio_mixer_sync() {
    // Construct an SWF tag stream containing:
    // 1. DefineSound (Tag 14): sound_id=1, UncompressedLe, 22050 Hz, 16-bit, Mono, 4 samples
    // 2. StartSound (Tag 15): sound_id=1, loop_count=2
    // 3. SoundStreamHead2 (Tag 45): 44100 Hz, 16-bit, Mono, UncompressedLe, avg=4
    // 4. SoundStreamBlock (Tag 19): 4 samples of 16-bit LE PCM
    // 5. StartSound2 (Tag 89): "Bell", loop_count=1
    let mut def_payload = Vec::new();
    def_payload.extend_from_slice(&1u16.to_le_bytes());
    def_payload.push(sound_flags_byte(
        SwfSoundFormat::UncompressedLe,
        SwfSoundRate::Rate22050,
        SwfSampleWidth::Bits16,
        SwfAudioChannels::Mono,
    ));
    def_payload.extend_from_slice(&4u32.to_le_bytes());
    for s in [1000i16, 2000, 3000, 4000] {
        def_payload.extend_from_slice(&s.to_le_bytes());
    }

    let mut start_payload = Vec::new();
    start_payload.extend_from_slice(&1u16.to_le_bytes());
    start_payload.push(0x04); // HasLoops
    start_payload.extend_from_slice(&2u16.to_le_bytes());

    let stream_head_payload = [
        0x34, // playback: 44100 (3<<4), 16-bit (1<<3), Mono (0<<2)
        sound_flags_byte(
            SwfSoundFormat::UncompressedLe,
            SwfSoundRate::Rate44100,
            SwfSampleWidth::Bits16,
            SwfAudioChannels::Mono,
        ),
        4,
        0, // avg_sample_count = 4
    ];

    let mut stream_block_payload = Vec::new();
    for s in [100i16, 200, 300, 400] {
        stream_block_payload.extend_from_slice(&s.to_le_bytes());
    }

    let mut start2_payload = Vec::new();
    start2_payload.extend_from_slice(b"Bell\0");
    start2_payload.push(0x00);

    let mut stream_bytes = Vec::new();
    stream_bytes.extend_from_slice(&make_swf_tag(14, &def_payload));
    stream_bytes.extend_from_slice(&make_swf_tag(15, &start_payload));
    stream_bytes.extend_from_slice(&make_swf_tag(45, &stream_head_payload));
    stream_bytes.extend_from_slice(&make_swf_tag(19, &stream_block_payload));
    stream_bytes.extend_from_slice(&make_swf_tag(SWF_TAG_START_SOUND2, &start2_payload));
    stream_bytes.extend_from_slice(&make_swf_tag(0, &[])); // End tag

    let mut engine = FlashAudioState::create_engine(44_100, 24).expect("engine");
    engine
        .register_pcm_sound(2, 44_100, SwfAudioChannels::Mono, &[500, 500], Some("Bell"))
        .expect("register Bell");

    let mut active_stream_fmt = None;
    for tag_res in SwfTagIter::from_payload(&stream_bytes) {
        let tag = tag_res.expect("valid SwfTag");
        if tag.header.kind == SwfTagKind::End {
            break;
        }
        if let Some(audio_tag) = parse_swf_audio_tag(&tag, active_stream_fmt).expect("parse audio")
        {
            match audio_tag {
                SwfAudioTag::DefineSound(ds) => {
                    let mut tmp = [0i16; 4];
                    assert_eq!(decode_define_sound_into(&ds, &mut tmp), Ok(4));
                    assert_eq!(tmp, [1000, 2000, 3000, 4000]);
                    engine
                        .register_define_sound(&ds)
                        .expect("register DefineSound");
                }
                SwfAudioTag::StartSound(ss) => {
                    engine.apply_start_sound(&ss).expect("StartSound");
                }
                SwfAudioTag::StartSound2(ss2) => {
                    engine.apply_start_sound2(&ss2).expect("StartSound2");
                }
                SwfAudioTag::SoundStreamHead(head) => {
                    assert_eq!(head.stream_head_rate_hz(), 44_100);
                    active_stream_fmt = Some(head.stream_properties.format);
                    engine.configure_sound_stream(head).expect("config stream");
                }
                SwfAudioTag::SoundStreamBlock(blk) => {
                    assert_eq!(blk.stream_block_len(), 8);
                    let pushed = engine.push_stream_block(&blk).expect("push stream block");
                    assert_eq!(pushed, 4);
                }
            }
        }
    }

    assert_eq!(engine.active_voice_count(), 2);
    assert_eq!(engine.stream_fifo_len(), 4);

    // Render 4 samples: voice 1 (22050->44100 step 0.5: [1000, 1000, 2000, 2000])
    // + voice 2 ([500, 500, 0, 0]) + stream ([100, 200, 300, 400])
    let mut out = [0i16; 4];
    assert_eq!(render_flash_audio_into(&mut engine, &mut out), Ok(4));
    assert_eq!(out, [1600, 1700, 2300, 2400]);
    assert_eq!(engine.stream_fifo_len(), 0);
    assert_eq!(engine.engine_mixed_frames(), 4);

    // Test game-clock synchronization (1 frame at 24 fps -> ~1837 samples @ 44100 Hz)
    let samples_for_frame = engine
        .advance_frame_clock(41_666_667)
        .expect("advance clock");
    assert!((1836..=1838).contains(&samples_for_frame));
}

#[test]
fn test_actionscript_sound_api_clipping_and_bridges() {
    let mut engine = FlashAudioState::create_engine(44_100, 30).expect("engine");
    engine
        .register_pcm_sound(10, 44_100, SwfAudioChannels::Mono, &[30_000; 64], None)
        .expect("register loud sound");

    // Start 4 overlapping voices to trigger positive saturation clamping (CWE-190)
    for _ in 0..4 {
        engine.as_sound_start(10, 0, 1).expect("Sound.start()");
    }
    assert_eq!(engine.active_voice_count(), 4);

    let mut ring: PcmRingBuffer<128> = PcmRingBuffer::empty();
    let bridged = bridge_render_to_ring_buffer(&mut engine, &mut ring, 16).expect("bridge");
    assert_eq!(bridged, 16);
    assert_eq!(ring.len(), 16);
    assert_eq!(ring.pop(), Some(i16::MAX)); // saturated cleanly, never wrapped!
    assert!(engine.engine_clipped_samples() >= 16);

    // stopAllSounds() silences everything
    engine.as_stop_all_sounds();
    assert_eq!(engine.active_voice_count(), 0);
    assert_eq!(engine.stream_fifo_len(), 0);

    // Verify VoiceLimitExceeded at MAX_ACTIVE_VOICES + 1
    for _ in 0..MAX_ACTIVE_VOICES {
        engine.as_sound_start(10, 0, 1).expect("alloc voice");
    }
    assert_eq!(
        engine.as_sound_start(10, 0, 1),
        Err(AudioBridgeError::VoiceLimitExceeded)
    );

    // Export to pcm_audio::PcmAudio and audio_device_sink::PcmPlaybackBuffer
    let pcm = export_flash_audio_to_pcm_audio(&mut engine, 32).expect("export pcm-audio");
    assert_eq!(pcm.sample_count(), 32);

    let sink_buf = export_flash_audio_to_sink_buffer(&mut engine, 32).expect("export sink buffer");
    assert_eq!(sink_buf.sample_count(), 32);
}

#[test]
fn test_sound_stream_head_mp3_and_adpcm_stream_block() {
    // SoundStreamHead2 with MP3 + LatencySeek = -529
    let mp3_head_bytes = [
        0x34, // playback 44100 16-bit mono
        sound_flags_byte(
            SwfSoundFormat::Mp3,
            SwfSoundRate::Rate44100,
            SwfSampleWidth::Bits16,
            SwfAudioChannels::Mono,
        ),
        0x80,
        0x04, // avg = 1152
        0xEF,
        0xFD, // latency_seek = -529 (0xFDEF)
    ];
    let head = parse_sound_stream_head(&mp3_head_bytes, true).expect("valid MP3 stream head");
    assert!(head.is_v2);
    assert_eq!(head.avg_sample_count, 1152);
    assert_eq!(head.latency_seek, -529);

    // SoundStreamHead v1 rejects UncompressedLe (format 3)
    let v1_le_bytes = [
        0x34,
        sound_flags_byte(
            SwfSoundFormat::UncompressedLe,
            SwfSoundRate::Rate44100,
            SwfSampleWidth::Bits16,
            SwfAudioChannels::Mono,
        ),
        0x00,
        0x04,
    ];
    assert_eq!(
        parse_sound_stream_head(&v1_le_bytes, false),
        Err(AudioBridgeError::UnsupportedSoundFormat { code: 3 })
    );

    // MP3 SoundStreamBlock parsing
    let mut mp3_blk = Vec::new();
    mp3_blk.extend_from_slice(&1152u16.to_le_bytes());
    mp3_blk.extend_from_slice(&0i16.to_le_bytes());
    mp3_blk.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
    let blk = parse_sound_stream_block(&mp3_blk, SwfSoundFormat::Mp3).expect("MP3 stream block");
    assert_eq!(blk.mp3_sample_count, Some(1152));
    assert_eq!(blk.mp3_seek_samples, Some(0));
    assert_eq!(blk.stream_block_len(), 4);

    // ADPCM SoundStreamBlock zero-alloc push
    let mut engine = FlashAudioState::create_engine(44_100, 24).expect("engine");
    let adpcm_head_bytes = [
        0x34,
        sound_flags_byte(
            SwfSoundFormat::Adpcm,
            SwfSoundRate::Rate44100,
            SwfSampleWidth::Bits16,
            SwfAudioChannels::Mono,
        ),
        32,
        0, // avg = 32
    ];
    let adpcm_head = parse_sound_stream_head(&adpcm_head_bytes, false).expect("ADPCM head");
    engine
        .configure_sound_stream(adpcm_head)
        .expect("configure");
    let adpcm_samples = vec![1000i16; 32];
    let adpcm_payload = encode_flash_adpcm(4, 1, &adpcm_samples);
    let adpcm_blk =
        parse_sound_stream_block(&adpcm_payload, SwfSoundFormat::Adpcm).expect("ADPCM block");
    let pushed = engine.push_stream_block(&adpcm_blk).expect("push ADPCM");
    assert_eq!(pushed, 32);
    assert_eq!(engine.stream_fifo_len(), 32);
}

#[test]
fn test_envelope_in_out_points_sync_flags_and_all_rates() {
    // Verify all 4 SWF sample rates and 8-bit UncompressedNe DefineSound
    let expected_rates = [
        (SwfSoundRate::Rate5512, 5_512u32),
        (SwfSoundRate::Rate11025, 11_025u32),
        (SwfSoundRate::Rate22050, 22_050u32),
        (SwfSoundRate::Rate44100, 44_100u32),
    ];
    for (idx, (rate_enum, hz)) in expected_rates.into_iter().enumerate() {
        assert_eq!(rate_enum.rate_to_hz(), hz);
        let mut raw = Vec::new();
        raw.extend_from_slice(&((idx as u16) + 1).to_le_bytes());
        raw.push(sound_flags_byte(
            SwfSoundFormat::UncompressedNe,
            rate_enum,
            SwfSampleWidth::Bits8,
            SwfAudioChannels::Mono,
        ));
        raw.extend_from_slice(&2u32.to_le_bytes());
        raw.extend_from_slice(&[128u8, 255u8]);
        let ds = parse_define_sound(&raw).expect("parse 8-bit DefineSound");
        let mut pcm = [0i16; 2];
        assert_eq!(decode_define_sound_into(&ds, &mut pcm), Ok(2));
        assert_eq!(pcm, [0, 32512]);
    }

    // Envelope interpolation (0 -> 32768 over pos44 0..=4) + in_point/out_point + sync flags
    let mut engine = FlashAudioState::create_engine(44_100, 24).expect("engine");
    engine
        .register_pcm_sound(20, 44_100, SwfAudioChannels::Mono, &[10_000i16; 16], None)
        .expect("register");

    let mut ss_bytes = Vec::new();
    ss_bytes.extend_from_slice(&20u16.to_le_bytes());
    ss_bytes.push(0x1B); // SyncNoMultiple (0x10) | HasEnvelope (0x08) | HasOutPoint (0x02) | HasInPoint (0x01)
    ss_bytes.extend_from_slice(&2u32.to_le_bytes()); // in_point = 2
    ss_bytes.extend_from_slice(&6u32.to_le_bytes()); // out_point = 6 (4 samples)
    ss_bytes.push(2); // 2 envelope points
    ss_bytes.extend_from_slice(&2u32.to_le_bytes());
    ss_bytes.extend_from_slice(&0u16.to_le_bytes());
    ss_bytes.extend_from_slice(&0u16.to_le_bytes());
    ss_bytes.extend_from_slice(&6u32.to_le_bytes());
    ss_bytes.extend_from_slice(&32_768u16.to_le_bytes());
    ss_bytes.extend_from_slice(&32_768u16.to_le_bytes());

    let start_tag = parse_start_sound(&ss_bytes).expect("StartSound with envelope");
    engine.apply_start_sound(&start_tag).expect("apply start");
    assert_eq!(engine.active_voice_count(), 1);

    // SyncNoMultiple prevents spawning a second voice for sound_id=20
    engine.apply_start_sound(&start_tag).expect("no multiple");
    assert_eq!(engine.active_voice_count(), 1);

    // Render 4 samples: ramp from 0% to 75% of 10000 -> [0, 2500, 5000, 7500]
    let mut out = [0i16; 4];
    assert_eq!(render_flash_audio_into(&mut engine, &mut out), Ok(4));
    assert_eq!(out, [0, 2500, 5000, 7500]);

    // SyncStop stops active voice
    let stop_tag = parse_start_sound(&[20, 0, 0x20]).expect("parse SyncStop");
    engine.as_sound_start(20, 0, 1).expect("restart");
    assert_eq!(engine.active_voice_count(), 1);
    engine.apply_start_sound(&stop_tag).expect("apply SyncStop");
    assert_eq!(engine.active_voice_count(), 0);
}
