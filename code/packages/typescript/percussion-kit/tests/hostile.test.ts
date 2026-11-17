import assert from "node:assert/strict";
import test from "node:test";

import {
  CHOKE_RELEASE_RAMP_SECONDS,
  DrumKitProfile,
  getDrumKit,
  getPercussionVoice,
  InvalidParamError,
  MAX_HIT_DURATION_SECONDS,
  MAX_HITS,
  MAX_PERCUSSION_MODES,
  MAX_SAMPLE_COUNT,
  MAX_SAMPLE_RATE_HZ,
  MAX_SEQUENCE_EVENTS,
  PercussionEnvelope,
  PercussionHitEvent,
  PercussionMode,
  PercussionNoiseProfile,
  PercussionSignal,
  PercussionVoiceProfile,
  renderPercussionHit,
  renderPercussionSequence,
  SplitMix64,
  validateDuration,
  validateHitCountCeiling,
  validateModeCountCeiling,
  validateNonNegative,
  validateNonNegativeInt,
  validatePositive,
  validatePositiveInt,
  validateSafeInteger,
  validateSampleCountCeiling,
  validateSampleRate,
  validateSequenceEventCountCeiling,
  validateUnit,
} from "../src/index.js";

test("CWE-20 & CWE-1321: Prototype pollution keys and invalid types rejected", () => {
  assert.throws(() => getPercussionVoice("__proto__"), /Unknown percussion voice/);
  assert.throws(() => getPercussionVoice("constructor"), /Unknown percussion voice/);
  assert.throws(() => getDrumKit("__proto__"), /Unknown drum kit/);
  assert.throws(() => getDrumKit("constructor"), /Unknown drum kit/);

  assert.throws(() => getPercussionVoice(123 as unknown as string), /hitId must be a non-empty string/);
  assert.throws(() => getPercussionVoice(""), /hitId must be a non-empty string/);
  assert.throws(() => getDrumKit(123 as unknown as string), /kit id must be a non-empty string/);
  assert.throws(() => getDrumKit(""), /kit id must be a non-empty string/);
});

test("CWE-400: Mode count bounds and resource limits", () => {
  const mode = new PercussionMode(100.0, 1.0, 0.1);
  const env = new PercussionEnvelope(0.001, 0.01, 0.1);

  // 33 modes rejected by PercussionVoiceProfile constructor
  const tooManyModes: PercussionMode[] = [];
  for (let i = 0; i <= MAX_PERCUSSION_MODES; i += 1) {
    tooManyModes.push(mode);
  }

  assert.throws(
    () => new PercussionVoiceProfile("test", "Test", "misc", 0.5, tooManyModes, null, env, 0.1),
    /exceeds MAX_PERCUSSION_MODES/,
  );

  // renderPercussionHit rejects maxModeCount > MAX_PERCUSSION_MODES and NaN
  assert.throws(
    () => renderPercussionHit("kick", 0.05, { maxModeCount: MAX_PERCUSSION_MODES + 1 }),
    /exceeds maximum allowable/,
  );
  assert.throws(
    () => renderPercussionHit("kick", 0.05, { maxModeCount: NaN }),
    /maxModeCount must be a safe integer/,
  );

  // renderPercussionSequence rejects maxModeCount > MAX_PERCUSSION_MODES and NaN
  const events = [new PercussionHitEvent("kick", 0.0, 0.05, 1.0)];
  assert.throws(
    () => renderPercussionSequence(events, { maxModeCount: MAX_PERCUSSION_MODES + 1 }),
    /exceeds maximum allowable/,
  );
  assert.throws(
    () => renderPercussionSequence(events, { maxModeCount: NaN }),
    /maxModeCount must be a safe integer/,
  );
});

test("CWE-400: Duration upper bounds on hit and sequence", () => {
  const tooLong = MAX_HIT_DURATION_SECONDS + 0.1;

  // renderPercussionHit rejects duration > MAX_HIT_DURATION_SECONDS
  assert.throws(
    () => renderPercussionHit("kick", tooLong),
    /exceeds maximum allowable duration/,
  );

  // PercussionSignal rejects duration > MAX_HIT_DURATION_SECONDS
  const kick = getPercussionVoice("kick");
  assert.throws(
    () => new PercussionSignal(kick, tooLong, 1.0, 44_100.0),
    /exceeds maximum allowable duration/,
  );

  // PercussionHitEvent rejects duration > MAX_HIT_DURATION_SECONDS
  assert.throws(
    () => new PercussionHitEvent("kick", 0.0, tooLong),
    /exceeds maximum allowable duration/,
  );
});

test("CWE-400: Sequence event count limits", () => {
  // Option maxSequenceEvents exceeds MAX_SEQUENCE_EVENTS or is NaN
  assert.throws(
    () => renderPercussionSequence([], { maxSequenceEvents: MAX_SEQUENCE_EVENTS + 1 }),
    /exceeds maximum allowable/,
  );
  assert.throws(
    () => renderPercussionSequence([], { maxSequenceEvents: NaN }),
    /maxSequenceEvents must be a safe integer/,
  );

  // Sequence exceeds configured limit
  const events = [
    new PercussionHitEvent("kick", 0.0, 0.05, 1.0),
    new PercussionHitEvent("kick", 0.1, 0.05, 1.0),
    new PercussionHitEvent("kick", 0.2, 0.05, 1.0),
  ];

  assert.throws(
    () => renderPercussionSequence(events, { maxSequenceEvents: 2 }),
    /Sequence contains 3 events, exceeding maxSequenceEvents=2/,
  );
});

test("CWE-20: Finite number and range validations across components", () => {
  // PercussionMode
  assert.throws(() => new PercussionMode(NaN, 1.0, 0.1), /frequencyHz/);
  assert.throws(() => new PercussionMode(Infinity, 1.0, 0.1), /frequencyHz/);
  assert.throws(() => new PercussionMode(-100.0, 1.0, 0.1), /frequencyHz/);
  assert.throws(() => new PercussionMode(100.0, NaN, 0.1), /amplitude/);
  assert.throws(() => new PercussionMode(100.0, -0.5, 0.1), /amplitude/);
  assert.throws(() => new PercussionMode(100.0, 1.0, NaN), /decaySeconds/);
  assert.throws(() => new PercussionMode(100.0, 1.0, -0.1), /decaySeconds/);

  // PercussionEnvelope
  assert.throws(() => new PercussionEnvelope(NaN, 0.01, 0.1), /attackSeconds/);
  assert.throws(() => new PercussionEnvelope(-0.01, 0.01, 0.1), /attackSeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, NaN, 0.1), /holdSeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, -0.01, 0.1), /holdSeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, 0.01, NaN), /decaySeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, 0.01, -0.1), /decaySeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, 0.01, 0.0), /decaySeconds must be > 0.0/);

  // PercussionHitEvent
  assert.throws(() => new PercussionHitEvent("", 0.0, 0.1, 1.0), /hitId must be a non-empty string/);
  assert.throws(() => new PercussionHitEvent("kick", -0.1, 0.1, 1.0), /startSeconds/);
  assert.throws(() => new PercussionHitEvent("kick", 0.0, -0.1, 1.0), /durationSeconds/);
  assert.throws(() => new PercussionHitEvent("kick", 0.0, 0.1, -0.1), /velocity/);
  assert.throws(() => new PercussionHitEvent("kick", 0.0, 0.1, 1.5), /velocity/);

  // PercussionNoiseProfile (Finding 3)
  assert.throws(() => new PercussionNoiseProfile(NaN, 0.1), /amplitude/);
  assert.throws(() => new PercussionNoiseProfile(-0.1, 0.1), /amplitude/);
  assert.throws(() => new PercussionNoiseProfile(0.5, NaN), /decaySeconds/);
  assert.throws(() => new PercussionNoiseProfile(0.5, -0.1), /decaySeconds/);
  assert.throws(() => new PercussionNoiseProfile(0.5, 0.0), /decaySeconds must be > 0.0/);
  assert.throws(() => new PercussionNoiseProfile(0.5, 0.1, NaN), (err: unknown) => {
    return err instanceof InvalidParamError && /seed must be a finite number/.test((err as Error).message);
  });
  assert.throws(() => new PercussionNoiseProfile(0.5, 0.1, Infinity), (err: unknown) => {
    return err instanceof InvalidParamError && /seed must be a finite number/.test((err as Error).message);
  });
  assert.throws(() => new PercussionNoiseProfile(0.5, 0.1, -Infinity), (err: unknown) => {
    return err instanceof InvalidParamError && /seed must be a finite number/.test((err as Error).message);
  });
  assert.throws(() => new PercussionNoiseProfile(0.5, 0.1, "bad" as unknown as number), /seed must be a number or bigint/);

  // DrumKitProfile (Finding 4)
  assert.throws(
    () => new DrumKitProfile("bad", "Bad", new Map([["kick", "not_a_voice" as unknown as PercussionVoiceProfile]])),
    /kit voice "kick" must be a PercussionVoiceProfile/,
  );
  assert.throws(
    () => new DrumKitProfile("bad", "Bad", { kick: "not_a_voice" as unknown as PercussionVoiceProfile }),
    /kit voice "kick" must be a PercussionVoiceProfile/,
  );
  assert.throws(
    () => new DrumKitProfile("bad", "Bad", 123 as unknown as Record<string, PercussionVoiceProfile>),
    /voicesByHitId must be a Map or object/,
  );
  assert.throws(
    () => new DrumKitProfile("bad", "Bad", new Map()),
    /DrumKitProfile must contain at least one voice profile/,
  );
});

test("CWE-20 & CWE-400: Primitive validation routines", () => {
  assert.throws(() => validateNonNegative("v", -1), /must be >= 0.0/);
  assert.throws(() => validateNonNegative("v", NaN), /must be a finite number/);
  assert.throws(() => validatePositive("v", 0), /must be > 0.0/);
  assert.throws(() => validatePositive("v", -5), /must be > 0.0/);
  assert.throws(() => validateUnit("v", -0.1), /must be in \[0.0, 1.0\]/);
  assert.throws(() => validateUnit("v", 1.1), /must be in \[0.0, 1.0\]/);
  assert.throws(() => validateDuration("v", MAX_HIT_DURATION_SECONDS + 1), /exceeds maximum allowable duration/);
  assert.throws(() => validateSampleRate("v", MAX_SAMPLE_RATE_HZ + 1), /exceeds MAX_SAMPLE_RATE_HZ/);
  assert.throws(() => validateSafeInteger("v", 1.5), /must be a safe integer/);
  assert.throws(() => validateSafeInteger("v", NaN), /must be a safe integer/);
  assert.throws(() => validatePositiveInt("v", 0), /must be > 0/);
  assert.throws(() => validateNonNegativeInt("v", -1), /must be >= 0/);
  assert.throws(() => validateSampleCountCeiling("v", MAX_SAMPLE_COUNT + 1), /exceeds maximum allowable/);
  assert.throws(() => validateModeCountCeiling("v", MAX_PERCUSSION_MODES + 1), /exceeds maximum allowable/);
  assert.throws(() => validateHitCountCeiling("v", MAX_HITS + 1), /exceeds maximum allowable/);
  assert.throws(() => validateSequenceEventCountCeiling("v", MAX_SEQUENCE_EVENTS + 1), /exceeds maximum allowable/);
});
