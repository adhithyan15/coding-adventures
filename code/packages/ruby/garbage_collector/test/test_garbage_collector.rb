# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_garbage_collector"

GC_MOD = CodingAdventures::GarbageCollector

# ================================================================
# Tests for the Mark-and-Sweep Garbage Collector
# ================================================================
#
# These tests verify:
#   1. Allocation increases heap size
#   2. deref retrieves the object
#   3. collect frees unreachable objects
#   4. collect preserves reachable objects
#   5. Reference cycles are correctly collected
#   6. Stats counters stay accurate
# ================================================================

class TestMarkAndSweepGC < Minitest::Test
  def setup
    @gc = GC_MOD::MarkAndSweepGC.new
  end

  def test_version_exists
    refute_nil GC_MOD::VERSION
  end

  # ------------------------------------------------------------------
  # Allocation
  # ------------------------------------------------------------------

  def test_allocate_returns_address
    addr = @gc.allocate(GC_MOD::ConsCell.new)
    assert addr.is_a?(Integer)
    assert addr >= GC_MOD::MarkAndSweepGC::HEAP_BASE_ADDRESS
  end

  def test_allocate_increases_heap_size
    assert_equal 0, @gc.heap_size
    @gc.allocate(GC_MOD::ConsCell.new)
    assert_equal 1, @gc.heap_size
    @gc.allocate(GC_MOD::LispSymbol.new(name: "x"))
    assert_equal 2, @gc.heap_size
  end

  def test_deref_returns_object
    cell = GC_MOD::ConsCell.new(car: 99, cdr: nil)
    addr = @gc.allocate(cell)
    retrieved = @gc.deref(addr)
    assert_equal 99, retrieved.car
  end

  def test_deref_invalid_address_raises
    assert_raises(KeyError) { @gc.deref(0) }
  end

  # ------------------------------------------------------------------
  # Collection — unreachable objects are freed
  # ------------------------------------------------------------------

  def test_collect_frees_unreachable
    @gc.allocate(GC_MOD::ConsCell.new(car: 1))
    @gc.allocate(GC_MOD::LispSymbol.new(name: "x"))
    assert_equal 2, @gc.heap_size
    freed = @gc.collect(roots: [])
    assert_equal 2, freed
    assert_equal 0, @gc.heap_size
  end

  def test_collect_preserves_reachable
    addr = @gc.allocate(GC_MOD::LispSymbol.new(name: "alive"))
    _dead = @gc.allocate(GC_MOD::LispSymbol.new(name: "dead"))
    assert_equal 2, @gc.heap_size
    freed = @gc.collect(roots: [addr])
    assert_equal 1, freed
    assert_equal 1, @gc.heap_size
    assert_equal "alive", @gc.deref(addr).name
  end

  def test_collect_preserves_transitively_reachable
    # Build a chain: a → b → c
    c = @gc.allocate(GC_MOD::LispSymbol.new(name: "c"))
    b = @gc.allocate(GC_MOD::ConsCell.new(car: c, cdr: nil))
    a = @gc.allocate(GC_MOD::ConsCell.new(car: b, cdr: nil))
    freed = @gc.collect(roots: [a])
    assert_equal 0, freed
    assert_equal 3, @gc.heap_size
  end

  # ------------------------------------------------------------------
  # Cycles
  # ------------------------------------------------------------------

  def test_collect_handles_reference_cycle
    # a.cdr = b, b.cdr = a — a cycle with no external root
    cell_a = GC_MOD::ConsCell.new
    cell_b = GC_MOD::ConsCell.new
    addr_a = @gc.allocate(cell_a)
    addr_b = @gc.allocate(cell_b)
    @gc.deref(addr_a).cdr = addr_b
    @gc.deref(addr_b).cdr = addr_a
    freed = @gc.collect(roots: [])
    assert_equal 2, freed
    assert_equal 0, @gc.heap_size
  end

  def test_collect_keeps_cycle_when_root_points_in
    cell_a = GC_MOD::ConsCell.new
    cell_b = GC_MOD::ConsCell.new
    addr_a = @gc.allocate(cell_a)
    addr_b = @gc.allocate(cell_b)
    @gc.deref(addr_a).cdr = addr_b
    @gc.deref(addr_b).cdr = addr_a
    # Root points to a — both should survive
    freed = @gc.collect(roots: [addr_a])
    assert_equal 0, freed
    assert_equal 2, @gc.heap_size
  end

  # ------------------------------------------------------------------
  # Stats
  # ------------------------------------------------------------------

  def test_stats_allocation_count
    3.times { @gc.allocate(GC_MOD::LispSymbol.new(name: "x")) }
    assert_equal 3, @gc.stats[:total_allocations]
  end

  def test_stats_collection_count
    @gc.collect(roots: [])
    @gc.collect(roots: [])
    assert_equal 2, @gc.stats[:total_collections]
  end

  def test_stats_freed_count
    @gc.allocate(GC_MOD::LispSymbol.new(name: "a"))
    @gc.allocate(GC_MOD::LispSymbol.new(name: "b"))
    @gc.collect(roots: [])
    assert_equal 2, @gc.stats[:total_freed]
  end

  def test_stats_heap_size
    @gc.allocate(GC_MOD::ConsCell.new)
    assert_equal 1, @gc.stats[:heap_size]
  end

  # ------------------------------------------------------------------
  # valid_address?
  # ------------------------------------------------------------------

  def test_valid_address_true
    addr = @gc.allocate(GC_MOD::ConsCell.new)
    assert @gc.valid_address?(addr)
  end

  def test_valid_address_false_after_collect
    addr = @gc.allocate(GC_MOD::LispSymbol.new(name: "gone"))
    @gc.collect(roots: [])
    refute @gc.valid_address?(addr)
  end

  # ------------------------------------------------------------------
  # SymbolTable
  # ------------------------------------------------------------------

  def test_symbol_table_intern
    st = GC_MOD::SymbolTable.new(@gc)
    addr1 = st.intern("foo")
    addr2 = st.intern("foo")
    assert_equal addr1, addr2
  end

  def test_symbol_table_different_names
    st = GC_MOD::SymbolTable.new(@gc)
    a = st.intern("foo")
    b = st.intern("bar")
    refute_equal a, b
  end

  def test_symbol_table_all_addresses
    st = GC_MOD::SymbolTable.new(@gc)
    st.intern("x")
    st.intern("y")
    assert_equal 2, st.all_addresses.size
  end
end

class TestHeapObjects < Minitest::Test
  def test_cons_cell_references_integers
    cell = GC_MOD::ConsCell.new(car: 65536, cdr: 65537)
    assert_equal [65536, 65537], cell.references
  end

  def test_cons_cell_no_references_for_non_int
    cell = GC_MOD::ConsCell.new(car: "string", cdr: nil)
    assert_empty cell.references
  end

  def test_lisp_symbol_no_references
    sym = GC_MOD::LispSymbol.new(name: "foo")
    assert_empty sym.references
  end

  def test_lisp_closure_references_env_integers
    closure = GC_MOD::LispClosure.new(env: { "x" => 65536, "y" => "not_addr" })
    assert_equal [65536], closure.references
  end

  def test_marked_flag_starts_false
    obj = GC_MOD::ConsCell.new
    refute obj.marked
  end
end
