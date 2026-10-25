/** NN40 supplies data, never executable mutations. The public native loader
 * owns every acceptance decision; valid controls catch an always-reject stub. */
import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import { DenseNetwork } from "../src/index.js";

const corpus = JSON.parse(readFileSync(new URL(
  "../../../../specs/fixtures/neural-framework-v1/checkpoints.json", import.meta.url,
), "utf8"));

it("pins the shared NN40 corpus inventory", () => {
  expect(Object.keys(corpus).sort()).toEqual(["cases", "contract"]);
  expect(corpus.contract).toBe("NN40");
  expect(corpus.cases).toHaveLength(49);
  expect(new Set(corpus.cases.map((c: any) => c.id)).size).toBe(49);
  expect(corpus.cases.filter((c: any) => c.accept)).toHaveLength(3);
  for (const c of corpus.cases) {
    expect(Object.keys(c).sort()).toEqual(["accept", "checkpoint", "id"]);
    expect(typeof c.id).toBe("string");
    expect(c.id.length).toBeGreaterThan(0);
    expect(typeof c.accept).toBe("boolean");
  }
});

for (const c of corpus.cases) {
  it(`NN40 ${c.id}`, () => {
    if (c.accept) {
      const saved = DenseNetwork.fromCheckpoint(c.checkpoint).toCheckpoint();
      expect(saved).toEqual(c.checkpoint);
      expect(DenseNetwork.fromCheckpoint(saved).toCheckpoint()).toEqual(saved);
    } else {
      expect(() => DenseNetwork.fromCheckpoint(c.checkpoint)).toThrow();
    }
  });
}
