import assert from "node:assert/strict";
import test from "node:test";

import {
  loadValidatedFixture,
  runCorpus,
  runPythonValidator,
} from "./fixture_helpers.js";

const { fixture, raw } = loadValidatedFixture();

test("all GAME10 v1 vectors pass the actual Python validator and TS public exports", () => {
  runCorpus(fixture.cases);
});

test("the harness rejects vacuous, mutated, missing, duplicate, and unknown cases", () => {
  // The unmodified real implementation is the positive control for each
  // same-path negative control below.
  runCorpus(fixture.cases);

  assert.throws(
    () => runCorpus(fixture.cases, () => ({ error: "ZeroFixedTimestep" })),
    /standard_60hz_rounding/,
  );

  const alteredSuccess = fixture.cases.map((item) => structuredClone(item));
  const alphaCase = alteredSuccess.find((item) => item.id === "advance_fractional_alpha_floors");
  assert.ok(alphaCase);
  (alphaCase.expected.ok as Record<string, unknown>).alpha_q16 = 1;
  assert.throws(() => runCorpus(alteredSuccess), /advance_fractional_alpha_floors/);

  const alteredError = fixture.cases.map((item) => structuredClone(item));
  const refusalCase = alteredError.find((item) => item.id === "new_clock_zero_precedes_other_errors");
  assert.ok(refusalCase);
  refusalCase.expected.error = "InvalidMaxFrameSteps";
  assert.throws(() => runCorpus(alteredError), /new_clock_zero_precedes_other_errors/);

  assert.throws(() => runCorpus(fixture.cases.slice(1)), /requires all 23/);
  const duplicate = fixture.cases.map((item) => structuredClone(item));
  duplicate[duplicate.length - 1] = structuredClone(duplicate[0]);
  assert.throws(() => runCorpus(duplicate), /duplicate case ID/);

  const unknownOp = fixture.cases.map((item, index) => {
    return index === 0
      ? { ...structuredClone(item), op: "advance_frame_v10" }
      : structuredClone(item);
  });
  assert.throws(() => runCorpus(unknownOp), /unknown GAME10 operation/);
});

function replaceInsideCase(rawText: string, caseId: string, before: string, after: string): string {
  const caseStart = rawText.indexOf(`"id": "${caseId}"`);
  assert.notEqual(caseStart, -1, `fixture contains ${caseId}`);
  const caseEnd = rawText.indexOf("\n    },", caseStart);
  assert.notEqual(caseEnd, -1, `fixture case ${caseId} has a closing boundary`);
  const prefix = rawText.slice(0, caseStart);
  const section = rawText.slice(caseStart, caseEnd);
  const suffix = rawText.slice(caseEnd);
  assert.ok(section.includes(before), `fixture case ${caseId} contains ${before}`);
  return `${prefix}${section.replace(before, after)}${suffix}`;
}

test("Python validates original raw bytes and rejects duplicate keys and non-integer u32 JSON", () => {
  const originalValidation = runPythonValidator(raw);
  assert.equal(originalValidation.status, 0, originalValidation.stderr);

  const rawText = raw.toString("utf8");
  const duplicateKey = rawText.replace(
    '"suite": "game_clock_portable_v1",',
    '"suite": "game_clock_portable_v1",\n  "suite": "game_clock_portable_v1",',
  );
  assert.notEqual(duplicateKey, rawText);
  const duplicateResult = runPythonValidator(Buffer.from(duplicateKey));
  assert.notEqual(duplicateResult.status, 0);
  assert.match(duplicateResult.stderr, /duplicate JSON key/);

  const duplicateAlias = rawText.replace(
    '"suite": "game_clock_portable_v1",',
    '"suite": "game_clock_portable_v1",\n  "\\u0073uite": "game_clock_portable_v1",',
  );
  assert.notEqual(duplicateAlias, rawText);
  const duplicateAliasResult = runPythonValidator(Buffer.from(duplicateAlias));
  assert.notEqual(duplicateAliasResult.status, 0);
  assert.match(duplicateAliasResult.stderr, /duplicate JSON key/);

  for (const spelling of ["1.0", "1e0"]) {
    const malformedNumber = replaceInsideCase(
      rawText,
      "from_hz_60_rounding",
      '"max_frame_steps": 8',
      `"max_frame_steps": ${spelling}`,
    );
    const numberResult = runPythonValidator(Buffer.from(malformedNumber));
    assert.notEqual(numberResult.status, 0, `${spelling} is not a JSON integer u32`);
    assert.match(numberResult.stderr, /must be a JSON u32 integer/);
  }
});
