import assert from "node:assert/strict";
import test from "node:test";

// Deliberately import only types.js (not index.js). The serializers
// `toText` and `toJson` are installed on PortableScore.prototype as a side
// effect of loading portable_parser.js and json.js; until those modules load,
// the class ships fail-fast stubs. node:test runs each file in its own
// process, so this file sees the stubs.
import { PortableScore } from "../src/types.js";

test("PortableScore serializer stubs fail fast until the serializer modules load", () => {
  const score = new PortableScore({
    formatVersion: "music-machine-score/v2",
    title: "stub",
    ppq: 4,
    sampleRateHz: 8000,
    tempoEvents: [],
    meterEvents: [],
    instruments: [],
    tracks: [],
    events: [],
  });
  assert.throws(() => score.toText(), /toText not initialized/);
  assert.throws(() => score.toJson(), /toJson not initialized/);
});
