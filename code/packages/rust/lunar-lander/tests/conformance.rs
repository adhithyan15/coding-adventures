//! # Conformance and Adversarial Test Suite for Lunar Lander
//!
//! Validates:
//! - Full execution of all 39 test cases in `code/specs/fixtures/lunar-lander-v1/cases.json`
//! - Exact multi-turn trajectory integration parity across all flight phases
//! - Ground touchdown solver contact time interpolation and terminal impact velocity
//! - Proper fuel refund calculation without phantom fuel restoration
//! - All refusal arms and canonical error precedence
//! - CWE-369 (Division by Zero) and CWE-190 (Integer Overflow) adversarial immunization
//! - `paint-vm-ascii` (`P2D02`) declarative telemetry dashboard layout and gauge rendering

use lunar_lander::{
    isqrt_u128, new_flight, render_telemetry, render_telemetry_ascii, step_burn, telemetry_scene,
    to_paint_scene, BurnCommand, FlightStatus, LanderError, LanderState, LandingOutcome,
    DRY_MASS_LBS, INITIAL_ALTITUDE_MFT, INITIAL_FUEL_LBS, INITIAL_VELOCITY_MFT_S, MAX_ALTITUDE_MFT,
    MAX_BURN_RATE_LBS_S, MAX_DURATION_S, MAX_TIME_SECONDS, MAX_VELOCITY_MFT_S, MIN_VELOCITY_MFT_S,
};
use serde::Deserialize;

const CORPUS_JSON: &str = include_str!("../../../../specs/fixtures/lunar-lander-v1/cases.json");

#[derive(Debug, Deserialize)]
struct CorpusFile {
    version: u32,
    suite: String,
    cases: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
struct CorpusCase {
    id: String,
    #[allow(dead_code)]
    description: String,
    initial_state: JsonLanderState,
    #[serde(default)]
    turns: Vec<JsonTurn>,
    command: Option<JsonBurnCommand>,
    expected_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonLanderState {
    time_seconds: u32,
    altitude_mft: i64,
    velocity_mft_s: i64,
    fuel_lbs: u32,
    dry_mass_lbs: u32,
    status: String,
    impact_velocity_mft_s: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct JsonBurnCommand {
    burn_rate_lbs_s: u32,
    duration_seconds: u32,
}

#[derive(Debug, Deserialize)]
struct JsonTurn {
    turn: u32,
    command: JsonBurnCommand,
    expected_state: JsonLanderState,
}

fn parse_status(s: &str) -> FlightStatus {
    match s {
        "InFlight" => FlightStatus::InFlight,
        "Landed(Perfect)" => FlightStatus::Landed(LandingOutcome::Perfect),
        "Landed(Hard)" => FlightStatus::Landed(LandingOutcome::Hard),
        "Landed(Crash)" => FlightStatus::Landed(LandingOutcome::Crash),
        "Landed(Fatal)" => FlightStatus::Landed(LandingOutcome::Fatal),
        "Landed(Crater)" => FlightStatus::Landed(LandingOutcome::Crater),
        other => panic!("unknown flight status in fixture: {other}"),
    }
}

fn parse_error(s: &str) -> LanderError {
    match s {
        "FlightAlreadyEnded" => LanderError::FlightAlreadyEnded,
        "ExcessiveBurnRate" => LanderError::ExcessiveBurnRate,
        "InvalidDuration" => LanderError::InvalidDuration,
        "ArithmeticOverflow" => LanderError::ArithmeticOverflow,
        other => panic!("unknown expected error in fixture: {other}"),
    }
}

fn to_rust_state(json: &JsonLanderState) -> LanderState {
    LanderState {
        time_seconds: json.time_seconds,
        altitude_mft: json.altitude_mft,
        velocity_mft_s: json.velocity_mft_s,
        fuel_lbs: json.fuel_lbs,
        dry_mass_lbs: json.dry_mass_lbs,
        status: parse_status(&json.status),
        impact_velocity_mft_s: json.impact_velocity_mft_s,
    }
}

fn to_rust_cmd(json: &JsonBurnCommand) -> BurnCommand {
    BurnCommand {
        burn_rate_lbs_s: json.burn_rate_lbs_s,
        duration_seconds: json.duration_seconds,
    }
}

#[test]
fn test_all_39_corpus_cases() {
    let corpus: CorpusFile =
        serde_json::from_str(CORPUS_JSON).expect("cases.json should parse cleanly");
    assert_eq!(corpus.version, 1);
    assert_eq!(corpus.suite, "lunar_lander_deterministic_flight");
    assert_eq!(corpus.cases.len(), 39, "Corpus must have exactly 39 cases");

    for case in &corpus.cases {
        let initial_state = to_rust_state(&case.initial_state);

        if !case.turns.is_empty() {
            let mut state = initial_state;
            for turn in &case.turns {
                let cmd = to_rust_cmd(&turn.command);
                let expected = to_rust_state(&turn.expected_state);

                let validation = lunar_lander::validate_burn(&state, &cmd);
                assert!(
                    validation.is_ok(),
                    "Case {} turn {} validation failed: {:?}",
                    case.id,
                    turn.turn,
                    validation.err()
                );

                let next = step_burn(&state, &cmd).unwrap_or_else(|e| {
                    panic!("Case {} turn {} failed: {:?}", case.id, turn.turn, e)
                });

                assert_eq!(
                    next.time_seconds, expected.time_seconds,
                    "Case {} turn {} time mismatch",
                    case.id, turn.turn
                );
                assert_eq!(
                    next.altitude_mft, expected.altitude_mft,
                    "Case {} turn {} altitude mismatch",
                    case.id, turn.turn
                );
                assert_eq!(
                    next.velocity_mft_s, expected.velocity_mft_s,
                    "Case {} turn {} velocity mismatch",
                    case.id, turn.turn
                );
                assert_eq!(
                    next.fuel_lbs, expected.fuel_lbs,
                    "Case {} turn {} fuel mismatch",
                    case.id, turn.turn
                );
                assert_eq!(
                    next.dry_mass_lbs, expected.dry_mass_lbs,
                    "Case {} turn {} dry mass mismatch",
                    case.id, turn.turn
                );
                assert_eq!(
                    next.status, expected.status,
                    "Case {} turn {} status mismatch",
                    case.id, turn.turn
                );
                assert_eq!(
                    next.impact_velocity_mft_s, expected.impact_velocity_mft_s,
                    "Case {} turn {} impact velocity mismatch",
                    case.id, turn.turn
                );

                state = next;
            }
        } else if let Some(ref cmd_json) = case.command {
            let cmd = to_rust_cmd(cmd_json);
            let expected_err_str = case
                .expected_error
                .as_deref()
                .expect("refusal case must specify expected_error");
            let expected_err = parse_error(expected_err_str);

            let val_res = lunar_lander::validate_burn(&initial_state, &cmd);
            assert_eq!(
                val_res,
                Err(expected_err),
                "Case {} validate_burn error mismatch",
                case.id
            );

            let step_res = step_burn(&initial_state, &cmd);
            assert_eq!(
                step_res,
                Err(expected_err),
                "Case {} step_burn error mismatch",
                case.id
            );
        } else {
            panic!("Case {} has neither turns nor command", case.id);
        }
    }
}

#[test]
fn test_initial_flight_invariants() {
    let flight = new_flight();
    assert_eq!(flight.time_seconds, 0);
    assert_eq!(flight.altitude_mft, INITIAL_ALTITUDE_MFT);
    assert_eq!(flight.velocity_mft_s, INITIAL_VELOCITY_MFT_S);
    assert_eq!(flight.fuel_lbs, INITIAL_FUEL_LBS);
    assert_eq!(flight.dry_mass_lbs, DRY_MASS_LBS);
    assert_eq!(flight.status, FlightStatus::InFlight);
    assert_eq!(flight.impact_velocity_mft_s, None);
}

#[test]
fn test_isqrt_u128_exactness() {
    assert_eq!(isqrt_u128(0), 0);
    assert_eq!(isqrt_u128(1), 1);
    assert_eq!(isqrt_u128(2), 1);
    assert_eq!(isqrt_u128(3), 1);
    assert_eq!(isqrt_u128(4), 2);
    assert_eq!(isqrt_u128(15), 3);
    assert_eq!(isqrt_u128(16), 4);
    assert_eq!(isqrt_u128(10_274_400_000_000), 3_205_370);
    assert_eq!(
        isqrt_u128(u128::from(u64::MAX) * u128::from(u64::MAX)),
        u128::from(u64::MAX)
    );
    assert_eq!(isqrt_u128(u128::MAX), u128::from(u64::MAX));
}

#[test]
fn test_adversarial_cwe369_division_by_zero_prevention() {
    let hostile_state = LanderState {
        time_seconds: 0,
        altitude_mft: 100_000_000,
        velocity_mft_s: -2_000_000,
        fuel_lbs: 0,
        dry_mass_lbs: 0, // Corrupted dry mass!
        status: FlightStatus::InFlight,
        impact_velocity_mft_s: None,
    };
    let cmd = BurnCommand {
        burn_rate_lbs_s: 100,
        duration_seconds: 5,
    };
    assert_eq!(
        lunar_lander::validate_burn(&hostile_state, &cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    assert_eq!(
        step_burn(&hostile_state, &cmd),
        Err(LanderError::ArithmeticOverflow)
    );
}

#[test]
fn test_adversarial_cwe190_integer_overflow_prevention() {
    let mut state = new_flight();
    let valid_cmd = BurnCommand {
        burn_rate_lbs_s: 100,
        duration_seconds: 5,
    };

    // Hostile altitude boundaries
    state.altitude_mft = i64::MAX;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.altitude_mft = i64::MIN;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.altitude_mft = MAX_ALTITUDE_MFT + 1;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );

    // Hostile velocity boundaries
    state = new_flight();
    state.velocity_mft_s = i64::MAX;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.velocity_mft_s = i64::MIN;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.velocity_mft_s = MIN_VELOCITY_MFT_S - 1;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.velocity_mft_s = MAX_VELOCITY_MFT_S + 1;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );

    // Hostile fuel boundaries
    state = new_flight();
    state.fuel_lbs = INITIAL_FUEL_LBS + 1;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.fuel_lbs = u32::MAX;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );

    // Hostile time boundaries
    state = new_flight();
    state.time_seconds = MAX_TIME_SECONDS + 1;
    assert_eq!(
        lunar_lander::validate_burn(&state, &valid_cmd),
        Err(LanderError::ArithmeticOverflow)
    );
    state.time_seconds = MAX_TIME_SECONDS;
    let add_one_s = BurnCommand {
        burn_rate_lbs_s: 0,
        duration_seconds: 1,
    };
    assert_eq!(
        lunar_lander::validate_burn(&state, &add_one_s),
        Err(LanderError::ArithmeticOverflow)
    );

    // Hostile command bounds
    state = new_flight();
    let excessive_burn = BurnCommand {
        burn_rate_lbs_s: MAX_BURN_RATE_LBS_S + 1,
        duration_seconds: 5,
    };
    assert_eq!(
        lunar_lander::validate_burn(&state, &excessive_burn),
        Err(LanderError::ExcessiveBurnRate)
    );
    let zero_duration = BurnCommand {
        burn_rate_lbs_s: 50,
        duration_seconds: 0,
    };
    assert_eq!(
        lunar_lander::validate_burn(&state, &zero_duration),
        Err(LanderError::InvalidDuration)
    );
    let excessive_duration = BurnCommand {
        burn_rate_lbs_s: 50,
        duration_seconds: MAX_DURATION_S + 1,
    };
    assert_eq!(
        lunar_lander::validate_burn(&state, &excessive_duration),
        Err(LanderError::InvalidDuration)
    );
    let u32_max_duration = BurnCommand {
        burn_rate_lbs_s: 50,
        duration_seconds: u32::MAX,
    };
    assert_eq!(
        lunar_lander::validate_burn(&state, &u32_max_duration),
        Err(LanderError::InvalidDuration)
    );
}

#[test]
fn test_paint_vm_ascii_telemetry_hud() {
    let state = new_flight();
    let scene = to_paint_scene(&state, 0);
    assert_eq!(scene.width, 76.0);
    assert_eq!(scene.height, 5.0);

    let hud = render_telemetry_ascii(&state, 0);
    let lines: Vec<&str> = hud.lines().collect();
    assert_eq!(lines.len(), 5, "Telemetry HUD must contain exactly 5 lines");

    for (idx, line) in lines.iter().enumerate() {
        assert_eq!(
            line.chars().count(),
            76,
            "Line {} length mismatch: {:?}",
            idx,
            line
        );
    }

    assert!(hud.contains("LUNAR LANDER TELEMETRY"));
    assert!(hud.contains("TIME:"));
    assert!(hud.contains("ALTITUDE:"));
    assert!(hud.contains("VELOCITY:"));
    assert!(hud.contains("FUEL:"));
    assert!(hud.contains("BURN:"));
    assert!(hud.contains("RATE:"));
    assert!(hud.contains("DESCENT"));
    assert!(hud.contains('█'));

    // Check mid-flight state with partial gauge fill containing both █ and ░
    let mut mid_state = state;
    mid_state.fuel_lbs = 8_250;
    mid_state.altitude_mft = 60_000_000;
    let hud_mid = render_telemetry_ascii(&mid_state, 100);
    assert!(hud_mid.contains('█'));
    assert!(hud_mid.contains('░'));

    // Check convenience helpers
    let scene2 = telemetry_scene(&state);
    assert_eq!(scene2.width, 76.0);
    let hud2 = render_telemetry(&state);
    assert_eq!(hud2, hud);
}

#[test]
fn test_telemetry_states_and_outcomes() {
    let mut state = new_flight();

    // InFlight ascent
    state.velocity_mft_s = 50_000;
    let hud_ascent = render_telemetry_ascii(&state, 100);
    assert!(hud_ascent.contains("ASCENT"));
    assert!(hud_ascent.contains('▲'));

    // InFlight hover
    state.velocity_mft_s = 0;
    let hud_hover = render_telemetry_ascii(&state, 50);
    assert!(hud_hover.contains("HOVER"));
    assert!(hud_hover.contains('◆'));

    // Landed outcomes
    let outcomes = [
        (LandingOutcome::Perfect, "PERFECT"),
        (LandingOutcome::Hard, "HARD"),
        (LandingOutcome::Crash, "CRASH"),
        (LandingOutcome::Fatal, "FATAL"),
        (LandingOutcome::Crater, "CRATER"),
    ];

    for (outcome, text) in outcomes {
        state.status = FlightStatus::Landed(outcome);
        state.impact_velocity_mft_s = Some(-1000);
        let hud_landed = render_telemetry_ascii(&state, 0);
        assert!(
            hud_landed.contains("STATUS:"),
            "Landed HUD missing STATUS: label"
        );
        assert!(
            hud_landed.contains(text),
            "Landed HUD missing outcome text {}",
            text
        );
    }
}
