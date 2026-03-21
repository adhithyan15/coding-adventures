defmodule CodingAdventures.BranchPredictor.BTBTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.BTB

  # ── Construction ─────────────────────────────────────────────────────────

  test "new/0 creates BTB with default size 256" do
    btb = BTB.new()
    assert btb.size == 256
    assert btb.entries == %{}
    assert btb.lookups == 0
    assert btb.hits == 0
    assert btb.misses == 0
  end

  test "new/1 accepts custom size" do
    btb = BTB.new(size: 64)
    assert btb.size == 64
  end

  # ── Lookup miss (cold start) ────────────────────────────────────────────

  test "lookup/2 returns nil on cold start" do
    btb = BTB.new()
    {target, btb} = BTB.lookup(btb, 0x100)
    assert target == nil
    assert btb.lookups == 1
    assert btb.misses == 1
    assert btb.hits == 0
  end

  test "lookup/2 returns nil for unseen branches" do
    btb = BTB.new()
    {nil, btb} = BTB.lookup(btb, 0x100)
    {nil, btb} = BTB.lookup(btb, 0x200)
    {nil, btb} = BTB.lookup(btb, 0x300)
    assert btb.lookups == 3
    assert btb.misses == 3
  end

  # ── Update then hit ─────────────────────────────────────────────────────

  test "update then lookup returns correct target" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    {target, btb} = BTB.lookup(btb, 0x100)
    assert target == 0x200
    assert btb.hits == 1
  end

  test "update with branch_type" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200, "unconditional")
    entry = BTB.get_entry(btb, 0x100)
    assert entry.branch_type == "unconditional"
  end

  test "update overwrites existing entry" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    btb = BTB.update(btb, 0x100, 0x300)
    {target, _btb} = BTB.lookup(btb, 0x100)
    assert target == 0x300
  end

  test "default branch_type is conditional" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    entry = BTB.get_entry(btb, 0x100)
    assert entry.branch_type == "conditional"
  end

  # ── Multiple entries ────────────────────────────────────────────────────

  test "multiple branches stored independently" do
    btb = BTB.new(size: 1024)
    btb = BTB.update(btb, 0x100, 0x200)
    btb = BTB.update(btb, 0x300, 0x400)

    {t1, _} = BTB.lookup(btb, 0x100)
    {t2, _} = BTB.lookup(btb, 0x300)
    assert t1 == 0x200
    assert t2 == 0x400
  end

  test "three branches coexist" do
    btb = BTB.new(size: 256)
    btb = BTB.update(btb, 0x01, 0x200)
    btb = BTB.update(btb, 0x02, 0x400)
    btb = BTB.update(btb, 0x03, 0x600)

    {t1, _} = BTB.lookup(btb, 0x01)
    {t2, _} = BTB.lookup(btb, 0x02)
    {t3, _} = BTB.lookup(btb, 0x03)
    assert t1 == 0x200
    assert t2 == 0x400
    assert t3 == 0x600
  end

  # ── Branch types ────────────────────────────────────────────────────────

  test "custom branch types stored correctly" do
    btb = BTB.new(size: 256)

    entries = [
      {0x01, "conditional"},
      {0x02, "unconditional"},
      {0x03, "call"},
      {0x04, "return"}
    ]

    btb =
      Enum.reduce(entries, btb, fn {pc, btype}, acc ->
        BTB.update(acc, pc, pc + 0x100, btype)
      end)

    for {pc, btype} <- entries do
      entry = BTB.get_entry(btb, pc)
      assert entry != nil
      assert entry.branch_type == btype
    end
  end

  # ── Eviction (aliasing) ────────────────────────────────────────────────

  test "aliasing — new branch evicts old branch at same index" do
    btb = BTB.new(size: 4)
    btb = BTB.update(btb, 0, 0x100)
    btb = BTB.update(btb, 4, 0x200)

    {target0, _} = BTB.lookup(btb, 0)
    assert target0 == nil

    {target4, _} = BTB.lookup(btb, 4)
    assert target4 == 0x200
  end

  test "aliasing — tag mismatch causes miss" do
    btb = BTB.new(size: 2)
    btb = BTB.update(btb, 0, 0x100)
    btb = BTB.update(btb, 2, 0x200)
    {target, _} = BTB.lookup(btb, 0)
    assert target == nil
  end

  test "no eviction with large table" do
    btb = BTB.new(size: 4096)
    btb = BTB.update(btb, 0x100, 0x200)
    btb = BTB.update(btb, 0x104, 0x300)
    {t1, _} = BTB.lookup(btb, 0x100)
    {t2, _} = BTB.lookup(btb, 0x104)
    assert t1 == 0x200
    assert t2 == 0x300
  end

  # ── get_entry ───────────────────────────────────────────────────────────

  test "get_entry/2 returns entry when present" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200, "call")
    entry = BTB.get_entry(btb, 0x100)
    assert entry.tag == 0x100
    assert entry.target == 0x200
    assert entry.branch_type == "call"
  end

  test "get_entry/2 returns nil when not present" do
    btb = BTB.new()
    assert BTB.get_entry(btb, 0x100) == nil
  end

  test "get_entry/2 returns nil on tag mismatch" do
    btb = BTB.new(size: 4)
    btb = BTB.update(btb, 0, 0x100)
    btb = BTB.update(btb, 4, 0x200)
    assert BTB.get_entry(btb, 0) == nil
  end

  # ── Hit rate ────────────────────────────────────────────────────────────

  test "hit_rate/1 returns 0.0 with no lookups" do
    btb = BTB.new()
    assert BTB.hit_rate(btb) == 0.0
  end

  test "hit_rate/1 returns 100.0 when all hits" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    {_, btb} = BTB.lookup(btb, 0x100)
    {_, btb} = BTB.lookup(btb, 0x100)
    assert BTB.hit_rate(btb) == 100.0
  end

  test "hit_rate/1 returns 0.0 when all misses" do
    btb = BTB.new()
    {_, btb} = BTB.lookup(btb, 0x100)
    {_, btb} = BTB.lookup(btb, 0x200)
    assert BTB.hit_rate(btb) == 0.0
  end

  test "hit_rate/1 returns correct percentage" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    {_, btb} = BTB.lookup(btb, 0x100)
    {_, btb} = BTB.lookup(btb, 0x300)
    assert BTB.hit_rate(btb) == 50.0
  end

  # ── Reset ───────────────────────────────────────────────────────────────

  test "reset/1 clears entries and stats" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    {_, btb} = BTB.lookup(btb, 0x100)
    btb = BTB.reset(btb)

    assert btb.entries == %{}
    assert btb.lookups == 0
    assert btb.hits == 0
    assert btb.misses == 0
  end

  test "reset/1 preserves size" do
    btb = BTB.new(size: 128)
    btb = BTB.update(btb, 0x100, 0x200)
    btb = BTB.reset(btb)
    assert btb.size == 128
  end

  test "after reset, lookups return nil" do
    btb = BTB.new()
    btb = BTB.update(btb, 0x100, 0x200)
    btb = BTB.reset(btb)
    {target, _} = BTB.lookup(btb, 0x100)
    assert target == nil
  end

  # ── Immutability ────────────────────────────────────────────────────────

  test "update/3 returns new struct, original unchanged" do
    original = BTB.new()
    _updated = BTB.update(original, 0x100, 0x200)
    assert original.entries == %{}
  end

  test "lookup/2 returns new struct, original unchanged" do
    original = BTB.new()
    {_, _updated} = BTB.lookup(original, 0x100)
    assert original.lookups == 0
    assert original.misses == 0
  end

  # ── Capacity ────────────────────────────────────────────────────────────

  test "BTB with size 1 evicts on every new branch" do
    btb = BTB.new(size: 1)
    btb = BTB.update(btb, 0x100, 0x200)
    btb = BTB.update(btb, 0x300, 0x400)
    {t1, _} = BTB.lookup(btb, 0x100)
    {t2, _} = BTB.lookup(btb, 0x300)
    assert t1 == nil
    assert t2 == 0x400
  end
end
