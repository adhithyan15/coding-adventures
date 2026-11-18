import assert from "node:assert/strict";
import test from "node:test";

import {
  ArrangementSection,
  InstrumentDeclaration,
  isIdentifier,
  isValidMeter,
  MAX_EVENTS,
  MAX_INSTRUMENTS,
  MAX_LINES,
  MAX_PPQ,
  MAX_TRACKS,
  parsePortableScore,
  parsePortableScoreJson,
  parsePortableScoreWithLimits,
  parseScore,
  parseScoreWithLimits,
  PhraseBuilder,
  PhraseMotif,
  PortableScore,
  PortableScoreBuilder,
  PortableScoreEvent,
  renderPortableScoreToPcm,
  renderScoreToPcm,
  resolveInstrumentId,
  safeRoundSample,
  ScoreEvent,
  TextScore,
  TrackDeclaration,
  validateSampleRate,
} from "../src/index.js";
import { mixPcmSamples } from "../src/render.js";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const BASE_JSON: Record<string, unknown> = {
  format_version: "music-machine-score/v2",
  title: "Test",
  ppq: 120,
  sample_rate_hz: 44100,
  tempo_events: [],
  meter_events: [],
  instruments: [],
  tracks: [],
  events: [],
};

const PORTABLE_HEAD = "format: music-machine-score/v2\n";

/** A tiny, valid builder with one sine track so events can be added to it. */
function sineBuilder(options: { sampleRateHz?: number; ppq?: number } = {}): PortableScoreBuilder {
  const b = new PortableScoreBuilder({
    sampleRateHz: options.sampleRateHz ?? 8000,
    ppq: options.ppq ?? 4,
  });
  b.addInstrument("sine1", { kind: "sine", gain: 0.5 });
  b.addTrack("t1", { instrumentId: "sine1" });
  return b;
}

// ---------------------------------------------------------------------------
// validation.ts
// ---------------------------------------------------------------------------

test("validation: identifier and meter scanners reject bad characters", () => {
  // A legal first character followed by an illegal one hits the loop rejection.
  assert.equal(isIdentifier("ab$"), false);
  assert.equal(isIdentifier("a b"), false);
  assert.equal(isIdentifier("ok-name_1"), true);
  // Meter with an empty denominator is rejected before digit scanning.
  assert.equal(isValidMeter("4/"), false);
  assert.equal(isValidMeter("/4"), false);
  assert.equal(isValidMeter("3/4"), true);
});

test("validation: sample-rate and rounding guards", () => {
  assert.throws(() => validateSampleRate(500), /sample_rate_hz must be in/);
  assert.throws(() => validateSampleRate(10_000_000), /sample_rate_hz must be in/);
  assert.equal(validateSampleRate(8000), 8000);

  assert.throws(() => safeRoundSample(1e308, 10), /non-finite/);
  assert.throws(() => safeRoundSample(1e300, 1), /safe integer/);
  assert.equal(safeRoundSample(0.5, 8000), 4000);
});

test("validation: resolveInstrumentId selectors", () => {
  assert.throws(() => resolveInstrumentId("piano_naive", 5), /instrument or program, not both/);
  assert.equal(typeof resolveInstrumentId(), "string");
  assert.equal(resolveInstrumentId("standard"), "drumkit");
  assert.equal(typeof resolveInstrumentId(undefined, 1), "string");
});

// ---------------------------------------------------------------------------
// score_parser.ts
// ---------------------------------------------------------------------------

test("score parser: input-shape and resource guards", () => {
  assert.throws(() => parseScoreWithLimits(42 as unknown as string), /score text must be a string/);
  // MAX_LINES + 1 newline-only lines trips the line budget, not the byte budget.
  assert.throws(() => parseScore("\n".repeat(MAX_LINES + 1)), /line count/);
  // A text with no events at all is not a score.
  assert.throws(() => parseScore("title: Empty\n"), /at least one event/);
});

test("score parser: unknown directives and malformed directive names", () => {
  // Well-formed directive name that is not on the allow-list.
  assert.throws(() => parseScore("bogus: 1\nC4/q\n"), /unknown directive 'bogus'/);
  // A directive name starting with a digit is not a directive; it falls through
  // to token parsing where it fails for lacking a duration.
  assert.throws(() => parseScore("2x: 4\nC4/q\n"), /pitch\/duration/);
  // A name with an illegal inner character is likewise rejected as a directive.
  assert.throws(() => parseScore("a-b: 4\nC4/q\n"), /pitch\/duration/);
});

// ---------------------------------------------------------------------------
// portable_parser.ts
// ---------------------------------------------------------------------------

test("portable parser: input-shape and resource guards", () => {
  assert.throws(
    () => parsePortableScoreWithLimits(7 as unknown as string),
    /score text must be a string/,
  );
  assert.throws(
    () => parsePortableScoreWithLimits(`${PORTABLE_HEAD}title: x\n`, 10),
    /exceeds maxScoreLength/,
  );
  assert.throws(
    () => parsePortableScoreWithLimits("\n".repeat(MAX_LINES + 1)),
    /line count/,
  );
  assert.throws(
    () => parsePortableScoreWithLimits(`${PORTABLE_HEAD}title: ${"x".repeat(50)}\n`, 1_000_000, 20),
    /line length/,
  );
  // Comments only: the mandatory format line never appears.
  assert.throws(() => parsePortableScore("# nothing here\n"), /first directive must be/);
});

test("portable parser: instrument, track and event line errors", () => {
  assert.throws(
    () => parsePortableScore(`${PORTABLE_HEAD}instrument lonely\n`),
    /instrument lines must include id and properties/,
  );
  assert.throws(
    () => parsePortableScore(`${PORTABLE_HEAD}instrument a kind=sine volume=3\n`),
    /unknown instrument property 'volume'/,
  );
  assert.throws(
    () => parsePortableScore(`${PORTABLE_HEAD}instrument a kind=sine\ntrack t instrument=a color=red\n`),
    /unknown track property 'color'/,
  );
  assert.throws(
    () => parsePortableScore(`${PORTABLE_HEAD}instrument a kind=sine\ntrack t instrument=a\nevent t 0 10 chirp C4\n`),
    /event kind must be 'note', 'rest', or 'hit'/,
  );
  assert.throws(
    () => parsePortableScoreWithLimits(
      `${PORTABLE_HEAD}instrument a kind=sine\ntrack t instrument=a\nevent t 0 10 rest\nevent t 10 10 rest\n`,
      1_000_000,
      10_000,
      1,
    ),
    /event count exceeds maxEventCount/,
  );
});

test("portable parser: additive envelope validation", () => {
  const additive = (envelope: string): string =>
    `${PORTABLE_HEAD}instrument a kind=additive harmonics=1:1.0 envelope=${envelope}\n`;
  assert.throws(() => parsePortableScore(additive("a,b,c,d")), /non-finite value in envelope/);
  assert.throws(() => parsePortableScore(additive("1e9,0.1,0.5,0.1")), /envelope times must be/);
  assert.throws(() => parsePortableScore(additive("0.1,0.1,0.5")), /envelope must be attack,decay,sustain,release/);
});

test("portable text round-trips sine and silence instruments", () => {
  const b = sineBuilder();
  b.addInstrument("quiet", { kind: "silence", gain: 0.25 });
  b.addTrack("t2", { instrumentId: "quiet" });
  b.addNote("t1", 0, 4, "C4");
  const text = b.toText();
  assert.match(text, /instrument sine1 kind=sine gain=0\.5/);
  assert.match(text, /instrument quiet kind=silence gain=0\.25/);
  const reparsed = parsePortableScore(text);
  assert.equal(reparsed.instruments.length, 2);
  assert.equal(reparsed.toText(), text);
});

// ---------------------------------------------------------------------------
// json.ts
// ---------------------------------------------------------------------------

test("json parser: multi-byte payloads are bounded by bytes, not characters", () => {
  // 10 characters, 20 UTF-8 bytes: passes the cheap length check, fails the exact one.
  assert.throws(
    () => parsePortableScoreJson("é".repeat(10), { maxBytes: 15 }),
    /JSON byte length 20 exceeds maxBytes=15/,
  );
});

test("json parser: list-size budgets are enforced before element parsing", () => {
  const tooMany = (n: number): unknown[] => new Array<unknown>(n).fill(0);
  const cases: [string, number, RegExp][] = [
    ["tempo_events", MAX_EVENTS + 1, /tempo_events count/],
    ["meter_events", MAX_EVENTS + 1, /meter_events count/],
    ["instruments", MAX_INSTRUMENTS + 1, /instruments count/],
    ["tracks", MAX_TRACKS + 1, /tracks count/],
    ["events", MAX_EVENTS + 1, /events count/],
  ];
  for (const [key, n, pattern] of cases) {
    assert.throws(
      () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, [key]: tooMany(n) })),
      pattern,
      key,
    );
  }
});

test("json parser: top-level fields must be lists", () => {
  for (const key of ["tempo_events", "meter_events", "instruments", "tracks", "events"]) {
    assert.throws(
      () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, [key]: "nope" })),
      new RegExp(`${key} must be a list`),
      key,
    );
  }
});

test("json parser: event note entries must be strings", () => {
  const json = JSON.stringify({
    ...BASE_JSON,
    instruments: [{ id: "i1", kind: "sine" }],
    tracks: [{ id: "t1", instrument_id: "i1" }],
    events: [{ track_id: "t1", start_tick: 0, duration_tick: 10, kind: "note", notes: [60] }],
  });
  assert.throws(() => parsePortableScoreJson(json), /notes\[0\] must be a string/);
});

// ---------------------------------------------------------------------------
// builder.ts
// ---------------------------------------------------------------------------

test("builder: constructor and event argument guards", () => {
  assert.throws(() => new PortableScoreBuilder({ ppq: MAX_PPQ + 1 }), /ppq must be <=/);
  const b = sineBuilder();
  assert.throws(() => b.addChord("t1", 0, 4, []), /non-empty array/);
  assert.throws(() => b.addChord("t1", 0, 4, "C4" as unknown as string[]), /non-empty array/);
});

test("builder: beat arithmetic and measure helpers", () => {
  const b = sineBuilder({ ppq: 4 });
  // 0.1 beat at ppq=4 is 0.4 ticks: not representable.
  assert.throws(() => b.beatsToTicks(0.1), /not representable at ppq=4/);
  // Without any meter event the builder falls back to 4/4.
  assert.equal(b.measureTicks(), 16);
  assert.equal(b.measureStartTick(3), 32);
  // An explicit meter event takes over from the default.
  b.addMeter(0, "3/4");
  assert.equal(b.measureTicks(), 12);
  assert.equal(b.tickInMeasure(2, 1), 16);
});

test("builder: motif and section arguments are type-checked", () => {
  const b = sineBuilder();
  b.addNote("t1", 0, 4, "C4");
  assert.throws(
    () => b.applyMotif({} as unknown as PhraseMotif, "t1", 0),
    /motif must be a PhraseMotif/,
  );
  assert.throws(
    () => b.applySection({} as unknown as ArrangementSection, 0),
    /section must be an ArrangementSection/,
  );
  assert.throws(
    () => new PhraseBuilder({} as unknown as PortableScoreBuilder, "t1"),
    /scoreBuilder must be a PortableScoreBuilder/,
  );
  assert.throws(() => b.captureSection({ startTick: 8, endTick: 4 }), /endTick must be >= startTick/);
});

test("builder: captureSection filters by track and by tick window", () => {
  const b = sineBuilder();
  b.addInstrument("sine2", { kind: "sine" });
  b.addTrack("t2", { instrumentId: "sine2" });
  b.addNote("t1", 0, 4, "C4");
  b.addNote("t2", 0, 4, "E4");
  b.addNote("t1", 100, 4, "G4"); // outside the window

  const all = b.captureSection({ startTick: 0, endTick: 8 });
  assert.equal(all.events.length, 2);

  const onlyT2 = b.captureSection({ startTick: 0, endTick: 8, trackIds: ["t2"] });
  assert.equal(onlyT2.events.length, 1);
  assert.equal(onlyT2.events[0].trackId, "t2");
});

test("builder: captureSection rejects an event that overruns end_tick", () => {
  const b = sineBuilder();
  b.addNote("t1", 0, 16, "C4");
  assert.throws(
    () => b.captureSection({ startTick: 0, endTick: 8 }),
    /extends beyond end_tick/,
  );
});

test("builder: applySection supports per-track transpose and velocity maps", () => {
  const b = sineBuilder();
  b.addInstrument("sine2", { kind: "sine" });
  b.addTrack("t2", { instrumentId: "sine2" });
  b.addNote("t1", 0, 4, "C4", { velocity: 0.8 });
  b.addNote("t2", 0, 4, "C4", { velocity: 0.8 });
  const section = b.captureSection({ startTick: 0, endTick: 4 });

  b.applySection(section, 16, {
    transposeSemitones: { t1: 12 },
    velocityScale: { t2: 0.5 },
  });

  const built = b.build();
  const shifted = built.events.filter((e) => e.startTick === 16);
  assert.equal(shifted.length, 2);
  const t1 = shifted.find((e) => e.trackId === "t1")!;
  const t2 = shifted.find((e) => e.trackId === "t2")!;
  assert.deepEqual([...t1.notes], ["C5"]); // transposed up an octave
  assert.equal(t1.velocity, 0.8); // no velocity entry for t1
  assert.deepEqual([...t2.notes], ["C4"]); // no transpose entry for t2
  assert.equal(t2.velocity, 0.4); // 0.8 * 0.5

  // Per-track entries are validated like scalar options.
  assert.throws(
    () => b.applySection(section, 32, { transposeSemitones: { t1: 1.5 } }),
    /transposeSemitones\[t1\]/,
  );
  assert.throws(
    () => b.applySection(section, 32, { velocityScale: { t1: 2 } }),
    /velocityScale\[t1\]/,
  );
});

test("builder: applySection replays rests and hits with a track map", () => {
  const b = new PortableScoreBuilder({ sampleRateHz: 8000, ppq: 4 });
  b.addInstrument("kit", { kind: "drumkit" });
  b.addTrack("drums", { instrumentId: "kit" });
  b.addTrack("drums2", { instrumentId: "kit" });
  b.addHit("drums", 0, 2, "kick");
  b.addRest("drums", 2, 2);
  const section = b.captureSection({ startTick: 0, endTick: 4 });
  b.applySection(section, 8, { trackMap: { drums: "drums2" }, repeatCount: 2 });
  const built = b.build();
  assert.equal(built.events.filter((e) => e.trackId === "drums2").length, 4);
});

test("builder: applyMotif replays hits and rests", () => {
  const b = new PortableScoreBuilder({ sampleRateHz: 8000, ppq: 4 });
  b.addInstrument("kit", { kind: "drumkit" });
  b.addTrack("drums", { instrumentId: "kit" });
  const phrase = new PhraseBuilder(b, "drums");
  phrase.hit("snare", 0.5).rest(0.5);
  const motif = phrase.motif();
  assert.equal(motif.events.length, 2);
  b.applyMotif(motif, "drums", 16);
  assert.equal(b.build().events.filter((e) => e.startTick >= 16).length, 2);
});

// ---------------------------------------------------------------------------
// types.ts
// ---------------------------------------------------------------------------

function textScoreOptions(events: ScoreEvent[]): ConstructorParameters<typeof TextScore>[0] {
  return {
    title: "t",
    tempoBpm: 120,
    meter: "4/4",
    amplitude: 0.5,
    sampleRateHz: 8000,
    instrumentId: "sine",
    events,
  };
}

function restEvent(seconds: number): ScoreEvent {
  return new ScoreEvent("rest", null, null, "q", 1, seconds, "R/q");
}

test("TextScore constructor guards", () => {
  assert.throws(
    () => new TextScore({ ...textScoreOptions([restEvent(0.1)]), sampleRateHz: 10 }),
    /sampleRateHz must be in/,
  );
  assert.throws(() => new TextScore(textScoreOptions([])), /at least one event/);
  assert.throws(
    () => new TextScore(textScoreOptions(new Array<ScoreEvent>(MAX_EVENTS + 1).fill(restEvent(0.1)))),
    /exceeds MAX_EVENTS/,
  );
});

function scoreOptions(
  overrides: Partial<ConstructorParameters<typeof PortableScore>[0]> = {},
): ConstructorParameters<typeof PortableScore>[0] {
  return {
    formatVersion: "music-machine-score/v2",
    title: "t",
    ppq: 4,
    sampleRateHz: 8000,
    tempoEvents: [],
    meterEvents: [],
    instruments: [],
    tracks: [],
    events: [],
    ...overrides,
  };
}

test("PortableScore constructor guards", () => {
  assert.throws(() => new PortableScore(scoreOptions({ ppq: MAX_PPQ + 1 })), /ppq must be <=/);
  assert.throws(() => new PortableScore(scoreOptions({ sampleRateHz: 10 })), /sampleRateHz must be in/);

  const overflow = (n: number): never[] => new Array<never>(n).fill(undefined as never);
  assert.throws(
    () => new PortableScore(scoreOptions({ tempoEvents: overflow(MAX_EVENTS + 1) })),
    /tempoEvents count/,
  );
  assert.throws(
    () => new PortableScore(scoreOptions({ meterEvents: overflow(MAX_EVENTS + 1) })),
    /meterEvents count/,
  );
  assert.throws(
    () => new PortableScore(scoreOptions({ instruments: overflow(MAX_INSTRUMENTS + 1) })),
    /instruments count/,
  );
  assert.throws(
    () => new PortableScore(scoreOptions({ tracks: overflow(MAX_TRACKS + 1) })),
    /tracks count/,
  );
  assert.throws(
    () => new PortableScore(scoreOptions({ events: overflow(MAX_EVENTS + 1) })),
    /events count/,
  );
});

test("PortableScore rejects an event whose end tick overflows (forged event)", () => {
  const inst = new InstrumentDeclaration("i1", "sine", 1, "sine", null, null, null);
  const forged = {
    trackId: "t1",
    startTick: Number.MAX_SAFE_INTEGER,
    durationTick: 2,
    kind: "rest",
    notes: [],
    velocity: 1,
    sourceOrder: 0,
    hitId: null,
  } as unknown as PortableScoreEvent;
  assert.throws(
    () => new PortableScore(scoreOptions({
      instruments: [inst],
      tracks: [new TrackDeclaration("t1", "i1")],
      events: [forged],
    })),
    /exceeds safe integer/,
  );
});

// ---------------------------------------------------------------------------
// render.ts
// ---------------------------------------------------------------------------

/**
 * Build a TextScore whose advertised total duration lies. The renderer
 * pre-flights its budget from totalDurationSeconds(), then re-checks the
 * running cursor per event; a score that under-reports must still be stopped
 * by the per-event check.
 */
function underReportingScore(events: ScoreEvent[]): TextScore {
  const real = new TextScore(textScoreOptions(events));
  return Object.assign(Object.create(TextScore.prototype) as object, real, {
    totalDurationSeconds: () => 0.001,
  }) as TextScore;
}

test("renderScoreToPcm: per-event cursor check stops an under-reporting score", () => {
  const score = underReportingScore([restEvent(0.01), restEvent(0.01)]);
  // 0.01 s at 8 kHz is 80 samples per rest; the budget allows 100.
  assert.throws(() => renderScoreToPcm(score, 100), /cursorSampleIndex 160 exceeds maxSampleCount=100/);
});

test("mixPcmSamples: overflow and non-finite guards", () => {
  assert.throws(
    () => mixPcmSamples([], Number.MAX_SAFE_INTEGER, [1, 2]),
    /requiredCount exceeds safe integer/,
  );
  assert.throws(
    () => mixPcmSamples([Number.POSITIVE_INFINITY], 0, [1]),
    /must be finite/,
  );
});

test("renderPortableScoreToPcm: duration cap", () => {
  const b = sineBuilder({ sampleRateHz: 1000, ppq: 1 });
  b.addTempo(0, 1); // 1 BPM: one beat is a minute
  b.addNote("t1", 0, 11, "A4"); // 11 minutes > MAX_DURATION_SECONDS
  assert.throws(() => renderPortableScoreToPcm(b.build()), /exceeds MAX_DURATION_SECONDS/);
});

test("renderPortableScoreToPcm: defends against forged scores", () => {
  const inst = new InstrumentDeclaration("i1", "sine", 1, "sine", null, null, null);
  const goodTrack = new TrackDeclaration("t1", "i1");
  const event = new PortableScoreEvent("t1", 0, 4, "rest", []);

  const forge = (fields: Record<string, unknown>): PortableScore => {
    const base = new PortableScore(scoreOptions({ instruments: [inst], tracks: [goodTrack], events: [event] }));
    return Object.assign(Object.create(PortableScore.prototype) as object, base, fields) as PortableScore;
  };

  assert.throws(
    () => renderPortableScoreToPcm(forge({ tracks: [] })),
    /event references unknown track 't1'/,
  );
  assert.throws(
    () => renderPortableScoreToPcm(forge({ instruments: [] })),
    /references unknown instrument 'i1'/,
  );
});

test("renderPortableScoreToPcm: sample budget across events", () => {
  // Two back-to-back 1000-sample notes under a 1500-sample budget: the first
  // fits, the second would end at 2000.
  const b = sineBuilder({ sampleRateHz: 8000, ppq: 8 });
  b.addTempo(0, 60);
  b.addNote("t1", 0, 1, "A4");
  b.addNote("t1", 1, 1, "A4");
  b.addNote("t1", 2, 1, "A4");
  assert.throws(() => renderPortableScoreToPcm(b.build(), 1500), /exceeding maxSampleCount|score render would require/);
});
