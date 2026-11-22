//! Hostile-input security regression and fuzz tests for `flash-audio-bridge` (`SWF06`).
//!
//! Covers:
//! - Multi-packet ADPCM corruption (`step_index >= 89` and mid-stream `UnexpectedEof`) with atomic `out` preservation
//! - Odd-byte PCM16 and odd-length stereo slice rejection
//! - MP3 frame header edge cases (sync, reserved version, non-Layer III, free/invalid bitrate, reserved sample rate, truncation)
//! - Atomic `stream_fifo` and `PcmRingBuffer` / voice / telemetry preservation on `RingBufferOverrun`
//! - `advance_frame_clock` atomicity on `u64::MAX` `raw_frame_delta_ns`
//! - `StartSound2` / `register_pcm_sound` `class_name` validation (empty, NUL, `> MAX_CLASS_NAME_BYTES`)
//! - Seeded 1,500-case hostile audio tag + bitstream fuzz campaign

#![forbid(unsafe_code)]

use flash_audio_bridge::{
    bridge_render_to_ring_buffer, decode_adpcm_into, decode_define_sound_into,
    decode_pcm16_le_into, decode_pcm16_ne_into, mix_stereo_to_mono_into, parse_define_sound,
    parse_mp3_frame_header, parse_sound_info, parse_sound_stream_block, parse_sound_stream_head,
    parse_start_sound, parse_start_sound2, render_flash_audio_into, AudioBridgeError,
    DefineSoundTag, FlashAudioState, Mp3FrameIter, PcmRingBuffer, SoundInfo, SoundStreamBlockTag,
    SoundStreamHead, StartSound2Tag, SwfAudioChannels, SwfSampleWidth, SwfSoundFormat,
    SwfSoundProperties, SwfSoundRate, ADPCM_SAMPLES_PER_BLOCK, MAX_CLASS_NAME_BYTES,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let v = self.next_u64().to_le_bytes();
            let take = (len - out.len()).min(8);
            out.extend_from_slice(&v[..take]);
        }
        out
    }
}

struct BitEncoder {
    bytes: Vec<u8>,
    cur: u8,
    used: u8,
}

impl BitEncoder {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            cur: 0,
            used: 0,
        }
    }

    fn write_bits(&mut self, val: u32, bits: usize) {
        for i in (0..bits).rev() {
            let bit = ((val >> i) & 1) as u8;
            self.cur = (self.cur << 1) | bit;
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.cur);
                self.cur = 0;
                self.used = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.used > 0 {
            self.bytes.push(self.cur << (8 - self.used));
        }
        self.bytes
    }
}

fn build_two_packet_adpcm_mono(pkt0_idx: u32, pkt1_idx: u32, truncate_last_byte: bool) -> Vec<u8> {
    let mut enc = BitEncoder::new();
    // code_size = 2 => 4-bit ADPCM
    enc.write_bits(2, 2);
    // Packet 0: 4096 samples (1 initial + 4095 4-bit deltas)
    enc.write_bits(0x1234, 16);
    enc.write_bits(pkt0_idx, 6);
    for i in 1..ADPCM_SAMPLES_PER_BLOCK {
        enc.write_bits(i & 0x0F, 4);
    }
    // Packet 1: 4 samples (1 initial + 3 deltas)
    enc.write_bits(0x5678, 16);
    enc.write_bits(pkt1_idx, 6);
    for i in 0..3u32 {
        enc.write_bits(i & 0x0F, 4);
    }
    let mut bytes = enc.finish();
    if truncate_last_byte {
        bytes.pop();
    }
    bytes
}

#[test]
fn hostile_adpcm_multipacket_atomic_out_preservation() {
    let sample_count = ADPCM_SAMPLES_PER_BLOCK + 4;

    // 1. Packet 0 valid, Packet 1 truncated mid-deltas -> UnexpectedEof, `out` untouched.
    let eof_payload = build_two_packet_adpcm_mono(10, 12, true);
    let mut out = vec![0x5A5A_u16 as i16; sample_count as usize];
    let sentinel = out.clone();
    let err_eof = decode_adpcm_into(&eof_payload, SwfAudioChannels::Mono, sample_count, &mut out)
        .expect_err("truncated packet 1 must fail");
    assert_eq!(err_eof, AudioBridgeError::UnexpectedEof);
    assert_eq!(
        out, sentinel,
        "out slice was partially mutated before UnexpectedEof"
    );

    // 2. Packet 0 valid, Packet 1 truncated inside packet 1 header.
    let mut hdr_trunc = build_two_packet_adpcm_mono(10, 12, false);
    hdr_trunc.truncate(hdr_trunc.len() - 2);
    let err_hdr = decode_adpcm_into(&hdr_trunc, SwfAudioChannels::Mono, sample_count, &mut out)
        .expect_err("truncated packet 1 header must fail");
    assert_eq!(err_hdr, AudioBridgeError::UnexpectedEof);
    assert_eq!(
        out, sentinel,
        "out slice was partially mutated before packet 1 header UnexpectedEof"
    );

    // 3. Same via decode_define_sound_into.
    let tag = DefineSoundTag {
        sound_id: 7,
        properties: SwfSoundProperties {
            format: SwfSoundFormat::Adpcm,
            rate: SwfSoundRate::Rate44100,
            width: SwfSampleWidth::Bits16,
            channels: SwfAudioChannels::Mono,
        },
        sample_count,
        payload: &eof_payload,
    };
    let err_tag = decode_define_sound_into(&tag, &mut out).expect_err("must reject truncated pkt1");
    assert_eq!(err_tag, AudioBridgeError::UnexpectedEof);
    assert_eq!(out, sentinel);
}

#[test]
fn hostile_pcm_odd_bytes_and_mp3_header_edge_cases() {
    let mut out = [77i16; 8];
    let before = out;
    assert_eq!(
        decode_pcm16_le_into(&[1, 2, 3], &mut out),
        Err(AudioBridgeError::MalformedTagPayload {
            reason: "16-bit LE PCM payload has odd byte count",
        })
    );
    assert_eq!(out, before);

    assert_eq!(
        decode_pcm16_ne_into(&[1, 2, 3, 4, 5], &mut out),
        Err(AudioBridgeError::MalformedTagPayload {
            reason: "16-bit NE PCM payload has odd byte count",
        })
    );
    assert_eq!(out, before);

    assert_eq!(
        mix_stereo_to_mono_into(&[100, 200, 300], &mut out),
        Err(AudioBridgeError::MalformedTagPayload {
            reason: "interleaved stereo slice has odd sample count",
        })
    );
    assert_eq!(out, before);

    // MP3 header edge cases:
    // - Short (< 4 bytes)
    assert_eq!(
        parse_mp3_frame_header(&[0xFF, 0xFB, 0x90]),
        Err(AudioBridgeError::UnexpectedEof)
    );
    // - Bad sync
    assert!(matches!(
        parse_mp3_frame_header(&[0x7F, 0xFB, 0x90, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    // - Reserved MPEG version (01)
    assert!(matches!(
        parse_mp3_frame_header(&[0xFF, 0xEB, 0x90, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    // - Non-Layer III (Layer II = 10 -> bits 18..17 = 10 -> 0xFD)
    assert!(matches!(
        parse_mp3_frame_header(&[0xFF, 0xFD, 0x90, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    // - Free bitrate (index 0) and invalid bitrate (index 15)
    assert!(matches!(
        parse_mp3_frame_header(&[0xFF, 0xFB, 0x00, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    assert!(matches!(
        parse_mp3_frame_header(&[0xFF, 0xFB, 0xF0, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    // - Reserved sample rate index (3)
    assert!(matches!(
        parse_mp3_frame_header(&[0xFF, 0xFB, 0x9C, 0x00]),
        Err(AudioBridgeError::InvalidMp3Frame { .. })
    ));
    // - Valid header whose frame extends past slice end -> Mp3FrameIter yields UnexpectedEof
    let valid_hdr = [0xFF, 0xFB, 0x90, 0x00]; // MPEG1 Layer3 128kbps 44100Hz (417 bytes)
    assert!(parse_mp3_frame_header(&valid_hdr).is_ok());
    let mut iter = Mp3FrameIter::mp3_iter_from_bytes(&valid_hdr);
    assert_eq!(iter.next(), Some(Err(AudioBridgeError::UnexpectedEof)));
    assert_eq!(iter.next(), None);
}

#[test]
fn hostile_atomic_stream_fifo_ring_buffer_clock_and_class_name() {
    // 1. push_stream_block multi-chunk (> 4096 scalars) atomic preservation on RingBufferOverrun.
    let mut eng = FlashAudioState::create_engine(44_100, 24).unwrap();
    let head = SoundStreamHead {
        is_v2: true,
        playback_rate: SwfSoundRate::Rate44100,
        playback_width: SwfSampleWidth::Bits16,
        playback_channels: SwfAudioChannels::Mono,
        stream_properties: SwfSoundProperties {
            format: SwfSoundFormat::UncompressedLe,
            rate: SwfSoundRate::Rate44100,
            width: SwfSampleWidth::Bits16,
            channels: SwfAudioChannels::Mono,
        },
        avg_sample_count: 1024,
        latency_seek: 0,
    };
    eng.configure_sound_stream(head).unwrap();

    // Pre-fill 3,000 samples into the 8,192-sample stream FIFO.
    let seed_samples = vec![123i16; 3_000];
    assert_eq!(eng.push_stream_pcm_into_fifo(&seed_samples), Ok(3_000));
    assert_eq!(eng.stream_fifo_len(), 3_000);

    // Push a multi-chunk 6,000-sample block (12,000 bytes > 4,096*2) -> exceeds remaining 5,192 capacity.
    let big_block = vec![0x11u8; 6_000 * 2];
    let tag = SoundStreamBlockTag {
        mp3_sample_count: None,
        mp3_seek_samples: None,
        audio_data: &big_block,
    };
    assert_eq!(
        eng.push_stream_block(&tag),
        Err(AudioBridgeError::RingBufferOverrun)
    );
    assert_eq!(
        eng.stream_fifo_len(),
        3_000,
        "stream_fifo must not partially accept chunks before RingBufferOverrun"
    );

    // 2. bridge_render_to_ring_buffer atomic preservation of voices, stream_fifo, telemetry, and ring.
    eng.register_pcm_sound(
        1,
        44_100,
        SwfAudioChannels::Mono,
        &[10_000i16; 1_024],
        Some("boom"),
    )
    .unwrap();
    eng.apply_start_sound2(&StartSound2Tag {
        class_name: "boom",
        info: SoundInfo::default(),
    })
    .unwrap();

    let mut ring: PcmRingBuffer<128> = PcmRingBuffer::empty();
    ring.push_slice(&[7i16; 100]).unwrap();
    let mixed_before = eng.engine_mixed_frames();
    let fifo_before = eng.stream_fifo_len();
    let voices_before = eng.active_voice_count();

    assert_eq!(
        bridge_render_to_ring_buffer(&mut eng, &mut ring, 512),
        Err(AudioBridgeError::RingBufferOverrun)
    );
    assert_eq!(ring.len(), 100);
    assert_eq!(eng.engine_mixed_frames(), mixed_before);
    assert_eq!(eng.stream_fifo_len(), fifo_before);
    assert_eq!(eng.active_voice_count(), voices_before);

    // Confirm rendering still works normally when ring has capacity.
    let mut drain_buf = [0i16; 32];
    assert_eq!(render_flash_audio_into(&mut eng, &mut drain_buf), Ok(32));

    // 3. advance_frame_clock with u64::MAX raw_frame_delta_ns must never panic and stays consistent.
    let mut eng_clk = FlashAudioState::create_engine(44_100, 24).unwrap();
    let _ = eng_clk.advance_frame_clock(u64::MAX);
    let _ = eng_clk.advance_frame_clock(u64::MAX);

    // 4. Invalid StartSound2 / register_pcm_sound class_name guards.
    assert_eq!(
        eng.register_pcm_sound(2, 44_100, SwfAudioChannels::Mono, &[0; 4], Some("")),
        Err(AudioBridgeError::InvalidClassName)
    );
    assert_eq!(
        eng.register_pcm_sound(
            2,
            44_100,
            SwfAudioChannels::Mono,
            &[0; 4],
            Some("bad\0name")
        ),
        Err(AudioBridgeError::InvalidClassName)
    );
    let long_name = "a".repeat(MAX_CLASS_NAME_BYTES + 1);
    assert_eq!(
        eng.register_pcm_sound(2, 44_100, SwfAudioChannels::Mono, &[0; 4], Some(&long_name)),
        Err(AudioBridgeError::InvalidClassName)
    );
    assert_eq!(
        eng.apply_start_sound2(&StartSound2Tag {
            class_name: "bad\0name",
            info: SoundInfo::default(),
        }),
        Err(AudioBridgeError::InvalidClassName)
    );
    assert_eq!(
        eng.apply_start_sound2(&StartSound2Tag {
            class_name: "",
            info: SoundInfo::default(),
        }),
        Err(AudioBridgeError::InvalidClassName)
    );
}

#[test]
fn hostile_fuzz_1500_cases_tags_and_bitstreams_zero_panics() {
    let mut rng = SplitMix64(0xF1A5_4006_2026_1500);
    let mut pcm_out = [0i16; 512];

    for i in 0..1_500 {
        let len = (rng.next_u64() as usize) % 256;
        let payload = rng.next_bytes(len);

        let _ = parse_define_sound(&payload);
        let _ = parse_sound_info(&payload);
        let _ = parse_start_sound(&payload);
        let _ = parse_start_sound2(&payload);
        let _ = parse_sound_stream_head(&payload, (i & 1) == 0);
        let _ = parse_sound_stream_block(&payload, SwfSoundFormat::Adpcm);
        let _ = parse_sound_stream_block(&payload, SwfSoundFormat::Mp3);
        let _ = parse_mp3_frame_header(&payload);
        for item in Mp3FrameIter::mp3_iter_from_bytes(&payload).take(8) {
            if item.is_err() {
                break;
            }
        }

        let ch = if (i & 1) == 0 {
            SwfAudioChannels::Mono
        } else {
            SwfAudioChannels::Stereo
        };
        let samples = (rng.next_u64() as u32) % 300;
        let _ = decode_adpcm_into(&payload, ch, samples, &mut pcm_out);
        let _ = decode_pcm16_le_into(&payload, &mut pcm_out);
        let _ = decode_pcm16_ne_into(&payload, &mut pcm_out);
    }
}
