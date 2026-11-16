//! # Conformance and Verification Test Suite for `sim-discrete-audio-timer` (GAME15c)
//!
//! Validates:
//! - 555 Astable Multivibrator: frequency formulas, duty cycle, capacitor voltage ramp,
//!   and arcade tone presets (Pong paddle deflection ~480 Hz, wall bounce ~240 Hz, score ~120 Hz / 960 Hz).
//! - CWE-369: Divide-by-zero protection on zero resistance, zero capacitance, and invalid sample rates.
//! - Nyquist frequency protection against ultrasonic configurations.
//! - Deterministic LFSR noise generation: 17-bit and 15-bit Galois/Fibonacci registers with zero-seed guard (CWE-20).
//! - Discrete single-pole RC low-pass filter and diode shaping (symmetrical, asymmetrical, hard clip).
//! - Mechanical coin acceptor switch debouncer: contact bounce glitch rejection shorter than debounce threshold.
//! - Gate-level SR-latch debouncing using `logic-gates` sequential latch.
//! - Saturating credit counter (CWE-190) and credit consumption state machine.
//! - Polyphonic arcade cabinet audio mixing with 64-bit accumulator and hard saturation clamping.
//! - Integration: `audio-mixer-realtime` PCM ring buffer routing and `pcm-audio::PcmAudio` container export.

use audio_mixer_realtime::PcmRingBuffer;
use sim_discrete_audio_timer::{
    calculate_555_duty_q16, calculate_555_frequency, clamp_sample, diode_clip_sample,
    diode_shape_sample, export_noise_to_pcm_audio, export_system_to_pcm_audio,
    export_timer_to_pcm_audio, lfsr15_step, lfsr17_step, rc_filter_alpha_q16,
    render_noise_audio_into, render_noise_to_ring_buffer, render_system_audio_into,
    render_timer_audio_into, render_timer_to_ring_buffer, scale_gain, validate_sample_rate,
    CoinMechDebounce, DiodeMode, DiscreteArcadeAudio, DiscreteNoiseFilter, Timer555Astable,
    TimerAudioError, DEFAULT_LFSR15_SEED, DEFAULT_LFSR17_SEED,
    DEFAULT_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ, MIN_SAMPLE_RATE_HZ, PCM16_MAX, PCM16_MIN,
};

#[test]
fn test_cwe_369_sample_rate_validation() {
    assert!(matches!(
        validate_sample_rate(0),
        Err(TimerAudioError::InvalidSampleRate { rate: 0 })
    ));
    assert!(validate_sample_rate(MIN_SAMPLE_RATE_HZ).is_ok());
    assert!(validate_sample_rate(DEFAULT_SAMPLE_RATE_HZ).is_ok());
    assert!(validate_sample_rate(MAX_SAMPLE_RATE_HZ).is_ok());
    assert!(matches!(
        validate_sample_rate(MAX_SAMPLE_RATE_HZ + 1),
        Err(TimerAudioError::InvalidSampleRate { .. })
    ));

    // Constructing timer with invalid sample rate fails
    assert!(Timer555Astable::new_astable(1000, 10000, 100_000, 0).is_err());
    assert!(DiscreteNoiseFilter::new_noise_filter(
        true,
        250,
        0,
        16384,
        DiodeMode::Symmetrical
    )
    .is_err());
    assert!(DiscreteArcadeAudio::new_arcade_audio(0).is_err());
}

#[test]
fn test_cwe_369_timer_divide_by_zero_guards() {
    // R1 + 2*R2 == 0
    assert!(matches!(
        calculate_555_frequency(0, 0, 100_000),
        Err(TimerAudioError::ZeroResistance)
    ));
    assert!(matches!(
        calculate_555_duty_q16(0, 0),
        Err(TimerAudioError::ZeroResistance)
    ));
    assert!(matches!(
        Timer555Astable::new_astable(0, 0, 100_000, 44_100),
        Err(TimerAudioError::ZeroResistance)
    ));

    // C == 0
    assert!(matches!(
        calculate_555_frequency(1000, 10000, 0),
        Err(TimerAudioError::ZeroCapacitance)
    ));
    assert!(matches!(
        Timer555Astable::new_astable(1000, 10000, 0, 44_100),
        Err(TimerAudioError::ZeroCapacitance)
    ));
}

#[test]
fn test_cwe_20_nyquist_frequency_limit() {
    let sr = 44_100;
    let nyquist = sr / 2; // 22,050 Hz

    // Ultra low resistance / capacitance leading to ultrasonic frequency
    let res = Timer555Astable::new_astable(10, 10, 10, sr);
    assert!(matches!(res, Err(TimerAudioError::FrequencyTooHigh { freq, max }) if freq > nyquist && max == nyquist));
}

#[test]
fn test_555_frequency_and_presets() {
    let sr = 44_100;

    // Paddle Hit: ~480 Hz
    let paddle = Timer555Astable::paddle_hit_timer(sr).expect("paddle timer");
    assert!(paddle.timer_frequency_hz() >= 475 && paddle.timer_frequency_hz() <= 485);
    assert!(paddle.timer_duty_q16() > 32_768); // Duty > 50%

    // Wall Bounce: ~240 Hz
    let wall = Timer555Astable::wall_bounce_timer(sr).expect("wall timer");
    assert!(wall.timer_frequency_hz() >= 235 && wall.timer_frequency_hz() <= 245);

    // Score: ~120 Hz
    let score = Timer555Astable::score_timer(sr).expect("score timer");
    assert!(score.timer_frequency_hz() >= 118 && score.timer_frequency_hz() <= 125);

    // High Score: ~960 Hz
    let high = Timer555Astable::high_score_timer(sr).expect("high score timer");
    assert!(high.timer_frequency_hz() >= 955 && high.timer_frequency_hz() <= 965);
}

#[test]
fn test_555_capacitor_voltage_dynamics() {
    let mut timer = Timer555Astable::paddle_hit_timer(44_100).expect("paddle timer");
    timer.reset_timer();

    let v_init = timer.capacitor_voltage_q16();
    assert!((21_845..=43_690).contains(&v_init));

    // Step through half a period: voltage should stay bounded between V_low and V_high
    for _ in 0..100 {
        timer.step_timer();
        let v = timer.capacitor_voltage_q16();
        assert!((21_845..=43_690).contains(&v));
    }
}

#[test]
fn test_cwe_20_lfsr_seed_guard_and_determinism() {
    // 17-bit seed guard
    assert_eq!(lfsr17_step(0), lfsr17_step(DEFAULT_LFSR17_SEED));
    let s17_1 = lfsr17_step(1);
    let s17_2 = lfsr17_step(s17_1);
    assert_ne!(s17_1, s17_2);
    assert_eq!(lfsr17_step(lfsr17_step(1)), s17_2);

    // 15-bit seed guard
    assert_eq!(lfsr15_step(0), lfsr15_step(DEFAULT_LFSR15_SEED));
    let s15_1 = lfsr15_step(1);
    let s15_2 = lfsr15_step(s15_1);
    assert_ne!(s15_1, s15_2);
    assert_eq!(lfsr15_step(lfsr15_step(1)), s15_2);
}

#[test]
fn test_rc_filter_and_diode_shaping() {
    let sr = 44_100;
    let alpha_zero = rc_filter_alpha_q16(0, sr);
    assert_eq!(alpha_zero, 0);

    let alpha_250 = rc_filter_alpha_q16(250, sr);
    assert!(alpha_250 > 0 && alpha_250 < 65536);

    // Diode clipping
    assert_eq!(diode_clip_sample(10_000, 16_000), 10_000);
    assert_eq!(diode_clip_sample(20_000, 16_000), 16_000);
    assert_eq!(diode_clip_sample(-20_000, 16_000), -16_000);

    // Diode soft shaping
    let shaped_sym = diode_shape_sample(20_000, 16_000, DiodeMode::Symmetrical);
    assert!(shaped_sym > 16_000 && shaped_sym < 20_000);

    let shaped_asym_neg = diode_shape_sample(-20_000, 16_000, DiodeMode::Asymmetrical);
    assert_eq!(shaped_asym_neg, -20_000); // Unclipped on negative swing
}

#[test]
fn test_noise_filter_presets() {
    let sr = 44_100;
    let mut exp = DiscreteNoiseFilter::explosion_filter(sr).expect("explosion");
    let mut thud = DiscreteNoiseFilter::thud_filter(sr).expect("thud");

    let mut exp_samples = [0i16; 64];
    for s in exp_samples.iter_mut() {
        *s = exp.step_filter();
    }
    assert!(exp_samples.iter().any(|&s| s != 0));

    let mut thud_samples = [0i16; 64];
    for s in thud_samples.iter_mut() {
        *s = thud.step_filter();
    }
    assert!(thud_samples.iter().any(|&s| s != 0));
}

#[test]
fn test_coin_mech_debounce_glitch_rejection() {
    let mut debouncer = CoinMechDebounce::new_debouncer(16, 99);

    // Short bounce glitches of 1-5 cycles must be rejected
    for _ in 0..10 {
        assert!(!debouncer.step_debounce(true));
        assert!(!debouncer.step_debounce(true));
        assert!(!debouncer.step_debounce(false)); // bounce back to 0
    }
    assert_eq!(debouncer.current_credits(), 0);
    assert!(!debouncer.is_coin_latched());

    // Solid contact closure for 16 consecutive cycles triggers exactly one credit
    for _ in 0..15 {
        assert!(!debouncer.step_debounce(true));
    }
    assert!(debouncer.step_debounce(true)); // 16th cycle triggers credit!
    assert_eq!(debouncer.current_credits(), 1);
    assert!(debouncer.is_coin_latched());

    // Holding switch closed does NOT trigger additional credits
    for _ in 0..50 {
        assert!(!debouncer.step_debounce(true));
    }
    assert_eq!(debouncer.current_credits(), 1);

    // Releasing switch for 16 cycles resets debounced state
    for _ in 0..16 {
        assert!(!debouncer.step_debounce(false));
    }

    // Next coin closure triggers 2nd credit
    for _ in 0..15 {
        assert!(!debouncer.step_debounce(true));
    }
    assert!(debouncer.step_debounce(true));
    assert_eq!(debouncer.current_credits(), 2);
}

#[test]
fn test_coin_mech_logic_gates_sr_latch() {
    let mut debouncer = CoinMechDebounce::new_debouncer(16, 99);

    // Initial state: NC contact closed (resting switch)
    assert!(!debouncer.step_debounce_sr(0, 1));
    assert_eq!(debouncer.current_credits(), 0);

    // Coin strikes: NO contact closes (Set=1, Reset=0)
    assert!(debouncer.step_debounce_sr(1, 0)); // Credit latched!
    assert_eq!(debouncer.current_credits(), 1);

    // Contact bounces on NO contact while NC is still open (Set=0, Reset=0): latch holds state!
    assert!(!debouncer.step_debounce_sr(0, 0));
    assert_eq!(debouncer.current_credits(), 1);

    // NO contact bounces again (Set=1, Reset=0): does NOT retrigger credit
    assert!(!debouncer.step_debounce_sr(1, 0));
    assert_eq!(debouncer.current_credits(), 1);

    // Coin passes: switch returns to NC resting position (Set=0, Reset=1)
    assert!(!debouncer.step_debounce_sr(0, 1));
    assert_eq!(debouncer.current_credits(), 1);
}

#[test]
fn test_cwe_190_credit_saturation_and_consumption() {
    let mut debouncer = CoinMechDebounce::new_debouncer(16, 5); // Max 5 credits

    for _ in 0..10 {
        debouncer.insert_test_credit();
    }
    // Saturated at 5 (CWE-190 guard)
    assert_eq!(debouncer.current_credits(), 5);

    // Consume credits
    for i in (0..5).rev() {
        assert!(debouncer.consume_credit());
        assert_eq!(debouncer.current_credits(), i as u16);
    }

    // Cannot consume when 0
    assert!(!debouncer.consume_credit());
    assert_eq!(debouncer.current_credits(), 0);
}

#[test]
fn test_cwe_190_audio_saturation_clamping() {
    assert_eq!(clamp_sample(0), 0);
    assert_eq!(clamp_sample(PCM16_MAX as i64), PCM16_MAX);
    assert_eq!(clamp_sample(PCM16_MIN as i64), PCM16_MIN);
    assert_eq!(clamp_sample(100_000), PCM16_MAX);
    assert_eq!(clamp_sample(-100_000), PCM16_MIN);
    assert_eq!(clamp_sample(i64::MAX), PCM16_MAX);
    assert_eq!(clamp_sample(i64::MIN), PCM16_MIN);

    assert_eq!(scale_gain(1000, 65536), 1000);
    assert_eq!(scale_gain(1000, 32768), 500);
}

#[test]
fn test_discrete_arcade_audio_polyphonic_mixing() {
    let mut audio = DiscreteArcadeAudio::new_arcade_audio(44_100).expect("arcade audio");

    // Trigger all sound cues simultaneously
    audio.trigger_paddle_tone(1000);
    audio.trigger_wall_tone(1000);
    audio.trigger_score_tone(1000);
    audio.trigger_explosion(1000);

    let mut buffer = [0i16; 256];
    let rendered = audio.render_audio_frames(&mut buffer);
    assert_eq!(rendered, 256);
    assert!(buffer.iter().any(|&s| s != 0));
}

#[test]
fn test_ring_buffer_routing() {
    let mut timer = Timer555Astable::paddle_hit_timer(44_100).expect("paddle timer");
    let mut ring = PcmRingBuffer::<64>::empty();

    let routed = render_timer_to_ring_buffer(&mut timer, &mut ring, 32).expect("route timer");
    assert_eq!(routed, 32);
    assert_eq!(ring.len(), 32);

    let mut noise = DiscreteNoiseFilter::explosion_filter(44_100).expect("noise");
    let routed_noise =
        render_noise_to_ring_buffer(&mut noise, &mut ring, 32).expect("route noise");
    assert_eq!(routed_noise, 32);
    assert_eq!(ring.len(), 64);

    // Overrun should fail with MixerIntegrationError
    assert!(matches!(
        render_timer_to_ring_buffer(&mut timer, &mut ring, 1),
        Err(TimerAudioError::MixerIntegrationError)
    ));
}

#[test]
fn test_zero_alloc_buffer_rendering() {
    let mut timer = Timer555Astable::paddle_hit_timer(44_100).expect("paddle timer");
    let mut buf = [0i16; 128];
    assert_eq!(render_timer_audio_into(&mut timer, &mut buf).unwrap(), 128);

    let mut noise = DiscreteNoiseFilter::explosion_filter(44_100).expect("noise");
    assert_eq!(render_noise_audio_into(&mut noise, &mut buf).unwrap(), 128);

    let mut system = DiscreteArcadeAudio::new_arcade_audio(44_100).expect("system");
    system.trigger_paddle_tone(100);
    assert_eq!(render_system_audio_into(&mut system, &mut buf).unwrap(), 128);
}

#[test]
fn test_export_to_pcm_audio() {
    let mut timer = Timer555Astable::paddle_hit_timer(44_100).expect("paddle timer");
    let pcm = export_timer_to_pcm_audio(&mut timer, 1000).expect("export timer");
    assert_eq!(pcm.samples().len(), 1000);
    assert_eq!(pcm.format().sample_rate_hz(), 44_100.0);

    let mut noise = DiscreteNoiseFilter::explosion_filter(44_100).expect("noise");
    let pcm_noise = export_noise_to_pcm_audio(&mut noise, 500).expect("export noise");
    assert_eq!(pcm_noise.samples().len(), 500);

    let mut system = DiscreteArcadeAudio::new_arcade_audio(44_100).expect("system");
    let pcm_system = export_system_to_pcm_audio(&mut system, 800).expect("export system");
    assert_eq!(pcm_system.samples().len(), 800);
}
