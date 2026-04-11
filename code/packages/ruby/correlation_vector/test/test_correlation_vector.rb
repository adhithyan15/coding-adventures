# frozen_string_literal: true

# ================================================================
# Correlation Vector Test Suite
# ================================================================
#
# These tests cover the full API surface of CodingAdventures::CorrelationVector.
# They are organized into 7 groups matching the spec requirements:
#
# 1. Root lifecycle    -- create, contribute, passthrough, delete, error-on-deleted
# 2. Derivation        -- child ID format, ancestors, descendants
# 3. Merging           -- 3-way merge, ancestor tracking
# 4. Deep ancestry     -- 4-level chain, nearest-first / oldest-first ordering
# 5. Disabled log      -- enabled: false, no storage, IDs still generated
# 6. Serialization     -- roundtrip JSON through serialize/deserialize
# 7. ID uniqueness     -- 1000 creates, no collisions
#
# Reading these tests is itself a tutorial on how to use the library.
# Each test method documents WHY we're asserting what we're asserting.
# ================================================================

require_relative "test_helper"

# Convenience aliases so tests read like documentation.
CV = CodingAdventures::CorrelationVector
CVLog = CV::CVLog

class CodingAdventuresCorrelationVectorTest < Minitest::Test
  # ================================================================
  # Group 1: Root Lifecycle
  # ================================================================
  #
  # The basic workflow: create -> contribute -> passthrough -> delete.
  # This group verifies the fundamental operations work correctly and
  # that error conditions (contributing to a deleted entry) are caught.
  # ================================================================

  def test_create_returns_a_string_id
    # The simplest possible test: create returns a String.
    log = CVLog.new
    cv_id = log.create(origin_string: "app.ts")
    assert_kind_of String, cv_id
    refute_empty cv_id
  end

  def test_create_with_origin_uses_hash_base
    # The first 8 characters of the ID are the SHA-256 hash prefix
    # of the origin string. This lets you visually group related entities.
    log = CVLog.new
    cv_id = log.create(origin_string: "my_file.rb")
    parts = cv_id.split(".")
    # Format: <8-hex-chars>.<N>
    assert_equal 2, parts.length
    assert_match(/\A[0-9a-f]{8}\z/, parts[0], "base must be 8 lowercase hex chars")
    assert_match(/\A\d+\z/, parts[1], "N must be a positive integer")
    assert cv_id.end_with?(".1"), "first create should end with .1"
  end

  def test_create_synthetic_uses_zero_base
    # Synthetic entities (no natural origin) get the "00000000" base.
    # This is the universal marker for "born from nothing."
    log = CVLog.new
    cv_id = log.create(synthetic: true)
    assert cv_id.start_with?("00000000."), "synthetic ID must start with 00000000"
  end

  def test_create_stores_entry_in_log
    # After create, the entry must be retrievable via get.
    log = CVLog.new
    cv_id = log.create(origin_string: "input.csv")
    entry = log.get(cv_id)
    refute_nil entry
    assert_equal cv_id, entry.cv_id
  end

  def test_create_entry_has_origin
    # The origin is stored in the entry for forensic purposes.
    log = CVLog.new
    cv_id = log.create(origin_string: "orders_table", synthetic: false)
    entry = log.get(cv_id)
    refute_nil entry.origin
    assert_equal "orders_table", entry.origin.string
    assert_equal false, entry.origin.synthetic
  end

  def test_create_entry_starts_with_empty_contributions
    # A fresh entity has no history yet -- contributions accumulate over time.
    log = CVLog.new
    cv_id = log.create(origin_string: "fresh.rb")
    entry = log.get(cv_id)
    assert_equal [], entry.contributions
  end

  def test_contribute_appends_to_history
    # Each contribute call appends to the entity's history in order.
    log = CVLog.new
    cv_id = log.create(origin_string: "source.ts")
    log.contribute(cv_id, source: "parser", tag: "parsed")
    log.contribute(cv_id, source: "scope_analysis", tag: "resolved")

    history = log.history(cv_id)
    assert_equal 2, history.length
    assert_equal "parser", history[0].source
    assert_equal "parsed", history[0].tag
    assert_equal "scope_analysis", history[1].source
    assert_equal "resolved", history[1].tag
  end

  def test_contribute_with_meta
    # Meta carries domain-specific detail about what happened.
    log = CVLog.new
    cv_id = log.create(origin_string: "src.ts")
    log.contribute(cv_id, source: "renamer", tag: "renamed",
      meta: {"from" => "count", "to" => "a"})
    contribution = log.history(cv_id).first
    assert_equal "count", contribution.meta["from"]
    assert_equal "a", contribution.meta["to"]
  end

  def test_contribute_updates_pass_order
    # pass_order tracks which stages have contributed to the log (globally).
    log = CVLog.new
    cv_id = log.create(origin_string: "file.rb")
    log.contribute(cv_id, source: "parser", tag: "created")
    log.contribute(cv_id, source: "resolver", tag: "resolved")
    assert_includes log.pass_order, "parser"
    assert_includes log.pass_order, "resolver"
  end

  def test_contribute_deduplicates_pass_order
    # If a stage contributes twice, it only appears once in pass_order.
    # "visited" not "visit count" is what we track.
    log = CVLog.new
    cv_id = log.create(origin_string: "file.rb")
    log.contribute(cv_id, source: "optimizer", tag: "pass1")
    log.contribute(cv_id, source: "optimizer", tag: "pass2")
    assert_equal 1, log.pass_order.count("optimizer")
  end

  def test_passthrough_records_source_in_pass_order
    # passthrough says "I saw this entity but changed nothing."
    # It's lighter than contribute: no contribution appended, just pass_order updated.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.rb")
    result = log.passthrough(cv_id, source: "type_checker")
    assert_equal cv_id, result  # passthrough returns the same ID
    assert_includes log.pass_order, "type_checker"
  end

  def test_passthrough_does_not_add_to_contributions
    # passthrough is the identity contribution -- it doesn't fill the history array.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.rb")
    log.passthrough(cv_id, source: "type_checker")
    assert_empty log.history(cv_id)
  end

  def test_passthrough_returns_cv_id
    # passthrough always returns the same cv_id it was given.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.rb")
    returned = log.passthrough(cv_id, source: "noop_stage")
    assert_equal cv_id, returned
  end

  def test_delete_marks_entry_as_deleted
    # delete adds a DeletionRecord but does NOT remove the entry.
    # The entry is permanently in the log for forensic purposes.
    log = CVLog.new
    cv_id = log.create(origin_string: "temporary.rb")
    log.delete(cv_id, by: "dead_code_eliminator")

    entry = log.get(cv_id)
    refute_nil entry
    refute_nil entry.deleted
    assert_equal "dead_code_eliminator", entry.deleted.by
  end

  def test_delete_does_not_remove_history
    # History survives deletion -- you can always see what happened
    # to the entity before it was deleted.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.rb")
    log.contribute(cv_id, source: "optimizer", tag: "optimized")
    log.delete(cv_id, by: "eliminator")

    history = log.history(cv_id)
    assert_equal 1, history.length
    assert_equal "optimizer", history[0].source
  end

  def test_contribute_raises_on_deleted_entry
    # Contributing to a deleted entity is a logical error.
    # The entity no longer exists in the pipeline.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.rb")
    log.delete(cv_id, by: "eliminator")

    assert_raises(RuntimeError) do
      log.contribute(cv_id, source: "later_stage", tag: "too_late")
    end
  end

  def test_contribute_raises_on_unknown_entry
    # Contributing to an ID that was never created is also an error.
    log = CVLog.new
    assert_raises(RuntimeError) do
      log.contribute("does_not_exist.1", source: "stage", tag: "tag")
    end
  end

  def test_passthrough_raises_on_deleted_entry
    # Even a passthrough is rejected on deleted entries.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.rb")
    log.delete(cv_id, by: "eliminator")

    assert_raises(RuntimeError) do
      log.passthrough(cv_id, source: "late_stage")
    end
  end

  def test_get_returns_nil_for_unknown_id
    # get is safe to call on unknown IDs -- returns nil, not an exception.
    log = CVLog.new
    assert_nil log.get("nonexistent.999")
  end

  def test_history_returns_empty_for_unknown_id
    # history is also safe to call on unknown IDs.
    log = CVLog.new
    assert_equal [], log.history("nonexistent.999")
  end

  # ================================================================
  # Group 2: Derivation
  # ================================================================
  #
  # derive creates a child entity from a parent. The child's ID
  # is the parent's ID with a new numeric suffix.
  #
  # Example use case: destructuring {a, b} = x creates two derived
  # entities, both with x's cv_id as prefix.
  # ================================================================

  def test_derive_id_has_parent_as_prefix
    # A derived entity's ID extends the parent's ID with ".M".
    # This means: "a3f1.1" -> "a3f1.1.2" (the .2 is the counter).
    log = CVLog.new
    parent_id = log.create(origin_string: "parent.ts")
    child_id = log.derive(parent_id, source: "splitter", tag: "split")

    assert child_id.start_with?("#{parent_id}."),
      "derived ID #{child_id.inspect} should start with #{parent_id.inspect}."
  end

  def test_derive_creates_entry_with_parent_reference
    # The child entry stores the parent's ID for ancestry queries.
    log = CVLog.new
    parent_id = log.create(origin_string: "parent.ts")
    child_id = log.derive(parent_id, source: "splitter", tag: "split")

    child_entry = log.get(child_id)
    refute_nil child_entry
    assert_equal parent_id, child_entry.parent_cv_id
  end

  def test_derive_adds_initial_contribution
    # The initial contribution from the derive call is recorded.
    log = CVLog.new
    parent_id = log.create(origin_string: "parent.ts")
    child_id = log.derive(parent_id, source: "splitter", tag: "derived_left",
      meta: {"position" => "left"})

    history = log.history(child_id)
    assert_equal 1, history.length
    assert_equal "splitter", history[0].source
    assert_equal "derived_left", history[0].tag
  end

  def test_derive_two_children_from_same_parent
    # Both children share the parent prefix but get unique suffixes.
    log = CVLog.new
    parent_id = log.create(origin_string: "source.ts")
    child_a = log.derive(parent_id, source: "splitter", tag: "left")
    child_b = log.derive(parent_id, source: "splitter", tag: "right")

    refute_equal child_a, child_b
    assert child_a.start_with?("#{parent_id}.")
    assert child_b.start_with?("#{parent_id}.")
  end

  def test_ancestors_returns_parent_for_derived_child
    # ancestors(child) includes the parent as the immediate ancestor.
    log = CVLog.new
    parent_id = log.create(origin_string: "source.ts")
    child_id = log.derive(parent_id, source: "splitter", tag: "split")

    ancestor_ids = log.ancestors(child_id)
    assert_includes ancestor_ids, parent_id
  end

  def test_ancestors_nearest_first_for_derived
    # ancestors returns nearest-first: immediate parent comes before grandparent.
    log = CVLog.new
    root_id = log.create(origin_string: "root.ts")
    child_id = log.derive(root_id, source: "stage1", tag: "derived")

    ancestor_ids = log.ancestors(child_id)
    assert_equal [root_id], ancestor_ids
  end

  def test_descendants_returns_children
    # descendants(parent) includes all children.
    log = CVLog.new
    parent_id = log.create(origin_string: "parent.ts")
    child_a = log.derive(parent_id, source: "splitter", tag: "left")
    child_b = log.derive(parent_id, source: "splitter", tag: "right")

    desc = log.descendants(parent_id)
    assert_includes desc, child_a
    assert_includes desc, child_b
  end

  def test_descendants_includes_indirect_descendants
    # A grandchild is also a descendant of the grandparent.
    log = CVLog.new
    root_id = log.create(origin_string: "root.ts")
    child_id = log.derive(root_id, source: "stage1", tag: "child")
    grandchild_id = log.derive(child_id, source: "stage2", tag: "grandchild")

    desc = log.descendants(root_id)
    assert_includes desc, child_id
    assert_includes desc, grandchild_id
  end

  def test_descendants_empty_for_leaf_entity
    # A leaf entity (never derived from) has no descendants.
    log = CVLog.new
    leaf_id = log.create(origin_string: "leaf.ts")
    assert_empty log.descendants(leaf_id)
  end

  def test_derive_raises_if_parent_not_found
    # You can't derive from a nonexistent parent.
    log = CVLog.new
    assert_raises(RuntimeError) do
      log.derive("does_not_exist.1", source: "stage", tag: "tag")
    end
  end

  def test_derive_raises_if_parent_deleted
    # You can't derive from a deleted entity (it's gone from the pipeline).
    log = CVLog.new
    parent_id = log.create(origin_string: "parent.ts")
    log.delete(parent_id, by: "eliminator")

    assert_raises(RuntimeError) do
      log.derive(parent_id, source: "stage", tag: "tag")
    end
  end

  # ================================================================
  # Group 3: Merging
  # ================================================================
  #
  # merge combines multiple entities into one. The merged entity
  # knows about all its parents. This models operations like:
  # - function inlining (call site + body -> merged expression)
  # - SQL JOIN (two rows -> one result row)
  # - neural network aggregation (multiple inputs -> one node)
  # ================================================================

  def test_merge_creates_entry_with_all_parents
    # The merged entry's merged_from array lists all parent IDs.
    log = CVLog.new
    a_id = log.create(origin_string: "source_a.ts")
    b_id = log.create(origin_string: "source_b.ts")
    c_id = log.create(origin_string: "source_c.ts")

    merged_id = log.merge([a_id, b_id, c_id], source: "join_stage", tag: "merged")
    entry = log.get(merged_id)

    refute_nil entry
    assert_equal [a_id, b_id, c_id].sort, entry.merged_from.sort
  end

  def test_merge_id_uses_hash_of_sorted_parents
    # The merged ID's base is SHA-256 of sorted parent IDs, 8 chars.
    # This is deterministic: same parents -> same base, regardless of order.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.create(origin_string: "b.ts")

    merged_id = log.merge([a_id, b_id], source: "merger", tag: "merged")
    parts = merged_id.split(".")
    # Base should be exactly 8 hex chars
    assert_match(/\A[0-9a-f]{8}\z/, parts[0], "merged base must be 8 hex chars")
  end

  def test_merge_ancestors_includes_all_parents
    # ancestors(merged) includes all parent IDs.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.create(origin_string: "b.ts")
    c_id = log.create(origin_string: "c.ts")

    merged_id = log.merge([a_id, b_id, c_id], source: "join", tag: "joined")
    ancestor_ids = log.ancestors(merged_id)

    assert_includes ancestor_ids, a_id
    assert_includes ancestor_ids, b_id
    assert_includes ancestor_ids, c_id
  end

  def test_merge_with_two_parents
    # Basic 2-way merge.
    log = CVLog.new
    left_id = log.create(origin_string: "left.ts")
    right_id = log.create(origin_string: "right.ts")

    merged_id = log.merge([left_id, right_id], source: "inline", tag: "inlined")
    refute_nil log.get(merged_id)
    assert_includes log.ancestors(merged_id), left_id
    assert_includes log.ancestors(merged_id), right_id
  end

  def test_merge_adds_initial_contribution
    # The initial contribution from the merge call is recorded.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.create(origin_string: "b.ts")

    merged_id = log.merge([a_id, b_id], source: "merger", tag: "merged",
      meta: {"strategy" => "join"})
    history = log.history(merged_id)
    assert_equal 1, history.length
    assert_equal "merger", history[0].source
    assert_equal "merged", history[0].tag
  end

  def test_merge_raises_if_parent_not_found
    # All parents must exist in the log.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")

    assert_raises(RuntimeError) do
      log.merge([a_id, "does_not_exist.1"], source: "merger", tag: "merged")
    end
  end

  # ================================================================
  # Group 4: Deep Ancestry Chain
  # ================================================================
  #
  # Verifies that ancestry and lineage work correctly for multi-level
  # derivation chains: A -> B -> C -> D.
  #
  # ancestors(D) should return [C, B, A] -- nearest first.
  # lineage(D) should return [A, B, C, D] -- oldest first.
  # ================================================================

  def test_deep_chain_ancestors_nearest_first
    # A -> B -> C -> D
    # ancestors(D) = [C, B, A] (immediate parent first)
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.derive(a_id, source: "stage1", tag: "step1")
    c_id = log.derive(b_id, source: "stage2", tag: "step2")
    d_id = log.derive(c_id, source: "stage3", tag: "step3")

    ancestor_ids = log.ancestors(d_id)

    # Nearest-first: C, B, A
    assert_equal c_id, ancestor_ids[0], "immediate parent should be first"
    assert_equal b_id, ancestor_ids[1]
    assert_equal a_id, ancestor_ids[2]
    assert_equal 3, ancestor_ids.length
  end

  def test_deep_chain_lineage_oldest_first
    # lineage(D) returns [A, B, C, D] -- oldest first, D at the end.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.derive(a_id, source: "stage1", tag: "step1")
    c_id = log.derive(b_id, source: "stage2", tag: "step2")
    d_id = log.derive(c_id, source: "stage3", tag: "step3")

    chain = log.lineage(d_id)

    assert_equal 4, chain.length
    assert_equal a_id, chain[0].cv_id, "oldest ancestor should be first"
    assert_equal b_id, chain[1].cv_id
    assert_equal c_id, chain[2].cv_id
    assert_equal d_id, chain[3].cv_id, "entity itself should be last"
  end

  def test_deep_chain_all_are_descendants
    # All of B, C, D are descendants of A.
    log = CVLog.new
    a_id = log.create(origin_string: "root.ts")
    b_id = log.derive(a_id, source: "s1", tag: "t1")
    c_id = log.derive(b_id, source: "s2", tag: "t2")
    d_id = log.derive(c_id, source: "s3", tag: "t3")

    desc = log.descendants(a_id)
    assert_includes desc, b_id
    assert_includes desc, c_id
    assert_includes desc, d_id
  end

  def test_ancestors_returns_empty_for_root
    # A root entity has no ancestors.
    log = CVLog.new
    root_id = log.create(origin_string: "root.ts")
    assert_empty log.ancestors(root_id)
  end

  def test_lineage_returns_single_entry_for_root
    # A root entity's lineage is just itself.
    log = CVLog.new
    root_id = log.create(origin_string: "root.ts")
    chain = log.lineage(root_id)
    assert_equal 1, chain.length
    assert_equal root_id, chain[0].cv_id
  end

  # ================================================================
  # Group 5: Disabled Log
  # ================================================================
  #
  # When enabled: false, all write operations succeed silently (no
  # exceptions), IDs are still generated and returned (the caller
  # needs them), but nothing is stored in the log.
  #
  # This allows production code to "turn off" tracing with zero
  # overhead, while still getting cv_ids attached to entities.
  # ================================================================

  def test_disabled_create_returns_id_without_storing
    # enabled: false -- create returns a cv_id but doesn't store it.
    log = CVLog.new(enabled: false)
    cv_id = log.create(origin_string: "file.ts")
    assert_kind_of String, cv_id
    refute_empty cv_id
    assert_nil log.get(cv_id)  # nothing stored
  end

  def test_disabled_create_returns_valid_id_format
    # The ID format is preserved even when disabled.
    log = CVLog.new(enabled: false)
    cv_id = log.create(origin_string: "source.ts")
    parts = cv_id.split(".")
    assert_equal 2, parts.length
    assert_match(/\A[0-9a-f]{8}\z/, parts[0])
    assert_match(/\A\d+\z/, parts[1])
  end

  def test_disabled_contribute_is_noop
    # contribute returns nil and does nothing when disabled.
    log = CVLog.new(enabled: false)
    result = log.contribute("any.1", source: "stage", tag: "tag")
    assert_nil result
    assert_empty log.pass_order
  end

  def test_disabled_derive_returns_id
    # derive still returns a child ID (the caller needs it),
    # but doesn't validate the parent or store anything.
    log = CVLog.new(enabled: false)
    parent_id = "fake_parent.1"  # doesn't need to exist when disabled
    child_id = log.derive(parent_id, source: "splitter", tag: "split")
    assert_kind_of String, child_id
    assert child_id.start_with?("#{parent_id}."),
      "derived ID should extend parent even when disabled"
    assert_nil log.get(child_id)
  end

  def test_disabled_merge_returns_id
    # merge returns a merged ID without storing anything.
    log = CVLog.new(enabled: false)
    merged_id = log.merge(["a.1", "b.2"], source: "merger", tag: "merged")
    assert_kind_of String, merged_id
    assert_nil log.get(merged_id)
  end

  def test_disabled_delete_is_noop
    # delete does nothing when disabled.
    log = CVLog.new(enabled: false)
    result = log.delete("fake.1", by: "eliminator")
    assert_nil result
  end

  def test_disabled_passthrough_returns_cv_id
    # passthrough returns the same cv_id without recording anything.
    log = CVLog.new(enabled: false)
    returned = log.passthrough("any.1", source: "type_checker")
    assert_equal "any.1", returned
    assert_empty log.pass_order
  end

  def test_disabled_get_returns_nil
    # Nothing is stored, so get always returns nil.
    log = CVLog.new(enabled: false)
    cv_id = log.create(origin_string: "file.ts")
    assert_nil log.get(cv_id)
  end

  def test_disabled_history_returns_empty
    # Nothing is stored, so history always returns [].
    log = CVLog.new(enabled: false)
    assert_equal [], log.history("any.1")
  end

  def test_disabled_ancestors_returns_empty
    log = CVLog.new(enabled: false)
    assert_equal [], log.ancestors("any.1")
  end

  def test_disabled_descendants_returns_empty
    log = CVLog.new(enabled: false)
    assert_equal [], log.descendants("any.1")
  end

  def test_disabled_lineage_returns_empty
    log = CVLog.new(enabled: false)
    assert_equal [], log.lineage("any.1")
  end

  # ================================================================
  # Group 6: Serialization Roundtrip
  # ================================================================
  #
  # Build a CVLog with diverse content (roots, derivations, merges,
  # deletions), serialize to JSON, deserialize back, and verify
  # that every piece of information survived intact.
  #
  # This tests the full JsonValue pipeline end-to-end.
  # ================================================================

  def test_serialize_returns_json_string
    log = CVLog.new
    log.create(origin_string: "test.ts")
    json = log.serialize
    assert_kind_of String, json
    # Valid JSON must start with { (we serialize as an object)
    assert json.start_with?("{"), "serialized form should be a JSON object"
  end

  def test_deserialize_restores_enabled_flag
    log = CVLog.new(enabled: true)
    restored = CVLog.deserialize(log.serialize)
    assert_equal true, restored.enabled

    log2 = CVLog.new(enabled: false)
    restored2 = CVLog.deserialize(log2.serialize)
    assert_equal false, restored2.enabled
  end

  def test_serialize_roundtrip_root_entry
    # A root entry survives the roundtrip with its origin intact.
    log = CVLog.new
    cv_id = log.create(origin_string: "source.ts")
    log.contribute(cv_id, source: "parser", tag: "created",
      meta: {"token" => "IDENTIFIER"})

    restored = CVLog.deserialize(log.serialize)
    entry = restored.get(cv_id)

    refute_nil entry
    assert_equal cv_id, entry.cv_id
    assert_equal "source.ts", entry.origin.string
    assert_equal 1, entry.contributions.length
    assert_equal "parser", entry.contributions[0].source
    assert_equal "created", entry.contributions[0].tag
  end

  def test_serialize_roundtrip_derived_entry
    # A derived entry survives the roundtrip with parent_cv_id intact.
    log = CVLog.new
    parent_id = log.create(origin_string: "parent.ts")
    child_id = log.derive(parent_id, source: "splitter", tag: "split")

    restored = CVLog.deserialize(log.serialize)
    child_entry = restored.get(child_id)

    refute_nil child_entry
    assert_equal parent_id, child_entry.parent_cv_id
  end

  def test_serialize_roundtrip_merged_entry
    # A merged entry survives with merged_from intact.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.create(origin_string: "b.ts")
    merged_id = log.merge([a_id, b_id], source: "merger", tag: "merged")

    restored = CVLog.deserialize(log.serialize)
    merged_entry = restored.get(merged_id)

    refute_nil merged_entry
    assert_equal [a_id, b_id].sort, merged_entry.merged_from.sort
  end

  def test_serialize_roundtrip_deleted_entry
    # A deleted entry survives with the deletion record intact.
    log = CVLog.new
    cv_id = log.create(origin_string: "entity.ts")
    log.delete(cv_id, by: "eliminator")

    restored = CVLog.deserialize(log.serialize)
    entry = restored.get(cv_id)

    refute_nil entry
    refute_nil entry.deleted
    assert_equal "eliminator", entry.deleted.by
  end

  def test_serialize_roundtrip_pass_order
    # The global pass_order survives the roundtrip.
    log = CVLog.new
    cv_id = log.create(origin_string: "e.ts")
    log.contribute(cv_id, source: "parser", tag: "created")
    log.contribute(cv_id, source: "resolver", tag: "resolved")

    restored = CVLog.deserialize(log.serialize)
    assert_includes restored.pass_order, "parser"
    assert_includes restored.pass_order, "resolver"
  end

  def test_serialize_roundtrip_counter
    # The counter survives -- restored log continues generating unique IDs.
    log = CVLog.new
    3.times { |i| log.create(origin_string: "file#{i}.ts") }
    json = log.serialize

    restored = CVLog.deserialize(json)
    # Creating in the restored log should get a counter > 3
    new_id = restored.create(origin_string: "new.ts")
    parts = new_id.split(".")
    assert parts.last.to_i > 3,
      "restored counter should continue from where we left off, got #{new_id}"
  end

  def test_serialize_complex_log
    # A complex log with everything: roots, derivations, merges, deletions.
    log = CVLog.new
    a_id = log.create(origin_string: "a.ts")
    b_id = log.create(origin_string: "b.ts")
    log.contribute(a_id, source: "parser", tag: "parsed")
    log.contribute(b_id, source: "parser", tag: "parsed")
    child_id = log.derive(a_id, source: "splitter", tag: "split")
    merged_id = log.merge([a_id, b_id], source: "merger", tag: "joined")
    log.delete(b_id, by: "eliminator")
    log.passthrough(a_id, source: "type_checker")

    restored = CVLog.deserialize(log.serialize)

    # All entries survived
    refute_nil restored.get(a_id)
    refute_nil restored.get(b_id)
    refute_nil restored.get(child_id)
    refute_nil restored.get(merged_id)

    # Deletion survived
    assert restored.get(b_id).deleted
    assert_equal "eliminator", restored.get(b_id).deleted.by

    # Parent linkage survived
    assert_equal a_id, restored.get(child_id).parent_cv_id
    assert_includes restored.get(merged_id).merged_from, a_id
    assert_includes restored.get(merged_id).merged_from, b_id
  end

  # ================================================================
  # Group 7: ID Uniqueness
  # ================================================================
  #
  # Creates many entities and verifies no two get the same ID.
  # This tests the counter-based uniqueness guarantee.
  # ================================================================

  def test_id_uniqueness_1000_creates_same_origin
    # 1000 entities with the same origin string -- all IDs must be unique.
    # The counter prevents collision even when the hash base is identical.
    log = CVLog.new
    ids = Set.new
    1000.times do
      id = log.create(origin_string: "same_origin.ts")
      assert ids.add?(id), "Collision detected: #{id.inspect} was already seen"
    end
    assert_equal 1000, ids.size
  end

  def test_id_uniqueness_mixed_origins
    # 1000 entities with varied origins -- no collisions.
    log = CVLog.new
    ids = Set.new
    1000.times do |i|
      id = log.create(origin_string: "file_#{i}.ts")
      assert ids.add?(id), "Collision detected: #{id.inspect} was already seen"
    end
    assert_equal 1000, ids.size
  end

  def test_id_uniqueness_mix_of_creates_and_derives
    # Mix of roots and derivatives -- all IDs unique across the board.
    log = CVLog.new
    ids = Set.new

    100.times do |i|
      root_id = log.create(origin_string: "file_#{i}.ts")
      ids.add(root_id)
      5.times do
        child_id = log.derive(root_id, source: "splitter", tag: "child")
        assert ids.add?(child_id), "Collision: #{child_id.inspect}"
      end
    end

    # 100 roots + 500 children = 600 unique IDs
    assert_equal 600, ids.size
  end

  def test_id_uniqueness_synthetic_entities
    # Synthetic entities share the "00000000" base but get unique counters.
    log = CVLog.new
    ids = Set.new
    500.times do
      id = log.create(synthetic: true)
      assert ids.add?(id), "Synthetic ID collision: #{id.inspect}"
    end
    assert_equal 500, ids.size
  end

  # ================================================================
  # Additional edge case tests
  # ================================================================

  def test_same_origin_produces_same_base
    # Two entities with the same origin string get the same base prefix.
    # The counter differentiates them (N is unique per log).
    log = CVLog.new
    id1 = log.create(origin_string: "shared.ts")
    id2 = log.create(origin_string: "shared.ts")

    base1 = id1.split(".").first
    base2 = id2.split(".").first
    assert_equal base1, base2, "same origin should produce same base"
    refute_equal id1, id2, "but IDs must be unique via different N"
  end

  def test_different_origins_produce_different_bases_usually
    # Different origin strings (usually) produce different bases.
    # SHA-256 makes collisions astronomically unlikely.
    log = CVLog.new
    id1 = log.create(origin_string: "file_a.ts")
    id2 = log.create(origin_string: "file_b.ts")

    base1 = id1.split(".").first
    base2 = id2.split(".").first
    # This could theoretically fail with a hash collision (p ~= 2^-32),
    # but in practice it won't.
    refute_equal base1, base2
  end

  def test_merge_deterministic_id
    # Two merges of the same parents (same order) get the same base.
    # This is the determinism guarantee: reproducible builds get reproducible IDs.

    # We can't directly test without actually creating these entries.
    # Instead, verify that two fresh logs produce the same merged base.
    log1 = CVLog.new
    parent1 = log1.create(origin_string: "p1")
    parent2 = log1.create(origin_string: "p2")
    merged1 = log1.merge([parent1, parent2], source: "m", tag: "t")

    # In a second fresh log, same operations -- but different N values
    # because the counter resets. The base (from hash) will be the same.
    log2 = CVLog.new
    parent3 = log2.create(origin_string: "p1")
    parent4 = log2.create(origin_string: "p2")
    merged2 = log2.merge([parent3, parent4], source: "m", tag: "t")

    # Both logs should produce the same IDs because same origin strings
    # -> same SHA-256 bases -> same N values from fresh counters.
    assert_equal merged1, merged2
  end

  def test_pass_order_ordering
    # pass_order records sources in first-appearance order.
    log = CVLog.new
    id1 = log.create(origin_string: "e1.ts")
    id2 = log.create(origin_string: "e2.ts")

    log.contribute(id1, source: "first_stage", tag: "t1")
    log.contribute(id2, source: "second_stage", tag: "t2")
    log.contribute(id1, source: "first_stage", tag: "t3")  # duplicate, not re-added

    assert_equal ["first_stage", "second_stage"], log.pass_order
  end

  def test_entry_pass_order_deduplication
    # The entry-level pass_order is also deduplicated.
    log = CVLog.new
    cv_id = log.create(origin_string: "e.ts")
    log.contribute(cv_id, source: "optimizer", tag: "pass1")
    log.contribute(cv_id, source: "optimizer", tag: "pass2")
    log.contribute(cv_id, source: "optimizer", tag: "pass3")

    entry = log.get(cv_id)
    assert_equal ["optimizer"], entry.pass_order
  end
end
