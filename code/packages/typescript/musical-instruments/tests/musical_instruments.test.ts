import assert from "node:assert/strict";
import test from "node:test";

import {
  AdsrEnvelope,
  ADSREnvelope,
  allGmPrograms,
  allInstruments,
  brassNaive,
  celestaNaive,
  clarinetNaive,
  DEFAULT_MAX_PARTIAL_COUNT,
  DEFAULT_MAX_SAMPLE_COUNT,
  DEFAULT_SAMPLE_RATE_HZ,
  effectNaive,
  fluteNaive,
  getGmProgram,
  getInstrument,
  glockenspielNaive,
  GM_PROGRAMS,
  GMProgram,
  HarmonicPartial,
  InstrumentNoteRender,
  InstrumentProfile,
  InstrumentSignal,
  instrumentForGmProgram,
  kalimbaNaive,
  malletNaive,
  marimbaNaive,
  MAX_NOTE_DURATION_SECONDS,
  MAX_OCTAVE,
  MAX_PARTIALS,
  MAX_SAMPLE_COUNT,
  MAX_SAMPLE_RATE_HZ,
  MIN_OCTAVE,
  Note,
  noteToFrequency,
  organNaive,
  parseNote,
  pianoNaive,
  pluckNaive,
  renderInstrumentNote,
  renderInstrumentNotePcm,
  resolveNote,
  silence,
  sine,
  SplitMix64,
  synthLeadNaive,
  synthPadNaive,
  timpaniNaive,
  tubularBellsNaive,
  TWO_PI,
  VariationProfile,
  vibraphoneNaive,
  violinNaive,
  xylophoneNaive,
} from "../src/index.js";

const EPSILON = 1e-6;

function assertClose(actual: number, expected: number, eps = EPSILON, msg?: string): void {
  assert.ok(
    Math.abs(actual - expected) <= eps,
    `${msg ?? "assertClose"}: expected ${expected}, got ${actual} (diff ${Math.abs(actual - expected)})`,
  );
}

test("Constants match MUS05 specifications", () => {
  assert.equal(MAX_PARTIALS, 64);
  assert.equal(MAX_NOTE_DURATION_SECONDS, 60.0);
  assert.equal(MAX_SAMPLE_RATE_HZ, 192_000.0);
  assert.equal(MAX_SAMPLE_COUNT, 16_777_216);
  assert.equal(DEFAULT_MAX_PARTIAL_COUNT, 64);
  assert.equal(DEFAULT_MAX_SAMPLE_COUNT, 10_000_000);
  assert.equal(DEFAULT_SAMPLE_RATE_HZ, 44_100.0);
  assert.equal(MIN_OCTAVE, -10);
  assert.equal(MAX_OCTAVE, 20);
  assertClose(TWO_PI, 2.0 * Math.PI);
});

test("Note construction and parsing without regex", () => {
  const a4 = new Note("A", "", 4);
  assert.equal(a4.letter, "A");
  assert.equal(a4.accidental, "");
  assert.equal(a4.octave, 4);
  assert.equal(a4.spelling(), "A");
  assert.equal(a4.chromaticIndex(), 9);
  assert.equal(a4.semitonesFromA4(), 0);
  assertClose(a4.frequency(), 440.0);
  assert.equal(a4.toString(), "A4");

  const parsedA4 = parseNote("A4");
  assert.equal(parsedA4.letter, "A");
  assert.equal(parsedA4.accidental, "");
  assert.equal(parsedA4.octave, 4);
  assertClose(parsedA4.frequency(), 440.0);

  const parsedCSharp5 = parseNote("C#5");
  assert.equal(parsedCSharp5.letter, "C");
  assert.equal(parsedCSharp5.accidental, "#");
  assert.equal(parsedCSharp5.octave, 5);
  assert.equal(parsedCSharp5.spelling(), "C#");
  assert.equal(parsedCSharp5.chromaticIndex(), 1);
  assert.equal(parsedCSharp5.semitonesFromA4(), 4);
  assertClose(parsedCSharp5.frequency(), 440.0 * 2.0 ** (4 / 12));

  const parsedDb3 = parseNote("Db3");
  assert.equal(parsedDb3.letter, "D");
  assert.equal(parsedDb3.accidental, "b");
  assert.equal(parsedDb3.octave, 3);
  assert.equal(parsedDb3.chromaticIndex(), 1);

  const parsedNegative = parseNote("C-1");
  assert.equal(parsedNegative.octave, -1);
  assert.equal(parsedNegative.toString(), "C-1");

  assertClose(noteToFrequency("A4"), 440.0);
  assert.equal(resolveNote(a4), a4);
  assert.equal(resolveNote("A4").toString(), "A4");
});

test("HarmonicPartial stores validated parameters", () => {
  const part = new HarmonicPartial(2.0, 0.5, 0.25);
  assert.equal(part.frequencyMultiplier, 2.0);
  assert.equal(part.amplitude, 0.5);
  assert.equal(part.phaseCycles, 0.25);

  const defaultPhase = new HarmonicPartial(1.0, 0.8);
  assert.equal(defaultPhase.phaseCycles, 0.0);
});

test("AdsrEnvelope evaluates stages and rendered duration correctly", () => {
  const env = new AdsrEnvelope(0.1, 0.2, 0.5, 0.3);
  assert.equal(env.attackSeconds, 0.1);
  assert.equal(env.decaySeconds, 0.2);
  assert.equal(env.sustainLevel, 0.5);
  assert.equal(env.releaseSeconds, 0.3);

  // renderedDurationSeconds = noteDuration + releaseSeconds
  assertClose(env.renderedDurationSeconds(1.0), 1.3);

  // Attack stage (0 to 0.1s): linear from 0 to 1
  assertClose(env.valueAt(0.0, 1.0), 0.0);
  assertClose(env.valueAt(0.05, 1.0), 0.5);
  assertClose(env.valueAt(0.1, 1.0), 1.0);

  // Decay stage (0.1 to 0.3s): linear from 1 to sustainLevel (0.5)
  assertClose(env.valueAt(0.2, 1.0), 0.75);
  assertClose(env.valueAt(0.3, 1.0), 0.5);

  // Sustain stage (0.3 to 1.0s): holds sustainLevel (0.5)
  assertClose(env.valueAt(0.5, 1.0), 0.5);
  assertClose(env.valueAt(0.999, 1.0), 0.5);

  // Release stage (1.0 to 1.3s): linear from sustainLevel (0.5) to 0.0
  assertClose(env.valueAt(1.0, 1.0), 0.5);
  assertClose(env.valueAt(1.15, 1.0), 0.25);
  assertClose(env.valueAt(1.3, 1.0), 0.0);

  // Post-release (> 1.3s): 0.0
  assertClose(env.valueAt(1.5, 1.0), 0.0);

  // Instant attack and decay
  const instant = new AdsrEnvelope(0.0, 0.0, 0.8, 0.1);
  assertClose(instant.valueAt(0.0, 0.5), 0.8);
  assertClose(instant.valueAt(0.2, 0.5), 0.8);
});

test("SplitMix64 is strictly deterministic and produces bounded floats", () => {
  const rng1 = new SplitMix64(42);
  const rng2 = new SplitMix64(42);

  for (let i = 0; i < 50; i += 1) {
    const u1 = rng1.nextU64();
    const u2 = rng2.nextU64();
    assert.equal(u1, u2);
  }

  const rngSigned = new SplitMix64(12345);
  for (let i = 0; i < 100; i += 1) {
    const s = rngSigned.nextFloatSigned();
    assert.ok(s >= -1.0 && s <= 1.0, `Signed float out of range: ${s}`);
  }

  const rngUnit = new SplitMix64(54321);
  for (let i = 0; i < 100; i += 1) {
    const u = rngUnit.nextFloatUnit();
    assert.ok(u >= 0.0 && u <= 1.0, `Unit float out of range: ${u}`);
  }
});

test("VariationProfile correctly exposes getters and aliases", () => {
  const defaultVar = new VariationProfile();
  assert.equal(defaultVar.pitchCentsJitter, 0.0);
  assert.equal(defaultVar.amplitudeJitter, 0.0);
  assert.equal(defaultVar.attackJitterMs, 0.0);
  assert.equal(defaultVar.seed, null);
  assert.equal(defaultVar.harmonicJitter, 0.0);

  const custom = new VariationProfile({
    pitchJitterCents: 5.0,
    amplitudeJitter: 0.1,
    timingJitterSeconds: 0.002,
    seed: 99,
    harmonicJitter: 0.05,
  });
  assert.equal(custom.pitchCentsJitter, 5.0);
  assert.equal(custom.pitchJitterCents, 5.0);
  assert.equal(custom.amplitudeJitter, 0.1);
  assert.equal(custom.attackJitterMs, 2.0);
  assert.equal(custom.timingJitterSeconds, 0.002);
  assert.equal(custom.seed, 99);
  assert.equal(custom.harmonicJitter, 0.05);
});

test("InstrumentProfile stores properties and computes amplitude normalizer", () => {
  const p1 = new HarmonicPartial(1.0, 1.0);
  const p2 = new HarmonicPartial(2.0, 0.5);
  const env = new AdsrEnvelope(0.01, 0.02, 0.8, 0.05);
  const inst = new InstrumentProfile(
    "test_lead",
    "Test Lead",
    "additive",
    0.8,
    [p1, p2],
    env,
  );
  assert.equal(inst.id, "test_lead");
  assert.equal(inst.displayName, "Test Lead");
  assert.equal(inst.synthesisKind, "additive");
  assert.equal(inst.gain, 0.8);
  assert.equal(inst.harmonicProfile.length, 2);
  assertClose(inst.amplitudeNormalizer(), 1.5);
});

test("All 21 built-in naive instrument presets exist and can be retrieved", () => {
  const presets = allInstruments();
  assert.equal(presets.length, 21);

  const expectedIds = [
    "sine",
    "silence",
    "flute_naive",
    "clarinet_naive",
    "violin_naive",
    "piano_naive",
    "pluck_naive",
    "brass_naive",
    "organ_naive",
    "mallet_naive",
    "celesta_naive",
    "glockenspiel_naive",
    "vibraphone_naive",
    "marimba_naive",
    "xylophone_naive",
    "tubular_bells_naive",
    "timpani_naive",
    "kalimba_naive",
    "synth_lead_naive",
    "synth_pad_naive",
    "effect_naive",
  ];

  for (const id of expectedIds) {
    const inst = getInstrument(id);
    assert.equal(inst.id, id);
    assert.ok(inst.gain >= 0.0 && inst.gain <= 1.0);
  }

  // Verify individual preset factories
  assert.equal(sine().id, "sine");
  assert.equal(silence().id, "silence");
  assert.equal(fluteNaive().id, "flute_naive");
  assert.equal(clarinetNaive().id, "clarinet_naive");
  assert.equal(violinNaive().id, "violin_naive");
  assert.equal(pianoNaive().id, "piano_naive");
  assert.equal(pluckNaive().id, "pluck_naive");
  assert.equal(brassNaive().id, "brass_naive");
  assert.equal(organNaive().id, "organ_naive");
  assert.equal(malletNaive().id, "mallet_naive");
  assert.equal(celestaNaive().id, "celesta_naive");
  assert.equal(glockenspielNaive().id, "glockenspiel_naive");
  assert.equal(vibraphoneNaive().id, "vibraphone_naive");
  assert.equal(marimbaNaive().id, "marimba_naive");
  assert.equal(xylophoneNaive().id, "xylophone_naive");
  assert.equal(tubularBellsNaive().id, "tubular_bells_naive");
  assert.equal(timpaniNaive().id, "timpani_naive");
  assert.equal(kalimbaNaive().id, "kalimba_naive");
  assert.equal(synthLeadNaive().id, "synth_lead_naive");
  assert.equal(synthPadNaive().id, "synth_pad_naive");
  assert.equal(effectNaive().id, "effect_naive");
});

test("General MIDI 128 programs mapping", () => {
  const programs = allGmPrograms();
  assert.equal(programs.length, 128);
  assert.equal(GM_PROGRAMS.length, 128);

  const prog1 = getGmProgram(1);
  assert.equal(prog1.name, "Acoustic Grand Piano");
  assert.equal(prog1.family, "piano");
  assert.equal(prog1.instrumentId, "piano_naive");
  const inst1 = instrumentForGmProgram(1);
  assert.equal(inst1.id, "piano_naive");

  const fluteProg = getGmProgram(74);
  assert.equal(fluteProg.name, "Flute");
  assert.equal(fluteProg.instrumentId, "flute_naive");
  const fluteInst = instrumentForGmProgram(74);
  assert.equal(fluteInst.id, "flute_naive");

  const prog128 = getGmProgram(128);
  assert.equal(prog128.name, "Gunshot");
  assert.equal(prog128.instrumentId, "effect_naive");
});

test("InstrumentSignal continuous evaluation and silence", () => {
  const inst = fluteNaive();
  const sig = new InstrumentSignal(440.0, inst, 0.5, 0.8, 44_100.0);
  assert.equal(sig.fundamentalHz, 440.0);
  assert.equal(sig.noteDurationSeconds, 0.5);
  assert.equal(sig.amplitude, 0.8);

  const v0 = sig.valueAt(0.0);
  assert.equal(v0, 0.0); // attack starts at 0

  const vMid = sig.valueAt(0.2);
  assert.ok(Number.isFinite(vMid));

  const silenceInst = silence();
  const silenceSig = new InstrumentSignal(440.0, silenceInst, 0.5);
  assert.equal(silenceSig.valueAt(0.1), 0.0);
});

test("InstrumentSignal seeded variation changes signal deterministically", () => {
  const inst = fluteNaive();
  const var1 = new VariationProfile({
    pitchCentsJitter: 10.0,
    amplitudeJitter: 0.1,
    attackJitterMs: 5.0,
    seed: 42,
  });
  const var2 = new VariationProfile({
    pitchCentsJitter: 10.0,
    amplitudeJitter: 0.1,
    attackJitterMs: 5.0,
    seed: 42,
  });
  const varDiff = new VariationProfile({
    pitchCentsJitter: 10.0,
    amplitudeJitter: 0.1,
    attackJitterMs: 5.0,
    seed: 99,
  });

  const sig1 = new InstrumentSignal(440.0, inst, 0.5, 1.0, 44_100.0, var1);
  const sig2 = new InstrumentSignal(440.0, inst, 0.5, 1.0, 44_100.0, var2);
  const sigDiff = new InstrumentSignal(440.0, inst, 0.5, 1.0, 44_100.0, varDiff);

  assertClose(sig1.valueAt(0.15), sig2.valueAt(0.15));
  assert.notEqual(sig1.valueAt(0.15), sigDiff.valueAt(0.15));
});

test("renderInstrumentNote and renderInstrumentNotePcm produce valid buffers", () => {
  const rendered = renderInstrumentNote("A4", 0.1, {
    instrumentId: "flute_naive",
    sampleRateHz: 44_100.0,
    amplitude: 0.5,
  });

  assert.ok(rendered instanceof InstrumentNoteRender);
  assert.equal(rendered.note.toString(), "A4");
  assertClose(rendered.fundamentalHz, 440.0);
  assert.equal(rendered.instrument.id, "flute_naive");
  assert.ok(rendered.floatingSamples.sampleCount() > 0);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
  assert.equal(
    rendered.floatingSamples.sampleCount(),
    rendered.pcmBuffer.sampleCount(),
  );

  const pcmDirect = renderInstrumentNotePcm("A4", 0.1, {
    instrumentId: "flute_naive",
    sampleRateHz: 44_100.0,
    amplitude: 0.5,
  });
  assert.equal(pcmDirect.sampleCount(), rendered.pcmBuffer.sampleCount());
  assert.deepEqual(pcmDirect.samples, rendered.pcmBuffer.samples);
});

test("Validation helper edge cases", () => {
  assert.throws(() => new Note("A", "", 1.5 as number), /Octave must be a finite integer/);
  assert.throws(() => new Note("A", "", -11), /out of range/);
  assert.throws(() => new Note("A", "", 21), /out of range/);
  assert.throws(() => new Note("A", "", 0).chromaticIndex.call({ spelling: () => "invalid" }), /Unknown chromatic index/);
  assert.throws(() => parseNote("A-"), /Negative sign must be followed by digits/);
  assert.throws(() => parseNote("A?5"), /Invalid character in octave/);
  assert.throws(() => parseNote("A-99999999999999999999"), /exceeds safe integer limit/);
  assert.throws(() => resolveNote(12345 as unknown as Note), /must be a Note instance/);

  // Object duck-typing in resolveNote
  const duckNote = resolveNote({
    letter: "C",
    accidental: "",
    octave: 4,
    frequency: () => 261.63,
  } as unknown as Note);
  assert.equal(duckNote.letter, "C");
  assert.equal(duckNote.frequency(), 261.63);

  // Note with negative octave
  const negNote = parseNote("C-2");
  assert.equal(negNote.octave, -2);
  assert.ok(negNote.frequency() > 0);
  assert.equal(noteToFrequency("C-2"), negNote.frequency());

  // Note spelling and index checks
  const noteSharp = parseNote("F#4");
  assert.equal(noteSharp.accidental, "#");
  assert.equal(noteSharp.spelling(), "F#");
  assert.equal(noteSharp.chromaticIndex(), 6);
  const noteNat = parseNote("C4");
  assert.equal(noteNat.accidental, "");
  assert.equal(noteNat.spelling(), "C");
  assert.equal(noteNat.chromaticIndex(), 0);
});

test("InstrumentProfile edge cases and parameter validation", () => {
  const env = new AdsrEnvelope(0.01, 0.01, 0.5, 0.01);
  const part = new HarmonicPartial(1.0, 1.0);

  assert.throws(() => new InstrumentProfile("", "Name", "additive", 1.0, [part], env), /id must be a non-empty string/);
  assert.throws(() => new InstrumentProfile("id", "", "additive", 1.0, [part], env), /displayName must be a non-empty string/);
  assert.throws(() => new InstrumentProfile("id", "Name", "invalid" as unknown as any, 1.0, [part], env), /Unsupported synthesisKind/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, null as unknown as any, env), /harmonicProfile must be an array/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, ["bad"] as unknown as any, env), /must be a HarmonicPartial/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, [], env), /Non-silent instruments must contain at least one/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, [part], null as unknown as any), /envelopeProfile must be an AdsrEnvelope/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, [part], env, null as unknown as any), /variationProfile must be a VariationProfile/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, [part], env, new VariationProfile(), 0), /must be > 0|must be in \[1, 128\]/);
  assert.throws(() => new InstrumentProfile("id", "Name", "additive", 1.0, [part], env, new VariationProfile(), 129), /gmProgram must be in \[1, 128\]/);

  // Silence profile normalizer
  const sil = silence();
  assert.equal(sil.amplitudeNormalizer(), 1.0);
  assert.equal(sil.family, "utility");
});

test("GM catalog and preset lookups", () => {
  assert.throws(() => getInstrument("non_existent_preset"), /Unknown instrument/);
  assert.throws(() => getGmProgram(0), /must be > 0|must be in \[1, 128\]/i);
  assert.throws(() => getGmProgram(129), /must be in \[1, 128\]/i);
  assert.throws(() => instrumentForGmProgram(0), /must be > 0|must be in \[1, 128\]/i);
  assert.throws(() => instrumentForGmProgram(129), /must be in \[1, 128\]/i);

  // GMProgram constructor validations
  assert.throws(() => new GMProgram(0, "name", "fam", "id"), /must be > 0/);
  assert.throws(() => new GMProgram(129, "name", "fam", "id"), /must be in \[1, 128\]/);
  assert.throws(() => new GMProgram(1, "", "fam", "id"), /name must be a non-empty string/);
  assert.throws(() => new GMProgram(1, "name", "", "id"), /family must be a non-empty string/);
  assert.throws(() => new GMProgram(1, "name", "fam", ""), /instrumentId must be a non-empty string/);

  const programs = allGmPrograms();
  assert.equal(programs.length, 128);
  assert.equal(allInstruments().length, 21);
});

test("InstrumentSignal properties and edge cases", () => {
  const inst = sine();
  const sig = new InstrumentSignal(440.0, inst, 0.2, 0.8, 48_000.0);
  assert.equal(sig.fundamentalHz, 440.0);
  assert.equal(sig.noteDurationSeconds, 0.2);
  assert.equal(sig.sampleRateHz, 48_000.0);
  assert.equal(sig.amplitude, 0.8);
  assert.equal(sig.instrument, inst);

  // Before t=0 throws RangeError
  assert.throws(() => sig.valueAt(-0.1), /timeSeconds must be >= 0.0/);

  // Signal with silence instrument
  const silSig = new InstrumentSignal(440.0, silence(), 0.1, 1.0, 44_100.0);
  assert.equal(silSig.valueAt(0.05), 0.0);

  // Signal constructor validation
  assert.throws(() => new InstrumentSignal(0, inst, 0.1, 1.0, 44_100.0), /fundamentalHz/);
  assert.throws(() => new InstrumentSignal(440.0, null as unknown as any, 0.1, 1.0, 44_100.0), /instrument must be an InstrumentProfile/);

  // Render with custom InstrumentProfile instance
  const customRender = renderInstrumentNote("C4", 0.05, {
    instrument: inst,
    sampleRateHz: 48_000.0,
  });
  assert.equal(customRender.instrument.id, "sine");
});
