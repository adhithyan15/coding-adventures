//! Golden parity checks against the MUS07 Python reference implementation.

use percussion_kit::{
    all_drum_kits, all_standard_hit_ids, canonical_hit_id, get_drum_kit, get_percussion_voice,
    render_percussion_hit, HitRenderOptions,
};

#[test]
fn test_all_15_standard_starter_hit_ids_present() {
    let hit_ids = all_standard_hit_ids();
    assert_eq!(hit_ids.len(), 15);

    let expected = [
        "kick",
        "snare",
        "closed_hihat",
        "open_hihat",
        "pedal_hihat",
        "low_tom",
        "mid_tom",
        "high_tom",
        "crash",
        "ride",
        "clap",
        "cowbell",
        "tambourine",
        "shaker",
        "woodblock",
    ];

    assert_eq!(hit_ids, expected);

    // Verify all 15 voices resolve successfully in standard kit
    for id in expected {
        let voice = get_percussion_voice(id, None).expect("voice should resolve");
        assert!(!voice.id.is_empty());
        assert!(!voice.display_name.is_empty());
        assert!(voice.gain > 0.0 && voice.gain <= 1.0);
    }
}

#[test]
fn test_all_drum_kits_contains_standard_kit() {
    let kits = all_drum_kits();
    assert_eq!(kits.len(), 1);
    assert_eq!(kits[0].id, "standard_kit");
    assert_eq!(kits[0].display_name, "Standard Kit");
    assert_eq!(get_drum_kit("standard_kit").unwrap().id, "standard_kit");
}

#[test]
fn test_voice_specific_acoustic_characteristics() {
    // 1. Woodblock: dry, wooden, hollow resonance with very short decay (< 0.10s)
    let wb = get_percussion_voice("woodblock", None).unwrap();
    assert_eq!(wb.family, "woodblock");
    assert!(wb.envelope.rendered_duration_seconds(0.02).unwrap() < 0.10);
    assert!(wb.click_amplitude > 0.5);

    // 2. Cowbell: dual clashing metallic modes (587 Hz and 845 Hz)
    let cb = get_percussion_voice("cowbell", None).unwrap();
    assert_eq!(cb.family, "hand_percussion");
    let freqs: Vec<f64> = cb.modes.iter().map(|m| m.frequency_hz).collect();
    assert!(freqs.contains(&587.0));
    assert!(freqs.contains(&845.0));

    // 3. Shaker: soft attack, stochastic grains with high noise amplitude
    let sh = get_percussion_voice("shaker", None).unwrap();
    assert_eq!(sh.family, "hand_percussion");
    assert!(sh.envelope.attack_seconds > 0.005);
    let sh_noise = sh.noise_profile.expect("shaker noise profile");
    assert!(sh_noise.amplitude >= 0.90);

    // 4. Clap: hand cavity resonance modes and snappy decay
    let cl = get_percussion_voice("clap", None).unwrap();
    assert_eq!(cl.family, "hand_percussion");
    assert!(cl.modes.len() >= 3);
    assert!(cl.noise_profile.is_some());

    // 5. Tambourine: high zil shimmering modes
    let tb = get_percussion_voice("tambourine", None).unwrap();
    assert_eq!(tb.family, "hand_percussion");
    assert!(tb.modes.iter().any(|m| m.frequency_hz > 9000.0));

    // 6. Tom frequency progression: low < mid < high
    let low_freq = get_percussion_voice("low_tom", None).unwrap().modes[0].frequency_hz;
    let mid_freq = get_percussion_voice("mid_tom", None).unwrap().modes[0].frequency_hz;
    let high_freq = get_percussion_voice("high_tom", None).unwrap().modes[0].frequency_hz;
    assert!(low_freq < mid_freq && mid_freq < high_freq);

    // 7. Crash cymbal rings longer than ride cymbal
    let crash_dur = get_percussion_voice("crash", None)
        .unwrap()
        .envelope
        .rendered_duration_seconds(0.0)
        .unwrap();
    let ride_dur = get_percussion_voice("ride", None)
        .unwrap()
        .envelope
        .rendered_duration_seconds(0.0)
        .unwrap();
    assert!(crash_dur > ride_dur);
}

#[test]
fn test_golden_kick_parity() {
    let opts = HitRenderOptions::new()
        .sample_rate_hz(8000.0)
        .amplitude(1.0);

    let res = render_percussion_hit("kick", 0.01, &opts).unwrap();
    assert_eq!(res.floating_samples.sample_count(), 1840);
    assert_eq!(res.pcm_buffer.sample_count(), 1840);

    let expected_floats = [
        0.2580922271683029,
        0.25029885743455077,
        0.3094996672572265,
        0.2776634891757474,
        0.347839984517213,
    ];
    let expected_pcm = [8457, 8202, 10141, 9098, 11398];

    for i in 0..5 {
        let actual = res.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-9,
            "kick float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            res.pcm_buffer.samples()[i],
            expected_pcm[i],
            "kick pcm sample {i} mismatch"
        );
    }
}

#[test]
fn test_golden_snare_parity() {
    let opts = HitRenderOptions::new()
        .sample_rate_hz(8000.0)
        .amplitude(1.0);

    let res = render_percussion_hit("snare", 0.01, &opts).unwrap();
    assert_eq!(res.floating_samples.sample_count(), 1520);
    assert_eq!(res.pcm_buffer.sample_count(), 1520);

    let expected_floats = [
        -0.0809619415993958,
        -0.23226659961116677,
        0.5727491910288124,
        0.609137376179235,
        0.02664838511990162,
    ];
    let expected_pcm = [-2653, -7611, 18767, 19960, 873];

    for i in 0..5 {
        let actual = res.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-9,
            "snare float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            res.pcm_buffer.samples()[i],
            expected_pcm[i],
            "snare pcm sample {i} mismatch"
        );
    }
}

#[test]
fn test_golden_closed_hihat_parity() {
    let opts = HitRenderOptions::new()
        .sample_rate_hz(8000.0)
        .amplitude(1.0);

    let res = render_percussion_hit("closed_hihat", 0.01, &opts).unwrap();
    assert_eq!(res.floating_samples.sample_count(), 560);
    assert_eq!(res.pcm_buffer.sample_count(), 560);

    let expected_floats = [
        0.20846074260352912,
        0.13370076839158718,
        0.025007470340133297,
        -0.023519267225397857,
        0.14762376473902417,
    ];
    let expected_pcm = [6831, 4381, 819, -771, 4837];

    for i in 0..5 {
        let actual = res.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-9,
            "closed_hihat float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            res.pcm_buffer.samples()[i],
            expected_pcm[i],
            "closed_hihat pcm sample {i} mismatch"
        );
    }
}

#[test]
fn test_golden_open_hihat_parity() {
    let opts = HitRenderOptions::new()
        .sample_rate_hz(8000.0)
        .amplitude(1.0);

    let res = render_percussion_hit("open_hihat", 0.01, &opts).unwrap();
    assert_eq!(res.floating_samples.sample_count(), 2640);
    assert_eq!(res.pcm_buffer.sample_count(), 2640);

    let expected_floats = [
        -0.08597682233566663,
        0.11142398343636203,
        -0.14800065900707124,
        0.4101268456344719,
        0.27875265597562177,
    ];
    let expected_pcm = [-2817, 3651, -4850, 13439, 9134];

    for i in 0..5 {
        let actual = res.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-9,
            "open_hihat float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            res.pcm_buffer.samples()[i],
            expected_pcm[i],
            "open_hihat pcm sample {i} mismatch"
        );
    }
}

#[test]
fn test_golden_woodblock_parity() {
    let opts = HitRenderOptions::new()
        .sample_rate_hz(8000.0)
        .amplitude(1.0);

    let res = render_percussion_hit("woodblock", 0.01, &opts).unwrap();
    assert_eq!(res.floating_samples.sample_count(), 520);
    assert_eq!(res.pcm_buffer.sample_count(), 520);

    let expected_floats = [
        0.21257309576241645,
        0.5154918568547093,
        0.5399253623981772,
        0.4487473592632752,
        0.3222788749643938,
    ];
    let expected_pcm = [6965, 16891, 17692, 14704, 10560];

    for i in 0..5 {
        let actual = res.floating_samples.samples()[i];
        let diff = (actual - expected_floats[i]).abs();
        assert!(
            diff < 1e-9,
            "woodblock float sample {i} mismatch: got {actual}, expected {}",
            expected_floats[i]
        );
        assert_eq!(
            res.pcm_buffer.samples()[i],
            expected_pcm[i],
            "woodblock pcm sample {i} mismatch"
        );
    }
}

#[test]
fn test_canonical_alias_resolution() {
    assert_eq!(canonical_hit_id("bd"), Some("kick"));
    assert_eq!(canonical_hit_id("bass_drum"), Some("kick"));
    assert_eq!(canonical_hit_id("sd"), Some("snare"));
    assert_eq!(canonical_hit_id("snare-drum"), Some("snare"));
    assert_eq!(canonical_hit_id("ch"), Some("closed_hihat"));
    assert_eq!(canonical_hit_id("closed-hat"), Some("closed_hihat"));
    assert_eq!(canonical_hit_id("oh"), Some("open_hihat"));
    assert_eq!(canonical_hit_id("open-hat"), Some("open_hihat"));
    assert_eq!(canonical_hit_id("ph"), Some("pedal_hihat"));
    assert_eq!(canonical_hit_id("foot_hihat"), Some("pedal_hihat"));
    assert_eq!(canonical_hit_id("lt"), Some("low_tom"));
    assert_eq!(canonical_hit_id("floor_tom"), Some("low_tom"));
    assert_eq!(canonical_hit_id("mt"), Some("mid_tom"));
    assert_eq!(canonical_hit_id("ht"), Some("high_tom"));
    assert_eq!(canonical_hit_id("cc"), Some("crash"));
    assert_eq!(canonical_hit_id("crash_cymbal"), Some("crash"));
    assert_eq!(canonical_hit_id("rc"), Some("ride"));
    assert_eq!(canonical_hit_id("ride_cymbal"), Some("ride"));
    assert_eq!(canonical_hit_id("rim"), Some("rimshot"));
    assert_eq!(canonical_hit_id("side_stick"), Some("rimshot"));
    assert_eq!(canonical_hit_id("rs"), Some("rimshot"));
    assert_eq!(canonical_hit_id("cp"), Some("clap"));
    assert_eq!(canonical_hit_id("handclap"), Some("clap"));
    assert_eq!(canonical_hit_id("cb"), Some("cowbell"));
    assert_eq!(canonical_hit_id("tb"), Some("tambourine"));
    assert_eq!(canonical_hit_id("tamb"), Some("tambourine"));
    assert_eq!(canonical_hit_id("sh"), Some("shaker"));
    assert_eq!(canonical_hit_id("wb"), Some("woodblock"));
    assert_eq!(canonical_hit_id("wood_block"), Some("woodblock"));

    // Also verify alias lookup through get_percussion_voice
    let v_bd = get_percussion_voice("bd", None).unwrap();
    assert_eq!(v_bd.id, "kick_naive");

    let v_ch = get_percussion_voice("closed-hat", None).unwrap();
    assert_eq!(v_ch.id, "closed_hihat_naive");

    let v_rim = get_percussion_voice("side_stick", None).unwrap();
    assert_eq!(v_rim.id, "rimshot_naive");
}
