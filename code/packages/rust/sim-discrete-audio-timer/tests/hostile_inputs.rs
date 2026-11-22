//! Hostile-input and deterministic fuzz test suite for `sim-discrete-audio-timer`.

#![forbid(unsafe_code)]

use sim_discrete_audio_timer::{
    calculate_555_duty_q16, calculate_555_frequency, clamp_sample, diode_clip_sample,
    diode_shape_sample, export_noise_to_pcm_audio, export_system_to_pcm_audio,
    export_timer_to_pcm_audio, lfsr15_step, lfsr17_step, rc_filter_alpha_q16,
    render_noise_audio_into, render_system_audio_into, render_timer_audio_into, scale_gain,
    validate_sample_rate, CoinMechDebounce, DiodeMode, DiscreteArcadeAudio, DiscreteNoiseFilter,
    Timer555Astable, TimerAudioError, MAX_SAMPLES,
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

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn next_i16(&mut self) -> i16 {
        (self.next_u64() & 0xFFFF) as i16
    }

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() & 0xFFFF) as u16
    }
}

#[test]
fn hostile_diode_shape_and_clip_extremes() {
    let samples = [
        i32::MIN,
        i32::MIN + 1,
        -131_072,
        -32_769,
        -32_768,
        -1,
        0,
        1,
        32_767,
        32_768,
        131_071,
        131_072,
        i32::MAX - 1,
        i32::MAX,
    ];
    let thresholds = [i16::MIN, i16::MIN + 1, -1, 0, 1, 16_384, i16::MAX];
    let modes = [
        DiodeMode::HardClip,
        DiodeMode::Symmetrical,
        DiodeMode::Asymmetrical,
    ];

    // Explicit regression checks for reproduced CWE-190 panics
    let _ = diode_shape_sample(i32::MAX, 0, DiodeMode::Symmetrical);
    let _ = diode_shape_sample(i32::MIN, 0, DiodeMode::Symmetrical);
    let _ = diode_shape_sample(i32::MAX, 0, DiodeMode::Asymmetrical);
    let _ = diode_shape_sample(i32::MIN, i16::MIN, DiodeMode::Symmetrical);

    for &sample in &samples {
        for &th in &thresholds {
            let clipped = diode_clip_sample(sample, th);
            let th_abs = i32::from(th.unsigned_abs()).min(i32::from(i16::MAX));
            assert!(i32::from(clipped) >= -32_768 && i32::from(clipped) <= th_abs);

            for &mode in &modes {
                let shaped = diode_shape_sample(sample, th, mode);
                assert!((i16::MIN..=i16::MAX).contains(&shaped));
            }
        }
    }
}

#[test]
fn hostile_555_and_rc_filter_boundaries() {
    assert_eq!(
        calculate_555_frequency(0, 0, 100),
        Err(TimerAudioError::ZeroResistance)
    );
    assert_eq!(
        calculate_555_frequency(1_000, 1_000, 0),
        Err(TimerAudioError::ZeroCapacitance)
    );
    assert!(calculate_555_frequency(u32::MAX, u32::MAX, u64::MAX).is_ok());
    assert!(calculate_555_frequency(1, 0, 1).is_ok());
    assert!(calculate_555_frequency(0, u32::MAX, 1).is_ok());

    assert_eq!(
        calculate_555_duty_q16(0, 0),
        Err(TimerAudioError::ZeroResistance)
    );
    assert!(calculate_555_duty_q16(u32::MAX, u32::MAX).is_ok());
    assert!(calculate_555_duty_q16(u32::MAX, 0).is_ok());
    assert!(calculate_555_duty_q16(0, u32::MAX).is_ok());

    assert_eq!(rc_filter_alpha_q16(0, 44_100), 0);
    assert_eq!(rc_filter_alpha_q16(1_000, 0), 0);
    assert_eq!(rc_filter_alpha_q16(0, 0), 0);
    assert!(rc_filter_alpha_q16(u32::MAX, u32::MAX) <= 65_536);
    assert!(rc_filter_alpha_q16(u32::MAX, 1) <= 65_536);
    assert!(rc_filter_alpha_q16(1, u32::MAX) <= 65_536);

    assert!(validate_sample_rate(0).is_err());
    assert!(validate_sample_rate(u32::MAX).is_err());
    assert!(validate_sample_rate(192_001).is_err());
    assert_eq!(clamp_sample(i64::MIN), i16::MIN);
    assert_eq!(clamp_sample(i64::MAX), i16::MAX);
    assert_eq!(scale_gain(i32::MIN, u32::MAX), i32::MIN);
    assert_eq!(scale_gain(i32::MAX, u32::MAX), i32::MAX);
}

#[test]
fn hostile_mutated_structs_and_export_limits() {
    let mut timer = Timer555Astable::paddle_hit_timer(44_100).unwrap();
    timer.amplitude = i16::MIN;
    timer.gain_q16 = u32::MAX;
    timer.phase_acc = u32::MAX;
    timer.output_high = false;
    let _ = timer.step_timer();
    timer.output_high = true;
    let _ = timer.step_timer();
    let _ = timer.capacitor_voltage_q16();

    timer.sample_rate_hz = 0;
    let mut buf = [0i16; 16];
    assert!(matches!(
        render_timer_audio_into(&mut timer, &mut buf),
        Err(TimerAudioError::InvalidSampleRate { rate: 0 })
    ));
    timer.sample_rate_hz = 44_100;
    assert!(matches!(
        export_timer_to_pcm_audio(&mut timer, MAX_SAMPLES + 1),
        Err(TimerAudioError::InvalidParameter { .. })
    ));

    let mut filter = DiscreteNoiseFilter::explosion_filter(44_100).unwrap();
    filter.rc_state = i32::MIN;
    filter.alpha_q16 = u32::MAX;
    filter.diode_threshold = i16::MIN;
    filter.gain_q16 = u32::MAX;
    let _ = filter.step_filter();
    filter.rc_state = i32::MAX;
    let _ = filter.step_filter();
    filter.sample_rate_hz = u32::MAX;
    assert!(matches!(
        render_noise_audio_into(&mut filter, &mut buf),
        Err(TimerAudioError::InvalidSampleRate { .. })
    ));
    filter.sample_rate_hz = 44_100;
    assert!(matches!(
        export_noise_to_pcm_audio(&mut filter, MAX_SAMPLES + 1),
        Err(TimerAudioError::InvalidParameter { .. })
    ));

    let mut sys = DiscreteArcadeAudio::new_arcade_audio(44_100).unwrap();
    sys.master_gain_q16 = u32::MAX;
    sys.trigger_paddle_tone(u32::MAX);
    sys.trigger_wall_tone(u32::MAX);
    sys.trigger_score_tone(u32::MAX);
    sys.trigger_explosion(u32::MAX);
    let _ = sys.step_arcade_audio();
    sys.sample_rate_hz = 0;
    assert!(matches!(
        render_system_audio_into(&mut sys, &mut buf),
        Err(TimerAudioError::InvalidSampleRate { rate: 0 })
    ));
    sys.sample_rate_hz = 44_100;
    assert!(matches!(
        export_system_to_pcm_audio(&mut sys, MAX_SAMPLES + 1),
        Err(TimerAudioError::InvalidParameter { .. })
    ));

    let mut coin = CoinMechDebounce::new_debouncer(0, 0);
    for _ in 0..16 {
        let _ = coin.step_debounce_sr(255, 2);
        let _ = coin.step_debounce(true);
        coin.insert_test_credit();
    }
    while coin.consume_credit() {}
    assert!(!coin.consume_credit());
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0xD15C_A0D1_0000_0001);
    let mut timer = Timer555Astable::paddle_hit_timer(48_000).unwrap();
    let mut filter = DiscreteNoiseFilter::thud_filter(48_000).unwrap();
    let mut sys = DiscreteArcadeAudio::new_arcade_audio(48_000).unwrap();
    let mut coin = CoinMechDebounce::new_debouncer(1, 5);

    for i in 0..1_000 {
        let s = match i % 5 {
            0 => i32::MIN,
            1 => i32::MAX,
            _ => rng.next_i32(),
        };
        let th = match i % 4 {
            0 => i16::MIN,
            1 => i16::MAX,
            2 => 0,
            _ => rng.next_i16(),
        };
        let mode = match i % 3 {
            0 => DiodeMode::HardClip,
            1 => DiodeMode::Symmetrical,
            _ => DiodeMode::Asymmetrical,
        };
        let _ = diode_clip_sample(s, th);
        let _ = diode_shape_sample(s, th, mode);
        let _ = scale_gain(s, rng.next_u32());

        let r1 = if i % 7 == 0 { 0 } else { rng.next_u32() };
        let r2 = if i % 11 == 0 {
            u32::MAX
        } else {
            rng.next_u32()
        };
        let cap = if i % 13 == 0 { 0 } else { rng.next_u64() };
        let sr = match i % 6 {
            0 => 0,
            1 => u32::MAX,
            _ => rng.next_u32(),
        };
        let _ = calculate_555_frequency(r1, r2, cap);
        let _ = calculate_555_duty_q16(r1, r2);
        let _ = rc_filter_alpha_q16(rng.next_u32(), sr);
        let _ = validate_sample_rate(sr);
        let _ = Timer555Astable::new_astable(r1, r2, cap, sr);
        let _ = DiscreteNoiseFilter::new_noise_filter(i % 2 == 0, rng.next_u32(), sr, th, mode);

        let _ = lfsr17_step(rng.next_u32());
        let _ = lfsr15_step(rng.next_u16());

        timer.amplitude = th;
        timer.gain_q16 = rng.next_u32();
        timer.phase_acc = rng.next_u32();
        let _ = timer.step_timer();
        let _ = timer.capacitor_voltage_q16();

        filter.rc_state = s;
        filter.diode_threshold = th;
        filter.diode_mode = mode;
        filter.gain_q16 = rng.next_u32();
        let _ = filter.step_filter();

        if i % 10 == 0 {
            sys.trigger_paddle_tone(rng.next_u32() % 64);
            sys.trigger_wall_tone(rng.next_u32() % 64);
            sys.trigger_score_tone(rng.next_u32() % 64);
            sys.trigger_explosion(rng.next_u32() % 64);
        }
        sys.master_gain_q16 = rng.next_u32();
        let _ = sys.step_arcade_audio();

        let _ = coin.step_debounce((rng.next_u32() & 1) == 1);
        let _ = coin.step_debounce_sr((rng.next_u32() % 4) as u8, (rng.next_u32() % 4) as u8);
    }
}
