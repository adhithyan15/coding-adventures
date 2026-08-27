import { describe, expect, it } from "vitest";
import type {
  LogicalId,
  ProvenanceContributor,
  RevisionId,
} from "@coding-adventures/forme-types";
import {
  createOutputProvenance,
  isRevisionIdShape,
} from "../src/index.js";

const FIRST = "01952c0d-7e63-7000-8000-000000000001" as LogicalId;
const SECOND = "01952c0d-7e63-7000-8000-000000000002" as LogicalId;
const REV_A = `blake2b:${"a".repeat(64)}` as RevisionId;
const REV_B = `blake2b:${"b".repeat(64)}` as RevisionId;

describe("createOutputProvenance", () => {
  it("sorts contributors and hashes the normalized set deterministically", () => {
    const forward = createOutputProvenance([
      { identity: FIRST, revision: REV_A },
      { identity: SECOND, revision: REV_B },
    ]);
    const reverse = createOutputProvenance([
      { identity: SECOND, revision: REV_B },
      { identity: FIRST, revision: REV_A },
    ]);

    expect(reverse).toEqual(forward);
    expect(forward.contributors.map(({ identity }) => identity)).toEqual([FIRST, SECOND]);
    expect(isRevisionIdShape(forward.revision)).toBe(true);
  });

  it("deduplicates an identical logical/revision pair", () => {
    const pair: ProvenanceContributor = { identity: FIRST, revision: REV_A };
    const provenance = createOutputProvenance([pair, pair]);
    expect(provenance.contributors).toEqual([pair]);
    expect(Object.isFrozen(provenance)).toBe(true);
    expect(Object.isFrozen(provenance.contributors)).toBe(true);
    expect(Object.isFrozen(provenance.contributors[0])).toBe(true);
  });

  it("supports a deterministic empty aggregate", () => {
    expect(createOutputProvenance([])).toEqual(createOutputProvenance([]));
    expect(createOutputProvenance([]).contributors).toEqual([]);
  });

  it("rejects a non-array input with an actionable diagnostic", () => {
    expect(() => createOutputProvenance(null as never)).toThrow(
      "createOutputProvenance: contributors must be an array",
    );
  });

  it("rejects a non-object contributor with its index", () => {
    expect(() => createOutputProvenance([null] as never)).toThrow(
      "createOutputProvenance: contributors[0] must be an object",
    );
    expect(() => createOutputProvenance([[]] as never)).toThrow(
      "createOutputProvenance: contributors[0] must be an object",
    );
  });

  it("rejects malformed logical and revision IDs with field paths", () => {
    expect(() => createOutputProvenance([
      { identity: "post" as LogicalId, revision: REV_A },
    ])).toThrow("contributors[0].identity must be a lowercase UUIDv7 LogicalId");
    expect(() => createOutputProvenance([
      { identity: FIRST, revision: "draft" as RevisionId },
    ])).toThrow("contributors[0].revision must be a RevisionId");
    expect(() => createOutputProvenance([{ revision: REV_A }] as never)).toThrow(
      "contributors[0].identity must be a lowercase UUIDv7 LogicalId",
    );
    expect(() => createOutputProvenance([{ identity: FIRST }] as never)).toThrow(
      "contributors[0].revision must be a RevisionId",
    );
  });

  it("rejects conflicting revisions for one logical identity", () => {
    expect(() => createOutputProvenance([
      { identity: FIRST, revision: REV_A },
      { identity: FIRST, revision: REV_B },
    ])).toThrow(`logical identity ${FIRST} has conflicting revisions`);
  });
});
