// Tier-0 golden parity (MUS01/03/04/07): the canonical score must render to
// exactly the PCM recorded in code/specs/fixtures/music-machine/expected_render.json,
// the same bytes the Python and Rust lanes produce.  Since TRIG-CONV (PHY00a) all
// three lanes take every sine from the bit-exact `trig` port, which is what makes
// a byte-for-byte shared golden possible.  Tests run from the package directory.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import { parsePortableScore, renderPortableScoreToPcm } from "../src/index.js";

const DIR = resolve(process.cwd(), "../../../specs/fixtures/music-machine");

test("canonical score renders to the shared golden PCM, byte for byte", () => {
  const expected = JSON.parse(readFileSync(resolve(DIR, "expected_render.json"), "utf8")) as {
    total_samples: number;
    clipped_sample_count: number;
    pcm16_le_sha256: string;
    first_32_samples: number[];
    last_32_samples: number[];
  };
  const score = parsePortableScore(readFileSync(resolve(DIR, "canonical_score.txt"), "utf8"));
  const rendered = renderPortableScoreToPcm(score);
  const samples = Array.from(rendered.pcmBuffer.samples);

  assert.equal(samples.length, expected.total_samples);
  assert.equal(rendered.pcmBuffer.clippedSampleCount, expected.clipped_sample_count);

  const bytes = Buffer.alloc(samples.length * 2);
  samples.forEach((v, i) => bytes.writeInt16LE(v, i * 2));
  assert.equal(createHash("sha256").update(bytes).digest("hex"), expected.pcm16_le_sha256);
  assert.deepEqual(samples.slice(0, 32), expected.first_32_samples);
  assert.deepEqual(samples.slice(-32), expected.last_32_samples);
});
