import assert from "node:assert/strict";
import test from "node:test";
import { sampleCountForDuration } from "@coding-adventures/oscillator";
import { assertProperty, Gen } from "@coding-adventures/test-bootstrap";

import {
  AdsrEnvelope,
  HarmonicPartial,
  InstrumentProfile,
  InstrumentSignal,
  Note,
  parseNote,
  renderInstrumentNote,
  sine,
  SplitMix64,
  VariationProfile,
} from "../src/index.js";

test("PBT: Note parse and string round-trip invariant", () => {
  const validSpellings: [string, string][] = [
    ["C", ""], ["C", "#"], ["D", "b"], ["D", ""], ["D", "#"],
    ["E", "b"], ["E", ""], ["F", ""], ["F", "#"], ["G", "b"],
    ["G", ""], ["G", "#"], ["A", "b"], ["A", ""], ["A", "#"],
    ["B", "b"], ["B", ""],
  ];

  const noteGen = new Gen((rng) => {
    const pair = validSpellings[rng.bounded(validSpellings.length)];
    const letter = pair[0];
    const acc = pair[1];
    // Octave in [-4, 8]
    const oct = rng.bounded(13) - 4;
    return new Note(letter, acc, oct);
  });

  assertProperty(
    noteGen,
    (originalNote) => {
      const formatted = originalNote.toString();
      const parsed = parseNote(formatted);
      assert.equal(parsed.letter, originalNote.letter);
      assert.equal(parsed.accidental, originalNote.accidental);
      assert.equal(parsed.octave, originalNote.octave);
      assert.ok(Math.abs(parsed.frequency() - originalNote.frequency()) <= 1e-9);
      return true;
    },
    { numTests: 60 },
  );
});

test("PBT: ADSR envelope valueAt invariant: value lies in [0.0, 1.0]", () => {
  const envGen = new Gen((rng) => {
    // durations in [0.001, 1.0]
    const u1 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const u2 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const u3 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const u4 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const uTime = Number(rng.nextU64() & 0xffffn) / 65535.0;

    const atk = 0.001 + u1 * 0.5;
    const dec = 0.001 + u2 * 0.5;
    const sus = u3; // [0.0, 1.0]
    const rel = 0.001 + u4 * 0.5;
    const env = new AdsrEnvelope(atk, dec, sus, rel);
    const noteDur = 0.5;
    const totalDur = env.renderedDurationSeconds(noteDur);
    const time = uTime * (totalDur + 0.5);

    return { env, time, noteDur };
  });

  assertProperty(
    envGen,
    ({ env, time, noteDur }) => {
      const val = env.valueAt(time, noteDur);
      assert.ok(
        val >= -1e-9 && val <= 1.0 + 1e-9,
        `Envelope value ${val} out of [0.0, 1.0] at time ${time}`,
      );
      return true;
    },
    { numTests: 80 },
  );
});

test("PBT: SplitMix64 determinism invariant: identical seed produces identical sequence", () => {
  const seedGen = new Gen((rng) => rng.nextU64());

  assertProperty(
    seedGen,
    (seed) => {
      const prng1 = new SplitMix64(seed);
      const prng2 = new SplitMix64(seed);

      for (let i = 0; i < 20; i += 1) {
        assert.equal(prng1.nextU64(), prng2.nextU64());
        assert.equal(prng1.nextFloatSigned(), prng2.nextFloatSigned());
        assert.equal(prng1.nextFloatUnit(), prng2.nextFloatUnit());
      }
      return true;
    },
    { numTests: 40 },
  );
});

test("PBT: InstrumentSignal amplitude is bounded by amplitude * gain", () => {
  const sigGen = new Gen((rng) => {
    const uAmp = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const uGain = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const uTime = Number(rng.nextU64() & 0xffffn) / 65535.0;

    const amp = 0.1 + uAmp * 0.9;
    const gain = 0.1 + uGain * 0.9;
    const env = new AdsrEnvelope(0.01, 0.02, 0.8, 0.05);
    const inst = new InstrumentProfile(
      "pbt_inst",
      "PBT Inst",
      "additive",
      gain,
      [new HarmonicPartial(1.0, 1.0), new HarmonicPartial(2.0, 0.5)],
      env,
    );
    const sig = new InstrumentSignal(440.0, inst, 0.2, amp, 44_100.0);
    const time = uTime * 0.3;
    return { sig, amp, gain, time };
  });

  assertProperty(
    sigGen,
    ({ sig, amp, gain, time }) => {
      const sample = sig.valueAt(time);
      const maxExpected = amp * gain + 1e-6;
      assert.ok(
        Math.abs(sample) <= maxExpected,
        `Signal sample ${sample} exceeds max bound ${maxExpected} at time ${time}`,
      );
      return true;
    },
    { numTests: 50 },
  );
});

test("PBT: Rendered note sample count matches expected duration * sampleRate", () => {
  const renderGen = new Gen((rng) => {
    const uDur = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const dur = 0.02 + uDur * 0.1; // [0.02, 0.12]
    return dur;
  });

  assertProperty(
    renderGen,
    (dur) => {
      const rendered = renderInstrumentNote("A4", dur, {
        instrument: sine(),
        sampleRateHz: 44_100.0,
      });
      const expectedTotalDur = sine().envelopeProfile.renderedDurationSeconds(dur);
      const expectedCount = sampleCountForDuration(expectedTotalDur, 44_100.0);
      assert.equal(rendered.floatingSamples.sampleCount(), expectedCount);
      return true;
    },
    { numTests: 30 },
  );
});
