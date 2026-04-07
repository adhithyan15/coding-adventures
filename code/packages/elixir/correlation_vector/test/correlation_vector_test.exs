defmodule CodingAdventures.CorrelationVectorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CorrelationVector
  alias CodingAdventures.CorrelationVector.{Origin, Contribution}

  # ===========================================================================
  # Group 1: Root lifecycle
  # ===========================================================================
  #
  # Create a root CV, contribute to it, pass it through a stage, then delete it.
  # Verify the ID format, contribution order, and deletion semantics.

  describe "root lifecycle" do
    test "create a root CV with an origin — ID format is base.N" do
      log = CorrelationVector.new()
      origin = %Origin{source: "app.ts", location: "5:12"}
      {cv_id, log} = CorrelationVector.create(log, origin)

      # The ID must be "base.N" — exactly one dot, N >= 1
      assert Regex.match?(~r/^[0-9a-f]{8}\.\d+$/, cv_id)

      entry = CorrelationVector.get(log, cv_id)
      assert entry != nil
      assert entry.id == cv_id
      assert entry.parent_ids == []
      assert entry.origin == origin
      assert entry.contributions == []
      assert entry.deleted == nil
    end

    test "create without origin — base is 00000000" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)

      assert String.starts_with?(cv_id, "00000000.")
      assert CorrelationVector.get(log, cv_id) != nil
    end

    test "two creates with same origin produce sequential IDs" do
      log = CorrelationVector.new()
      origin = %Origin{source: "file.ts", location: "1:1"}
      {cv_id1, log} = CorrelationVector.create(log, origin)
      {cv_id2, _log} = CorrelationVector.create(log, origin)

      [base1, n1] = String.split(cv_id1, ".")
      [base2, n2] = String.split(cv_id2, ".")

      assert base1 == base2
      assert String.to_integer(n2) == String.to_integer(n1) + 1
    end

    test "contribute to a root CV — contribution appears in history" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)

      log = CorrelationVector.contribute(log, cv_id, "parser", "created", %{token: "IDENTIFIER"})
      log = CorrelationVector.contribute(log, cv_id, "scope_analysis", "resolved", %{binding: "local:x"})

      history = CorrelationVector.history(log, cv_id)
      assert length(history) == 2
      assert Enum.at(history, 0) == %Contribution{source: "parser", tag: "created", meta: %{token: "IDENTIFIER"}}
      assert Enum.at(history, 1) == %Contribution{source: "scope_analysis", tag: "resolved", meta: %{binding: "local:x"}}
    end

    test "pass_order accumulates unique sources in contribution order" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)

      log = CorrelationVector.contribute(log, cv_id, "parser", "created")
      log = CorrelationVector.contribute(log, cv_id, "scope", "resolved")
      # second contribution from same source — should NOT re-add to pass_order
      log = CorrelationVector.contribute(log, cv_id, "parser", "updated")

      assert log.pass_order == ["parser", "scope"]
    end

    test "passthrough — recorded as 'passthrough' tag contribution" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)

      log = CorrelationVector.passthrough(log, cv_id, "type_checker")

      history = CorrelationVector.history(log, cv_id)
      assert length(history) == 1
      assert Enum.at(history, 0) == %Contribution{source: "type_checker", tag: "passthrough", meta: %{}}
    end

    test "delete — deletion record stored, further contributions raise" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.contribute(log, cv_id, "parser", "created")

      log = CorrelationVector.delete(log, cv_id, "dce", "unreachable", %{entry: "main"})

      entry = CorrelationVector.get(log, cv_id)
      assert entry.deleted != nil
      assert entry.deleted.source == "dce"
      assert entry.deleted.reason == "unreachable"
      assert entry.deleted.meta == %{entry: "main"}

      # Contributing to a deleted entry must raise
      assert_raise RuntimeError, fn ->
        CorrelationVector.contribute(log, cv_id, "another", "attempt")
      end
    end

    test "delete — history includes deletion as final synthetic contribution" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.contribute(log, cv_id, "parser", "created")
      log = CorrelationVector.delete(log, cv_id, "dce", "unreachable", %{})

      history = CorrelationVector.history(log, cv_id)
      assert length(history) == 2
      last = List.last(history)
      assert last.tag == "deleted"
      assert last.source == "dce"
    end

    test "contribute with no meta defaults to empty map" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.contribute(log, cv_id, "stage", "tagged")

      [contrib] = CorrelationVector.history(log, cv_id)
      assert contrib.meta == %{}
    end
  end

  # ===========================================================================
  # Group 2: Derivation
  # ===========================================================================

  describe "derivation" do
    test "derive two children from one parent — both have parent ID as prefix" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      {child_a, log} = CorrelationVector.derive(log, parent_id)
      {child_b, _log} = CorrelationVector.derive(log, parent_id)

      assert String.starts_with?(child_a, parent_id <> ".")
      assert String.starts_with?(child_b, parent_id <> ".")
      assert child_a != child_b
    end

    test "derived child has correct parent_ids" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      {child_id, log} = CorrelationVector.derive(log, parent_id)

      entry = CorrelationVector.get(log, child_id)
      assert entry.parent_ids == [parent_id]
    end

    test "ancestors(child) returns [parent_cv_id]" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      {child_id, log} = CorrelationVector.derive(log, parent_id)

      assert CorrelationVector.ancestors(log, child_id) == [parent_id]
    end

    test "descendants(parent) returns both child IDs" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      {child_a, log} = CorrelationVector.derive(log, parent_id)
      {child_b, log} = CorrelationVector.derive(log, parent_id)

      descendants = CorrelationVector.descendants(log, parent_id) |> Enum.sort()
      assert Enum.sort([child_a, child_b]) == descendants
    end

    test "derive with an origin stores the origin in the child entry" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      origin = %Origin{source: "splitter", location: "col:0-5"}
      {child_id, log} = CorrelationVector.derive(log, parent_id, origin)

      entry = CorrelationVector.get(log, child_id)
      assert entry.origin == origin
    end

    test "derived child sequence numbers start at 1 per parent" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      {child_a, log} = CorrelationVector.derive(log, parent_id)
      {child_b, _log} = CorrelationVector.derive(log, parent_id)

      assert child_a == parent_id <> ".1"
      assert child_b == parent_id <> ".2"
    end
  end

  # ===========================================================================
  # Group 3: Merging
  # ===========================================================================

  describe "merging" do
    test "merge three CVs into one — parent_ids lists all three" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log)
      {b, log} = CorrelationVector.create(log)
      {c, log} = CorrelationVector.create(log)

      {merged_id, log} = CorrelationVector.merge(log, [a, b, c])

      entry = CorrelationVector.get(log, merged_id)
      assert Enum.sort(entry.parent_ids) == Enum.sort([a, b, c])
    end

    test "ancestors(merged) returns all three parents" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log)
      {b, log} = CorrelationVector.create(log)
      {c, log} = CorrelationVector.create(log)

      {merged_id, log} = CorrelationVector.merge(log, [a, b, c])

      ancs = CorrelationVector.ancestors(log, merged_id)
      assert Enum.sort(ancs) == Enum.sort([a, b, c])
    end

    test "merge with no origin uses 00000000 base" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log)
      {b, log} = CorrelationVector.create(log)

      {merged_id, _log} = CorrelationVector.merge(log, [a, b])
      assert String.starts_with?(merged_id, "00000000.")
    end

    test "merge with an origin uses origin-based base" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log)
      {b, log} = CorrelationVector.create(log)

      origin = %Origin{source: "join_stage", location: "orders.id=customers.id"}
      {merged_id, _log} = CorrelationVector.merge(log, [a, b], origin)

      # Should NOT start with 00000000 (it's a real origin)
      refute String.starts_with?(merged_id, "00000000.")
    end

    test "derive on a deleted parent is allowed (tombstone record)" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.delete(log, parent_id, "dce", "unreachable", %{})

      # Deriving from a deleted parent is allowed (we can create tombstone records)
      {child_id, log} = CorrelationVector.derive(log, parent_id)
      assert CorrelationVector.get(log, child_id) != nil
    end
  end

  # ===========================================================================
  # Group 4: Deep ancestry chain
  # ===========================================================================

  describe "deep ancestry chain" do
    test "A → B → C → D: ancestors(D) = [C, B, A] (nearest first)" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log, %Origin{source: "root", location: "0:0"})
      {b, log} = CorrelationVector.derive(log, a)
      {c, log} = CorrelationVector.derive(log, b)
      {d, log} = CorrelationVector.derive(log, c)

      ancs = CorrelationVector.ancestors(log, d)
      assert ancs == [c, b, a]
    end

    test "lineage(D) returns all four entries oldest-first" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log, %Origin{source: "root", location: "0:0"})
      {b, log} = CorrelationVector.derive(log, a)
      {c, log} = CorrelationVector.derive(log, b)
      {d, log} = CorrelationVector.derive(log, c)

      lineage = CorrelationVector.lineage(log, d)
      assert length(lineage) == 4

      ids = Enum.map(lineage, & &1.id)
      assert ids == [a, b, c, d]
    end

    test "lineage returns empty list for unknown cv_id" do
      log = CorrelationVector.new()
      assert CorrelationVector.lineage(log, "unknown.1") == []
    end

    test "ancestors returns empty list for root CV" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      assert CorrelationVector.ancestors(log, cv_id) == []
    end

    test "ancestors returns empty list for unknown cv_id" do
      log = CorrelationVector.new()
      assert CorrelationVector.ancestors(log, "unknown.1") == []
    end

    test "descendants returns empty list for leaf CV" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      assert CorrelationVector.descendants(log, cv_id) == []
    end
  end

  # ===========================================================================
  # Group 5: Disabled log
  # ===========================================================================

  describe "disabled log" do
    test "all operations complete without error" do
      log = CorrelationVector.new(false)

      {cv_id, log} = CorrelationVector.create(log, %Origin{source: "x", location: "1:1"})
      assert is_binary(cv_id)

      log = CorrelationVector.contribute(log, cv_id, "stage", "tagged")
      {child_id, log} = CorrelationVector.derive(log, cv_id)
      {merged_id, log} = CorrelationVector.merge(log, [cv_id, child_id])
      log = CorrelationVector.passthrough(log, cv_id, "checker")
      log = CorrelationVector.delete(log, cv_id, "dce", "unreachable")

      # None of these should raise
      assert is_binary(child_id)
      assert is_binary(merged_id)
      assert is_map(log)
    end

    test "get returns nil for any cv_id (nothing was stored)" do
      log = CorrelationVector.new(false)
      {cv_id, log} = CorrelationVector.create(log)

      assert CorrelationVector.get(log, cv_id) == nil
    end

    test "history returns empty list" do
      log = CorrelationVector.new(false)
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.contribute(log, cv_id, "stage", "tagged")

      assert CorrelationVector.history(log, cv_id) == []
    end

    test "all CV IDs are still generated and unique" do
      log = CorrelationVector.new(false)
      origin = %Origin{source: "file.ts", location: "1:1"}

      {id1, log} = CorrelationVector.create(log, origin)
      {id2, log} = CorrelationVector.create(log, origin)
      {id3, _log} = CorrelationVector.create(log, origin)

      assert id1 != id2
      assert id2 != id3
      assert id1 != id3
    end

    test "disabled log: ancestors, descendants, lineage all return empty" do
      log = CorrelationVector.new(false)
      {cv_id, log} = CorrelationVector.create(log)

      assert CorrelationVector.ancestors(log, cv_id) == []
      assert CorrelationVector.descendants(log, cv_id) == []
      assert CorrelationVector.lineage(log, cv_id) == []
    end
  end

  # ===========================================================================
  # Group 6: Serialization roundtrip
  # ===========================================================================

  describe "serialization roundtrip" do
    test "serialize produces a map with expected structure" do
      log = CorrelationVector.new()
      origin = %Origin{source: "app.ts", location: "5:12", timestamp: "2026-04-05T00:00:00Z"}
      {cv_id, log} = CorrelationVector.create(log, origin)
      log = CorrelationVector.contribute(log, cv_id, "parser", "created", %{token: "ID"})

      map = CorrelationVector.serialize(log)

      assert Map.has_key?(map, "entries")
      assert Map.has_key?(map, "pass_order")
      assert Map.has_key?(map, "enabled")
      assert map["enabled"] == true
      assert map["pass_order"] == ["parser"]
      assert Map.has_key?(map["entries"], cv_id)

      entry_map = map["entries"][cv_id]
      assert entry_map["id"] == cv_id
      assert entry_map["parent_ids"] == []
      assert entry_map["origin"]["source"] == "app.ts"
      assert entry_map["origin"]["location"] == "5:12"
      assert entry_map["origin"]["timestamp"] == "2026-04-05T00:00:00Z"
      assert length(entry_map["contributions"]) == 1
    end

    test "to_json_string produces valid JSON" do
      log = CorrelationVector.new()
      {_cv_id, log} = CorrelationVector.create(log)

      assert {:ok, json} = CorrelationVector.to_json_string(log)
      assert is_binary(json)
      assert String.contains?(json, "entries")
      assert String.contains?(json, "pass_order")
      assert String.contains?(json, "enabled")
    end

    test "from_json_string roundtrips a simple log" do
      log = CorrelationVector.new()
      origin = %Origin{source: "app.ts", location: "5:12"}
      {cv_id, log} = CorrelationVector.create(log, origin)
      log = CorrelationVector.contribute(log, cv_id, "parser", "created", %{})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      entry2 = CorrelationVector.get(log2, cv_id)
      assert entry2 != nil
      assert entry2.id == cv_id
      assert entry2.origin.source == "app.ts"
      assert length(entry2.contributions) == 1
      assert List.first(entry2.contributions).source == "parser"
      assert log2.pass_order == ["parser"]
      assert log2.enabled == true
    end

    test "roundtrip preserves derivation structure" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log)
      {child_id, log} = CorrelationVector.derive(log, parent_id)

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      child_entry = CorrelationVector.get(log2, child_id)
      assert child_entry.parent_ids == [parent_id]
    end

    test "roundtrip preserves merge parent_ids" do
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log)
      {b, log} = CorrelationVector.create(log)
      {merged_id, log} = CorrelationVector.merge(log, [a, b])

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      merged_entry = CorrelationVector.get(log2, merged_id)
      assert Enum.sort(merged_entry.parent_ids) == Enum.sort([a, b])
    end

    test "roundtrip preserves deletion record" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.delete(log, cv_id, "dce", "unreachable", %{entry: "main"})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      entry2 = CorrelationVector.get(log2, cv_id)
      assert entry2.deleted != nil
      assert entry2.deleted.source == "dce"
      assert entry2.deleted.reason == "unreachable"
    end

    test "roundtrip with complex log — every entry is preserved" do
      log = CorrelationVector.new()
      origin = %Origin{source: "input.ts", location: "1:1", timestamp: nil, meta: %{}}
      {root_id, log} = CorrelationVector.create(log, origin)
      log = CorrelationVector.contribute(log, root_id, "parser", "created")
      {child_a, log} = CorrelationVector.derive(log, root_id)
      {child_b, log} = CorrelationVector.derive(log, root_id)
      log = CorrelationVector.contribute(log, child_a, "scope", "resolved")
      {merged_id, log} = CorrelationVector.merge(log, [child_a, child_b])
      log = CorrelationVector.delete(log, child_b, "dce", "unreachable", %{})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      for id <- [root_id, child_a, child_b, merged_id] do
        original = CorrelationVector.get(log, id)
        restored = CorrelationVector.get(log2, id)
        assert restored != nil, "Entry #{id} missing after roundtrip"
        assert restored.id == original.id
        assert restored.parent_ids == original.parent_ids
        assert length(restored.contributions) == length(original.contributions)
      end
    end

    test "from_json_string returns error on invalid JSON" do
      assert {:error, _} = CorrelationVector.from_json_string("not json at all {{{")
    end

    test "after roundtrip, new creates produce unique IDs (counters rebuilt)" do
      log = CorrelationVector.new()
      origin = %Origin{source: "f.ts", location: "1:1"}
      {cv_id, log} = CorrelationVector.create(log, origin)

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      # A second create with same origin should produce a different ID
      {cv_id2, _log2} = CorrelationVector.create(log2, origin)
      assert cv_id2 != cv_id
    end
  end

  # ===========================================================================
  # Group 7: ID uniqueness
  # ===========================================================================

  describe "ID uniqueness" do
    test "10,000 creates with same origin produce unique IDs" do
      log = CorrelationVector.new()
      origin = %Origin{source: "bulk.ts", location: "0:0"}

      {ids, _log} =
        Enum.reduce(1..10_000, {[], log}, fn _, {acc, l} ->
          {cv_id, l} = CorrelationVector.create(l, origin)
          {[cv_id | acc], l}
        end)

      unique_count = ids |> Enum.uniq() |> length()
      assert unique_count == 10_000
    end

    test "creates with mixed origins produce no collisions" do
      log = CorrelationVector.new()

      origins = [
        %Origin{source: "file_a.ts", location: "1:1"},
        %Origin{source: "file_b.ts", location: "2:2"},
        %Origin{source: "file_c.ts", location: "3:3"}
      ]

      {ids, _log} =
        Enum.reduce(1..300, {[], log}, fn i, {acc, l} ->
          origin = Enum.at(origins, rem(i, 3))
          {cv_id, l} = CorrelationVector.create(l, origin)
          {[cv_id | acc], l}
        end)

      unique_count = ids |> Enum.uniq() |> length()
      assert unique_count == 300
    end

    test "derived IDs do not collide with root IDs" do
      log = CorrelationVector.new()
      {parent_id, log} = CorrelationVector.create(log, %Origin{source: "f.ts", location: "1:1"})
      {child_id, _log} = CorrelationVector.derive(log, parent_id)

      assert parent_id != child_id
    end

    test "get returns nil for nonexistent cv_id" do
      log = CorrelationVector.new()
      assert CorrelationVector.get(log, "nonexistent.1") == nil
    end

    test "history returns empty list for nonexistent cv_id" do
      log = CorrelationVector.new()
      assert CorrelationVector.history(log, "nonexistent.1") == []
    end
  end

  # ===========================================================================
  # Additional edge-case tests for >95% coverage
  # ===========================================================================

  describe "edge cases" do
    test "new/0 defaults to enabled: true" do
      log = CorrelationVector.new()
      assert log.enabled == true
    end

    test "new/1 with false is disabled" do
      log = CorrelationVector.new(false)
      assert log.enabled == false
    end

    test "contribute on nonexistent cv_id in enabled log — no crash, no entry created" do
      log = CorrelationVector.new()
      # cv_id was never added to log (e.g., from a disabled session)
      log2 = CorrelationVector.contribute(log, "ghost.1", "stage", "tagged")
      assert CorrelationVector.get(log2, "ghost.1") == nil
    end

    test "delete on nonexistent cv_id — no crash" do
      log = CorrelationVector.new()
      log2 = CorrelationVector.delete(log, "ghost.1", "stage", "gone", %{})
      assert log2 == log
    end

    test "passthrough records source in pass_order" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.passthrough(log, cv_id, "checker")
      assert "checker" in log.pass_order
    end

    test "origin with timestamp and meta serializes correctly" do
      log = CorrelationVector.new()
      origin = %Origin{
        source: "orders_table",
        location: "row_id:42",
        timestamp: "2026-01-01T12:00:00Z",
        meta: %{db: "postgres"}
      }
      {cv_id, log} = CorrelationVector.create(log, origin)
      map = CorrelationVector.serialize(log)
      entry_map = map["entries"][cv_id]
      assert entry_map["origin"]["timestamp"] == "2026-01-01T12:00:00Z"
      assert entry_map["origin"]["meta"]["db"] == "postgres"
    end

    test "deletion meta is preserved through roundtrip" do
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.delete(log, cv_id, "dce", "unused", %{flag: "hot"})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      entry = CorrelationVector.get(log2, cv_id)
      assert entry.deleted.meta["flag"] == "hot"
    end

    test "serialize/1 with empty log produces well-formed structure" do
      log = CorrelationVector.new()
      map = CorrelationVector.serialize(log)
      assert map["entries"] == %{}
      assert map["pass_order"] == []
      assert map["enabled"] == true
    end

    test "disabled log serialize still includes enabled: false" do
      log = CorrelationVector.new(false)
      map = CorrelationVector.serialize(log)
      assert map["enabled"] == false
    end

    test "history for a fresh entry with no contributions returns empty list" do
      # Exercises the `base_contribs` path where entry exists but has no
      # contributions and is not deleted — history returns [].
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      # No contribute or delete calls — history must be empty list
      assert CorrelationVector.history(log, cv_id) == []
    end

    test "passthrough on a deleted CV raises RuntimeError" do
      # passthrough delegates to contribute, which guards against deleted entries.
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.delete(log, cv_id, "dce", "unreachable", %{})

      assert_raise RuntimeError, fn ->
        CorrelationVector.passthrough(log, cv_id, "some_checker")
      end
    end

    test "lineage for a merged node includes all parent entries" do
      # Exercises lineage when the entry has multiple parent_ids (from merge).
      # The result must contain entries for each parent, ordered oldest-first.
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log, %Origin{source: "src_a", location: "1:1"})
      {b, log} = CorrelationVector.create(log, %Origin{source: "src_b", location: "2:2"})
      {merged_id, log} = CorrelationVector.merge(log, [a, b])

      lineage = CorrelationVector.lineage(log, merged_id)
      lineage_ids = Enum.map(lineage, & &1.id)

      # Both parents and the merged node must appear; merged node is last
      assert merged_id in lineage_ids
      assert a in lineage_ids
      assert b in lineage_ids
      assert List.last(lineage_ids) == merged_id
    end

    test "descendants on a deeply nested chain" do
      # A → B → C → D: descendants(A) should include B, C, D.
      log = CorrelationVector.new()
      {a, log} = CorrelationVector.create(log)
      {b, log} = CorrelationVector.derive(log, a)
      {c, log} = CorrelationVector.derive(log, b)
      {d, log} = CorrelationVector.derive(log, c)

      desc = CorrelationVector.descendants(log, a) |> Enum.sort()
      assert desc == Enum.sort([b, c, d])
    end

    test "ancestors when parent entry is missing from log" do
      # If a CV entry lists parent_ids but the parent isn't in the log
      # (e.g., cross-log reference), ancestors still returns without crashing.
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)

      # Manually build an entry with a parent_id that doesn't exist in log
      alias CodingAdventures.CorrelationVector.Entry
      fake_entry = %Entry{
        id: cv_id,
        parent_ids: ["missing_parent.1"],
        origin: nil,
        contributions: [],
        deleted: nil
      }
      log = %{log | entries: Map.put(log.entries, cv_id, fake_entry)}

      # Should return the missing parent ID without crashing (it's not in entries,
      # so its own parent_ids are treated as [])
      ancs = CorrelationVector.ancestors(log, cv_id)
      assert "missing_parent.1" in ancs
    end

    test "meta with atom keys and atom value is stringified for serialization" do
      # Exercises stringify_keys (atom key → string key) and
      # stringify_value (atom value → string value) code paths.
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      # atom key + atom value in meta
      log = CorrelationVector.contribute(log, cv_id, "stage", "tagged", %{status: :active})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      entry = CorrelationVector.get(log2, cv_id)
      assert List.first(entry.contributions).meta["status"] == "active"
    end

    test "meta with list value is handled in serialization" do
      # Exercises the stringify_value list branch (line 837 in implementation).
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.contribute(log, cv_id, "stage", "tagged", %{"tags" => ["a", "b", "c"]})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      entry = CorrelationVector.get(log2, cv_id)
      assert List.first(entry.contributions).meta["tags"] == ["a", "b", "c"]
    end

    test "meta with nested map value is stringified recursively" do
      # Exercises the stringify_value map branch (line 836): nested maps with
      # atom keys inside the meta must also have their keys stringified.
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.contribute(log, cv_id, "stage", "tagged", %{info: %{count: 3}})

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      entry = CorrelationVector.get(log2, cv_id)
      info = List.first(entry.contributions).meta["info"]
      assert info["count"] == 3
    end

    test "serialize entry with nil origin produces nil origin in map" do
      # Exercises the serialize_origin(nil) clause.
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      # nil origin (synthetic entity)
      map = CorrelationVector.serialize(log)
      assert map["entries"][cv_id]["origin"] == nil
    end

    test "roundtrip with disabled log preserves enabled: false" do
      # Exercises from_json_string + deserialize when enabled is false.
      log = CorrelationVector.new(false)
      {_cv_id, log} = CorrelationVector.create(log)

      {:ok, json} = CorrelationVector.to_json_string(log)
      {:ok, log2} = CorrelationVector.from_json_string(json)

      assert log2.enabled == false
      assert log2.entries == %{}
    end

    test "delete then re-delete does not double-wrap the deletion record" do
      # Calling delete twice on the same CV ID — the second call should overwrite
      # (or simply update) the deletion record, and not crash.
      log = CorrelationVector.new()
      {cv_id, log} = CorrelationVector.create(log)
      log = CorrelationVector.delete(log, cv_id, "stage1", "first deletion", %{})
      log = CorrelationVector.delete(log, cv_id, "stage2", "second deletion", %{})

      entry = CorrelationVector.get(log, cv_id)
      # The second delete overwrites — source should be stage2
      assert entry.deleted.source == "stage2"
      assert entry.deleted.reason == "second deletion"
    end

    test "merge with empty parent list creates a synthetic CV entry" do
      # Merging with no parents is a valid edge case — the entry has no parent_ids.
      log = CorrelationVector.new()
      {merged_id, log} = CorrelationVector.merge(log, [])

      entry = CorrelationVector.get(log, merged_id)
      assert entry != nil
      assert entry.parent_ids == []
      assert String.starts_with?(merged_id, "00000000.")
    end
  end
end
