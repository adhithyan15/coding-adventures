import { describe, test } from "vitest";
import { transpileLattice } from "../src/index.js";
import { readFileSync } from "fs";
import { join } from "path";

const FIXTURE_PATH = join(import.meta.dirname, "fixtures", "engram.lattice");

describe("stack trace test", () => {
  test("transpile", () => {
    const source = readFileSync(FIXTURE_PATH, "utf8");
    try {
      transpileLattice(source);
    } catch(e: any) {
      if (e instanceof RangeError) {
        const lines = (e.stack || '').split('\n');
        const seen = new Set<string>();
        const unique: string[] = [];
        for (const line of lines.slice(1)) {
          const key = line.trim().replace(/:\d+:\d+\)?$/, ':N:N)');
          if (!seen.has(key)) {
            seen.add(key);
            unique.push(line);
          }
          if (seen.size > 25) break;
        }
        console.error('STACK TRACE:\n' + unique.join('\n'));
      }
      throw e;
    }
  });
});
