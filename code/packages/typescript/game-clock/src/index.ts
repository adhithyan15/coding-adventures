export {
  ClockError,
  DEFAULT_60HZ_DT_NS,
  DEFAULT_MAX_FRAME_DELTA_NS,
  DEFAULT_MAX_FRAME_STEPS,
  HARD_MAX_DELTA_NS,
  HARD_MAX_FRAME_STEPS,
  MAX_U32,
  MAX_U64,
  NS_PER_SECOND,
  advance_frame,
  from_hz,
  new_clock,
  standard_60hz,
  validate_state,
} from "./game_clock.js";
export type {
  ClockErrorCode,
  ClockStepResult,
  GameClockConfig,
  GameClockState,
  Result,
  ValidationResult,
} from "./game_clock.js";
