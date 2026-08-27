import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { afterEach, describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const realBuild = join(here, "..", "BUILD");
const roots: string[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

function executable(path: string, source: string): void {
  writeFileSync(path, source, "utf8");
  chmodSync(path, 0o755);
}

function fixture(): { root: string; packageDir: string; bin: string; marker: string } {
  const root = mkdtempSync(join(tmpdir(), "hl-build-guards-"));
  roots.push(root);
  const packages = join(root, "packages");
  const packageDir = join(packages, "human-language-data");
  for (const name of [
    "pixel-container",
    "paint-instructions",
    "paint-vm",
    "paint-vm-svg",
    "human-language-data",
  ]) {
    mkdirSync(join(packages, name), { recursive: true });
  }
  cpSync(realBuild, join(packageDir, "BUILD"));

  const bin = join(root, "bin");
  mkdirSync(bin);
  const marker = join(root, "npx-ran");
  executable(bin + "/npm", "#!/usr/bin/env bash\nexit \"${FAKE_NPM_STATUS:-0}\"\n");
  executable(
    bin + "/python3",
    "#!/usr/bin/env bash\n" +
      "if [ \"${1:-}\" = '-c' ]; then exit \"${FAKE_PYTHON3_PROBE_STATUS:-0}\"; fi\n" +
      "exit \"${FAKE_PYTHON3_RUN_STATUS:-0}\"\n",
  );
  executable(
    bin + "/py",
    "#!/usr/bin/env bash\n" +
      "if [ \"${1:-}\" = '-3' ] && [ \"${2:-}\" = '-c' ]; then exit 0; fi\n" +
      "if [ \"${1:-}\" = '-3' ]; then printf 'py-launcher\\n' >> \"$PYTHON_MARKER\"; fi\n" +
      "exit \"${FAKE_PY_RUN_STATUS:-0}\"\n",
  );
  executable(bin + "/python", "#!/usr/bin/env bash\nexit 127\n");
  executable(
    bin + "/npx",
    "#!/usr/bin/env bash\nprintf 'npx\\n' >> \"$NPX_MARKER\"\nexit 0\n",
  );
  return { root, packageDir, bin, marker };
}

function runBuild(
  test: ReturnType<typeof fixture>,
  overrides: Record<string, string>,
): ReturnType<typeof spawnSync> {
  return spawnSync("bash", ["BUILD"], {
    cwd: test.packageDir,
    encoding: "utf8",
    env: {
      ...process.env,
      ...overrides,
      PATH: `${test.bin}${delimiter}${process.env.PATH ?? ""}`,
      NPX_MARKER: test.marker,
      PYTHON_MARKER: join(test.root, "python-ran"),
    },
  });
}

describe("human-language-data BUILD guards", () => {
  it("stops immediately when a prerequisite install fails", () => {
    const test = fixture();
    const result = runBuild(test, { FAKE_NPM_STATUS: "17" });
    expect(result.status).toBe(17);
    expect(existsSync(test.marker)).toBe(false);
  });

  it("does not let a later test pass mask grammar-cell drift", () => {
    const test = fixture();
    const result = runBuild(test, { FAKE_PYTHON3_RUN_STATUS: "19" });
    expect(result.status).toBe(19);
    expect(existsSync(test.marker)).toBe(false);
  });

  it("falls back from a broken python3 alias to the Windows py launcher", () => {
    const test = fixture();
    const result = runBuild(test, { FAKE_PYTHON3_PROBE_STATUS: "1" });
    expect(result.status, `${result.stdout}\n${result.stderr}`).toBe(0);
    expect(existsSync(test.marker)).toBe(true);
    expect(existsSync(join(test.root, "python-ran"))).toBe(true);
  });
});
