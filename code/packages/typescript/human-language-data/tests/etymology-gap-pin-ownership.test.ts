import {
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  loadEtymologyGapPinOwners,
  validateEtymologyGapPinOwnerNames,
  type EtymologyGapPinOwner,
} from "./etymology-gap-pin-owners.js";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const realOwnerDirectory = resolve(packageRoot, "tests/etymology-gap-pin-owners");
const rootLedgerTest = resolve(packageRoot, "tests/root-ledger.test.ts");

const owner = (
  language: string,
  overrides: Partial<EtymologyGapPinOwner> = {},
): EtymologyGapPinOwner => ({
  language,
  withoutHook: 1,
  withoutVisible: 2,
  rationale: ["Migration baseline measured from the committed corpus."],
  ...overrides,
});

const moduleFor = (pin: unknown, extra: Record<string, unknown> = {}) => ({
  etymologyGapPinOwner: pin,
  ...extra,
});

const withFixture = (
  files: readonly string[],
  run: (directory: string, modules: Readonly<Record<string, unknown>>) => void,
): void => {
  const root = mkdtempSync(resolve(tmpdir(), "etymology-gap-pin-owners-"));
  try {
    const directory = resolve(root, "etymology-gap-pin-owners");
    mkdirSync(directory);
    const modules: Record<string, unknown> = {};
    for (const name of files) {
      writeFileSync(resolve(directory, name), "// fixture owner\n");
      const language = name.replace(/\.owner\.ts$/, "");
      modules[`./etymology-gap-pin-owners/${name}`] = moduleFor(owner(language));
    }
    run(directory, modules);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
};

describe("etymology-gap direct-owner loader", () => {
  it("loads two direct owners without an aggregate registry", () => {
    withFixture(["arabic.owner.ts", "bengali.owner.ts"], (directory, modules) => {
      const pins = loadEtymologyGapPinOwners(directory, modules, ["arabic", "bengali"]);
      expect(Object.keys(pins)).toEqual(["arabic", "bengali"]);
      expect(pins.arabic).toEqual(owner("arabic"));
    });
  });

  it("rejects unsafe, non-lowercase, reserved, and case-colliding names", () => {
    for (const name of [
      "Arabic.owner.ts",
      "arabic.ts",
      "arabic_owner.ts",
      "index.owner.ts",
      "manifest.owner.ts",
      "registry.owner.ts",
    ]) {
      expect(() => validateEtymologyGapPinOwnerNames([name]), name).toThrow();
    }
    expect(() =>
      validateEtymologyGapPinOwnerNames(["arabic.owner.ts", "ARABIC.owner.ts"]),
    ).toThrow(/case-folded/);
  });

  it("rejects a symbolic-link root, a non-directory root, and non-regular children", () => {
    const root = mkdtempSync(resolve(tmpdir(), "etymology-gap-pin-root-"));
    try {
      const target = resolve(root, "target");
      mkdirSync(target);
      writeFileSync(resolve(target, "arabic.owner.ts"), "// owner\n");
      const modules = {
        "./etymology-gap-pin-owners/arabic.owner.ts": moduleFor(owner("arabic")),
      };

      expect(() =>
        loadEtymologyGapPinOwners(resolve(root, "missing-root"), modules, ["arabic"]),
      ).toThrow(/ENOENT/);

      const linkedRoot = resolve(root, "linked-root");
      // A Windows directory junction is still a link to lstatSync, but unlike
      // a native symlink it does not require Developer Mode or elevation.
      symlinkSync(target, linkedRoot, process.platform === "win32" ? "junction" : "dir");
      expect(lstatSync(linkedRoot).isSymbolicLink()).toBe(true);
      expect(() => loadEtymologyGapPinOwners(linkedRoot, modules, ["arabic"])).toThrow(
        /must not be a symbolic link/,
      );

      const fileRoot = resolve(root, "file-root");
      writeFileSync(fileRoot, "not a directory\n");
      expect(() => loadEtymologyGapPinOwners(fileRoot, modules, ["arabic"])).toThrow(
        /must be a directory/,
      );

      const childRoot = resolve(root, "child-root");
      mkdirSync(childRoot);
      mkdirSync(resolve(childRoot, "arabic.owner.ts"));
      expect(() => loadEtymologyGapPinOwners(childRoot, modules, ["arabic"])).toThrow(
        /must be a regular file/,
      );
      rmSync(resolve(childRoot, "arabic.owner.ts"), { recursive: true });

      const linkedChild = resolve(childRoot, "arabic.owner.ts");
      try {
        // Linux CI must exercise a real file link. Windows does too when the
        // account has symlink privilege; otherwise a junction still proves
        // that the loader rejects a linked child before following it.
        symlinkSync(resolve(target, "arabic.owner.ts"), linkedChild, "file");
      } catch (error) {
        if (
          process.platform !== "win32" ||
          typeof error !== "object" ||
          error === null ||
          !("code" in error) ||
          (error.code !== "EPERM" && error.code !== "EACCES")
        ) {
          throw error;
        }
        const junctionTarget = resolve(root, "junction-child-target");
        mkdirSync(junctionTarget);
        symlinkSync(junctionTarget, linkedChild, "junction");
      }
      expect(lstatSync(linkedChild).isSymbolicLink()).toBe(true);
      expect(() => loadEtymologyGapPinOwners(childRoot, modules, ["arabic"])).toThrow(
        /must not be a symbolic link/,
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("requires one matching bounded-glob module for every regular owner file", () => {
    withFixture(["arabic.owner.ts"], (directory, modules) => {
      expect(() => loadEtymologyGapPinOwners(directory, {}, ["arabic"])).toThrow(
        /has no bounded-glob module/,
      );
      expect(() =>
        loadEtymologyGapPinOwners(
          directory,
          { ...modules, "./elsewhere/bengali.owner.ts": moduleFor(owner("bengali")) },
          ["arabic"],
        ),
      ).toThrow(/unexpected bounded-glob module/);
    });
  });

  it("requires exactly one owner export with exact fields and matching language", () => {
    withFixture(["arabic.owner.ts"], (directory, modules) => {
      const key = "./etymology-gap-pin-owners/arabic.owner.ts";
      expect(() =>
        loadEtymologyGapPinOwners(directory, { [key]: {} }, ["arabic"]),
      ).toThrow(/exactly the etymologyGapPinOwner export/);
      expect(() =>
        loadEtymologyGapPinOwners(
          directory,
          { [key]: moduleFor(owner("arabic"), { registry: {} }) },
          ["arabic"],
        ),
      ).toThrow(/exactly the etymologyGapPinOwner export/);
      expect(() =>
        loadEtymologyGapPinOwners(
          directory,
          { [key]: moduleFor({ ...owner("arabic"), aggregate: true }) },
          ["arabic"],
        ),
      ).toThrow(/exactly language, rationale, withoutHook, and withoutVisible/);
      expect(() =>
        loadEtymologyGapPinOwners(
          directory,
          { [key]: moduleFor(owner("bengali")) },
          ["arabic"],
        ),
      ).toThrow(/declares language 'bengali'/);
      expect(() => loadEtymologyGapPinOwners(directory, modules, ["arabic"])).not.toThrow();
    });
  });

  it("rejects malformed pin values and rationales", () => {
    withFixture(["arabic.owner.ts"], (directory) => {
      const key = "./etymology-gap-pin-owners/arabic.owner.ts";
      for (const value of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1, Number.NaN]) {
        expect(() =>
          loadEtymologyGapPinOwners(
            directory,
            { [key]: moduleFor(owner("arabic", { withoutHook: value })) },
            ["arabic"],
          ),
        ).toThrow(/withoutHook must be a non-negative safe integer/);
      }
      for (const rationale of [[], [""], [" padded "], ["ok", "\u0000"]]) {
        expect(() =>
          loadEtymologyGapPinOwners(
            directory,
            { [key]: moduleFor(owner("arabic", { rationale })) },
            ["arabic"],
          ),
        ).toThrow(/rationale/);
      }
    });
  });

  it("rejects duplicate, missing, unknown, and noncanonical expected track slugs", () => {
    withFixture(["arabic.owner.ts", "bengali.owner.ts"], (directory, modules) => {
      expect(() =>
        loadEtymologyGapPinOwners(directory, modules, ["arabic", "arabic"]),
      ).toThrow(/duplicate expected track/);
      expect(() =>
        loadEtymologyGapPinOwners(directory, modules, ["arabic"]),
      ).toThrow(/unknown track owner 'bengali'/);
      expect(() =>
        loadEtymologyGapPinOwners(directory, modules, ["arabic", "bengali", "chinese"]),
      ).toThrow(/missing track owner 'chinese'/);
      expect(() =>
        loadEtymologyGapPinOwners(directory, modules, ["Arabic", "bengali"]),
      ).toThrow(/unsafe expected track 'Arabic'/);
    });
  });
});

describe("real etymology-gap pin ownership", () => {
  it("uses regular direct-owner modules and retires the shared pin map", () => {
    expect(lstatSync(realOwnerDirectory).isDirectory()).toBe(true);
    const rootLedger = readFileSync(rootLedgerTest, "utf8");
    expect(rootLedger).not.toContain("ETYMOLOGY_GAP_PINS");
    expect(rootLedger).not.toMatch(/(?:arabic|bengali|hindi|tamil):\s*\{\s*withoutHook/);
    expect(rootLedger).toContain("loadEtymologyGapPinOwners");
  });

  it("keeps direct owners free of shared registry files", () => {
    for (const reserved of ["index.owner.ts", "manifest.owner.ts", "registry.owner.ts"]) {
      expect(() => lstatSync(resolve(realOwnerDirectory, reserved))).toThrow(/ENOENT/);
    }
  });
});
