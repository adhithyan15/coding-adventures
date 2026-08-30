import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const packageRoot = path.dirname(
  path.dirname(fileURLToPath(import.meta.url)),
);

describe("TypeScript build front", () => {
  it("pins the compiler and Node declarations behind an explicit typecheck script", () => {
    const packageJson = JSON.parse(
      fs.readFileSync(path.join(packageRoot, "package.json"), "utf-8"),
    ) as {
      scripts?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };
    const tsconfig = JSON.parse(
      fs.readFileSync(path.join(packageRoot, "tsconfig.json"), "utf-8"),
    ) as {
      compilerOptions?: { types?: string[] };
    };

    expect(packageJson.scripts?.typecheck).toBe(
      "tsc --noEmit -p tsconfig.json",
    );
    expect(packageJson.devDependencies?.typescript).toBe("7.0.2");
    expect(packageJson.devDependencies?.["@types/node"]).toBe("22.20.1");
    expect(tsconfig.compilerOptions?.types).toEqual(["node"]);
  });

  it("runs typechecking before coverage in the generic cross-platform BUILD front", () => {
    const commands = fs
      .readFileSync(path.join(packageRoot, "BUILD"), "utf-8")
      .split(/\r?\n/u)
      .map((line) => line.trim())
      .filter(Boolean);

    expect(commands).toEqual([
      "npm install --silent",
      "npm run typecheck",
      "npx vitest run --coverage",
    ]);
  });
});
