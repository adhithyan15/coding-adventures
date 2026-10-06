import { readFile } from "node:fs/promises";
import {
  openAuthoringSession,
  type AuthoringStorage,
} from "@coding-adventures/forme-authoring-core";
import { describe, expect, it } from "vitest";

interface ParityCase {
  readonly name: string;
  readonly valid: boolean;
  readonly value: unknown;
}

const cases = JSON.parse(
  await readFile(new URL("./fixtures/storage-parity.json", import.meta.url), "utf8"),
) as readonly ParityCase[];

describe("native storage parity corpus", () => {
  for (const parity of cases) {
    it(parity.name, async () => {
      const bytes = new TextEncoder().encode(JSON.stringify(parity.value));
      const storage: AuthoringStorage = {
        async load() {
          return {
            bytes,
            revision: "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
          };
        },
        async compareAndSwap() {
          throw new Error("parity loading must not write");
        },
      };
      const opened = openAuthoringSession({ storage });
      if (parity.valid) {
        await expect(opened).resolves.toBeDefined();
      } else {
        await expect(opened).rejects.toBeDefined();
      }
    });
  }
});
