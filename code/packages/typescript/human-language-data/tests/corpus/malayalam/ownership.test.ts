import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

it("rejects resurrection of the flat Malayalam corpus-test aggregate", () => {
  const ownerDirectory = dirname(fileURLToPath(import.meta.url));
  expect(existsSync(join(ownerDirectory, "..", "malayalam.test.ts"))).toBe(false);
});
