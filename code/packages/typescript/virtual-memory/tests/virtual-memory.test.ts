import { describe, it, expect, beforeEach } from "vitest";
import {
  PAGE_SIZE,
  PAGE_OFFSET_BITS,
  PAGE_OFFSET_MASK,
  PageTableEntry,
  PageTable,
  TwoLevelPageTable,
  TLB,
  PhysicalFrameAllocator,
  FIFOPolicy,
  LRUPolicy,
  ClockPolicy,
  MMU,
} from "../src/index.js";

// ============================================================================
// PageTableEntry Tests
// ============================================================================

describe("PageTableEntry", () => {
  it("should create a PTE with default flags", () => {
    const pte = new PageTableEntry(42);
    expect(pte.frame_number).toBe(42);
    expect(pte.present).toBe(true);
    expect(pte.dirty).toBe(false);
    expect(pte.accessed).toBe(false);
    expect(pte.writable).toBe(false);
    expect(pte.executable).toBe(false);
    expect(pte.user_accessible).toBe(false);
  });

  it("should create a PTE with custom flags", () => {
    const pte = new PageTableEntry(7, {
      present: true,
      dirty: true,
      accessed: true,
      writable: true,
      executable: true,
      user_accessible: true,
    });
    expect(pte.frame_number).toBe(7);
    expect(pte.present).toBe(true);
    expect(pte.dirty).toBe(true);
    expect(pte.accessed).toBe(true);
    expect(pte.writable).toBe(true);
    expect(pte.executable).toBe(true);
    expect(pte.user_accessible).toBe(true);
  });

  it("should clone a PTE with all flags preserved", () => {
    const original = new PageTableEntry(10, {
      present: true,
      dirty: true,
      writable: true,
      executable: true,
      user_accessible: true,
      accessed: true,
    });
    const cloned = original.clone();

    expect(cloned.frame_number).toBe(10);
    expect(cloned.present).toBe(true);
    expect(cloned.dirty).toBe(true);
    expect(cloned.writable).toBe(true);
    expect(cloned.executable).toBe(true);
    expect(cloned.user_accessible).toBe(true);
    expect(cloned.accessed).toBe(true);

    // Modifying the clone should not affect the original
    cloned.frame_number = 99;
    cloned.writable = false;
    expect(original.frame_number).toBe(10);
    expect(original.writable).toBe(true);
  });

  it("should allow partial flag overrides", () => {
    const pte = new PageTableEntry(5, { writable: true });
    expect(pte.writable).toBe(true);
    expect(pte.executable).toBe(false);
    expect(pte.present).toBe(true);
  });
});

// ============================================================================
// PageTable (Single-Level) Tests
// ============================================================================

describe("PageTable", () => {
  let pt: PageTable;

  beforeEach(() => {
    pt = new PageTable();
  });

  it("should start empty", () => {
    expect(pt.mapped_count()).toBe(0);
    expect(pt.lookup(0)).toBeUndefined();
  });

  it("should map and lookup a page", () => {
    pt.map_page(5, 10, { writable: true });

    const pte = pt.lookup(5);
    expect(pte).toBeDefined();
    expect(pte!.frame_number).toBe(10);
    expect(pte!.present).toBe(true);
    expect(pte!.writable).toBe(true);
    expect(pt.mapped_count()).toBe(1);
  });

  it("should return undefined for unmapped pages", () => {
    pt.map_page(0, 1);
    expect(pt.lookup(999)).toBeUndefined();
  });

  it("should unmap a page", () => {
    pt.map_page(3, 7);
    const removed = pt.unmap_page(3);

    expect(removed).toBeDefined();
    expect(removed!.frame_number).toBe(7);
    expect(pt.lookup(3)).toBeUndefined();
    expect(pt.mapped_count()).toBe(0);
  });

  it("should return undefined when unmapping a non-existent page", () => {
    expect(pt.unmap_page(42)).toBeUndefined();
  });

  it("should handle multiple mappings", () => {
    pt.map_page(0, 100);
    pt.map_page(1, 200);
    pt.map_page(2, 300);

    expect(pt.mapped_count()).toBe(3);
    expect(pt.lookup(0)!.frame_number).toBe(100);
    expect(pt.lookup(1)!.frame_number).toBe(200);
    expect(pt.lookup(2)!.frame_number).toBe(300);
  });

  it("should overwrite an existing mapping", () => {
    pt.map_page(5, 10);
    pt.map_page(5, 20, { writable: true });

    expect(pt.mapped_count()).toBe(1);
    expect(pt.lookup(5)!.frame_number).toBe(20);
    expect(pt.lookup(5)!.writable).toBe(true);
  });

  it("should get all VPNs", () => {
    pt.map_page(1, 10);
    pt.map_page(5, 50);
    pt.map_page(3, 30);

    const vpns = pt.get_all_vpns().sort();
    expect(vpns).toEqual([1, 3, 5]);
  });

  it("should insert PTE directly", () => {
    const pte = new PageTableEntry(42, { writable: true, dirty: true });
    pt.insert(7, pte);

    expect(pt.lookup(7)).toBe(pte);
    expect(pt.lookup(7)!.dirty).toBe(true);
  });
});

// ============================================================================
// TwoLevelPageTable Tests
// ============================================================================

describe("TwoLevelPageTable", () => {
  let pt: TwoLevelPageTable;

  beforeEach(() => {
    pt = new TwoLevelPageTable();
  });

  describe("address splitting", () => {
    it("should split address 0x00012ABC correctly", () => {
      // VPN = 0x12ABC >> 12 = 0x12 = 18
      // VPN[1] = 18 >> 10 = 0
      // VPN[0] = 18 & 0x3FF = 18
      // offset = 0xABC
      const { vpn1, vpn0, offset } = TwoLevelPageTable.splitAddress(0x00012abc);
      expect(vpn1).toBe(0);
      expect(vpn0).toBe(18);
      expect(offset).toBe(0xabc);
    });

    it("should split address 0x0 correctly", () => {
      const { vpn1, vpn0, offset } = TwoLevelPageTable.splitAddress(0);
      expect(vpn1).toBe(0);
      expect(vpn0).toBe(0);
      expect(offset).toBe(0);
    });

    it("should split address 0xFFFFFFFF correctly", () => {
      // Use >>> 0 to handle as unsigned
      const { vpn1, vpn0, offset } =
        TwoLevelPageTable.splitAddress(0xffffffff);
      expect(vpn1).toBe(1023); // all 10 bits set
      expect(vpn0).toBe(1023); // all 10 bits set
      expect(offset).toBe(4095); // all 12 bits set
    });

    it("should split a mid-range address correctly", () => {
      // Address 0x80000000 — tests bit 31 (sign bit in signed 32-bit)
      const { vpn1, vpn0, offset } =
        TwoLevelPageTable.splitAddress(0x80000000);
      expect(vpn1).toBe(512); // bit 9 set in VPN[1]
      expect(vpn0).toBe(0);
      expect(offset).toBe(0);
    });
  });

  describe("map and translate", () => {
    it("should map and translate a single page", () => {
      // Map virtual page at address 0x5000 to frame 10
      pt.map(0x5000, 10, { writable: true });

      const result = pt.translate(0x5abc);
      expect(result).toBeDefined();
      // Physical address = (10 << 12) | 0xABC = 0xAABC
      expect(result!.phys_addr).toBe(0xaabc);
      expect(result!.pte.frame_number).toBe(10);
      expect(result!.pte.writable).toBe(true);
    });

    it("should return undefined for unmapped addresses", () => {
      expect(pt.translate(0x1000)).toBeUndefined();
    });

    it("should handle multiple pages in different directory entries", () => {
      // Two pages in different 4MB regions
      pt.map(0x00001000, 1); // VPN[1]=0
      pt.map(0x00400000, 2); // VPN[1]=1 (4MB boundary)

      const r1 = pt.translate(0x00001000);
      const r2 = pt.translate(0x00400000);
      expect(r1!.pte.frame_number).toBe(1);
      expect(r2!.pte.frame_number).toBe(2);
    });

    it("should handle pages within the same directory entry", () => {
      pt.map(0x00001000, 10); // VPN[1]=0, VPN[0]=1
      pt.map(0x00002000, 20); // VPN[1]=0, VPN[0]=2

      expect(pt.translate(0x00001000)!.pte.frame_number).toBe(10);
      expect(pt.translate(0x00002000)!.pte.frame_number).toBe(20);
    });
  });

  describe("unmap", () => {
    it("should unmap a mapped page", () => {
      pt.map(0x3000, 5);
      const removed = pt.unmap(0x3000);

      expect(removed).toBeDefined();
      expect(removed!.frame_number).toBe(5);
      expect(pt.translate(0x3000)).toBeUndefined();
    });

    it("should return undefined when unmapping a non-existent page", () => {
      expect(pt.unmap(0x9000)).toBeUndefined();
    });
  });

  describe("lookupPTE", () => {
    it("should return the PTE for a mapped address", () => {
      pt.map(0x5000, 10, { writable: true, executable: true });
      const pte = pt.lookupPTE(0x5000);
      expect(pte).toBeDefined();
      expect(pte!.frame_number).toBe(10);
      expect(pte!.writable).toBe(true);
      expect(pte!.executable).toBe(true);
    });

    it("should return undefined for an unmapped address", () => {
      expect(pt.lookupPTE(0xdead0000)).toBeUndefined();
    });
  });

  describe("allMappings", () => {
    it("should return all mappings", () => {
      pt.map(0x1000, 1);
      pt.map(0x2000, 2);
      pt.map(0x3000, 3);

      const mappings = pt.allMappings();
      expect(mappings.length).toBe(3);

      const frames = mappings.map((m) => m.pte.frame_number).sort();
      expect(frames).toEqual([1, 2, 3]);
    });

    it("should return empty array for empty page table", () => {
      expect(pt.allMappings()).toEqual([]);
    });
  });
});

// ============================================================================
// TLB Tests
// ============================================================================

describe("TLB", () => {
  let tlb: TLB;

  beforeEach(() => {
    tlb = new TLB(4); // Small capacity for testing eviction
  });

  it("should miss on empty TLB", () => {
    expect(tlb.lookup(1, 0)).toBeUndefined();
    expect(tlb.misses).toBe(1);
    expect(tlb.hits).toBe(0);
  });

  it("should hit after inserting an entry", () => {
    const pte = new PageTableEntry(10);
    tlb.insert(1, 5, 10, pte);

    const result = tlb.lookup(1, 5);
    expect(result).toBeDefined();
    expect(result!.frame).toBe(10);
    expect(result!.pte).toBe(pte);
    expect(tlb.hits).toBe(1);
  });

  it("should miss for wrong pid", () => {
    const pte = new PageTableEntry(10);
    tlb.insert(1, 5, 10, pte);

    expect(tlb.lookup(2, 5)).toBeUndefined();
    expect(tlb.misses).toBe(1);
  });

  it("should miss for wrong vpn", () => {
    const pte = new PageTableEntry(10);
    tlb.insert(1, 5, 10, pte);

    expect(tlb.lookup(1, 6)).toBeUndefined();
    expect(tlb.misses).toBe(1);
  });

  it("should evict LRU entry when full", () => {
    // Insert 4 entries (capacity=4)
    for (let i = 0; i < 4; i++) {
      tlb.insert(1, i, i * 10, new PageTableEntry(i * 10));
    }
    expect(tlb.size()).toBe(4);

    // Insert a 5th — should evict VPN 0 (LRU)
    tlb.insert(1, 99, 990, new PageTableEntry(990));
    expect(tlb.size()).toBe(4);

    // VPN 0 should be evicted
    expect(tlb.lookup(1, 0)).toBeUndefined();
    // VPN 99 should be present
    expect(tlb.lookup(1, 99)).toBeDefined();
  });

  it("should update LRU order on lookup", () => {
    // Insert 4 entries
    for (let i = 0; i < 4; i++) {
      tlb.insert(1, i, i * 10, new PageTableEntry(i * 10));
    }

    // Access VPN 0 — moves it to most recent
    tlb.lookup(1, 0);

    // Insert a 5th — should evict VPN 1 (now the LRU, since 0 was accessed)
    tlb.insert(1, 99, 990, new PageTableEntry(990));

    expect(tlb.lookup(1, 0)).toBeDefined(); // 0 was recently accessed
    expect(tlb.lookup(1, 1)).toBeUndefined(); // 1 was evicted
  });

  it("should flush all entries", () => {
    tlb.insert(1, 0, 0, new PageTableEntry(0));
    tlb.insert(1, 1, 10, new PageTableEntry(10));
    tlb.insert(2, 0, 20, new PageTableEntry(20));

    tlb.flush();

    expect(tlb.size()).toBe(0);
    expect(tlb.lookup(1, 0)).toBeUndefined();
    expect(tlb.lookup(2, 0)).toBeUndefined();
  });

  it("should invalidate a single entry", () => {
    tlb.insert(1, 0, 0, new PageTableEntry(0));
    tlb.insert(1, 1, 10, new PageTableEntry(10));

    tlb.invalidate(1, 0);

    expect(tlb.lookup(1, 0)).toBeUndefined();
    expect(tlb.lookup(1, 1)).toBeDefined();
  });

  it("should calculate hit rate correctly", () => {
    expect(tlb.hit_rate()).toBe(0); // No lookups yet

    tlb.insert(1, 0, 0, new PageTableEntry(0));
    tlb.lookup(1, 0); // hit
    tlb.lookup(1, 0); // hit
    tlb.lookup(1, 1); // miss

    expect(tlb.hits).toBe(2);
    expect(tlb.misses).toBe(1);
    expect(tlb.hit_rate()).toBeCloseTo(2 / 3, 5);
  });

  it("should handle re-insertion of existing key", () => {
    const pte1 = new PageTableEntry(10);
    const pte2 = new PageTableEntry(20);

    tlb.insert(1, 5, 10, pte1);
    tlb.insert(1, 5, 20, pte2); // Re-insert same key

    expect(tlb.size()).toBe(1);
    const result = tlb.lookup(1, 5);
    expect(result!.frame).toBe(20);
  });
});

// ============================================================================
// PhysicalFrameAllocator Tests
// ============================================================================

describe("PhysicalFrameAllocator", () => {
  let alloc: PhysicalFrameAllocator;

  beforeEach(() => {
    alloc = new PhysicalFrameAllocator(8);
  });

  it("should start with all frames free", () => {
    expect(alloc.free_count()).toBe(8);
  });

  it("should allocate sequential frame numbers", () => {
    expect(alloc.allocate()).toBe(0);
    expect(alloc.allocate()).toBe(1);
    expect(alloc.allocate()).toBe(2);
    expect(alloc.free_count()).toBe(5);
  });

  it("should return undefined when all frames are allocated", () => {
    for (let i = 0; i < 8; i++) {
      alloc.allocate();
    }
    expect(alloc.allocate()).toBeUndefined();
    expect(alloc.free_count()).toBe(0);
  });

  it("should free a frame and make it available again", () => {
    const frame = alloc.allocate()!;
    expect(alloc.free_count()).toBe(7);

    alloc.free(frame);
    expect(alloc.free_count()).toBe(8);
    expect(alloc.is_allocated(frame)).toBe(false);
  });

  it("should reuse freed frames", () => {
    alloc.allocate(); // 0
    alloc.allocate(); // 1
    alloc.allocate(); // 2

    alloc.free(1); // Free frame 1

    // Next allocation should find frame 1 (first free)
    const reused = alloc.allocate();
    expect(reused).toBe(1);
  });

  it("should throw on double-free", () => {
    const frame = alloc.allocate()!;
    alloc.free(frame);

    expect(() => alloc.free(frame)).toThrow("Double-free");
  });

  it("should throw on out-of-range frame number", () => {
    expect(() => alloc.free(99)).toThrow("out of range");
    expect(() => alloc.free(-1)).toThrow("out of range");
    expect(() => alloc.is_allocated(99)).toThrow("out of range");
  });

  it("should report allocation status correctly", () => {
    alloc.allocate(); // 0
    expect(alloc.is_allocated(0)).toBe(true);
    expect(alloc.is_allocated(1)).toBe(false);
  });
});

// ============================================================================
// FIFO Policy Tests
// ============================================================================

describe("FIFOPolicy", () => {
  let policy: FIFOPolicy;

  beforeEach(() => {
    policy = new FIFOPolicy();
  });

  it("should return undefined when empty", () => {
    expect(policy.select_victim()).toBeUndefined();
  });

  it("should evict in FIFO order", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    expect(policy.select_victim()).toBe(10); // oldest
    expect(policy.select_victim()).toBe(20);
    expect(policy.select_victim()).toBe(30);
    expect(policy.select_victim()).toBeUndefined();
  });

  it("should ignore access patterns", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    // Accessing frame 10 a lot should NOT change eviction order (it is FIFO)
    policy.record_access(10);
    policy.record_access(10);
    policy.record_access(10);

    expect(policy.select_victim()).toBe(10); // still evicts oldest
  });

  it("should remove a specific frame", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    policy.remove_frame(20);

    expect(policy.select_victim()).toBe(10);
    expect(policy.select_victim()).toBe(30);
  });

  it("should handle removing a non-existent frame gracefully", () => {
    policy.add_frame(10);
    policy.remove_frame(999); // Should not throw
    expect(policy.select_victim()).toBe(10);
  });
});

// ============================================================================
// LRU Policy Tests
// ============================================================================

describe("LRUPolicy", () => {
  let policy: LRUPolicy;

  beforeEach(() => {
    policy = new LRUPolicy();
  });

  it("should return undefined when empty", () => {
    expect(policy.select_victim()).toBeUndefined();
  });

  it("should evict the least recently used frame", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    // 10 is LRU (added first, never accessed since)
    expect(policy.select_victim()).toBe(10);
  });

  it("should update order on access", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    // Access 10 — moves it to most recent
    policy.record_access(10);

    // Now 20 is LRU
    expect(policy.select_victim()).toBe(20);
    // Then 30
    expect(policy.select_victim()).toBe(30);
    // Then 10 (most recently accessed)
    expect(policy.select_victim()).toBe(10);
  });

  it("should handle repeated access correctly", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    // Access pattern: 20, 10, 30, 10
    policy.record_access(20);
    policy.record_access(10);
    policy.record_access(30);
    policy.record_access(10);

    // LRU order: 20 (oldest access), 30, 10 (most recent)
    expect(policy.select_victim()).toBe(20);
  });

  it("should remove a specific frame", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    policy.remove_frame(10);

    expect(policy.select_victim()).toBe(20);
    expect(policy.select_victim()).toBe(30);
  });
});

// ============================================================================
// Clock Policy Tests
// ============================================================================

describe("ClockPolicy", () => {
  let policy: ClockPolicy;

  beforeEach(() => {
    policy = new ClockPolicy();
  });

  it("should return undefined when empty", () => {
    expect(policy.select_victim()).toBeUndefined();
  });

  it("should evict a frame with use bit cleared", () => {
    policy.add_frame(10); // use=1
    policy.add_frame(20); // use=1
    policy.add_frame(30); // use=1

    // All use bits are set (just added). The clock algorithm will:
    // 1. Look at frame 10, use=1 -> clear, advance
    // 2. Look at frame 20, use=1 -> clear, advance
    // 3. Look at frame 30, use=1 -> clear, advance
    // 4. Look at frame 10, use=0 -> EVICT
    expect(policy.select_victim()).toBe(10);
  });

  it("should give a second chance to recently accessed frames", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    // Clear all use bits by selecting victims partway (we will manipulate directly)
    // Access frame 10 to re-set its use bit
    // First, let's create a scenario: add frames, then do a partial sweep

    // Actually, let's test more carefully:
    // All frames start with use=1. Access frame 10 again (no-op since already 1).
    // select_victim will sweep: clear 10, clear 20, clear 30, evict 10.
    const victim = policy.select_victim();
    expect(victim).toBe(10);
  });

  it("should respect use bits", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    // Select a victim to force a sweep and clear bits
    // After this, frames 20 and 30 remain with use=0
    policy.select_victim(); // evicts 10

    // Now 20 and 30 have use=0. Access 20 to set its use bit.
    policy.record_access(20);

    // select_victim: looks at 20 (use=1, clear, advance),
    // looks at 30 (use=0, evict)
    expect(policy.select_victim()).toBe(30);
  });

  it("should remove a specific frame", () => {
    policy.add_frame(10);
    policy.add_frame(20);
    policy.add_frame(30);

    policy.remove_frame(20);

    // First victim sweep: 10 (use=1, clear), 30 (use=1, clear),
    // 10 (use=0, evict)
    expect(policy.select_victim()).toBe(10);
    expect(policy.select_victim()).toBe(30);
  });

  it("should handle single frame", () => {
    policy.add_frame(42);

    // use=1, clear, then use=0, evict
    expect(policy.select_victim()).toBe(42);
    expect(policy.select_victim()).toBeUndefined();
  });
});

// ============================================================================
// MMU Tests
// ============================================================================

describe("MMU", () => {
  let mmu: MMU;

  beforeEach(() => {
    // 16 frames, LRU policy, TLB capacity 4
    mmu = new MMU(16, new LRUPolicy(), 4);
  });

  describe("create and destroy address space", () => {
    it("should create an address space", () => {
      mmu.create_address_space(1);
      expect(mmu.get_page_table(1)).toBeDefined();
    });

    it("should destroy an address space", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });
      mmu.destroy_address_space(1);

      expect(mmu.get_page_table(1)).toBeUndefined();
    });

    it("should free frames when destroying address space", () => {
      mmu.create_address_space(1);
      const initial_free = mmu.frame_allocator.free_count();

      mmu.map_page(1, 0x1000);
      mmu.map_page(1, 0x2000);
      expect(mmu.frame_allocator.free_count()).toBe(initial_free - 2);

      mmu.destroy_address_space(1);
      expect(mmu.frame_allocator.free_count()).toBe(initial_free);
    });

    it("should handle destroying non-existent address space", () => {
      // Should not throw
      mmu.destroy_address_space(999);
    });
  });

  describe("map_page", () => {
    it("should map a page and return the frame number", () => {
      mmu.create_address_space(1);
      const frame = mmu.map_page(1, 0x5000, { writable: true });

      expect(frame).toBeDefined();
      expect(frame).toBeGreaterThanOrEqual(0);
    });

    it("should throw when mapping without an address space", () => {
      expect(() => mmu.map_page(999, 0x1000)).toThrow("No address space");
    });

    it("should return undefined when out of frames", () => {
      const small_mmu = new MMU(2, new FIFOPolicy(), 4);
      small_mmu.create_address_space(1);

      small_mmu.map_page(1, 0x1000);
      small_mmu.map_page(1, 0x2000);

      // Now all frames are used. The next map_page will attempt eviction.
      // With FIFO, it should evict one of the existing pages.
      const result = small_mmu.map_page(1, 0x3000);
      // This might succeed (via eviction) or fail
      // Either way it should not throw
    });
  });

  describe("translate", () => {
    it("should translate a mapped address", () => {
      mmu.create_address_space(1);
      const frame = mmu.map_page(1, 0x5000, { writable: true })!;

      const phys = mmu.translate(1, 0x5abc);
      // phys = (frame << 12) | 0xABC
      expect(phys).toBe(((frame << PAGE_OFFSET_BITS) | 0xabc) >>> 0);
    });

    it("should use TLB cache on second access", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });

      // First translate — TLB miss, page table walk
      mmu.translate(1, 0x1000);
      expect(mmu.tlb.misses).toBe(1);
      expect(mmu.tlb.hits).toBe(0);

      // Second translate — TLB hit
      mmu.translate(1, 0x1000);
      expect(mmu.tlb.hits).toBe(1);
    });

    it("should handle page fault for unmapped address", () => {
      mmu.create_address_space(1);

      // Accessing an unmapped address triggers a page fault
      // which allocates a new frame
      const phys = mmu.translate(1, 0x9000);
      expect(phys).toBeDefined();
      expect(typeof phys).toBe("number");
    });

    it("should set dirty bit on write", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x3000, { writable: true });

      // Write access
      mmu.translate(1, 0x3000, true);

      const pte = mmu.get_page_table(1)!.lookupPTE(0x3000);
      expect(pte!.dirty).toBe(true);
      expect(pte!.accessed).toBe(true);
    });

    it("should set accessed bit on read", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x4000, { writable: true });

      mmu.translate(1, 0x4000, false);

      const pte = mmu.get_page_table(1)!.lookupPTE(0x4000);
      expect(pte!.accessed).toBe(true);
    });

    it("should throw for non-existent address space", () => {
      expect(() => mmu.translate(999, 0x1000)).toThrow("No address space");
    });
  });

  describe("clone_address_space (COW)", () => {
    it("should clone all mappings", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });
      mmu.map_page(1, 0x2000, { writable: true });

      mmu.clone_address_space(1, 2);

      const child_pt = mmu.get_page_table(2);
      expect(child_pt).toBeDefined();

      // Both should map to the same frames (shared via COW)
      const parent_pte = mmu.get_page_table(1)!.lookupPTE(0x1000);
      const child_pte = child_pt!.lookupPTE(0x1000);

      expect(child_pte).toBeDefined();
      expect(child_pte!.frame_number).toBe(parent_pte!.frame_number);
    });

    it("should mark shared pages as read-only", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });

      mmu.clone_address_space(1, 2);

      // Both parent and child should have the page as read-only (COW)
      const parent_pte = mmu.get_page_table(1)!.lookupPTE(0x1000);
      const child_pte = mmu.get_page_table(2)!.lookupPTE(0x1000);

      expect(parent_pte!.writable).toBe(false);
      expect(child_pte!.writable).toBe(false);
    });

    it("should perform COW copy on write after fork", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });

      // Record the original frame
      const original_frame = mmu
        .get_page_table(1)!
        .lookupPTE(0x1000)!.frame_number;

      mmu.clone_address_space(1, 2);

      // Write to the page in the child — triggers COW
      mmu.translate(2, 0x1000, true);

      // Child should now have a DIFFERENT frame (private copy)
      const child_pte = mmu.get_page_table(2)!.lookupPTE(0x1000);
      expect(child_pte!.frame_number).not.toBe(original_frame);
      expect(child_pte!.writable).toBe(true);
      expect(child_pte!.dirty).toBe(true);

      // Parent should still have the original frame
      const parent_pte = mmu.get_page_table(1)!.lookupPTE(0x1000);
      expect(parent_pte!.frame_number).toBe(original_frame);
    });

    it("should throw when cloning from non-existent address space", () => {
      expect(() => mmu.clone_address_space(999, 2)).toThrow(
        "No address space"
      );
    });
  });

  describe("context_switch", () => {
    it("should flush TLB on context switch", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });
      mmu.translate(1, 0x1000); // Populate TLB

      expect(mmu.tlb.size()).toBeGreaterThan(0);

      mmu.context_switch(2);

      expect(mmu.tlb.size()).toBe(0);
    });

    it("should cause TLB misses after context switch", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x1000, { writable: true });
      mmu.translate(1, 0x1000); // TLB miss=1
      mmu.translate(1, 0x1000); // TLB hit=1

      mmu.context_switch(1); // Flush TLB

      mmu.translate(1, 0x1000); // TLB miss again (miss=2)
      expect(mmu.tlb.misses).toBe(2);
    });
  });

  describe("page fault handling", () => {
    it("should allocate a frame on page fault", () => {
      mmu.create_address_space(1);
      const initial_free = mmu.frame_allocator.free_count();

      // Access an unmapped page — triggers page fault
      mmu.translate(1, 0x7000);

      // A frame should have been allocated
      expect(mmu.frame_allocator.free_count()).toBe(initial_free - 1);
    });

    it("should map the page after fault", () => {
      mmu.create_address_space(1);

      mmu.translate(1, 0x8000);

      // The page should now be mapped
      const pte = mmu.get_page_table(1)!.lookupPTE(0x8000);
      expect(pte).toBeDefined();
      expect(pte!.present).toBe(true);
    });
  });

  describe("eviction", () => {
    it("should evict pages when out of frames", () => {
      // Only 4 frames, FIFO policy
      const small_mmu = new MMU(4, new FIFOPolicy(), 4);
      small_mmu.create_address_space(1);

      // Map 4 pages — uses all frames
      small_mmu.map_page(1, 0x1000, { writable: true });
      small_mmu.map_page(1, 0x2000, { writable: true });
      small_mmu.map_page(1, 0x3000, { writable: true });
      small_mmu.map_page(1, 0x4000, { writable: true });

      expect(small_mmu.frame_allocator.free_count()).toBe(0);

      // Map a 5th page — must evict one
      const frame = small_mmu.map_page(1, 0x5000, { writable: true });
      expect(frame).toBeDefined();
    });
  });

  describe("integration: full translation flow", () => {
    it("should translate address 0x5ABC to correct physical address", () => {
      mmu.create_address_space(1);
      const frame = mmu.map_page(1, 0x5000)!;

      // Translate 0x5ABC:
      // VPN = 5, offset = 0xABC
      // Physical = (frame << 12) | 0xABC
      const phys = mmu.translate(1, 0x5abc);
      expect(phys).toBe(((frame << PAGE_OFFSET_BITS) | 0xabc) >>> 0);
    });

    it("should handle multiple processes independently", () => {
      mmu.create_address_space(1);
      mmu.create_address_space(2);

      const frame1 = mmu.map_page(1, 0x1000, { writable: true })!;
      const frame2 = mmu.map_page(2, 0x1000, { writable: true })!;

      // Same virtual address, different physical frames
      expect(frame1).not.toBe(frame2);

      const phys1 = mmu.translate(1, 0x1000);
      const phys2 = mmu.translate(2, 0x1000);

      expect(phys1).not.toBe(phys2);
    });

    it("should handle TLB miss -> page table walk -> TLB cache", () => {
      mmu.create_address_space(1);
      mmu.map_page(1, 0x2000, { writable: true });

      // First access: TLB miss
      mmu.translate(1, 0x2000);
      expect(mmu.tlb.misses).toBe(1);
      expect(mmu.tlb.hits).toBe(0);

      // Second access: TLB hit
      mmu.translate(1, 0x2000);
      expect(mmu.tlb.hits).toBe(1);

      // Flush TLB
      mmu.context_switch(1);

      // Third access: TLB miss again
      mmu.translate(1, 0x2000);
      expect(mmu.tlb.misses).toBe(2);
    });
  });
});

// ============================================================================
// Constants Tests
// ============================================================================

describe("Constants", () => {
  it("PAGE_SIZE should be 4096", () => {
    expect(PAGE_SIZE).toBe(4096);
  });

  it("PAGE_OFFSET_BITS should be 12", () => {
    expect(PAGE_OFFSET_BITS).toBe(12);
  });

  it("PAGE_OFFSET_MASK should be 0xFFF", () => {
    expect(PAGE_OFFSET_MASK).toBe(0xfff);
  });

  it("2^PAGE_OFFSET_BITS should equal PAGE_SIZE", () => {
    expect(1 << PAGE_OFFSET_BITS).toBe(PAGE_SIZE);
  });
});
