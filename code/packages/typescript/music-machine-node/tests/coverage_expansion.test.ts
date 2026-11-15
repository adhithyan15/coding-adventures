import assert from "node:assert/strict";
import test from "node:test";

import {
  AdsrEnvelope,
  HarmonicPartial,
  VariationProfile,
} from "@coding-adventures/musical-instruments";

import {
  ArrangementSection,
  ArrangementSectionEvent,
  assertSafeProperty,
  beatsForDurationSymbol,
  formatFloat,
  InstrumentDeclaration,
  isIdentifier,
  MeterEvent,
  meterParts,
  parsePortableScore,
  parsePortableScoreJson,
  parseScore,
  PhraseBuilder,
  PhraseMotif,
  PhraseMotifEvent,
  playPortableScore,
  playPortableScoreText,
  playScore,
  playScoreText,
  PortableScore,
  PortableScoreBuilder,
  PortableScoreEvent,
  renderPortableScoreToPcm,
  renderPortableScoreToWav,
  renderScoreToPcm,
  renderScoreToWav,
  resolveInstrumentId,
  ScoreEvent,
  TempoEvent,
  TempoMap,
  TextScore,
  toPortableScoreText,
  TrackDeclaration,
  transposeNoteText,
  validateFinite,
  validateIdentifier,
  validateMeter,
  validateNonNegative,
  validateNonNegativeInt,
  validatePositive,
  validatePositiveInt,
  validateSafeInteger,
  validateUnitFloat,
  warrantParsePortableScore,
  warrantParseScore,
  warrantRenderPortableScoreToPcm,
  warrantRenderScoreToPcm,
} from "../src/index.js";

test("Validation functions: comprehensive coverage", () => {
  // assertSafeProperty
  assert.throws(() => assertSafeProperty("__proto__"), /Forbidden property/);
  assert.throws(() => assertSafeProperty("constructor"), /Forbidden property/);
  assert.throws(() => assertSafeProperty("prototype"), /Forbidden property/);
  assert.throws(() => assertSafeProperty(123 as unknown as string), /property name must be a string/);
  assert.doesNotThrow(() => assertSafeProperty("validProperty"));

  // isIdentifier
  assert.equal(isIdentifier(""), false);
  assert.equal(isIdentifier("__proto__"), false);
  assert.equal(isIdentifier("123abc"), false);
  assert.equal(isIdentifier("-bad"), false);
  assert.equal(isIdentifier("good_id-1"), true);
  assert.equal(isIdentifier(null as unknown as string), false);

  // validateIdentifier
  assert.throws(() => validateIdentifier("id", 123 as unknown as string), /must be a string/);
  assert.throws(() => validateIdentifier("id", ""), /must be an identifier/);
  assert.equal(validateIdentifier("id", "track-1"), "track-1");

  // validateMeter & meterParts
  assert.deepEqual(meterParts("3/4"), [3, 4]);
  assert.deepEqual(meterParts("6/8"), [6, 8]);
  assert.throws(() => meterParts("4/0"), /meter must look like/);
  assert.throws(() => meterParts("0/4"), /meter must look like/);
  assert.throws(() => meterParts("bad"), /meter must look like/);
  assert.throws(() => meterParts(123 as unknown as string), /meter must look like/);

  // beatsForDurationSymbol
  assert.equal(beatsForDurationSymbol("w"), 4.0);
  assert.equal(beatsForDurationSymbol("h"), 2.0);
  assert.equal(beatsForDurationSymbol("q"), 1.0);
  assert.equal(beatsForDurationSymbol("e"), 0.5);
  assert.equal(beatsForDurationSymbol("s"), 0.25);
  assert.equal(beatsForDurationSymbol("q."), 1.5);
  assert.equal(beatsForDurationSymbol("h."), 3.0);
  assert.equal(beatsForDurationSymbol("w."), 6.0);
  assert.equal(beatsForDurationSymbol("e."), 0.75);
  assert.equal(beatsForDurationSymbol("s."), 0.375);
  assert.throws(() => beatsForDurationSymbol(""), /duration symbol must be non-empty/);
  assert.throws(() => beatsForDurationSymbol("q.."), /may have at most one dot/);
  assert.throws(() => beatsForDurationSymbol("z"), /unknown duration symbol 'z'/);
  assert.throws(() => beatsForDurationSymbol(123 as unknown as string), /duration symbol must be non-empty/);

  // resolveInstrumentId
  assert.equal(resolveInstrumentId("sine"), "sine");
  assert.equal(resolveInstrumentId("standard_kit"), "drumkit");
  assert.equal(resolveInstrumentId("drumkit"), "drumkit");
  assert.throws(() => resolveInstrumentId("non_existent_inst_xyz"), /Unknown instrument/i);

  // transposeNoteText
  assert.equal(transposeNoteText("C4", 0), "C4");
  assert.equal(transposeNoteText("C4", 12), "C5");
  assert.equal(transposeNoteText("C4", -12), "C3");
  assert.equal(transposeNoteText("C4", 2), "D4");
  assert.equal(transposeNoteText("B4", 1), "C5");
  assert.equal(transposeNoteText("C4", -1), "B3");
  assert.throws(() => transposeNoteText(123 as unknown as string, 0), /note text must be a string/);
  assert.throws(() => transposeNoteText("C4", Number.NaN), /semitoneOffset must be a safe integer/);

  // Numeric validators
  assert.throws(() => validateFinite("val", Number.NaN), /val must be a finite number/);
  assert.throws(() => validateFinite("val", "abc" as unknown as number), /val must be a finite number/);
  assert.throws(() => validatePositive("val", 0), /val must be > 0.0/);
  assert.throws(() => validatePositive("val", -5), /val must be > 0.0/);
  assert.throws(() => validatePositiveInt("val", 1.5), /val must be a safe integer/);
  assert.throws(() => validatePositiveInt("val", 0), /val must be a positive integer/);
  assert.throws(() => validateNonNegative("val", -1), /val must be >= 0.0/);
  assert.throws(() => validateNonNegativeInt("val", -1), /val must be an integer >= 0/);
  assert.throws(() => validateNonNegativeInt("val", 1.5), /val must be a safe integer/);
  assert.throws(() => validateSafeInteger("val", Number.MAX_SAFE_INTEGER + 10), /val must be a safe integer/);
  assert.throws(() => validateSafeInteger("val", 1.2), /val must be a safe integer/);
  assert.throws(() => validateUnitFloat("val", 1.5), /val must be in \[0.0, 1.0\]/);
  assert.throws(() => validateUnitFloat("val", -0.1), /val must be in \[0.0, 1.0\]/);

  // formatFloat
  assert.equal(formatFloat(10), "10.0");
  assert.equal(formatFloat(10.5), "10.5");
  assert.equal(formatFloat(10.12345678), "10.12345678");
  assert.throws(() => formatFloat(Number.NaN), /cannot format non-finite float/);
});

test("TempoMap: binary search tickAtSeconds and multiple tempos", () => {
  const tempos = [
    new TempoEvent(0, 120),
    new TempoEvent(480, 60),
    new TempoEvent(960, 240),
  ];
  const map = new TempoMap(tempos, 480);
  assert.equal(map.ppq, 480);

  // secondsAtTick
  const s0 = map.secondsAtTick(0);
  assert.equal(s0, 0);
  const s480 = map.secondsAtTick(480);
  assert.ok(Math.abs(s480 - 0.5) < 1e-6);
  const s960 = map.secondsAtTick(960);
  assert.ok(Math.abs(s960 - 1.5) < 1e-6);

  // tickAtSeconds
  assert.equal(map.tickAtSeconds(0), 0);
  assert.equal(map.tickAtSeconds(0.5), 480);
  assert.equal(map.tickAtSeconds(1.5), 960);
  assert.ok(map.tickAtSeconds(0.25) > 0);

  // Empty tempo events defaults to tick 0 @ 120 bpm
  const emptyMap = new TempoMap([], 480);
  assert.equal(emptyMap.secondsAtTick(480), 0.5);

  // Errors
  assert.throws(() => map.secondsAtTick(-1), /must be an integer >= 0/);
  assert.throws(() => map.tickAtSeconds(-1), /must be >= 0.0/);
});

test("Types: constructors, getters, and errors", () => {
  // ScoreEvent
  const ev1 = new ScoreEvent("note", "A4", null, "q", 1.0, 0.5, "A4/q");
  assert.equal(ev1.kind, "note");
  assert.equal(ev1.note, "A4");
  assert.equal(ev1.hitId, null);

  const ev2 = new ScoreEvent("rest", null, null, "h", 2.0, 1.0, "R/h");
  assert.equal(ev2.kind, "rest");
  assert.equal(ev2.note, null);

  const ev3 = new ScoreEvent("hit", null, "snare", "e", 0.5, 0.25, "snare/e");
  assert.equal(ev3.kind, "hit");
  assert.equal(ev3.hitId, "snare");

  assert.throws(() => new ScoreEvent("invalid" as unknown as "note", null, null, "q", 1, 1, ""), /event kind must be/);
  assert.throws(() => new ScoreEvent("note", null, null, "q", 1, 1, ""), /note events must include a note/);
  assert.throws(() => new ScoreEvent("hit", null, null, "q", 1, 1, ""), /hit events must include a hitId/);
  assert.throws(() => new ScoreEvent("hit", null, "bad_hit_xyz", "q", 1, 1, ""), /unknown hit_id/);

  // TextScore
  const textScore = new TextScore({
    title: "Test",
    tempoBpm: 120,
    meter: "4/4",
    amplitude: 0.2,
    sampleRateHz: 44100,
    instrumentId: "sine",
    gmProgram: null,
    drumkit: null,
    events: [ev1, ev2, ev3],
  });
  assert.equal(textScore.totalDurationBeats(), 3.5);
  assert.ok(textScore.totalDurationSeconds() > 1.0);
  assert.equal(textScore.secondsPerBeat(), 0.5);

  // TempoEvent & MeterEvent
  const te = new TempoEvent(0, 120);
  assert.equal(te.startTick, 0);
  assert.equal(te.bpm, 120);

  const me = new MeterEvent(0, "3/4");
  assert.equal(me.startTick, 0);
  assert.equal(me.meter, "3/4");

  // TrackDeclaration
  const td = new TrackDeclaration("lead", "sine");
  assert.equal(td.id, "lead");
  assert.equal(td.instrumentId, "sine");

  // PortableScoreEvent
  const pseNote = new PortableScoreEvent("lead", 0, 120, "note", ["C4"], 0.8, 0, null);
  assert.equal(pseNote.kind, "note");
  assert.deepEqual(pseNote.notes, ["C4"]);

  const pseHit = new PortableScoreEvent("lead", 120, 120, "hit", [], 0.8, 1, "kick");
  assert.equal(pseHit.kind, "hit");
  assert.equal(pseHit.hitId, "kick");

  const pseRest = new PortableScoreEvent("lead", 240, 120, "rest", [], 0.0, 2, null);
  assert.equal(pseRest.kind, "rest");

  assert.throws(() => new PortableScoreEvent("lead", 0, 120, "invalid" as unknown as "note", [], 0.8, 0, null), /kind must be/);
  assert.throws(() => new PortableScoreEvent("lead", 0, 120, "note", [], 0.8, 0, null), /note events must include at least one note/);
  assert.throws(() => new PortableScoreEvent("lead", 0, 120, "hit", ["C4"], 0.8, 0, "kick"), /hit events cannot include notes/);
  assert.throws(() => new PortableScoreEvent("lead", 0, 120, "hit", [], 0.8, 0, null), /hit events must include a hitId/);
  assert.throws(() => new PortableScoreEvent("lead", 0, 120, "rest", ["C4"], 0.8, 0, null), /rest events cannot include notes/);

  // PhraseMotifEvent & PhraseMotif
  const pme = new PhraseMotifEvent(0, 120, "note", ["C4"], 0.8);
  assert.equal(pme.durationTick, 120);
  const pm = new PhraseMotif(240, [pme]);
  assert.equal(pm.durationTick, 240);

  // ArrangementSectionEvent & ArrangementSection
  const ase = new ArrangementSectionEvent("lead", 0, 120, "note", ["C4"], 0.8, null);
  const sec = new ArrangementSection(240, [ase]);
  assert.equal(sec.durationTick, 240);
});

test("PortableScore: full constructor validations and totalDurationTicks", () => {
  const inst = new InstrumentDeclaration("synth", "sine", 0.5, "preset");
  const trk = new TrackDeclaration("melody", "synth");
  const ev = new PortableScoreEvent("melody", 0, 240, "note", ["C4"], 0.8, 0, null);

  const score = new PortableScore({
    formatVersion: "music-machine-score/v2",
    title: "Orchestra",
    ppq: 120,
    sampleRateHz: 44100,
    tempoEvents: [new TempoEvent(0, 120)],
    meterEvents: [new MeterEvent(0, "4/4")],
    instruments: [inst],
    tracks: [trk],
    events: [ev],
  });

  assert.equal(score.totalDurationTicks(), 240);

  // Validation errors
  assert.throws(
    () => new PortableScore({ ...score, formatVersion: "wrong/v1" }),
    /format_version must be 'music-machine-score\/v2'/,
  );
  assert.throws(
    () => new PortableScore({ ...score, tracks: [new TrackDeclaration("m1", "unknown_inst")] }),
    /references unknown instrument/,
  );
  assert.throws(
    () => new PortableScore({
      ...score,
      tracks: [trk, trk],
    }),
    /duplicate track id/,
  );
  assert.throws(
    () => new PortableScore({
      ...score,
      events: [new PortableScoreEvent("unknown_track", 0, 120, "note", ["C4"], 0.8, 0, null)],
    }),
    /references unknown track/,
  );
});

test("PortableScoreBuilder & PhraseBuilder: advanced sequencing and features", () => {
  const builder = new PortableScoreBuilder({ title: "Symphony", ppq: 480, sampleRateHz: 44100 });

  // Instrument with custom ADSR, partials
  builder.addInstrument("organ", {
    kind: "additive",
    envelope: new AdsrEnvelope(0.01, 0.05, 0.8, 0.1),
    harmonics: [new HarmonicPartial(1, 1.0, 0), new HarmonicPartial(2, 0.5, 0.1)],
    gain: 0.6,
  });

  // Instrument with GM program
  builder.addInstrument("acoustic_bass", {
    program: 32,
    gain: 0.7,
  });

  builder.addTrack("keys", { instrumentId: "organ" });
  builder.addTrack("bass", { instrumentId: "acoustic_bass" });

  builder.addMeter(0, "4/4");
  builder.addTempo(0, 120);

  // Builder events
  builder.addNote("keys", 0, 240, "C4", { velocity: 0.8 });
  builder.addChord("keys", 240, 240, ["E4", "G4"], { velocity: 0.7 });
  builder.addRest("keys", 480, 240);
  builder.addNote("bass", 0, 480, "C2", { velocity: 0.9 });

  // Measure and tick math
  assert.equal(builder.beatsToTicks(1.0), 480);
  assert.equal(builder.measureTicks("4/4"), 1920);
  assert.equal(builder.tickInMeasure(1, 1.0), 480);

  // PhraseBuilder
  const phrase = new PhraseBuilder(builder, "keys", 720);
  assert.equal(phrase.currentTick, 720);
  phrase.note("A4", 0.5, { velocity: 0.75 });
  phrase.chord(["C5", "E5"], 0.5);
  phrase.rest(0.5);
  phrase.advanceBeats(1.0);
  phrase.jumpToMeasure(4, 0.5);

  // Motifs
  const motif = phrase.motif({ startTick: 720 });
  assert.ok(motif.durationTick > 0);

  // Apply motif with transposition and repeat
  phrase.applyMotif(motif, {
    transposeSemitones: 2,
    velocityScale: 0.9,
    repeatCount: 2,
    advance: true,
  });

  // Sections
  const section = builder.captureSection({
    startTick: 0,
    endTick: 960,
  });
  assert.ok(section.events.length > 0);

  builder.applySection(section, 5000, {
    trackMap: { keys: "keys", bass: "bass" },
    transposeSemitones: -2,
    velocityScale: 0.95,
    repeatCount: 1,
  });

  const score = builder.build();
  assert.ok(score.events.length > 5);

  // toText
  const text = builder.toText();
  assert.ok(text.includes("format: music-machine-score/v2"));
  assert.ok(text.includes("track keys instrument=organ"));
});

test("PortableScore text parser: instruments with envelopes and partials", () => {
  const complexText = `format: music-machine-score/v2
title: Complex Synth
ppq: 240
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument lead kind=additive envelope=0.01,0.02,0.7,0.05 harmonics=1:1.0:0.0,2:0.5:0.1 gain=0.25
instrument bass program=34 gain=0.30
instrument drums kind=drumkit kit=standard_kit gain=0.40

track melody instrument=lead
track low instrument=bass
track rhythm instrument=drums

event melody 0 240 note C4 velocity=0.8
event melody 240 240 note E4,G4 velocity=0.75
event low 0 480 note C2 velocity=0.7
event rhythm 0 120 hit kick velocity=0.9
event rhythm 120 120 hit snare velocity=0.85
event melody 480 240 rest
`;

  const score = parsePortableScore(complexText);
  assert.equal(score.title, "Complex Synth");
  assert.equal(score.instruments.length, 3);
  assert.equal(score.tracks.length, 3);
  assert.equal(score.events.length, 6);

  // Check instrument parsing details
  const lead = score.instruments.find((i) => i.id === "lead")!;
  assert.ok(lead.envelope);
  assert.equal(lead.envelope?.attackSeconds, 0.01);
  assert.equal(lead.envelope?.decaySeconds, 0.02);
  assert.equal(lead.envelope?.sustainLevel, 0.7);
  assert.equal(lead.envelope?.releaseSeconds, 0.05);
  assert.ok(lead.harmonics);
  assert.equal(lead.harmonics?.length, 2);
  assert.equal(lead.gain, 0.25);

  // Roundtrip text serialization
  const outText = toPortableScoreText(score);
  const reScore = parsePortableScore(outText);
  assert.equal(reScore.title, score.title);
  assert.equal(reScore.events.length, score.events.length);
});

test("PortableScore JSON: full roundtrip with all instrument features", () => {
  const builder = new PortableScoreBuilder({ title: "Json Test", ppq: 120, sampleRateHz: 44100 });
  builder.addInstrument("lead", {
    kind: "additive",
    envelope: new AdsrEnvelope(0.02, 0.04, 0.8, 0.1),
    harmonics: [new HarmonicPartial(1, 1.0, 0), new HarmonicPartial(3, 0.3, 0.5)],
    gain: 0.3,
  });
  builder.addInstrument("drums", {
    profile: "standard_kit",
    gain: 0.5,
  });
  builder.addTrack("lead_trk", { instrumentId: "lead" });
  builder.addTrack("drum_trk", { instrumentId: "drums" });

  builder.addTempo(0, 130);
  builder.addMeter(0, "4/4");
  builder.addNote("lead_trk", 0, 120, "C4", { velocity: 0.8 });
  builder.addChord("lead_trk", 120, 120, ["E4", "G4"], { velocity: 0.7 });
  builder.addRest("lead_trk", 240, 120);
  builder.addHit("drum_trk", 0, 120, "kick", { velocity: 0.9 });

  const original = builder.build();
  const jsonStr = original.toJson();
  const parsed = parsePortableScoreJson(jsonStr);

  assert.equal(parsed.title, original.title);
  assert.equal(parsed.ppq, original.ppq);
  assert.equal(parsed.sampleRateHz, original.sampleRateHz);
  assert.equal(parsed.instruments.length, 2);
  assert.equal(parsed.tracks.length, 2);
  assert.equal(parsed.events.length, 4);

  const parsedLead = parsed.instruments[0];
  assert.equal(parsedLead.envelope?.attackSeconds, 0.02);
  assert.equal(parsedLead.harmonics?.length, 2);
  assert.equal(parsedLead.gain, 0.3);
});

test("PortableScore JSON parser error branches", () => {
  assert.throws(() => parsePortableScoreJson(123 as unknown as string), /jsonText must be a string/);
  assert.throws(() => parsePortableScoreJson("not json"), /JSON parse error/);
  assert.throws(() => parsePortableScoreJson("[]"), /JSON score root must be an object/);
  assert.throws(() => parsePortableScoreJson('{"unknown_key": 1}'), /unknown score keys/);

  const baseJson: Record<string, unknown> = {
    format_version: "music-machine-score/v2",
    title: "Bad",
    ppq: 120,
    sample_rate_hz: 44100,
    tempo_events: [],
    meter_events: [],
    instruments: [],
    tracks: [],
    events: [],
  };

  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, format_version: 123 })),
    /missing 'format_version'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, ppq: -1 })),
    /missing or invalid 'ppq'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, sample_rate_hz: "fast" })),
    /missing or invalid 'sample_rate_hz'/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, tempo_events: "not array" })),
    /tempo_events must be a list/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, meter_events: "not array" })),
    /meter_events must be a list/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, instruments: "not array" })),
    /instruments must be a list/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, tracks: "not array" })),
    /tracks must be a list/,
  );
  assert.throws(
    () => parsePortableScoreJson(JSON.stringify({ ...baseJson, events: "not array" })),
    /events must be a list/,
  );
});

test("Render: error checks and edge cases", () => {
  const score = parseScore("C4/q\n");
  assert.throws(() => renderScoreToPcm(score, 10), /exceeding maxSampleCount=/);
  assert.throws(
    () => renderScoreToPcm("not a score" as unknown as TextScore),
    /score must be a TextScore/,
  );

  const portableScore = parsePortableScore(`format: music-machine-score/v2
title: Empty
ppq: 120
sample_rate: 44100
tempo 0 120
meter 0 4/4
instrument synth profile=sine
track lead instrument=synth
event lead 0 120 note C4
`);
  assert.throws(
    () => renderPortableScoreToPcm(portableScore, 10),
    /exceeding maxSampleCount=/,
  );
  assert.throws(
    () => renderPortableScoreToPcm("not a portable score" as unknown as PortableScore),
    /score must be a PortableScore/,
  );
});

test("Audio device sink playback: playScore, playPortableScore, and text helpers", () => {
  const score = parseScore("title: PlayTest\nC4/q\n");
  const report1 = playScore(score);
  assert.equal(report1.sampleRateHz, 44100);
  assert.equal(report1.backendName, "noop");

  const portableScore = parsePortableScore(`format: music-machine-score/v2
title: PlayPortable
ppq: 120
sample_rate: 44100
tempo 0 120
meter 0 4/4
instrument synth profile=sine
track lead instrument=synth
event lead 0 120 note C4
`);
  const report2 = playPortableScore(portableScore);
  assert.equal(report2.sampleRateHz, 44100);
  assert.equal(report2.backendName, "noop");

  // Play text shortcuts
  const rep3 = playScoreText("C4/q\n");
  assert.equal(rep3.sampleRateHz, 44100);
  const rep4 = playPortableScoreText(toPortableScoreText(portableScore));
  assert.equal(rep4.sampleRateHz, 44100);
});

test("Warrant operations: score and portable score budgets", () => {
  assert.equal(warrantParseScore(100), 100);
  assert.throws(() => warrantParseScore(2_000_000), /score_bytes > MAX_SCORE_BYTES/);

  assert.equal(warrantParsePortableScore(100), 100);
  assert.throws(() => warrantParsePortableScore(2_000_000), /score_bytes > MAX_SCORE_BYTES/);

  assert.equal(warrantRenderScoreToPcm(10, 44100), 10);
  assert.throws(() => warrantRenderScoreToPcm(20_000, 44100), /event_count > MAX_EVENTS/);

  assert.equal(warrantRenderPortableScoreToPcm(10, 44100), 10);
  assert.throws(() => warrantRenderPortableScoreToPcm(20_000, 44100), /event_count > MAX_EVENTS/);
});

test("Score parser: directive branches, GM program, and comments", () => {
  const scoreText = `# Top comment
title: Advanced Score
# Middle comment
tempo: 140
meter: 3/4
amplitude: 0.3
sample_rate: 22050
program: 25

C4/q. D4/e E4/h | R/q F4/q |
`;
  const score = parseScore(scoreText);
  assert.equal(score.title, "Advanced Score");
  assert.equal(score.tempoBpm, 140);
  assert.equal(score.meter, "3/4");
  assert.equal(score.amplitude, 0.3);
  assert.equal(score.sampleRateHz, 22050);
  assert.equal(score.gmProgram, 25);
  assert.equal(score.events.length, 5);

  const rendered = renderScoreToPcm(score);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
  assert.equal(rendered.renderedNotes.length, 4);

  // Error: both instrument and program
  assert.throws(
    () => parseScore("instrument: sine\nprogram: 1\nC4/q\n"),
    /score may specify instrument or program, not both/,
  );
  assert.throws(
    () => parseScore("program: 1\ninstrument: sine\nC4/q\n"),
    /score may specify instrument or program, not both/,
  );

  // Directives after music
  assert.throws(
    () => parseScore("C4/q\ntitle: Late\n"),
    /directive 'title' appears after music/,
  );

  // Empty directive value
  assert.throws(
    () => parseScore("title:\nC4/q\n"),
    /directive 'title' is empty/,
  );
});

test("Portable parser: property and harmonics error branches", () => {
  // Missing format version
  assert.throws(
    () => parsePortableScore("title: No Format\n"),
    /first directive must be 'format: music-machine-score\/v2'/,
  );

  // Duplicate properties
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s profile=sine profile=flute\n"),
    /duplicate property 'profile'/,
  );

  // Malformed property
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s invalid_prop\n"),
    /must be key=value/,
  );

  // Instrument without selectors
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s gain=0.5\n"),
    /instrument must specify exactly one of/,
  );

  // Instrument with both profile and program
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s profile=sine program=1\n"),
    /instrument must specify exactly one of/,
  );

  // Track with unknown instrument
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ntrack t instrument=ghost\n"),
    /references unknown instrument 'ghost'/,
  );

  // Event with unknown track
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\ninstrument s profile=sine\ntrack t instrument=s\nevent ghost 0 120 note C4\n"),
    /event references unknown track 'ghost'/,
  );

  // Unknown directive
  assert.throws(
    () => parsePortableScore("format: music-machine-score/v2\nunknown_directive foo bar\n"),
    /unknown command 'unknown_directive'/,
  );
});

test("Builder and Section error handling", () => {
  const b = new PortableScoreBuilder();
  b.addInstrument("lead", { profile: "flute_naive" });
  b.addInstrument("s1", { kind: "sine" });
  b.addInstrument("s2", { kind: "silence" });
  b.addTrack("trk", { instrumentId: "lead" });

  // PhraseMotifEvent errors
  assert.throws(
    () => new PhraseMotifEvent(0, 120, "note", []),
    /note motif events must include at least one note/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 120, "hit", [], 1.0, null),
    /hit motif events must include a hitId/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 120, "hit", ["C4"], 1.0, "kick"),
    /hit motif events cannot include notes/,
  );
  assert.throws(
    () => new PhraseMotifEvent(0, 120, "rest", ["C4"]),
    /rest motif events cannot include notes/,
  );

  // Motif event out of bounds
  assert.throws(
    () => new PhraseMotif(100, [new PhraseMotifEvent(50, 60, "note", ["C4"])]),
    /event extends past motif duration_tick/,
  );

  // ArrangementSection event out of bounds
  assert.throws(
    () => new ArrangementSection(100, [new ArrangementSectionEvent("trk", 50, 60, "note", ["C4"], 1.0, null)]),
    /section event extends beyond end_tick/,
  );

  // Builder instrument selector error
  assert.throws(
    () => b.addInstrument("bad1", {}),
    /instrument must specify exactly one of profile, program, or kind/,
  );
  assert.throws(
    () => b.addInstrument("bad2", { profile: "sine", program: 1 }),
    /instrument must specify exactly one of profile, program, or kind/,
  );
  assert.throws(
    () => b.addInstrument("bad3", { kind: "unsupported_kind" }),
    /kind must be sine, silence, drumkit, or additive/,
  );

  // Duplicate instrument detected on build
  const bDup = new PortableScoreBuilder();
  bDup.addInstrument("inst1", { profile: "sine" });
  bDup.addInstrument("inst1", { profile: "sine" });
  assert.throws(() => bDup.build(), /duplicate instrument id 'inst1'/);

  // Unknown instrument referenced by track detected on build
  const bBadTrk = new PortableScoreBuilder();
  bBadTrk.addTrack("t1", { instrumentId: "ghost_inst" });
  assert.throws(() => bBadTrk.build(), /references unknown instrument 'ghost_inst'/);

  // Unknown track referenced by event detected on build
  const bBadEv = new PortableScoreBuilder();
  bBadEv.addInstrument("inst1", { profile: "sine" });
  bBadEv.addTrack("t1", { instrumentId: "inst1" });
  bBadEv.addNote("unknown_track", 0, 120, "C4");
  assert.throws(() => bBadEv.build(), /event references unknown track 'unknown_track'/);

  // Capture motif invalid bounds
  assert.throws(
    () => b.captureMotif("trk", { startTick: 200, endTick: 100 }),
    /endTick must be >= startTick/,
  );
});

test("Render: multi-track with drumkit hi-hat choke, pan, and GM program", () => {
  const text = `format: music-machine-score/v2
title: Choke and GM
ppq: 120
sample_rate: 22050
tempo 0 120
meter 0 4/4

instrument lead program=73 gain=0.25
instrument drums kind=drumkit kit=standard_kit gain=0.30

track flute instrument=lead
track kit instrument=drums

event flute 0 120 note C5 velocity=0.8
event flute 120 120 note D5 velocity=0.7
event kit 0 120 hit open_hihat velocity=0.85
event kit 60 60 hit closed_hihat velocity=0.8
event kit 120 120 hit kick velocity=0.9
`;

  const score = parsePortableScore(text);
  const rendered = renderPortableScoreToPcm(score);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
  assert.equal(rendered.renderedHits.length, 3);
  assert.equal(rendered.renderedNotes.length, 2);

  const wav = renderPortableScoreToWav(score);
  assert.ok(wav.length > 44);
});

