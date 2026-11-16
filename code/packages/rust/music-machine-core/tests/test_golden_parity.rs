//! Tier-0 Golden Fixture Parity Tests (MUS01 / MUS03 / MUS04 / MUS05 / MUS07).

use std::path::Path;
use coding_adventures_sha256::sha256_hex;
use music_machine_core::{
    parse_portable_score, parse_portable_score_json, render_portable_score_to_pcm,
};

#[test]
fn test_golden_fixture_parity() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/fixtures/music-machine");
    let txt_path = fixture_dir.join("canonical_score.txt");
    let json_path = fixture_dir.join("canonical_score.json");
    let expected_render_path = fixture_dir.join("expected_render.json");

    let txt_content = std::fs::read_to_string(&txt_path).expect("read canonical_score.txt");
    let json_content = std::fs::read_to_string(&json_path).expect("read canonical_score.json");
    let expected_render_content = std::fs::read_to_string(&expected_render_path).expect("read expected_render.json");

    // 1. Parse text score and assert equivalence to JSON-parsed score
    let score_from_text = parse_portable_score(&txt_content).expect("parse canonical_score.txt");
    let score_from_json = parse_portable_score_json(&json_content).expect("parse canonical_score.json");

    assert_eq!(score_from_text.title, score_from_json.title);
    assert_eq!(score_from_text.format_version, score_from_json.format_version);
    assert_eq!(score_from_text.ppq, score_from_json.ppq);
    assert_eq!(score_from_text.sample_rate_hz, score_from_json.sample_rate_hz);
    assert_eq!(score_from_text.instruments.len(), score_from_json.instruments.len());
    assert_eq!(score_from_text.tracks.len(), score_from_json.tracks.len());
    assert_eq!(score_from_text.events.len(), score_from_json.events.len());
    assert_eq!(score_from_text.tempo_events, score_from_json.tempo_events);
    assert_eq!(score_from_text.meter_events, score_from_json.meter_events);

    for (inst_t, inst_j) in score_from_text.instruments.iter().zip(&score_from_json.instruments) {
        assert_eq!(inst_t.id, inst_j.id);
        assert_eq!(inst_t.kind, inst_j.kind);
        assert_eq!(inst_t.profile_id, inst_j.profile_id);
        assert_eq!(inst_t.kit, inst_j.kit);
        assert!((inst_t.gain - inst_j.gain).abs() < 1e-6);
    }

    for (tr_t, tr_j) in score_from_text.tracks.iter().zip(&score_from_json.tracks) {
        assert_eq!(tr_t.id, tr_j.id);
        assert_eq!(tr_t.instrument_id, tr_j.instrument_id);
    }

    for (ev_t, ev_j) in score_from_text.events.iter().zip(&score_from_json.events) {
        assert_eq!(ev_t.track_id, ev_j.track_id);
        assert_eq!(ev_t.start_tick, ev_j.start_tick);
        assert_eq!(ev_t.duration_tick, ev_j.duration_tick);
        assert_eq!(ev_t.kind, ev_j.kind);
        assert_eq!(ev_t.notes, ev_j.notes);
        assert_eq!(ev_t.hit_id, ev_j.hit_id);
        assert!((ev_t.velocity - ev_j.velocity).abs() < 1e-6);
    }

    // 2. Deterministic JSON serialization and roundtrip
    let serialized_json = score_from_text.to_json();
    let score_roundtrip = parse_portable_score_json(&serialized_json).expect("parse roundtripped JSON");
    assert_eq!(score_from_text.title, score_roundtrip.title);
    assert_eq!(score_from_text.events.len(), score_roundtrip.events.len());

    // 3. Render score to signed 16-bit mono PCM and assert exact parity metrics
    let rendered = render_portable_score_to_pcm(&score_from_text, 0).expect("render_portable_score_to_pcm");
    let samples = rendered.pcm_buffer.samples();

    assert_eq!(samples.len(), 96_138, "total_samples mismatch");
    assert_eq!(rendered.clipped_sample_count, 0, "clipped_sample_count mismatch");

    // Convert samples to little-endian i16 bytes for SHA-256
    let mut le_bytes = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        le_bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let pcm_sha256 = sha256_hex(&le_bytes);
    assert_eq!(
        pcm_sha256,
        "8b578c829577edc2f726758e21a53f7a234975253f1b9442402ba7e63bc730aa",
        "PCM little-endian SHA-256 mismatch"
    );

    // Assert first 32 and last 32 samples against expected_render.json
    let expected = SerdeJsonMinimal::parse(&expected_render_content);
    assert_eq!(&samples[..32], &expected.first_32_samples[..]);
    assert_eq!(&samples[samples.len() - 32..], &expected.last_32_samples[..]);
}

// Minimal manual parser for expected_render.json arrays to keep tests zero-dep
struct SerdeJsonMinimal {
    first_32_samples: Vec<i16>,
    last_32_samples: Vec<i16>,
}

impl SerdeJsonMinimal {
    fn parse(text: &str) -> Self {
        fn extract_array(json: &str, key: &str) -> Vec<i16> {
            let key_pos = json.find(key).unwrap_or_else(|| panic!("key {key} not found"));
            let bracket_open = json[key_pos..].find('[').expect("array start") + key_pos;
            let bracket_close = json[bracket_open..].find(']').expect("array end") + bracket_open;
            let inner = &json[bracket_open + 1..bracket_close];
            inner
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<i16>().expect("i16 parse"))
                .collect()
        }

        Self {
            first_32_samples: extract_array(text, "\"first_32_samples\""),
            last_32_samples: extract_array(text, "\"last_32_samples\""),
        }
    }
}
