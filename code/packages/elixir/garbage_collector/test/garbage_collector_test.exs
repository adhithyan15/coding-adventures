defmodule CodingAdventures.GarbageCollectorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.GarbageCollector
  alias CodingAdventures.GarbageCollector.{ConsCell, LispClosure, Symbol, SymbolTable}

  test "allocates objects at monotonically increasing heap addresses" do
    gc = GarbageCollector.new()
    {gc, first} = GarbageCollector.allocate(gc, GarbageCollector.cons_cell(42, nil))
    {gc, second} = GarbageCollector.allocate(gc, GarbageCollector.symbol("next"))

    assert first == 0x10000
    assert second == 0x10001
    assert GarbageCollector.heap_size(gc) == 2
    assert {:ok, %ConsCell{}} = GarbageCollector.deref(gc, first)
    assert GarbageCollector.valid_address?(gc, second)
  end

  test "returns errors and raises for invalid or freed addresses" do
    gc = GarbageCollector.new()
    {gc, address} = GarbageCollector.allocate(gc, GarbageCollector.symbol("gone"))

    assert {:ok, %Symbol{}} = GarbageCollector.deref(gc, address)
    {gc, 1} = GarbageCollector.collect(gc, [])

    assert GarbageCollector.deref(gc, address) == {:error, :invalid_address}
    assert GarbageCollector.deref(gc, "not an address") == {:error, :invalid_address}
    assert_raise KeyError, fn -> GarbageCollector.deref!(gc, address) end
    refute GarbageCollector.valid_address?(gc, address)
    refute GarbageCollector.valid_address?(gc, "not an address")
  end

  test "keeps a root and its transitive references alive" do
    gc = GarbageCollector.new()
    {gc, tail} = GarbageCollector.allocate(gc, GarbageCollector.symbol("tail"))
    {gc, middle} = GarbageCollector.allocate(gc, GarbageCollector.cons_cell(tail, nil))
    {gc, head} = GarbageCollector.allocate(gc, GarbageCollector.cons_cell(1, middle))

    {gc, freed} = GarbageCollector.collect(gc, [head])

    assert freed == 0
    assert GarbageCollector.heap_size(gc) == 3
    assert GarbageCollector.valid_address?(gc, tail)
  end

  test "collects unreachable cycles" do
    gc = GarbageCollector.new()
    left = GarbageCollector.cons_cell()
    right = GarbageCollector.cons_cell()
    {gc, left_address} = GarbageCollector.allocate(gc, left)
    {gc, right_address} = GarbageCollector.allocate(gc, right)

    gc = %{
      gc
      | heap:
          gc.heap
          |> Map.put(left_address, %{left | cdr: right_address})
          |> Map.put(right_address, %{right | cdr: left_address})
    }

    {gc, freed} = GarbageCollector.collect(gc, [])

    assert freed == 2
    assert GarbageCollector.heap_size(gc) == 0
  end

  test "scans nested root lists and maps" do
    gc = GarbageCollector.new()
    {gc, from_list} = GarbageCollector.allocate(gc, GarbageCollector.symbol("list-root"))
    {gc, from_map} = GarbageCollector.allocate(gc, GarbageCollector.symbol("map-root"))
    {gc, unreachable} = GarbageCollector.allocate(gc, GarbageCollector.symbol("unreachable"))

    {gc, freed} = GarbageCollector.collect(gc, [[from_list], %{global: from_map, literal: 42}])

    assert freed == 1
    assert GarbageCollector.valid_address?(gc, from_list)
    assert GarbageCollector.valid_address?(gc, from_map)
    refute GarbageCollector.valid_address?(gc, unreachable)
  end

  test "tracks collection statistics" do
    gc = GarbageCollector.new()
    {gc, root} = GarbageCollector.allocate(gc, GarbageCollector.symbol("root"))
    {gc, _temp} = GarbageCollector.allocate(gc, GarbageCollector.symbol("temp"))
    {gc, 1} = GarbageCollector.collect(gc, [root])

    assert GarbageCollector.stats(gc) == %{
             total_allocations: 2,
             total_collections: 1,
             total_freed: 1,
             heap_size: 1
           }
  end

  test "reports references from heap object structs" do
    closure = GarbageCollector.lisp_closure("lambda", %{"x" => 0x10000, "y" => "plain"}, ["arg"])

    assert GarbageCollector.references(GarbageCollector.cons_cell(0x10000, "tail")) == [0x10000]
    assert GarbageCollector.references(GarbageCollector.symbol("plain")) == []
    assert GarbageCollector.references(closure) == [0x10000]
    assert GarbageCollector.references(%{value: 0x10000}) == []
    assert %LispClosure{code: "lambda", params: ["arg"]} = closure
  end

  test "symbol table interns, looks up, lists, and reallocates live symbols" do
    gc = GarbageCollector.new()
    table = SymbolTable.new()

    {table, gc, first} = SymbolTable.intern(table, gc, "foo")
    {table, gc, second} = SymbolTable.intern(table, gc, "foo")
    {table, gc, other} = SymbolTable.intern(table, gc, "bar")

    assert first == second
    assert first != other
    assert SymbolTable.lookup(table, gc, "foo") == {:ok, first}
    assert SymbolTable.lookup(table, gc, "missing") == :error
    assert SymbolTable.all_symbols(table, gc) == %{"foo" => first, "bar" => other}

    {gc, 2} = GarbageCollector.collect(gc, [])

    assert SymbolTable.lookup(table, gc, "foo") == :error
    assert SymbolTable.all_symbols(table, gc) == %{}

    {table, gc, fresh} = SymbolTable.intern(table, gc, "foo")
    assert fresh != first
    assert SymbolTable.all_symbols(table, gc) == %{"foo" => fresh}
  end
end
