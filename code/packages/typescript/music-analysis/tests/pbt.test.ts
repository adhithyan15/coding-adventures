import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  frequencyToNoteA440,
  rmsEnergy,
  spectralCentroid,
  spectralRolloff,
  yinPitchSimple,
  zeroCrossingRate,
} from "../src/index.js";

function noteToFrequency(noteStr: string): number {
  const pitchClasses: Record<string, number> = {
    C: 0, "C#": 1, D: 2, "D#": 3, E: 4, F: 5, "F#": 6, G: 7, "G#": 8, A: 9, "A#": 10, B: 11,
  };
  let pc = noteStr.slice(0, 1);
  let rest = noteStr.slice(1);
  if (rest.startsWith("#")) {
    pc = noteStr.slice(0, 2);
    rest = noteStr.slice(2);
  }
  const octave = parseInt(rest, 10);
  const midi = (octave + 1) * 12 + pitchClasses[pc];
  return 440.0 * Math.pow(2.0, (midi - 69) / 12.0);
}

function nextF64(rng: SplitMix64): number {
  return Number(rng.nextU64() >> 11n) * (1.0 / Number(1n << 53n));
}

test("PBT: frequencyToNote invertibility", () => {
  const prng = new SplitMix64(0xFEEDFACE11223344n);
  for (let iter = 0; iter < 50; iter++) {
    // Sample random frequency in [50 Hz, 4000 Hz]
    const hz = 50.0 + nextF64(prng) * 3950.0;
    const matched = frequencyToNoteA440(hz);

    // Parse note back via note-frequency
    const nominalHz = noteToFrequency(matched.note);

    // Reconstruct frequency: f = nominal * 2^(cents / 1200)
    const reconstructedHz = nominalHz * Math.pow(2.0, matched.cents / 1200.0);
    const relErr = Math.abs(hz - reconstructedHz) / hz;
    assert.ok(
      relErr < 1e-3,
      `hz: ${hz}, reconstructed: ${reconstructedHz}, relErr: ${relErr}`,
    );
  }
});

test("PBT: audio feature invariants (RMS >= 0, 0 <= ZCR <= 1, 0 <= centroid <= Nyquist)", () => {
  const prng = new SplitMix64(0xDEADBEEFCAFEBABEn);
  const sr = 44100.0;
  for (let iter = 0; iter < 20; iter++) {
    const len = Math.floor(nextF64(prng) * 200.0) + 20;
    const samples = new Float64Array(len);
    for (let i = 0; i < len; i++) {
      samples[i] = nextF64(prng) * 2.0 - 1.0;
    }

    const rms = rmsEnergy(samples);
    assert.ok(rms >= 0.0);

    const zcr = zeroCrossingRate(samples);
    assert.ok(zcr >= 0.0 && zcr <= 1.0);

    // Positive spectrum
    const spec = new Float64Array(len);
    for (let i = 0; i < len; i++) {
      spec[i] = nextF64(prng) * 10.0;
    }
    const centroid = spectralCentroid(spec, sr);
    assert.ok(centroid >= 0.0 && centroid <= sr / 2.0);
  }
});

test("PBT: spectral rolloff monotonicity (r50 <= r85 <= r95)", () => {
  const prng = new SplitMix64(0x55AA55AA33CC33CCn);
  const sr = 44100.0;
  for (let iter = 0; iter < 20; iter++) {
    const kBins = Math.floor(nextF64(prng) * 100.0) + 10;
    const spec = new Float64Array(kBins);
    for (let i = 0; i < kBins; i++) {
      spec[i] = nextF64(prng) * 5.0;
    }

    const r50 = spectralRolloff(spec, sr, 0.50);
    const r85 = spectralRolloff(spec, sr, 0.85);
    const r95 = spectralRolloff(spec, sr, 0.95);

    assert.ok(r50 <= r85 + 1e-6);
    assert.ok(r85 <= r95 + 1e-6);
  }
});

test("PBT: algorithm determinism", () => {
  const prng = new SplitMix64(0x1357924680BDFEn);
  const sr = 44100.0;
  const sig = new Float64Array(1024);
  for (let i = 0; i < 1024; i++) {
    sig[i] = nextF64(prng) * 2.0 - 1.0;
  }

  const p1 = yinPitchSimple(sig, sr);
  const p2 = yinPitchSimple(sig, sr);
  assert.equal(p1, p2);

  const r1 = rmsEnergy(sig);
  const r2 = rmsEnergy(sig);
  assert.equal(r1, r2);
});
