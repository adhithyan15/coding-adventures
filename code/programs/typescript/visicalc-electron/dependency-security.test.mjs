// dependency-security.test.mjs — pin the minimum safe Electron release.
//
// GitHub's alert state is useful operational evidence, but it is not the
// source of truth for a reproducible checkout. These tests read both checked-in
// manifests so a future resolver cannot silently restore the vulnerable
// Electron 43.8.0 release.

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const minimumSafe = [43, 8, 1];

function numericVersion(value) {
  const match = String(value).match(/^(?:\^|~)?(\d+)\.(\d+)\.(\d+)$/u);
  assert.ok(match, `expected a numeric Electron version, received ${value}`);
  return match.slice(1).map(Number);
}

function atLeast(left, right) {
  for (let index = 0; index < right.length; index += 1) {
    if (left[index] !== right[index]) return left[index] > right[index];
  }
  return true;
}

const packageJson = JSON.parse(await readFile(
  new URL("./package.json", import.meta.url),
  "utf8",
));
const packageLock = JSON.parse(await readFile(
  new URL("./package-lock.json", import.meta.url),
  "utf8",
));

test("the declared Electron floor excludes GHSA-jr5w-w6x2-84p9", () => {
  assert.ok(atLeast(
    numericVersion(packageJson.dependencies.electron),
    minimumSafe,
  ));
});

test("the lockfile resolves Electron outside the vulnerable range", () => {
  assert.ok(atLeast(
    numericVersion(packageLock.packages["node_modules/electron"].version),
    minimumSafe,
  ));
});
