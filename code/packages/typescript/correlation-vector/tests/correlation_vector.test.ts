/**
 * Tests for @coding-adventures/correlation-vector
 *
 * Coverage targets: ≥95% lines, branches, functions, statements.
 *
 * Test groups follow the spec (CV00-correlation-vector.md §Test Coverage):
 *
 *   1. Root lifecycle     — create, contribute, passthrough, delete, error on deleted
 *   2. Derivation         — child IDs, parentIds, ancestors, descendants, chains
 *   3. Merging            — 3-way merge, parentIds, ancestors of merged
 *   4. Deep ancestry      — A→B→C→D chain, ancestors(D), lineage(D)
 *   5. Disabled log       — create returns ID, get undefined, contribute no-op
 *   6. Serialization      — roundtrip: serialize → deserialize → identical
 *   7. ID uniqueness      — 10,000 creates with same origin → all unique
 *   8. Edge cases         — unknown cvId ops, merge with origin, toJsonString/fromJsonString
 */

import { describe, it, expect } from "vitest";
import {
  CVLog,
  type Origin,
  type Contribution,
  type CVEntry,
  type DeletionRecord,
  VERSION,
} from "../src/index.js";

// ─── Helper: make a minimal Origin ──────────────────────────────────────────
function makeOrigin(source: string, location: string): Origin {
  return { source, location, meta: {} };
}

// ─── 1. Root Lifecycle ───────────────────────────────────────────────────────

describe("Root lifecycle", () => {
  it("creates a root CV with an origin and returns an ID in base.N format", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "5:12"));

    // The ID must match base.N: 8 hex chars, dot, one or more digits.
    expect(cvId).toMatch(/^[0-9a-f]{8}\.\d+$/);

    // The entry must exist in the log.
    const entry = log.get(cvId);
    expect(entry).toBeDefined();
    expect(entry!.id).toBe(cvId);
    expect(entry!.parentIds).toEqual([]);
    expect(entry!.origin?.source).toBe("app.ts");
    expect(entry!.origin?.location).toBe("5:12");
    expect(entry!.contributions).toEqual([]);
    expect(entry!.deleted).toBeNull();
  });

  it("creates a synthetic root CV (no origin) with base 00000000", () => {
    const log = new CVLog();
    const cvId = log.create();

    expect(cvId).toMatch(/^00000000\.\d+$/);
    const entry = log.get(cvId);
    expect(entry!.origin).toBeNull();
  });

  it("increments the sequence for the same origin", () => {
    const log = new CVLog();
    const id1 = log.create(makeOrigin("app.ts", "5:12"));
    const id2 = log.create(makeOrigin("app.ts", "5:12"));

    // Same base, different sequence numbers.
    const base1 = id1.split(".")[0];
    const base2 = id2.split(".")[0];
    expect(base1).toBe(base2);

    const n1 = parseInt(id1.split(".")[1], 10);
    const n2 = parseInt(id2.split(".")[1], 10);
    expect(n2).toBe(n1 + 1);
  });

  it("creates separate bases for different origins", () => {
    const log = new CVLog();
    const id1 = log.create(makeOrigin("app.ts", "5:12"));
    const id2 = log.create(makeOrigin("utils.ts", "10:0"));

    const base1 = id1.split(".")[0];
    const base2 = id2.split(".")[0];
    expect(base1).not.toBe(base2);
  });

  it("records contributions in order", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "1:0"));

    log.contribute(cvId, "parser", "created", { token: "IDENTIFIER" });
    log.contribute(cvId, "scope_analysis", "resolved", { binding: "local:x" });
    log.contribute(cvId, "variable_renamer", "renamed", { from: "x", to: "a" });

    const h = log.history(cvId);
    expect(h).toHaveLength(3);
    expect(h[0].source).toBe("parser");
    expect(h[0].tag).toBe("created");
    expect(h[1].source).toBe("scope_analysis");
    expect(h[2].source).toBe("variable_renamer");
    expect(h[2].meta).toEqual({ from: "x", to: "a" });
  });

  it("records passthrough as a contribution with tag 'passthrough'", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    log.passthrough(cvId, "type_checker");

    const h = log.history(cvId);
    expect(h).toHaveLength(1);
    expect(h[0].source).toBe("type_checker");
    expect(h[0].tag).toBe("passthrough");
    expect(h[0].meta).toEqual({});
  });

  it("records deletion and marks the entry as deleted", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    log.delete(cvId, "dce", "unreachable", { entryPoint: "main" });

    const entry = log.get(cvId);
    expect(entry!.deleted).not.toBeNull();
    expect(entry!.deleted!.source).toBe("dce");
    expect(entry!.deleted!.reason).toBe("unreachable");
    expect(entry!.deleted!.meta).toEqual({ entryPoint: "main" });
  });

  it("throws when contributing to a deleted CV", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    log.delete(cvId, "dce", "dead code");

    expect(() => log.contribute(cvId, "later_stage", "tag")).toThrow();
  });

  it("tracks pass_order: first-seen order of source names", () => {
    const log = new CVLog();
    const id1 = log.create(makeOrigin("a.ts", "1:0"));
    const id2 = log.create(makeOrigin("b.ts", "2:0"));

    log.contribute(id1, "parser", "created");
    log.contribute(id2, "parser", "created");     // duplicate — should NOT add again
    log.contribute(id1, "scope_analysis", "resolved");
    log.passthrough(id2, "linter");
    log.delete(id2, "dce", "unused");

    // "parser" first, then "scope_analysis", then "linter", then "dce"
    expect(log.passOrder).toEqual(["parser", "scope_analysis", "linter", "dce"]);
  });

  it("contribute with default meta {}", () => {
    const log = new CVLog();
    const cvId = log.create();
    log.contribute(cvId, "stage", "action"); // no meta argument
    expect(log.history(cvId)[0].meta).toEqual({});
  });

  it("delete with default meta {}", () => {
    const log = new CVLog();
    const cvId = log.create();
    log.delete(cvId, "dce", "dead"); // no meta argument
    expect(log.get(cvId)!.deleted!.meta).toEqual({});
  });

  it("contribute to unknown cvId silently ignores", () => {
    const log = new CVLog();
    // Should not throw, just no-op.
    expect(() => log.contribute("nonexistent.1", "stage", "tag")).not.toThrow();
  });

  it("passthrough to unknown cvId silently ignores", () => {
    const log = new CVLog();
    expect(() => log.passthrough("nonexistent.1", "stage")).not.toThrow();
  });

  it("delete to unknown cvId silently ignores", () => {
    const log = new CVLog();
    expect(() => log.delete("nonexistent.1", "stage", "reason")).not.toThrow();
  });

  it("origin with timestamp is preserved", () => {
    const log = new CVLog();
    const origin: Origin = {
      source: "app.ts",
      location: "1:0",
      timestamp: "2026-04-05T10:00:00Z",
      meta: { nodeKind: "Identifier" },
    };
    const cvId = log.create(origin);
    const entry = log.get(cvId);
    expect(entry!.origin!.timestamp).toBe("2026-04-05T10:00:00Z");
    expect(entry!.origin!.meta).toEqual({ nodeKind: "Identifier" });
  });
});

// ─── 2. Derivation ──────────────────────────────────────────────────────────

describe("Derivation", () => {
  it("first derived child has ID parent.1", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const childId = log.derive(parentId);

    // Child ID = parent ID + ".1"
    expect(childId).toBe(`${parentId}.1`);
    const entry = log.get(childId);
    expect(entry!.parentIds).toEqual([parentId]);
  });

  it("second derived child has ID parent.2", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    log.derive(parentId);
    const childId2 = log.derive(parentId);

    expect(childId2).toBe(`${parentId}.2`);
    const entry = log.get(childId2);
    expect(entry!.parentIds).toEqual([parentId]);
  });

  it("ancestors(child) returns [parentId]", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const childId = log.derive(parentId);

    expect(log.ancestors(childId)).toEqual([parentId]);
  });

  it("descendants(parent) returns both children", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const child1 = log.derive(parentId);
    const child2 = log.derive(parentId);

    const desc = log.descendants(parentId);
    expect(desc).toContain(child1);
    expect(desc).toContain(child2);
    expect(desc).toHaveLength(2);
  });

  it("derive chain: A → B → C produces correct IDs and parents", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("app.ts", "1:0"));
    const b = log.derive(a);
    const c = log.derive(b);

    expect(b).toBe(`${a}.1`);
    expect(c).toBe(`${b}.1`);
    expect(log.get(c)!.parentIds).toEqual([b]);
    expect(log.ancestors(c)).toEqual([b, a]);
  });

  it("derive with optional origin stores the origin", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const childOrigin = makeOrigin("splitter", "col:0-5");
    const childId = log.derive(parentId, childOrigin);

    const entry = log.get(childId);
    expect(entry!.origin?.source).toBe("splitter");
  });

  it("derive without origin has null origin", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const childId = log.derive(parentId);
    expect(log.get(childId)!.origin).toBeNull();
  });
});

// ─── 3. Merging ─────────────────────────────────────────────────────────────

describe("Merging", () => {
  it("3-way merge: parentIds lists all three parents", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const b = log.create(makeOrigin("b.ts", "1:0"));
    const c = log.create(makeOrigin("c.ts", "1:0"));

    const mergedId = log.merge([a, b, c]);
    const entry = log.get(mergedId);

    expect(entry!.parentIds).toEqual([a, b, c]);
  });

  it("merged CV has 00000000 base when no origin is given", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const b = log.create(makeOrigin("b.ts", "1:0"));

    const mergedId = log.merge([a, b]);
    expect(mergedId).toMatch(/^00000000\.\d+$/);
  });

  it("merged CV has origin-based base when origin is given", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const b = log.create(makeOrigin("b.ts", "1:0"));

    const mergedId = log.merge([a, b], makeOrigin("join_stage", "a.id=b.id"));
    // Base should NOT be 00000000 — it should be a hash of the join origin.
    expect(mergedId).not.toMatch(/^00000000\./);
    const entry = log.get(mergedId);
    expect(entry!.origin?.source).toBe("join_stage");
  });

  it("ancestors(merged) returns all three parents", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const b = log.create(makeOrigin("b.ts", "1:0"));
    const c = log.create(makeOrigin("c.ts", "1:0"));

    const mergedId = log.merge([a, b, c]);
    const ancs = log.ancestors(mergedId);

    expect(ancs).toContain(a);
    expect(ancs).toContain(b);
    expect(ancs).toContain(c);
    expect(ancs).toHaveLength(3);
  });

  it("descendants of parents includes merged CV", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const b = log.create(makeOrigin("b.ts", "1:0"));
    const mergedId = log.merge([a, b]);

    expect(log.descendants(a)).toContain(mergedId);
    expect(log.descendants(b)).toContain(mergedId);
  });
});

// ─── 4. Deep Ancestry Chain ─────────────────────────────────────────────────

describe("Deep ancestry chain", () => {
  it("A→B→C→D: ancestors(D) = [C, B, A] (nearest first)", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("src.ts", "1:0"));
    const b = log.derive(a);
    const c = log.derive(b);
    const d = log.derive(c);

    const ancs = log.ancestors(d);
    expect(ancs).toEqual([c, b, a]);
  });

  it("A→B→C→D: lineage(D) = [A, B, C, D] (oldest first)", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("src.ts", "1:0"));
    const b = log.derive(a);
    const c = log.derive(b);
    const d = log.derive(c);

    const lineage = log.lineage(d);
    expect(lineage).toHaveLength(4);
    expect(lineage[0].id).toBe(a);
    expect(lineage[1].id).toBe(b);
    expect(lineage[2].id).toBe(c);
    expect(lineage[3].id).toBe(d);
  });

  it("descendants(A) includes B, C, and D", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("src.ts", "1:0"));
    const b = log.derive(a);
    const c = log.derive(b);
    const d = log.derive(c);

    const desc = log.descendants(a);
    expect(desc).toContain(b);
    expect(desc).toContain(c);
    expect(desc).toContain(d);
    expect(desc).toHaveLength(3);
  });

  it("lineage of a root returns just that entry", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("src.ts", "1:0"));
    const lineage = log.lineage(a);
    expect(lineage).toHaveLength(1);
    expect(lineage[0].id).toBe(a);
  });

  it("ancestors of a root returns []", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("src.ts", "1:0"));
    expect(log.ancestors(a)).toEqual([]);
  });

  it("descendants of a leaf returns []", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("src.ts", "1:0"));
    const b = log.derive(a);
    expect(log.descendants(b)).toEqual([]);
  });
});

// ─── 5. Disabled Log ────────────────────────────────────────────────────────

describe("Disabled log", () => {
  it("create still returns a valid CV ID even when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "5:12"));

    expect(cvId).toMatch(/^[0-9a-f]{8}\.\d+$/);
  });

  it("get returns undefined when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "5:12"));
    expect(log.get(cvId)).toBeUndefined();
  });

  it("contribute is a no-op when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "5:12"));
    log.contribute(cvId, "parser", "created");
    expect(log.history(cvId)).toEqual([]);
  });

  it("passthrough is a no-op when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "5:12"));
    log.passthrough(cvId, "type_checker");
    expect(log.history(cvId)).toEqual([]);
  });

  it("delete is a no-op when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "5:12"));
    log.delete(cvId, "dce", "dead code");
    expect(log.get(cvId)).toBeUndefined();
  });

  it("derive returns a valid ID when disabled", () => {
    const log = new CVLog(false);
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const childId = log.derive(parentId);

    expect(childId).toBe(`${parentId}.1`);
    expect(log.get(childId)).toBeUndefined();
  });

  it("merge returns a valid ID when disabled", () => {
    const log = new CVLog(false);
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const b = log.create(makeOrigin("b.ts", "1:0"));
    const mergedId = log.merge([a, b]);

    expect(mergedId).toMatch(/^00000000\.\d+$/);
    expect(log.get(mergedId)).toBeUndefined();
  });

  it("ancestors returns [] when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    expect(log.ancestors(cvId)).toEqual([]);
  });

  it("descendants returns [] when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    expect(log.descendants(cvId)).toEqual([]);
  });

  it("history returns [] when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    expect(log.history(cvId)).toEqual([]);
  });

  it("lineage returns [] when disabled", () => {
    const log = new CVLog(false);
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    expect(log.lineage(cvId)).toEqual([]);
  });

  it("sequence counters advance even when disabled (uniqueness guaranteed)", () => {
    const log = new CVLog(false);
    const id1 = log.create(makeOrigin("app.ts", "1:0"));
    const id2 = log.create(makeOrigin("app.ts", "1:0"));

    expect(id1).not.toBe(id2);
    const n1 = parseInt(id1.split(".")[1], 10);
    const n2 = parseInt(id2.split(".")[1], 10);
    expect(n2).toBe(n1 + 1);
  });
});

// ─── 6. Serialization Roundtrip ─────────────────────────────────────────────

describe("Serialization roundtrip", () => {
  /**
   * Build a representative CVLog with roots, derivations, merges, and deletions,
   * then verify that serialize → deserialize → identical entries.
   */
  it("full roundtrip preserves all entries, contributions, deletions", () => {
    const log = new CVLog();

    // Roots
    const r1 = log.create(makeOrigin("app.ts", "1:0"));
    const r2 = log.create(makeOrigin("app.ts", "10:5"));
    const r3 = log.create();

    // Contributions
    log.contribute(r1, "parser", "created", { token: "Identifier" });
    log.contribute(r1, "scope_analysis", "resolved", { binding: "local:x" });
    log.passthrough(r2, "type_checker");
    log.contribute(r2, "variable_renamer", "renamed", { from: "y", to: "b" });

    // Derivation
    const c1 = log.derive(r1);
    log.contribute(c1, "inliner", "inlined");

    // Merge
    const m1 = log.merge([r1, r2], makeOrigin("join", "r1.id=r2.id"));
    log.contribute(m1, "combiner", "merged");

    // Deletion
    const d1 = log.create(makeOrigin("dead.ts", "5:0"));
    log.contribute(d1, "dce_pass", "analyzed");
    log.delete(d1, "dce", "unreachable", { entryPoint: r1 });

    // Serialize → plain object
    const serialized = log.serialize();
    const s = serialized as { entries: Record<string, unknown>; pass_order: string[]; enabled: boolean };

    expect(s.enabled).toBe(true);
    expect(s.pass_order).toContain("parser");
    expect(s.pass_order).toContain("dce");

    // Deserialize
    const restored = CVLog.deserialize(serialized);

    // Compare every entry
    for (const [id, entry] of log.entries) {
      const rEntry = restored.get(id);
      expect(rEntry).toBeDefined();
      expect(rEntry!.id).toBe(entry.id);
      expect(rEntry!.parentIds).toEqual(entry.parentIds);
      expect(rEntry!.contributions).toEqual(entry.contributions);
      expect(rEntry!.deleted).toEqual(entry.deleted);

      if (entry.origin) {
        expect(rEntry!.origin).toEqual(entry.origin);
      } else {
        expect(rEntry!.origin).toBeNull();
      }
    }

    // Pass order is preserved
    expect(restored.passOrder).toEqual(log.passOrder);
    expect(restored.enabled).toBe(log.enabled);
  });

  it("deserialized log continues to generate non-colliding IDs", () => {
    const log = new CVLog();
    const r1 = log.create(makeOrigin("app.ts", "1:0"));
    const r2 = log.create(makeOrigin("app.ts", "1:0")); // same origin → sequence 2

    const restored = CVLog.deserialize(log.serialize());

    // Creating another CV with the same origin should get sequence 3
    const r3 = restored.create(makeOrigin("app.ts", "1:0"));
    expect(r3).not.toBe(r1);
    expect(r3).not.toBe(r2);

    const n3 = parseInt(r3.split(".")[1], 10);
    const n2 = parseInt(r2.split(".")[1], 10);
    expect(n3).toBeGreaterThan(n2);
  });

  it("deserialized log continues to generate non-colliding child IDs", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const c1 = log.derive(parentId);
    const c2 = log.derive(parentId);

    const restored = CVLog.deserialize(log.serialize());
    const c3 = restored.derive(parentId);

    expect(c3).not.toBe(c1);
    expect(c3).not.toBe(c2);
    expect(c3).toBe(`${parentId}.3`);
  });

  it("toJsonString / fromJsonString roundtrip works", () => {
    const log = new CVLog();
    const r1 = log.create(makeOrigin("app.ts", "1:0"));
    log.contribute(r1, "parser", "created");
    const c1 = log.derive(r1);
    log.contribute(c1, "inliner", "processed");

    const json = log.toJsonString();
    expect(typeof json).toBe("string");

    const restored = CVLog.fromJsonString(json);
    expect(restored.get(r1)!.contributions[0].source).toBe("parser");
    expect(restored.get(c1)!.parentIds).toEqual([r1]);
  });

  it("serialize → JSON.stringify → JSON.parse → deserialize roundtrip", () => {
    const log = new CVLog();
    const r = log.create(makeOrigin("app.ts", "1:0"));
    log.contribute(r, "parser", "created");

    const json = JSON.stringify(log.serialize());
    const restored = CVLog.deserialize(JSON.parse(json) as object);

    const entry = restored.get(r);
    expect(entry!.contributions[0].tag).toBe("created");
  });

  it("serialized disabled log roundtrips with enabled=false", () => {
    const log = new CVLog(false);
    const id = log.create(makeOrigin("app.ts", "1:0"));

    const restored = CVLog.deserialize(log.serialize());
    expect(restored.enabled).toBe(false);
    // Disabled log has no entries to compare.
    expect(restored.entries.size).toBe(0);
  });

  it("origin with timestamp serializes/deserializes correctly", () => {
    const log = new CVLog();
    const ts = "2026-04-05T10:00:00Z";
    const r = log.create({
      source: "app.ts",
      location: "1:0",
      timestamp: ts,
      meta: { key: "val" },
    });

    const restored = CVLog.deserialize(log.serialize());
    const entry = restored.get(r);
    expect(entry!.origin!.timestamp).toBe(ts);
    expect(entry!.origin!.meta).toEqual({ key: "val" });
  });

  it("origin without timestamp serializes null and deserializes to undefined", () => {
    const log = new CVLog();
    const r = log.create(makeOrigin("app.ts", "1:0"));

    const restored = CVLog.deserialize(log.serialize());
    const entry = restored.get(r);
    // After deserialization, timestamp should be undefined (not null).
    expect(entry!.origin!.timestamp).toBeUndefined();
  });
});

// ─── 7. ID Uniqueness ────────────────────────────────────────────────────────

describe("ID uniqueness", () => {
  it("10,000 creates with the same origin produce all unique IDs", () => {
    const log = new CVLog();
    const ids = new Set<string>();
    const origin = makeOrigin("app.ts", "5:12");

    for (let i = 0; i < 10_000; i++) {
      ids.add(log.create(origin));
    }

    expect(ids.size).toBe(10_000);
  });

  it("10,000 creates with mixed origins produce all unique IDs", () => {
    const log = new CVLog();
    const ids = new Set<string>();

    for (let i = 0; i < 5_000; i++) {
      ids.add(log.create(makeOrigin("app.ts", `${i}:0`)));
    }
    for (let i = 0; i < 5_000; i++) {
      ids.add(log.create(makeOrigin("utils.ts", `${i}:0`)));
    }

    expect(ids.size).toBe(10_000);
  });

  it("10,000 derives from the same parent produce all unique IDs", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    const ids = new Set<string>();

    for (let i = 0; i < 10_000; i++) {
      ids.add(log.derive(parentId));
    }

    expect(ids.size).toBe(10_000);
  });
});

// ─── 8. Edge Cases and Additional Coverage ───────────────────────────────────

describe("Edge cases", () => {
  it("VERSION export is a string", () => {
    expect(typeof VERSION).toBe("string");
  });

  it("get on unknown ID returns undefined", () => {
    const log = new CVLog();
    expect(log.get("nonexistent.1")).toBeUndefined();
  });

  it("ancestors on unknown ID returns []", () => {
    const log = new CVLog();
    expect(log.ancestors("nonexistent.1")).toEqual([]);
  });

  it("descendants on ID with no children returns []", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    expect(log.descendants(cvId)).toEqual([]);
  });

  it("history on unknown ID returns []", () => {
    const log = new CVLog();
    expect(log.history("nonexistent.1")).toEqual([]);
  });

  it("lineage on unknown ID returns []", () => {
    const log = new CVLog();
    expect(log.lineage("nonexistent.1")).toEqual([]);
  });

  it("derive on deleted parent is allowed (tombstone use-case)", () => {
    const log = new CVLog();
    const parentId = log.create(makeOrigin("app.ts", "1:0"));
    log.delete(parentId, "dce", "dead code");

    // Should not throw — deriving from deleted parents is allowed.
    expect(() => log.derive(parentId)).not.toThrow();
  });

  it("merge with single parent behaves correctly", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("a.ts", "1:0"));
    const merged = log.merge([a]);

    const entry = log.get(merged);
    expect(entry!.parentIds).toEqual([a]);
  });

  it("serialized entries have snake_case keys (interop with other languages)", () => {
    const log = new CVLog();
    const r = log.create(makeOrigin("app.ts", "1:0"));
    log.contribute(r, "parser", "created");

    const serialized = log.serialize() as {
      entries: Record<string, Record<string, unknown>>;
    };
    const entry = serialized.entries[r];

    expect(entry).toHaveProperty("parent_ids");
    expect(entry).not.toHaveProperty("parentIds");
    expect(entry).toHaveProperty("contributions");
    expect(entry).toHaveProperty("deleted");
    expect(entry).toHaveProperty("origin");
  });

  it("serialized log has pass_order key (snake_case)", () => {
    const log = new CVLog();
    const serialized = log.serialize() as Record<string, unknown>;
    expect(serialized).toHaveProperty("pass_order");
    expect(serialized).not.toHaveProperty("passOrder");
  });

  it("deep chain with branches: A→(B,C); B→D; ancestors(D)=[B,A]", () => {
    const log = new CVLog();
    const a = log.create(makeOrigin("app.ts", "1:0"));
    const b = log.derive(a);  // a.1
    const c = log.derive(a);  // a.2
    const d = log.derive(b);  // a.1.1

    expect(log.ancestors(d)).toEqual([b, a]);
    expect(log.ancestors(c)).toEqual([a]);
    expect(log.descendants(a)).toContain(b);
    expect(log.descendants(a)).toContain(c);
    expect(log.descendants(a)).toContain(d);
  });

  it("merge ancestors traverse into merged parents' ancestors", () => {
    const log = new CVLog();
    const root1 = log.create(makeOrigin("a.ts", "1:0"));
    const root2 = log.create(makeOrigin("b.ts", "1:0"));
    const child1 = log.derive(root1);
    const merged = log.merge([child1, root2]);

    const ancs = log.ancestors(merged);
    // Merged's direct parents are child1 and root2.
    // child1's parent is root1.
    // So ancestors should include child1, root2, and root1.
    expect(ancs).toContain(child1);
    expect(ancs).toContain(root2);
    expect(ancs).toContain(root1);
  });

  it("history returns a copy (mutations don't affect the log)", () => {
    const log = new CVLog();
    const cvId = log.create(makeOrigin("app.ts", "1:0"));
    log.contribute(cvId, "parser", "created");

    const h = log.history(cvId);
    h.push({ source: "mutant", tag: "injected", meta: {} });

    // The log's internal history should not have been mutated.
    expect(log.history(cvId)).toHaveLength(1);
  });

  it("CVLog enabled property is accessible", () => {
    const enabledLog = new CVLog(true);
    const disabledLog = new CVLog(false);

    expect(enabledLog.enabled).toBe(true);
    expect(disabledLog.enabled).toBe(false);
  });
});
