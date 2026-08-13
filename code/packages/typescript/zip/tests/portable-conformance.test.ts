import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { inflateRawSync } from "node:zlib";
import { describe, expect, it } from "vitest";

import {
  RawInflateError,
  crc32,
  rawDeflate,
  rawInflateCounted,
} from "../src/index.js";

interface OutputHex {
  hex: string;
}

interface OutputRepeat {
  repeat_hex: string;
  count: number;
}

type Output = OutputHex | OutputRepeat;

interface ExpectedInflate {
  output: Output;
  bytes_consumed: number;
}

interface ExpectedError {
  error_id: string;
}

interface ExpectedDeflate {
  output: Output;
}

interface ExpectedCrc32 {
  crc32_hex: string;
}

interface FixtureCase {
  id: string;
  operation: "inflate" | "inflate-error" | "deflate-interoperability" | "crc32";
  input_hex?: string;
  max_output?: number;
  chunks_hex?: string[];
  initial_crc32_hex?: string;
  expected: ExpectedInflate | ExpectedError | ExpectedDeflate | ExpectedCrc32;
}

interface FixtureDocument {
  limits: {
    default_max_output: number;
    hard_max_output: number;
  };
  error_ids: string[];
  cases: FixtureCase[];
}

const FIXTURE_PATH = fileURLToPath(
  new URL("../../../../specs/fixtures/zip-raw-rfc1951-v1/cases.json", import.meta.url),
);
const FIXTURES = JSON.parse(readFileSync(FIXTURE_PATH, "utf8")) as FixtureDocument;

function fromHex(value: string): Uint8Array {
  return Uint8Array.from(Buffer.from(value, "hex"));
}

function materialize(output: Output): Uint8Array {
  if ("hex" in output) return fromHex(output.hex);
  return new Uint8Array(output.count).fill(Number.parseInt(output.repeat_hex, 16));
}

describe("ZIP raw RFC 1951 v1 language-neutral fixtures", () => {
  it("keeps the public hard and default caps at 256 MiB", () => {
    expect(FIXTURES.limits).toEqual({
      default_max_output: 256 * 1024 * 1024,
      hard_max_output: 256 * 1024 * 1024,
    });
  });

  for (const fixture of FIXTURES.cases) {
    if (fixture.operation === "inflate") {
      it(`inflates ${fixture.id}`, () => {
        const expected = fixture.expected as ExpectedInflate;
        const result = rawInflateCounted(
          fromHex(fixture.input_hex!),
          fixture.max_output,
        );
        expect(result.output).toEqual(materialize(expected.output));
        expect(result.bytesConsumed).toBe(expected.bytes_consumed);
      });
    } else if (fixture.operation === "inflate-error") {
      it(`fails closed for ${fixture.id}`, () => {
        const expected = fixture.expected as ExpectedError;
        try {
          rawInflateCounted(fromHex(fixture.input_hex!), fixture.max_output);
          throw new Error("fixture unexpectedly decoded");
        } catch (error: unknown) {
          expect(error).toBeInstanceOf(RawInflateError);
          const inflateError = error as RawInflateError;
          expect(inflateError.code).toBe(expected.error_id);
          expect(inflateError.message).not.toMatch(/(?:0x|[0-9]{2,})/);
        }
      });
    } else if (fixture.operation === "deflate-interoperability") {
      it(`foreign-decodes ${fixture.id}`, () => {
        const expected = fixture.expected as ExpectedDeflate;
        const compressed = rawDeflate(fromHex(fixture.input_hex!));
        expect(Uint8Array.from(inflateRawSync(compressed))).toEqual(
          materialize(expected.output),
        );
      });
    } else {
      it(`checks CRC-32 for ${fixture.id}`, () => {
        const expected = fixture.expected as ExpectedCrc32;
        let actual = Number.parseInt(fixture.initial_crc32_hex ?? "00000000", 16);
        for (const chunk of fixture.chunks_hex!) actual = crc32(fromHex(chunk), actual);
        expect(actual.toString(16).padStart(8, "0")).toBe(expected.crc32_hex);
      });
    }
  }
});
