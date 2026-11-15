import assert from "node:assert/strict";
import test from "node:test";

import {
  AdsrEnvelope,
  HarmonicPartial,
  InstrumentProfile,
  InstrumentSignal,
  MAX_NOTE_DURATION_SECONDS,
  MAX_OCTAVE,
  MAX_PARTIALS,
  MAX_SAMPLE_COUNT,
  MAX_SAMPLE_RATE_HZ,
  MIN_OCTAVE,
  Note,
  parseNote,
  renderInstrumentNote,
  sine,
  SplitMix64,
  validateDuration,
  validateNonNegativeInt,
  validatePartialCountCeiling,
  validatePositiveInt,
  validateSafeInteger,
  validateSampleCountCeiling,
  validateSampleRate,
  VariationProfile,
} from "../src/index.js";

test("CWE-20: Note constructor and parseNote reject invalid inputs and prototype keys", () => {
  // Prototype pollution keys
  assert.throws(() => new Note("__proto__", "", 4), /Forbidden property name/);
  assert.throws(() => new Note("A", "constructor", 4), /Forbidden property name/);
  assert.throws(() => parseNote("__proto__"), /Rejected prototype property name/);
  assert.throws(() => parseNote("constructor"), /Rejected prototype property name/);

  // Invalid types and formats
  assert.throws(() => parseNote(123 as unknown as string), /Expected string input/);
  assert.throws(() => parseNote(""), /Note string length must be between/);
  assert.throws(() => parseNote("A".repeat(35)), /Note string length must be between/);
  assert.throws(() => parseNote("H4"), /Invalid note "H4"/);
  assert.throws(() => parseNote("A"), /Note string length must be between/);
  assert.throws(() => parseNote("A#"), /Invalid note/);
  assert.throws(() => parseNote("A-"), /Negative sign must be followed by digits/);
  assert.throws(() => parseNote("A4foo"), /Invalid character in octave/);

  // Non-finite or out-of-bounds octaves
  assert.throws(() => new Note("A", "", Number.NaN), /finite integer/);
  assert.throws(() => new Note("A", "", Number.POSITIVE_INFINITY), /finite integer/);
  assert.throws(() => new Note("A", "", 1.5), /finite integer/);
  assert.throws(() => new Note("A", "", MIN_OCTAVE - 1), /out of range/);
  assert.throws(() => new Note("A", "", MAX_OCTAVE + 1), /out of range/);
});

test("CWE-20: HarmonicPartial rejects negative or non-finite numbers", () => {
  assert.throws(() => new HarmonicPartial(Number.NaN, 1.0), /finite/);
  assert.throws(() => new HarmonicPartial(0.0, 1.0), /> 0.0/);
  assert.throws(() => new HarmonicPartial(-1.0, 1.0), /> 0.0/);
  assert.throws(() => new HarmonicPartial(1.0, -0.5), />= 0.0/);
  assert.throws(() => new HarmonicPartial(1.0, Number.POSITIVE_INFINITY), /finite/);
  assert.throws(() => new HarmonicPartial(1.0, 1.0, Number.NaN), /finite/);
});

test("CWE-20 & CWE-400: AdsrEnvelope rejects negative, non-finite, or excessive durations", () => {
  assert.throws(() => new AdsrEnvelope(Number.NaN, 0.1, 0.5, 0.1), /finite/);
  assert.throws(() => new AdsrEnvelope(-0.1, 0.1, 0.5, 0.1), />= 0.0/);
  assert.throws(() => new AdsrEnvelope(MAX_NOTE_DURATION_SECONDS + 1, 0.1, 0.5, 0.1), /exceeds MAX_NOTE_DURATION_SECONDS/);
  assert.throws(() => new AdsrEnvelope(0.1, MAX_NOTE_DURATION_SECONDS + 1, 0.5, 0.1), /exceeds MAX_NOTE_DURATION_SECONDS/);
  assert.throws(() => new AdsrEnvelope(0.1, 0.1, 1.5, 0.1), /in \[0.0, 1.0\]/);
  assert.throws(() => new AdsrEnvelope(0.1, 0.1, -0.1, 0.1), /in \[0.0, 1.0\]/);
  assert.throws(() => new AdsrEnvelope(0.1, 0.1, 0.5, MAX_NOTE_DURATION_SECONDS + 1), /exceeds MAX_NOTE_DURATION_SECONDS/);

  const env = new AdsrEnvelope(0.01, 0.01, 0.9, 0.02);
  assert.throws(() => env.valueAt(Number.NaN, 1.0), /finite/);
  assert.throws(() => env.valueAt(-0.1, 1.0), />= 0.0/);
  assert.throws(() => env.valueAt(0.1, MAX_NOTE_DURATION_SECONDS + 5), /exceeds MAX_NOTE_DURATION_SECONDS/);
  assert.throws(() => env.renderedDurationSeconds(MAX_NOTE_DURATION_SECONDS + 5), /exceeds MAX_NOTE_DURATION_SECONDS/);
});

test("CWE-20: VariationProfile and SplitMix64 reject bad inputs", () => {
  assert.throws(() => new SplitMix64(Number.NaN), /finite/);
  assert.throws(() => new SplitMix64("seed" as unknown as number), /number or bigint/);

  assert.throws(() => new VariationProfile({ pitchCentsJitter: -1.0 }), />= 0.0/);
  assert.throws(() => new VariationProfile({ amplitudeJitter: Number.NaN }), /finite/);
  assert.throws(() => new VariationProfile({ attackJitterMs: -5.0 }), />= 0.0/);
  assert.throws(() => new VariationProfile({ harmonicJitter: -0.1 }), />= 0.0/);
  assert.throws(() => new VariationProfile({ seed: Number.NaN }), /finite/);
});

test("CWE-400: InstrumentProfile enforces maximum partial count bounds", () => {
  const env = new AdsrEnvelope(0.01, 0.01, 0.9, 0.02);
  const tooManyPartials: HarmonicPartial[] = [];
  for (let i = 1; i <= MAX_PARTIALS + 1; i += 1) {
    tooManyPartials.push(new HarmonicPartial(i, 1.0 / i));
  }

  assert.throws(
    () => new InstrumentProfile("overflow", "Overflow", "additive", 1.0, tooManyPartials, env),
    /exceeds MAX_PARTIALS/,
  );

  // Non-silent instruments must have >= 1 partial
  assert.throws(
    () => new InstrumentProfile("empty", "Empty", "additive", 1.0, [], env),
    /at least one harmonic partial/,
  );

  // Invalid synthesisKind
  assert.throws(
    () => new InstrumentProfile("bad", "Bad", "unknown" as any, 1.0, [new HarmonicPartial(1, 1)], env),
    /Unsupported synthesisKind/,
  );
});

test("CWE-400: renderInstrumentNote enforces resource ceilings", () => {
  // Excessive duration
  assert.throws(
    () => renderInstrumentNote("A4", MAX_NOTE_DURATION_SECONDS + 1),
    /exceeds MAX_NOTE_DURATION_SECONDS/,
  );

  // Excessive sample rate
  assert.throws(
    () => renderInstrumentNote("A4", 0.1, { sampleRateHz: MAX_SAMPLE_RATE_HZ + 1 }),
    /exceeds MAX_SAMPLE_RATE_HZ/,
  );

  // Sample rate zero or negative
  assert.throws(
    () => renderInstrumentNote("A4", 0.1, { sampleRateHz: 0 }),
    /> 0.0/,
  );

  // Exceeds Nyquist frequency
  assert.throws(
    () => renderInstrumentNote("A4", 0.1, { sampleRateHz: 800.0 }), // Nyquist = 400 Hz < 440 Hz
    /must be below Nyquist frequency/,
  );

  // maxSampleCount ceiling
  assert.throws(
    () => renderInstrumentNote("A4", 0.1, { maxSampleCount: MAX_SAMPLE_COUNT + 1 }),
    /exceeds maximum allowable/,
  );

  // Budget exceeded during render
  assert.throws(
    () => renderInstrumentNote("A4", 1.0, { maxSampleCount: 10 }),
    /exceeding maxSampleCount=10/,
  );

  // maxPartialCount ceiling
  assert.throws(
    () => renderInstrumentNote("A4", 0.1, { maxPartialCount: 0 }),
    /> 0/,
  );
  assert.throws(
    () => renderInstrumentNote("A4", 0.1, { maxPartialCount: MAX_PARTIALS + 1 }),
    /exceeds maximum allowable/,
  );
});

test("CWE-20 & CWE-400: Primitive validation routines reject out-of-bounds inputs", () => {
  assert.throws(() => validateDuration("dur", MAX_NOTE_DURATION_SECONDS + 1.0), /exceeds MAX_NOTE_DURATION_SECONDS/);
  assert.throws(() => validateSampleRate("rate", MAX_SAMPLE_RATE_HZ + 1.0), /exceeds MAX_SAMPLE_RATE_HZ/);
  assert.throws(() => validateSafeInteger("int", 1.5), /must be a safe integer/);
  assert.throws(() => validateSafeInteger("int", NaN), /must be a safe integer/);
  assert.throws(() => validateSafeInteger("int", "1" as unknown as number), /must be a safe integer/);
  assert.throws(() => validatePositiveInt("int", 0), /must be > 0/);
  assert.throws(() => validateNonNegativeInt("int", -1), /must be >= 0/);
  assert.throws(() => validateSampleCountCeiling("count", MAX_SAMPLE_COUNT + 10), /exceeds maximum allowable/);
  assert.throws(() => validatePartialCountCeiling("count", MAX_PARTIALS + 10), /exceeds maximum allowable/);
});

