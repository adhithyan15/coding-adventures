/**
 * Tests for the Branch Target Buffer (BTB).
 *
 * The BTB caches branch target addresses — it answers "WHERE does the branch go?"
 * These tests verify lookup/update behavior, eviction, aliasing, and statistics.
 */

import { describe, expect, it } from "vitest";
import { BranchTargetBuffer, createBTBEntry } from "../src/btb.js";

// ─── Lookup Miss ────────────────────────────────────────────────────────────

describe("BTB lookup miss", () => {
  it("lookup miss on cold start", () => {
    /** Fresh BTB returns null for any lookup. */
    const btb = new BranchTargetBuffer(256);
    expect(btb.lookup(0x100)).toBeNull();
  });

  it("miss increments counter", () => {
    const btb = new BranchTargetBuffer(256);
    btb.lookup(0x100);
    expect(btb.misses).toBe(1);
    expect(btb.hits).toBe(0);
    expect(btb.lookups).toBe(1);
  });
});

// ─── Update and Hit ─────────────────────────────────────────────────────────

describe("BTB update and hit", () => {
  it("update then lookup hits", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200, "conditional");
    const target = btb.lookup(0x100);
    expect(target).toBe(0x200);
  });

  it("hit increments counter", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    btb.lookup(0x100);
    expect(btb.hits).toBe(1);
    expect(btb.misses).toBe(0);
  });

  it("update overwrites previous target", () => {
    /** Updating the same branch with a new target overwrites the old one. */
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    btb.update(0x100, 0x300);
    expect(btb.lookup(0x100)).toBe(0x300);
  });

  it("multiple branches", () => {
    /** Multiple different branches can coexist in the BTB. */
    const btb = new BranchTargetBuffer(256);
    // Use addresses that don't alias (different index = pc % 256)
    btb.update(0x01, 0x200);
    btb.update(0x02, 0x400);
    btb.update(0x03, 0x600);

    expect(btb.lookup(0x01)).toBe(0x200);
    expect(btb.lookup(0x02)).toBe(0x400);
    expect(btb.lookup(0x03)).toBe(0x600);
  });
});

// ─── Branch Types ───────────────────────────────────────────────────────────

describe("BTB branch types", () => {
  it("default branch type", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    const entry = btb.getEntry(0x100);
    expect(entry).not.toBeNull();
    expect(entry!.branchType).toBe("conditional");
  });

  it("custom branch types", () => {
    const btb = new BranchTargetBuffer(256);
    const branches: Array<[number, string]> = [
      [0x01, "conditional"],
      [0x02, "unconditional"],
      [0x03, "call"],
      [0x04, "return"],
    ];

    for (const [pc, btype] of branches) {
      btb.update(pc, pc + 0x100, btype);
    }

    for (const [pc, btype] of branches) {
      const entry = btb.getEntry(pc);
      expect(entry).not.toBeNull();
      expect(entry!.branchType).toBe(btype);
    }
  });
});

// ─── Eviction ───────────────────────────────────────────────────────────────

describe("BTB eviction", () => {
  it("eviction on aliasing", () => {
    /**
     * Two branches aliasing to the same slot: second evicts first.
     *
     * With size=4:
     *   Branch A at 0x100 -> index 0 (0x100 % 4 = 0)
     *   Branch B at 0x104 -> index 0 (0x104 % 4 = 0)
     */
    const btb = new BranchTargetBuffer(4);
    btb.update(0x100, 0x200);
    expect(btb.lookup(0x100)).toBe(0x200);

    // Branch B evicts Branch A
    btb.update(0x104, 0x300);
    expect(btb.lookup(0x104)).toBe(0x300);

    // Branch A is now evicted — tag mismatch -> miss
    expect(btb.lookup(0x100)).toBeNull();
  });

  it("no eviction with large table", () => {
    /** Large table avoids aliasing for nearby branches. */
    const btb = new BranchTargetBuffer(4096);
    btb.update(0x100, 0x200);
    btb.update(0x104, 0x300);
    expect(btb.lookup(0x100)).toBe(0x200);
    expect(btb.lookup(0x104)).toBe(0x300);
  });
});

// ─── BTBEntry ───────────────────────────────────────────────────────────────

describe("BTBEntry", () => {
  it("default entry", () => {
    const entry = createBTBEntry();
    expect(entry.valid).toBe(false);
    expect(entry.tag).toBe(0);
    expect(entry.target).toBe(0);
    expect(entry.branchType).toBe("");
  });

  it("custom entry", () => {
    const entry = createBTBEntry(true, 0x100, 0x200, "call");
    expect(entry.valid).toBe(true);
    expect(entry.tag).toBe(0x100);
    expect(entry.target).toBe(0x200);
    expect(entry.branchType).toBe("call");
  });
});

// ─── Get Entry ──────────────────────────────────────────────────────────────

describe("BTB getEntry", () => {
  it("getEntry returns null on miss", () => {
    const btb = new BranchTargetBuffer(256);
    expect(btb.getEntry(0x100)).toBeNull();
  });

  it("getEntry returns entry on hit", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200, "unconditional");
    const entry = btb.getEntry(0x100);
    expect(entry).not.toBeNull();
    expect(entry!.valid).toBe(true);
    expect(entry!.tag).toBe(0x100);
    expect(entry!.target).toBe(0x200);
  });

  it("getEntry returns null on tag mismatch", () => {
    /** An occupied entry with wrong tag -> returns null. */
    const btb = new BranchTargetBuffer(4);
    btb.update(0x100, 0x200); // index 0
    // PC 0x104 also maps to index 0 but different tag
    expect(btb.getEntry(0x104)).toBeNull();
  });
});

// ─── Statistics ─────────────────────────────────────────────────────────────

describe("BTB statistics", () => {
  it("hit rate zero lookups", () => {
    const btb = new BranchTargetBuffer(256);
    expect(btb.hitRate).toBe(0.0);
  });

  it("hit rate all hits", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    for (let i = 0; i < 10; i++) {
      btb.lookup(0x100);
    }
    expect(btb.hitRate).toBe(100.0);
  });

  it("hit rate all misses", () => {
    const btb = new BranchTargetBuffer(256);
    for (let i = 0; i < 10; i++) {
      btb.lookup(0x100);
    }
    expect(btb.hitRate).toBe(0.0);
  });

  it("hit rate mixed", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    btb.lookup(0x100); // hit
    btb.lookup(0x100); // hit
    btb.lookup(0x200); // miss (never updated)
    btb.lookup(0x300); // miss
    expect(btb.hitRate).toBe(50.0);
  });
});

// ─── Reset ──────────────────────────────────────────────────────────────────

describe("BTB reset", () => {
  it("reset clears entries", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    btb.reset();
    expect(btb.lookup(0x100)).toBeNull();
  });

  it("reset clears statistics", () => {
    const btb = new BranchTargetBuffer(256);
    btb.update(0x100, 0x200);
    btb.lookup(0x100);
    btb.reset();
    expect(btb.lookups).toBe(0);
    expect(btb.hits).toBe(0);
    expect(btb.misses).toBe(0);
  });
});
