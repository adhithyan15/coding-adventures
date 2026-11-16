import assert from "node:assert/strict";
import test from "node:test";
import { assertProperty, Gen } from "@coding-adventures/test-bootstrap";

import {
  formatPortableScoreJson,
  mixPcmSamples,
  parsePortableScoreJson,
  PortableScoreBuilder,
  TempoEvent,
  TempoMap,
  transposeNoteText,
} from "../src/index.js";

test("PBT: TempoMap monotonicity (t1 < t2 implies secondsAtTick(t1) <= secondsAtTick(t2))", () => {
  const tempoGen = new Gen((rng) => {
    const ppq = 120 * (rng.bounded(4) + 1);
    const numEvents = rng.bounded(5) + 1;
    const events: TempoEvent[] = [new TempoEvent(0, 60 + rng.bounded(120))];
    let tick = 0;
    for (let i = 1; i < numEvents; i += 1) {
      tick += (rng.bounded(4) + 1) * ppq;
      events.push(new TempoEvent(tick, 60 + rng.bounded(120)));
    }
    const t1 = rng.bounded(tick + 1000);
    const t2 = t1 + rng.bounded(1000);
    return { ppq, events, t1, t2 };
  });

  assertProperty(
    tempoGen,
    ({ ppq, events, t1, t2 }) => {
      const tm = new TempoMap(events, ppq);
      const s1 = tm.secondsAtTick(t1);
      const s2 = tm.secondsAtTick(t2);
      assert.ok(
        s2 >= s1,
        `Expected s2 >= s1 but got s1=${String(s1)}, s2=${String(s2)} at t1=${String(t1)}, t2=${String(t2)}`,
      );
      return true;
    },
    { numTests: 60 },
  );
});

test("PBT: mixPcmSamples 16-bit saturating arithmetic bounds", () => {
  const mixGen = new Gen((rng) => {
    const len = rng.bounded(32) + 1;
    const target: number[] = [];
    const source: number[] = [];
    for (let i = 0; i < len; i += 1) {
      target.push(rng.bounded(65536) - 32768);
      source.push(rng.bounded(65536) - 32768);
    }
    const offset = rng.bounded(16);
    return { target, source, offset };
  });

  assertProperty(
    mixGen,
    ({ target, source, offset }) => {
      const clipped = mixPcmSamples(target, offset, source);
      assert.ok(clipped >= 0);
      for (let i = 0; i < target.length; i += 1) {
        assert.ok(target[i] >= -32768 && target[i] <= 32767);
      }
      return true;
    },
    { numTests: 60 },
  );
});

test("PBT: transposeNoteText symmetry and octave boundaries", () => {
  const notes = ["C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5"];
  const transGen = new Gen((rng) => {
    const note = notes[rng.bounded(notes.length)];
    const semitones = rng.bounded(25) - 12; // [-12, 12]
    return { note, semitones };
  });

  assertProperty(
    transGen,
    ({ note, semitones }) => {
      const transposed = transposeNoteText(note, semitones);
      const restored = transposeNoteText(transposed, -semitones);
      assert.equal(restored, note);
      return true;
    },
    { numTests: 60 },
  );
});

test("PBT: PortableScore JSON serialization determinism and idempotence", () => {
  const scoreGen = new Gen((rng) => {
    const builder = new PortableScoreBuilder({
      title: `Score ${String(rng.bounded(1000))}`,
      ppq: 120,
      sampleRateHz: 22050,
    });
    builder
      .addTempo(0, 80 + rng.bounded(80))
      .addMeter(0, "4/4")
      .addInstrument("lead", { kind: "sine", gain: 0.5 })
      .addTrack("melody", { instrumentId: "lead" });

    const notes = ["C4", "E4", "G4", "A4"];
    let tick = 0;
    const noteCount = rng.bounded(4) + 1;
    for (let i = 0; i < noteCount; i += 1) {
      builder.addNote(
        "melody",
        tick,
        120,
        notes[rng.bounded(notes.length)],
        { velocity: (rng.bounded(10) + 1) / 10.0 },
      );
      tick += 120;
    }
    return builder.build();
  });

  assertProperty(
    scoreGen,
    (score) => {
      const json1 = score.toJson();
      const parsed1 = parsePortableScoreJson(json1);
      const json2 = parsed1.toJson();
      assert.equal(json1, json2);
      return true;
    },
    { numTests: 30 },
  );
});
