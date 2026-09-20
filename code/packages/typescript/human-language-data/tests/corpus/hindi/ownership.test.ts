import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

it("rejects resurrection of the flat Hindi corpus-test aggregate", () => {
  const ownerDirectory = dirname(fileURLToPath(import.meta.url));
  expect(existsSync(join(ownerDirectory, "..", "hindi.test.ts"))).toBe(false);
});
