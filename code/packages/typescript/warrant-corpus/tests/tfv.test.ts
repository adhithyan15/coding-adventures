import { describe, expect, it } from "vitest";
import { CorpusError } from "../src/errors.ts";
import { parseTfv, MAX_DEPTH } from "../src/tfv.ts";

const doc = (...lines: string[]): string => `tfv 1\n${lines.map((line) => `${line}\n`).join("")}`;
const refuses = (text: string): void => {
  expect(() => parseTfv(text)).toThrow(CorpusError);
};

describe("parseTfv", () => {
  it("parses every value kind", () => {
    const records = parseTfv(
      doc(
        "a/b#1 R7 Kn N Kt T Kf F Ki I-12 Kx X00ff Ks S6162 Kl L2 I1 X",
        "second R1 Km M1 S6f6b I340282366920938463463374607431768211456",
      ),
    );
    expect(records).toHaveLength(2);
    const [id, fields] = records[0] as (typeof records)[number];
    expect(id).toBe("a/b#1");
    expect(fields.get("n")).toEqual(["null"]);
    expect(fields.get("t")).toEqual(["bool", true]);
    expect(fields.get("f")).toEqual(["bool", false]);
    expect(fields.get("i")).toEqual(["int", -12n]);
    expect(fields.get("x")).toEqual(["bytes", Uint8Array.of(0, 255)]);
    expect(fields.get("s")).toEqual(["text", "ab"]);
    expect(fields.get("l")).toEqual(["list", [["int", 1n], ["bytes", new Uint8Array(0)]]]);
    expect((records[1] as (typeof records)[number])[1].get("m")).toEqual([
      "map",
      [[["text", "ok"], ["int", 2n ** 128n]]],
    ]);
  });

  it("requires the header and a final newline", () => {
    refuses("");
    refuses("tfv 2\n");
    refuses("tfv 1");
    refuses("tfv 1\nx R0");
    expect(parseTfv("tfv 1\n")).toEqual([]);
  });

  it("refuses malformed records", () => {
    for (const line of [
      "",
      "id",
      "id R1",
      "id R01 Ka N",
      "id Rx Ka N",
      "id R1 a N",
      "id R1 KA N",
      "id R2 Ka N Ka N",
      "id R1 Ka N extra",
      "id R1 Ka",
      "bad id R0",
      "id R1 Ka I-0",
      "id R1 Ka I01",
      "id R1 Ka Ix",
      "id R1 Ka X0",
      "id R1 Ka XFF",
      "id R1 Ka Sg0",
      "id R1 Ka Sc0",
      "id R1 Ka L2 N",
      "id R1 Ka L-1",
      "id R1 Ka L99 N",
      "id R1 Ka M1 N",
      "id R1 Ka Q",
      "id R1 Ka Nx",
      "id R1 Ka Tx",
      "id\r R0",
    ]) {
      refuses(doc(line));
    }
    refuses(doc("same R0", "same R0"));
  });

  it("bounds nesting", () => {
    const nested = (depth: number): string => `${"L1 ".repeat(depth)}N`;
    expect(parseTfv(doc(`id R1 Ka ${nested(MAX_DEPTH)}`))).toHaveLength(1);
    refuses(doc(`id R1 Ka ${nested(MAX_DEPTH + 2)}`));
  });
});
