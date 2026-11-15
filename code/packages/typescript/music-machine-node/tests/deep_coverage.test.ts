import assert from "node:assert/strict";
import test from "node:test";

import {
  HarmonicPartial,
} from "@coding-adventures/musical-instruments";

import {
  ArrangementSection,
  ArrangementSectionEvent,
  beatsForDurationSymbol,
  MAX_DURATION_SECONDS,
  MAX_SAMPLE_COUNT,
  parsePortableScore,
  parsePortableScoreJson,
  parseScore,
  PhraseMotif,
  PhraseMotifEvent,
  PortableScore,
  PortableScoreBuilder,
  PortableScoreEvent,
  renderPortableScoreToPcm,
  renderScoreToPcm,
  ScoreEvent,
  TextScore,
  validateIdentifier,
  validateSafeInteger,
  validateSampleRate,
} from "../src/index.js";
import { mixPcmSamples } from "../src/render.js";

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

test("JSON parser granular error branches", () => {
  // tempo_events errors
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, tempo_events: ["not-obj"] })),
    /tempo_events\[0\] must be a mapping/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, tempo_events: [{ bpm: 120 }] })),
    /tempo_events\[0\] missing 'start_tick'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, tempo_events: [{ start_tick: 0 }] })),
    /tempo_events\[0\] missing 'bpm'/,
  );

  // meter_events errors
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, meter_events: [123] })),
    /meter_events\[0\] must be a mapping/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, meter_events: [{ meter: "4/4" }] })),
    /meter_events\[0\] missing 'start_tick'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, meter_events: [{ start_tick: 0 }] })),
    /meter_events\[0\] missing 'meter'/,
  );

  // instruments errors
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, instruments: [null] })),
    /instruments\[0\] must be a mapping/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, instruments: [{ gain: 1.0 }] })),
    /instruments\[0\] missing 'id'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      instruments: [{ id: "i1", harmonics: "not-array" }],
    })),
    /harmonics must be a list of partials/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      instruments: [{ id: "i1", harmonics: [[1]] }],
    })),
    /malformed partial at 0/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      instruments: [{ id: "i1", harmonics: [[-1, 1]] }],
    })),
    /malformed partial multiplier at 0/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      instruments: [{ id: "i1", harmonics: [[1, -1]] }],
    })),
    /malformed partial amplitude at 0/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      instruments: [{ id: "i1", harmonics: [[1, 1, "bad"]] }],
    })),
    /malformed partial phase at 0/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      instruments: [{ id: "i1", envelope: "not-obj" }],
    })),
    /envelope must be \[atk, dec, sus, rel\]/,
  );

  // tracks errors
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, tracks: [123] })),
    /tracks\[0\] must be a mapping/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, tracks: [{ instrument_id: "i1" }] })),
    /tracks\[0\] missing 'id'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, tracks: [{ id: "t1" }] })),
    /tracks\[0\] missing 'instrument_id'/,
  );

  // events errors
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...BASE_JSON, events: ["bad"] })),
    /events\[0\] must be a mapping/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      events: [{ start_tick: 0, duration_tick: 120, kind: "note" }],
    })),
    /events\[0\] missing 'track_id'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      events: [{ track_id: "t1", duration_tick: 120, kind: "note" }],
    })),
    /events\[0\] missing 'start_tick'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      events: [{ track_id: "t1", start_tick: 0, kind: "note" }],
    })),
    /events\[0\] missing 'duration_tick'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      events: [{ track_id: "t1", start_tick: 0, duration_tick: 120, kind: "unknown_kind" }],
    })),
    /events\[0\] invalid kind/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      events: [{ track_id: "t1", start_tick: 0, duration_tick: 120, kind: "note", notes: "not-array" }],
    })),
    /events\[0\] notes must be a list of strings|note events must include at least one note/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({
      ...BASE_JSON,
      events: [{ track_id: "t1", start_tick: 0, duration_tick: 120, kind: "hit" }],
    })),
    /events\[0\] hitId must be a string|hit events must include a hitId/,
  );
});

test("Portable text parser granular error branches", () => {
  // Title missing argument
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntitle\n"),
    /unknown command 'title'|title lines must look like/,
  );

  // PPQ missing argument or invalid
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nppq\n"),
    /unknown command 'ppq'/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nppq: -10\n"),
    /ppq must be in/,
  );

  // Sample rate missing argument or invalid
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nsample_rate: not-int\n"),
    /sample_rate must be in/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nsample_rate: 999999\n"),
    /sample_rate must be in/,
  );

  // Tempo line errors
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntempo 0\n"),
    /tempo lines must be:/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntempo -10 120\n"),
    /invalid tempo start_tick/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntempo 0 invalid\n"),
    /tempo bpm must be > 0.0/,
  );

  // Meter line errors
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nmeter 0\n"),
    /meter lines must be:/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nmeter -5 4/4\n"),
    /invalid meter start_tick/,
  );

  // Track line errors
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntrack\n"),
    /track lines must include id/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntrack t foo=bar\n"),
    /track must include instrument=<id>/,
  );

  // Event line errors
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nevent t 0\n"),
    /event lines must be:/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s profile=sine\ntrack t instrument=s\nevent t -1 120 note C4\n"),
    /invalid event start_tick/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s profile=sine\ntrack t instrument=s\nevent t 0 -10 note C4\n"),
    /invalid event duration_tick/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s profile=sine\ntrack t instrument=s\nevent t 0 120 hit\n"),
    /hit events must (include|specify) a hit_id/,
  );

  // Envelope and Harmonics property parsing errors
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=1:1 envelope=0.01,0.02,0.8\n"),
    /envelope must be attack,decay,sustain,release/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=bad\n"),
    /harmonic partial 'bad' must be multiplier:amplitude/,
  );
});

test("Render: multi-event score, bounds, and saturating arithmetic", () => {
  // TextScore containing both note, rest, and hit
  const mixedScore = new TextScore({
    title: "Mixed",
    tempoBpm: 120,
    meter: "4/4",
    amplitude: 0.2,
    sampleRateHz: 44100,
    instrumentId: "sine",
    drumkit: "standard_kit",
    events: [
      new ScoreEvent("note", "C4", null, "q", 1.0, 0.5, "C4/q"),
      new ScoreEvent("rest", null, null, "q", 1.0, 0.5, "R/q"),
      new ScoreEvent("hit", null, "kick", "q", 1.0, 0.5, "kick/q"),
    ],
  });
  const mixedRendered = renderScoreToPcm(mixedScore);
  assert.ok(mixedRendered.pcmBuffer.sampleCount() > 0);
  assert.equal(mixedRendered.renderedNotes.length, 1);
  assert.equal(mixedRendered.renderedHits.length, 1);

  // Exceeding MAX_SAMPLE_COUNT limit
  assert.throws(
    () => renderScoreToPcm(mixedScore, MAX_SAMPLE_COUNT + 10),
    /exceeds MAX_SAMPLE_COUNT/,
  );

  // TextScore exceeding MAX_DURATION_SECONDS
  const longEvents: ScoreEvent[] = [];
  for (let i = 0; i < 700; i += 1) {
    longEvents.push(new ScoreEvent("note", "C4", null, "q", 1.0, 1.0, "C4/q"));
  }
  const longScore = new TextScore({
    title: "Long",
    tempoBpm: 60,
    meter: "4/4",
    amplitude: 0.1,
    sampleRateHz: 8000,
    instrumentId: "sine",
    events: longEvents,
  });
  assert.throws(
    () => renderScoreToPcm(longScore, 10_000_000),
    /exceeds MAX_DURATION_SECONDS/,
  );

  // PortableScore exceeding MAX_SAMPLE_COUNT
  const b = new PortableScoreBuilder({ sampleRateHz: 8000 });
  b.addInstrument("s", { kind: "sine" });
  b.addTrack("t", { instrumentId: "s" });
  b.addNote("t", 0, 120, "C4");
  const pScore = b.build();
  assert.throws(
    () => renderPortableScoreToPcm(pScore, MAX_SAMPLE_COUNT + 1),
    /exceeds MAX_SAMPLE_COUNT/,
  );

  // PortableScore with trailing rest that extends beyond notes
  const bRest = new PortableScoreBuilder({ sampleRateHz: 8000 });
  bRest.addInstrument("s", { kind: "sine" });
  bRest.addTrack("t", { instrumentId: "s" });
  bRest.addNote("t", 0, 120, "C4");
  bRest.addRest("t", 120, 240); // trailing rest extends past note
  const pScoreRest = bRest.build();
  const renderedRest = renderPortableScoreToPcm(pScoreRest);
  assert.ok(renderedRest.pcmBuffer.sampleCount() > 0);

  // mixPcmSamples 16-bit saturating clipping
  const dest = [32000, -32000, 0];
  const src = [2000, -2000, 100];
  const clipped = mixPcmSamples(dest, 0, src);
  assert.equal(clipped, 2); // both positive and negative clipped
  assert.equal(dest[0], 32767);
  assert.equal(dest[1], -32768);
  assert.equal(dest[2], 100);
});

test("Ultra branch coverage: builder, motifs, sections, harmonics, directives", () => {
  // PhraseMotifEvent branches
  assert.throws(
    () => new PhraseMotifEvent(0, 100, "invalid" as any),
    /kind must be 'note', 'rest', or 'hit'/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 100, "note", []),
    /note motif events must include at least one note/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 100, "hit", ["C4"], 1.0, "kick"),
    /hit motif events cannot include notes/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 100, "hit", [], 1.0, null),
    /hit motif events must include a hitId/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 100, "hit", [], 1.0, "bad_hit"),
    /unknown hit_id/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 100, "rest", ["C4"]),
    /rest motif events cannot include notes/,
  );

  // ArrangementSectionEvent branches
  assert.throws(
    () => new ArrangementSectionEvent("trk", 0, 100, "invalid" as any),
    /kind must be 'note', 'rest', or 'hit'/,
  );
  assert.throws(
    () => new ArrangementSectionEvent("trk", 0, 100, "note", []),
    /note section events must include at least one note/,
  );
  assert.throws(
    () => new ArrangementSectionEvent("trk", 0, 100, "hit", ["C4"], 1.0, "kick"),
    /hit section events cannot include notes/,
  );
  assert.throws(
    () => new ArrangementSectionEvent("trk", 0, 100, "hit", [], 1.0, null),
    /hit section events must include a hitId/,
  );
  assert.throws(
    () => new ArrangementSectionEvent("trk", 0, 100, "hit", [], 1.0, "bad_hit"),
    /unknown hit_id/,
  );
  assert.throws(
    () => new ArrangementSectionEvent("trk", 0, 100, "rest", ["C4"]),
    /rest section events cannot include notes/,
  );

  // PhraseMotif & ArrangementSection invalid items
  assert.throws(
    () => new PhraseMotif(100, ["not-event" as any]),
    /events must be PhraseMotifEvent instances/,
  );
  assert.throws(
    () => new ArrangementSection(100, ["not-event" as any]),
    /events must be ArrangementSectionEvent instances/,
  );

  // PortableScoreBuilder branches
  assert.throws(
    () => new PortableScoreBuilder({ sampleRateHz: 0 }),
    /sampleRateHz must be in \(0, 192000\]/,
  );
  assert.throws(
    () => new PortableScoreBuilder({ sampleRateHz: 999999 }),
    /sampleRateHz must be in \(0, 192000\]/,
  );
  assert.throws(
    () => new PortableScoreBuilder({ formatVersion: "v1" }),
    /formatVersion must be 'music-machine-score\/v2'/,
  );
  assert.throws(
    () => new PortableScoreBuilder().addInstrument("i", { kind: "invalid" as any }),
    /kind must be sine, silence, drumkit, or additive/,
  );

  // Portable parser property and harmonics branches
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s not_key_value\n"),
    /must be key=value/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=sine kind=sine\n"),
    /duplicate property 'kind'/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=\n"),
    /must provide harmonics/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=,\n"),
    /harmonics must contain at least one partial/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=" + Array(70).fill("1:1").join(",") + "\n"),
    /exceeds MAX_PARTIALS/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=0:1\n"),
    /invalid multiplier in partial/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=1:-1\n"),
    /invalid amplitude in partial/,
  );
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s kind=additive harmonics=1:1:bad\n"),
    /invalid phase in partial/,
  );

  // Score parser directive branches
  assert.throws(
    () => parseScore("title: Test\nfoo: bar\nC4/q\n"),
    /unknown directive 'foo'/,
  );
  assert.throws(
    () => parseScore("title: Test\nC4/q\ntempo: 120\n"),
    /directive 'tempo' appears after music/,
  );
  assert.throws(
    () => parseScore("title: Test\ntempo:\nC4/q\n"),
    /directive 'tempo' is empty/,
  );
  assert.throws(
    () => parseScore("title: Test\n" + "a".repeat(10005) + "\n"),
    /exceeds maxLineLength/,
  );

  // ScoreEvent & PortableScoreEvent branch coverage
  assert.throws(
    () => new ScoreEvent("invalid" as any, null, null, "q", 1.0, 1.0, "x"),
    /event kind must be 'note', 'rest', or 'hit'/,
  );
  assert.throws(
    () => new ScoreEvent("note", "", null, "q", 1.0, 1.0, "x"),
    /note events must include a note/,
  );
  assert.throws(
    () => new ScoreEvent("hit", null, "", "q", 1.0, 1.0, "x"),
    /hit events must include a hitId/,
  );
  assert.throws(
    () => new PortableScoreEvent("t", 0, 120, "hit", ["C4"], 1.0, 0, "kick"),
    /hit events cannot include notes/,
  );
  assert.throws(
    () => new PortableScoreEvent("t", 0, 120, "hit", [], 1.0, 0, null),
    /hit events must include a hitId/,
  );
  assert.throws(
    () => new PortableScoreEvent("t", 0, 120, "hit", [], 1.0, 0, "bad_hit"),
    /unknown hit_id/,
  );
  assert.throws(
    () => new PortableScoreEvent("t", 0, 120, "rest", ["C4"], 1.0, 0, null),
    /rest events cannot include notes/,
  );

  // validation.ts branches
  assert.throws(() => validateSafeInteger("x", "bad" as any), /must be a safe integer/);
  assert.throws(() => validateSampleRate(500), /sample_rate_hz must be in/);
  assert.throws(() => validateSampleRate(200000), /sample_rate_hz must be in/);
  assert.throws(() => validateIdentifier("id", "123bad"), /must be an identifier/);
  assert.throws(() => beatsForDurationSymbol("q.."), /at most one dot/);
  assert.throws(() => beatsForDurationSymbol("z"), /unknown duration symbol/);
});
