import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_EVENTS,
  MAX_INSTRUMENTS,
  MAX_JSON_DEPTH,
  MAX_LINE_LENGTH,
  MAX_LINES,
  MAX_PARTIALS,
  MAX_PPQ,
  MAX_SAMPLE_COUNT,
  MAX_SAMPLE_RATE_HZ,
  MAX_SCORE_BYTES,
  MAX_TRACKS,
  MIN_SAMPLE_RATE_HZ,
  mixPcmSamples,
  parsePortableScore,
  parsePortableScoreJson,
  parseScore,
  PortableScoreBuilder,
  renderPortableScoreToPcm,
  renderScoreToPcm,
  ScoreEvent,
  TempoMap,
  validateIdentifier,
  validateMeter,
} from "../src/index.js";

test("CWE-1321: Prototype pollution guards reject forbidden properties in inputs and parsers", () => {
  assert.throws(() => validateIdentifier("id", "__proto__"), /Forbidden property name/);
  assert.throws(() => validateIdentifier("id", "constructor"), /Forbidden property name/);
  assert.throws(() => validateIdentifier("id", "prototype"), /Forbidden property name/);

  // Prototype pollution attempt in score text
  const protoScore = `title: Malicious
__proto__: evil
tempo: 120
meter: 4/4
C4/q
`;
  assert.throws(() => parseScore(protoScore), /Forbidden property name|unknown directive/);

  // Prototype pollution attempt in JSON
  const protoJson = `{"__proto__": {"polluted": true}, "format_version": "music-machine-score/v2", "title": "Attack", "ppq": 120, "sample_rate_hz": 44100, "tempo_events": [], "meter_events": [], "instruments": [], "tracks": [], "events": []}`;
  assert.throws(() => parsePortableScoreJson(protoJson), /Forbidden property name|unknown score keys/);
  assert.equal(({} as Record<string, unknown>).polluted, undefined);
});

test("CWE-20: Rejection of non-finite numbers (NaN, ±Infinity)", () => {
  assert.throws(
    () => parseScore("tempo: NaN\nmeter: 4/4\nC4/q\n"),
    /tempo must be a number/,
  );
  assert.throws(
    () => parseScore("tempo: Infinity\nmeter: 4/4\nC4/q\n"),
    /must be a finite number/,
  );
  assert.throws(
    () => parseScore("tempo: -120\nmeter: 4/4\nC4/q\n"),
    /must be > 0.0/,
  );
  assert.throws(
    () => parseScore("amplitude: 1.5\nmeter: 4/4\nC4/q\n"),
    /amplitude must be in \[0.0, 1.0\]/,
  );
  assert.throws(
    () => parseScore("sample_rate: -44100\nmeter: 4/4\nC4/q\n"),
    /sample_rate must be an integer/,
  );

  const builder = new PortableScoreBuilder();
  assert.throws(() => builder.addTempo(0, Number.NaN), /must be a finite number/);
  assert.throws(() => builder.addTempo(0, Number.POSITIVE_INFINITY), /must be a finite number/);
  assert.throws(() => builder.addNote("t1", 0, 120, "C4", { velocity: Number.NaN }), /must be a finite number/);
});

test("CWE-20: Meter validation rejects malformed and negative/zero components", () => {
  assert.throws(() => validateMeter(""), /meter must look like/);
  assert.throws(() => validateMeter("0/4"), /meter must look like/);
  assert.throws(() => validateMeter("4/0"), /meter must look like/);
  assert.throws(() => validateMeter("-4/4"), /meter must look like/);
  assert.throws(() => validateMeter("4"), /meter must look like/);
  assert.throws(() => validateMeter("4/4/4"), /meter must look like/);
  assert.throws(() => validateMeter("4/four"), /meter must look like/);
});

test("CWE-20: Malformed token parsing rejects bad duration symbols and empty pitches", () => {
  assert.throws(() => parseScore("C4\n"), /must look like pitch\/duration/);
  assert.throws(() => parseScore("/q\n"), /must look like pitch\/duration/);
  assert.throws(() => parseScore("C4/\n"), /must look like pitch\/duration/);
  assert.throws(() => parseScore("C4/x\n"), /unknown duration symbol/);
  assert.throws(() => parseScore("C4/q..\n"), /may have at most one dot/);
  assert.throws(() => parseScore("InvalidNote99/q\n"), /invalid/i);
});

test("CWE-400: Resource limit enforcement on score dimensions and bounds", () => {
  // Score byte length exceeding limit
  const hugeText = "tempo: 120\n".repeat(100_000);
  assert.throws(() => parseScore(hugeText, { maxScoreLength: 100 }), /exceeds maxScoreLength/);

  // Line length exceeding limit
  const longLine = "title: " + "A".repeat(MAX_LINE_LENGTH + 1) + "\nC4/q\n";
  assert.throws(() => parseScore(longLine), /exceeds maxLineLength/);

  // Event count exceeding limit
  const manyEvents = "tempo: 120\nmeter: 4/4\n" + ("C4/q ".repeat(100) + "\n").repeat(101);
  assert.throws(() => parseScore(manyEvents), /exceeds maxEventCount/);

  // Sample rate limits
  assert.throws(
    () => parseScore(`sample_rate: ${String(MAX_SAMPLE_RATE_HZ + 1)}\nC4/q\n`),
    /sample_rate must be an integer/,
  );
  assert.throws(
    () => parseScore(`sample_rate: ${String(MIN_SAMPLE_RATE_HZ - 1)}\nC4/q\n`),
    /sample_rate must be an integer/,
  );
});

test("CWE-400 & CWE-674: JSON nesting depth recursion budget", () => {
  let nested = '{"title": "Deep"}';
  for (let i = 0; i < MAX_JSON_DEPTH + 2; i += 1) {
    nested = `{"layer": ${nested}}`;
  }
  assert.throws(() => parsePortableScoreJson(nested), /JSON nesting depth exceeds/);

  // JSON byte length budget
  const largeJson = " ".repeat(MAX_SCORE_BYTES + 10);
  assert.throws(() => parsePortableScoreJson(largeJson), /JSON byte length/);
});

test("CWE-190: Integer overflow guards on event ticks and sample indices", () => {
  const builder = new PortableScoreBuilder();
  builder
    .addTempo(0, 120)
    .addMeter(0, "4/4")
    .addInstrument("lead", { kind: "sine" })
    .addTrack("t1", { instrumentId: "lead" });

  assert.throws(
    () => builder.addNote("t1", Number.MAX_SAFE_INTEGER, 100, "C4"),
    /exceeds safe integer/,
  );
});

test("CWE-20: Track and instrument incompatibility errors", () => {
  const textScore = `format: music-machine-score/v2
title: Incompatible
ppq: 120
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument lead profile=flute_naive gain=1.0
instrument drums kind=drumkit gain=1.0

track mel instrument=lead
track perc instrument=drums

event mel 0 120 hit kick
`;
  const score = parsePortableScore(textScore);
  assert.throws(
    () => renderPortableScoreToPcm(score),
    /uses pitched instrument 'lead' and cannot play hit events/,
  );

  const textScore2 = `format: music-machine-score/v2
title: Incompatible 2
ppq: 120
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument drums kind=drumkit gain=1.0
track perc instrument=drums
event perc 0 120 note C4
`;
  const score2 = parsePortableScore(textScore2);
  assert.throws(
    () => renderPortableScoreToPcm(score2),
    /uses drumkit instrument 'drums' and cannot play note events/,
  );
});

test("Hostile regression: NaN bypass in limits and mixPcmSamples", () => {
  const validScoreText = "title: Test\ntempo: 120\nmeter: 4/4\nC4/q\n";
  const validPortableText = `format: music-machine-score/v2
title: Test
ppq: 120
sample_rate: 44100
tempo 0 120
meter 0 4/4
instrument s profile=sine
track t instrument=s
event t 0 120 note C4
`;
  const validJson = JSON.stringify({
    format_version: "music-machine-score/v2",
    title: "Test",
    ppq: 120,
    sample_rate_hz: 44100,
    tempo_events: [],
    meter_events: [],
    instruments: [],
    tracks: [],
    events: [],
  });

  // maxScoreLength: NaN
  assert.throws(
    () => parseScore(validScoreText, { maxScoreLength: Number.NaN }),
    /must be a safe integer/,
  );
  assert.throws(
    () => parsePortableScore(validPortableText, { maxScoreLength: Number.NaN }),
    /must be a safe integer/,
  );

  // maxLineLength: NaN
  assert.throws(
    () => parseScore(validScoreText, { maxLineLength: Number.NaN }),
    /must be a safe integer/,
  );
  assert.throws(
    () => parsePortableScore(validPortableText, { maxLineLength: Number.NaN }),
    /must be a safe integer/,
  );

  // maxEventCount: NaN
  assert.throws(
    () => parseScore(validScoreText, { maxEventCount: Number.NaN }),
    /must be a safe integer/,
  );
  assert.throws(
    () => parsePortableScore(validPortableText, { maxEventCount: Number.NaN }),
    /must be a safe integer/,
  );

  // maxBytes: NaN in JSON parser
  assert.throws(
    () => parsePortableScoreJson(validJson, { maxBytes: Number.NaN }),
    /must be a safe integer/,
  );

  // maxDepth: NaN in JSON parser
  assert.throws(
    () => parsePortableScoreJson(validJson, { maxDepth: Number.NaN }),
    /must be a safe integer/,
  );

  // NaN in mixPcmSamples
  const targetBuf = [0, 0, 0];
  assert.throws(
    () => mixPcmSamples(targetBuf, 0, [Number.NaN]),
    /must be finite/,
  );
  assert.throws(
    () => mixPcmSamples(targetBuf, 0, [Number.POSITIVE_INFINITY]),
    /must be finite/,
  );

  // Ceilings exceeding global MAX constants
  assert.throws(
    () => parseScore(validScoreText, { maxScoreLength: MAX_SCORE_BYTES + 1 }),
    /maxScoreLength exceeds MAX_SCORE_BYTES/,
  );
  assert.throws(
    () => parseScore(validScoreText, { maxLineLength: MAX_LINE_LENGTH + 1 }),
    /maxLineLength exceeds maxLineLength/,
  );
  assert.throws(
    () => parseScore(validScoreText, { maxEventCount: MAX_EVENTS + 1 }),
    /maxEventCount exceeds MAX_EVENTS/,
  );

  assert.throws(
    () => parsePortableScore(validPortableText, { maxScoreLength: MAX_SCORE_BYTES + 1 }),
    /maxScoreLength exceeds MAX_SCORE_BYTES/,
  );
  assert.throws(
    () => parsePortableScore(validPortableText, { maxLineLength: MAX_LINE_LENGTH + 1 }),
    /maxLineLength exceeds maxLineLength/,
  );
  assert.throws(
    () => parsePortableScore(validPortableText, { maxEventCount: MAX_EVENTS + 1 }),
    /maxEventCount exceeds MAX_EVENTS/,
  );

  assert.throws(
    () => parsePortableScoreJson(validJson, { maxBytes: MAX_SCORE_BYTES + 1 }),
    /maxBytes exceeds MAX_SCORE_BYTES/,
  );
  assert.throws(
    () => parsePortableScoreJson(validJson, { maxDepth: MAX_JSON_DEPTH + 1 }),
    /maxDepth exceeds MAX_JSON_DEPTH/,
  );
  assert.throws(
    () => parsePortableScoreJson(validJson, { maxBytes: 5 }),
    /JSON byte length exceeds maxBytes=5/,
  );
});
