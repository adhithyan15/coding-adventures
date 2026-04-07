"""Tests for the correlation-vector package.

Coverage groups
---------------
1. Root lifecycle  — create, contribute, passthrough, delete, ValueError on deleted,
                     pass_order deduplication
2. Derivation      — child IDs, parent_ids, ancestors, descendants, chains
3. Merging         — 3-way merge, parent_ids, ancestors
4. Deep ancestry   — A→B→C→D chain, ancestors (nearest first), lineage (oldest first)
5. Disabled log    — returns IDs, stores nothing, get=None, history/ancestors=[]
6. Serialisation   — to_json_string → from_json_string roundtrip
7. ID uniqueness   — 10,000 creates with same origin → all unique

These tests are written in plain ``pytest`` style with descriptive names so
that the failure message alone tells you which contract was violated.
"""

from __future__ import annotations

import re

import pytest

from coding_adventures_correlation_vector import (
    CVEntry,
    CVLog,
    Contribution,
    DeletionRecord,
    Origin,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_CV_ID_PATTERN = re.compile(r"^[0-9a-f]{8}(\.\d+)+$")


def assert_valid_id(cv_id: str) -> None:
    """Assert that *cv_id* matches the ``base.N[.M…]`` format."""
    assert _CV_ID_PATTERN.match(cv_id), (
        f"CV ID '{cv_id}' does not match expected pattern 'hex8.N[.M…]'"
    )


def make_origin(source: str = "test.ts", location: str = "1:0") -> Origin:
    """Convenience factory for test Origins."""
    return Origin(source=source, location=location)


# ===========================================================================
# Group 1 — Root lifecycle
# ===========================================================================


class TestRootLifecycle:
    """Tests covering: create, contribute, passthrough, delete, errors."""

    def test_create_with_origin_returns_valid_id(self) -> None:
        """create(origin) must return an ID matching the base.N pattern."""
        log = CVLog()
        cv_id = log.create(make_origin("app.ts", "5:12"))
        assert_valid_id(cv_id)

    def test_create_without_origin_uses_synthetic_base(self) -> None:
        """create() without an origin must use the '00000000' base."""
        log = CVLog()
        cv_id = log.create()
        assert cv_id.startswith("00000000.")

    def test_create_stores_entry(self) -> None:
        """Created CV must be retrievable via get()."""
        log = CVLog()
        cv_id = log.create(make_origin())
        entry = log.get(cv_id)
        assert entry is not None
        assert entry.id == cv_id

    def test_create_entry_has_correct_origin(self) -> None:
        """The stored entry must carry the exact origin passed in."""
        log = CVLog()
        origin = make_origin("db.sqlite", "row:42")
        cv_id = log.create(origin)
        assert log.get(cv_id).origin == origin  # type: ignore[union-attr]

    def test_create_entry_has_no_parents(self) -> None:
        """Root CVs must have an empty parent_ids list."""
        log = CVLog()
        cv_id = log.create(make_origin())
        assert log.get(cv_id).parent_ids == []  # type: ignore[union-attr]

    def test_create_sequence_increments_per_base(self) -> None:
        """Two creates with the same origin must produce .1 then .2."""
        log = CVLog()
        origin = make_origin("file.ts", "0:0")
        id1 = log.create(origin)
        id2 = log.create(origin)
        assert id1.endswith(".1")
        assert id2.endswith(".2")
        # Same base
        assert id1.split(".")[0] == id2.split(".")[0]

    def test_create_different_origins_different_bases(self) -> None:
        """Origins with different content must produce different base segments."""
        log = CVLog()
        id1 = log.create(make_origin("file_a.ts", "1:0"))
        id2 = log.create(make_origin("file_b.ts", "1:0"))
        assert id1.split(".")[0] != id2.split(".")[0]

    def test_contribute_appends_to_history(self) -> None:
        """contribute() must add a Contribution to the entry's history."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "parser", "created", {"token": "IDENT"})
        hist = log.history(cv_id)
        assert len(hist) == 1
        assert hist[0].source == "parser"
        assert hist[0].tag == "created"
        assert hist[0].meta == {"token": "IDENT"}

    def test_contribute_multiple_in_order(self) -> None:
        """Multiple contributions must appear in call order."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "stage_a", "tag_a")
        log.contribute(cv_id, "stage_b", "tag_b")
        log.contribute(cv_id, "stage_c", "tag_c")
        hist = log.history(cv_id)
        assert [c.tag for c in hist] == ["tag_a", "tag_b", "tag_c"]

    def test_contribute_default_meta_is_empty_dict(self) -> None:
        """Calling contribute without meta must store an empty dict."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "stage", "tag")
        assert log.history(cv_id)[0].meta == {}

    def test_passthrough_recorded_in_history(self) -> None:
        """passthrough() must appear in the history with tag 'passthrough'."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.passthrough(cv_id, "type_checker")
        hist = log.history(cv_id)
        assert len(hist) == 1
        assert hist[0].source == "type_checker"
        assert hist[0].tag == "passthrough"

    def test_delete_sets_deletion_record(self) -> None:
        """delete() must populate the deleted field of the entry."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.delete(cv_id, "dce", "unreachable", {"entry": "main"})
        entry = log.get(cv_id)
        assert entry is not None
        assert entry.deleted is not None
        assert entry.deleted.source == "dce"
        assert entry.deleted.reason == "unreachable"
        assert entry.deleted.meta == {"entry": "main"}

    def test_contribute_to_deleted_entry_raises_value_error(self) -> None:
        """contribute() on a deleted CV must raise ValueError."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.delete(cv_id, "dce", "unreachable")
        with pytest.raises(ValueError, match="deleted"):
            log.contribute(cv_id, "renamer", "renamed")

    def test_pass_order_deduplication(self) -> None:
        """The same source appearing multiple times in pass_order only once."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "parser", "created")
        log.contribute(cv_id, "parser", "annotated")  # same source again
        log.contribute(cv_id, "renamer", "renamed")
        assert log.pass_order.count("parser") == 1
        assert "renamer" in log.pass_order

    def test_pass_order_reflects_insertion_order(self) -> None:
        """pass_order must list sources in the order they first appeared."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "alpha", "t1")
        log.contribute(cv_id, "beta", "t2")
        log.contribute(cv_id, "alpha", "t3")  # duplicate, must NOT re-append
        log.contribute(cv_id, "gamma", "t4")
        assert log.pass_order == ["alpha", "beta", "gamma"]

    def test_get_unknown_id_returns_none(self) -> None:
        """get() on a non-existent ID must return None."""
        log = CVLog()
        assert log.get("deadbeef.99") is None

    def test_history_unknown_id_returns_empty_list(self) -> None:
        """history() on a non-existent ID must return []."""
        log = CVLog()
        assert log.history("deadbeef.99") == []

    def test_delete_default_meta_is_empty_dict(self) -> None:
        """Calling delete without meta must store an empty dict."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.delete(cv_id, "dce", "reason")
        assert log.get(cv_id).deleted.meta == {}  # type: ignore[union-attr]

    def test_passthrough_deduplicates_pass_order(self) -> None:
        """passthrough adds to pass_order only if source not already there."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.passthrough(cv_id, "checker")
        log.passthrough(cv_id, "checker")  # duplicate
        assert log.pass_order.count("checker") == 1

    def test_delete_deduplicates_pass_order(self) -> None:
        """delete adds source to pass_order only if not already present."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "dce", "scanned")
        log.delete(cv_id, "dce", "removed")  # same source
        assert log.pass_order.count("dce") == 1


# ===========================================================================
# Group 2 — Derivation
# ===========================================================================


class TestDerivation:
    """Tests covering: derive, parent IDs, child ID format, ancestors,
    descendants."""

    def test_derive_returns_valid_id(self) -> None:
        """derive() must return a valid CV ID."""
        log = CVLog()
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert_valid_id(child)

    def test_derive_child_id_is_parent_dot_one(self) -> None:
        """First child of a parent must end with parent_id + '.1'."""
        log = CVLog()
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert child == f"{parent}.1"

    def test_derive_second_child_id_is_parent_dot_two(self) -> None:
        """Second child of the same parent must end with parent_id + '.2'."""
        log = CVLog()
        parent = log.create(make_origin())
        _ = log.derive(parent)
        child2 = log.derive(parent)
        assert child2 == f"{parent}.2"

    def test_derive_entry_has_parent_in_parent_ids(self) -> None:
        """The derived entry's parent_ids must contain the parent CV ID."""
        log = CVLog()
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert log.get(child).parent_ids == [parent]  # type: ignore[union-attr]

    def test_ancestors_of_child_includes_parent(self) -> None:
        """ancestors(child) must include parent."""
        log = CVLog()
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert parent in log.ancestors(child)

    def test_ancestors_of_root_is_empty(self) -> None:
        """A root CV has no ancestors."""
        log = CVLog()
        cv_id = log.create(make_origin())
        assert log.ancestors(cv_id) == []

    def test_descendants_of_parent_includes_both_children(self) -> None:
        """descendants(parent) must include both derived children."""
        log = CVLog()
        parent = log.create(make_origin())
        child1 = log.derive(parent)
        child2 = log.derive(parent)
        desc = log.descendants(parent)
        assert child1 in desc
        assert child2 in desc

    def test_descendants_of_leaf_is_empty(self) -> None:
        """A leaf CV (no children) has no descendants."""
        log = CVLog()
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert log.descendants(child) == []

    def test_derive_with_origin_stored(self) -> None:
        """derive(parent, origin=...) must store the origin on the new entry."""
        log = CVLog()
        parent = log.create(make_origin())
        new_origin = make_origin("splitter", "col:0-5")
        child = log.derive(parent, origin=new_origin)
        assert log.get(child).origin == new_origin  # type: ignore[union-attr]

    def test_derive_chain_parent_ids_each_level(self) -> None:
        """In a A→B→C chain, B.parent_ids=[A] and C.parent_ids=[B]."""
        log = CVLog()
        a = log.create(make_origin())
        b = log.derive(a)
        c = log.derive(b)
        assert log.get(b).parent_ids == [a]  # type: ignore[union-attr]
        assert log.get(c).parent_ids == [b]  # type: ignore[union-attr]

    def test_descendants_includes_grandchildren(self) -> None:
        """descendants(root) must include both children and grandchildren."""
        log = CVLog()
        root = log.create(make_origin())
        child = log.derive(root)
        grandchild = log.derive(child)
        desc = log.descendants(root)
        assert child in desc
        assert grandchild in desc


# ===========================================================================
# Group 3 — Merging
# ===========================================================================


class TestMerging:
    """Tests covering: merge, multi-parent IDs, ancestors of merged CV."""

    def test_merge_three_way_returns_valid_id(self) -> None:
        """merge([a, b, c]) must return a valid CV ID."""
        log = CVLog()
        a = log.create(make_origin("a.ts", "1:0"))
        b = log.create(make_origin("b.ts", "1:0"))
        c = log.create(make_origin("c.ts", "1:0"))
        merged = log.merge([a, b, c])
        assert_valid_id(merged)

    def test_merge_parent_ids_contains_all_parents(self) -> None:
        """merged entry's parent_ids must list all three parents."""
        log = CVLog()
        a = log.create(make_origin("a.ts", "1:0"))
        b = log.create(make_origin("b.ts", "1:0"))
        c = log.create(make_origin("c.ts", "1:0"))
        merged = log.merge([a, b, c])
        entry = log.get(merged)
        assert entry is not None
        assert set(entry.parent_ids) == {a, b, c}

    def test_merge_ancestors_include_all_parents(self) -> None:
        """ancestors(merged) must include all three parent CV IDs."""
        log = CVLog()
        a = log.create(make_origin("a.ts", "1:0"))
        b = log.create(make_origin("b.ts", "1:0"))
        c = log.create(make_origin("c.ts", "1:0"))
        merged = log.merge([a, b, c])
        anc = set(log.ancestors(merged))
        assert {a, b, c}.issubset(anc)

    def test_merge_without_origin_uses_synthetic_base(self) -> None:
        """merge without origin must use the '00000000' base."""
        log = CVLog()
        a = log.create(make_origin())
        b = log.create(make_origin())
        merged = log.merge([a, b])
        assert merged.startswith("00000000.")

    def test_merge_with_origin_uses_origin_base(self) -> None:
        """merge with an explicit origin must use that origin's base."""
        log = CVLog()
        a = log.create(make_origin("x.ts", "0:0"))
        b = log.create(make_origin("y.ts", "0:0"))
        join_origin = make_origin("join", "x.id=y.id")
        merged = log.merge([a, b], origin=join_origin)
        # The base must NOT be 00000000 (it was computed from join_origin).
        assert not merged.startswith("00000000.")

    def test_merge_entry_stored(self) -> None:
        """The merged entry must be retrievable via get()."""
        log = CVLog()
        a = log.create(make_origin())
        b = log.create(make_origin())
        merged = log.merge([a, b])
        assert log.get(merged) is not None


# ===========================================================================
# Group 4 — Deep ancestry
# ===========================================================================


class TestDeepAncestry:
    """Tests covering: A→B→C→D chain, ancestors order, lineage order."""

    def _build_chain(self) -> tuple[CVLog, str, str, str, str]:
        """Build a 4-level chain A→B→C→D and return (log, A, B, C, D)."""
        log = CVLog()
        a = log.create(make_origin("root.ts", "0:0"))
        b = log.derive(a)
        c = log.derive(b)
        d = log.derive(c)
        return log, a, b, c, d

    def test_ancestors_of_d_is_c_b_a_nearest_first(self) -> None:
        """ancestors(D) must be [C, B, A] (nearest ancestor first)."""
        log, a, b, c, d = self._build_chain()
        assert log.ancestors(d) == [c, b, a]

    def test_ancestors_of_c_is_b_a(self) -> None:
        """ancestors(C) must be [B, A]."""
        log, a, b, c, d = self._build_chain()
        assert log.ancestors(c) == [b, a]

    def test_lineage_of_d_has_four_entries(self) -> None:
        """lineage(D) must return 4 entries (A, B, C, D)."""
        log, a, b, c, d = self._build_chain()
        lin = log.lineage(d)
        assert len(lin) == 4

    def test_lineage_oldest_ancestor_first(self) -> None:
        """lineage(D) must be ordered [A, B, C, D] — oldest first."""
        log, a, b, c, d = self._build_chain()
        lin = log.lineage(d)
        assert [e.id for e in lin] == [a, b, c, d]

    def test_lineage_self_is_last(self) -> None:
        """lineage(D)[-1] must be D's own entry."""
        log, a, b, c, d = self._build_chain()
        lin = log.lineage(d)
        assert lin[-1].id == d

    def test_descendants_of_a_includes_b_c_d(self) -> None:
        """descendants(A) in the chain must include B, C, and D."""
        log, a, b, c, d = self._build_chain()
        desc = set(log.descendants(a))
        assert {b, c, d}.issubset(desc)

    def test_lineage_of_root_is_single_entry(self) -> None:
        """lineage(A) for a root must be just [A]."""
        log, a, b, c, d = self._build_chain()
        lin = log.lineage(a)
        assert len(lin) == 1
        assert lin[0].id == a

    def test_lineage_unknown_id_returns_empty(self) -> None:
        """lineage() on an unknown ID must return []."""
        log = CVLog()
        assert log.lineage("deadbeef.99") == []


# ===========================================================================
# Group 5 — Disabled log
# ===========================================================================


class TestDisabledLog:
    """Tests covering: enabled=False — IDs returned, nothing stored."""

    def test_disabled_create_returns_id(self) -> None:
        """create() in a disabled log must still return a valid CV ID."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        assert_valid_id(cv_id)

    def test_disabled_create_without_origin_returns_id(self) -> None:
        """create() without origin in a disabled log must return synthetic ID."""
        log = CVLog(enabled=False)
        cv_id = log.create()
        assert cv_id.startswith("00000000.")

    def test_disabled_create_stores_nothing(self) -> None:
        """get() in a disabled log must return None for created IDs."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        assert log.get(cv_id) is None

    def test_disabled_contribute_is_noop(self) -> None:
        """contribute() in a disabled log must not raise and store nothing."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "parser", "created")  # must not raise

    def test_disabled_history_returns_empty_list(self) -> None:
        """history() in a disabled log must always return []."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "parser", "created")
        assert log.history(cv_id) == []

    def test_disabled_derive_returns_id(self) -> None:
        """derive() in a disabled log must still return a valid CV ID."""
        log = CVLog(enabled=False)
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert child == f"{parent}.1"

    def test_disabled_merge_returns_id(self) -> None:
        """merge() in a disabled log must still return a valid CV ID."""
        log = CVLog(enabled=False)
        a = log.create(make_origin("a.ts", "0:0"))
        b = log.create(make_origin("b.ts", "0:0"))
        merged = log.merge([a, b])
        assert_valid_id(merged)

    def test_disabled_delete_is_noop(self) -> None:
        """delete() in a disabled log must not raise."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        log.delete(cv_id, "dce", "unreachable")  # must not raise

    def test_disabled_passthrough_is_noop(self) -> None:
        """passthrough() in a disabled log must not raise."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        log.passthrough(cv_id, "checker")  # must not raise

    def test_disabled_ancestors_returns_empty_list(self) -> None:
        """ancestors() in a disabled log must return []."""
        log = CVLog(enabled=False)
        parent = log.create(make_origin())
        child = log.derive(parent)
        assert log.ancestors(child) == []

    def test_disabled_descendants_returns_empty_list(self) -> None:
        """descendants() in a disabled log must return []."""
        log = CVLog(enabled=False)
        parent = log.create(make_origin())
        _ = log.derive(parent)
        assert log.descendants(parent) == []

    def test_disabled_lineage_returns_empty_list(self) -> None:
        """lineage() in a disabled log must return []."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        assert log.lineage(cv_id) == []

    def test_disabled_pass_order_stays_empty(self) -> None:
        """pass_order must stay empty when the log is disabled."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        log.contribute(cv_id, "parser", "created")
        log.passthrough(cv_id, "checker")
        assert log.pass_order == []


# ===========================================================================
# Group 6 — Serialisation roundtrip
# ===========================================================================


class TestSerialisationRoundtrip:
    """Tests covering: to_json_string → from_json_string identity."""

    def _build_complex_log(self) -> CVLog:
        """Build a CVLog with roots, derivations, merges, and deletions."""
        log = CVLog()
        # Root 1 — with origin
        root1 = log.create(Origin("app.ts", "1:0", timestamp="2024-01-01T00:00:00Z"))
        log.contribute(root1, "parser", "created", {"token": "IDENT"})
        log.contribute(root1, "scope", "resolved", {"binding": "local"})
        log.passthrough(root1, "type_checker")
        # Root 2 — synthetic
        root2 = log.create()
        log.contribute(root2, "parser", "created")
        # Derived from root1
        child1 = log.derive(root1)
        log.contribute(child1, "renamer", "renamed", {"from": "x", "to": "a"})
        child2 = log.derive(root1)
        # Merged
        merged = log.merge([root2, child1], origin=Origin("join", "r2+c1"))
        log.contribute(merged, "merger", "joined")
        # Deleted
        log.delete(root2, "dce", "unreachable", {"entry": "main"})
        return log

    def test_serialize_returns_dict(self) -> None:
        """serialize() must return a plain dict."""
        log = self._build_complex_log()
        data = log.serialize()
        assert isinstance(data, dict)

    def test_serialize_has_entries_key(self) -> None:
        """Serialised dict must have an 'entries' key."""
        log = self._build_complex_log()
        data = log.serialize()
        assert "entries" in data

    def test_serialize_has_pass_order_key(self) -> None:
        """Serialised dict must have a 'pass_order' key."""
        log = self._build_complex_log()
        data = log.serialize()
        assert "pass_order" in data

    def test_serialize_has_enabled_key(self) -> None:
        """Serialised dict must have an 'enabled' key."""
        log = self._build_complex_log()
        data = log.serialize()
        assert "enabled" in data

    def test_to_json_string_returns_string(self) -> None:
        """to_json_string() must return a str."""
        log = self._build_complex_log()
        s = log.to_json_string()
        assert isinstance(s, str)

    def test_from_json_string_returns_cvlog(self) -> None:
        """from_json_string() must return a CVLog instance."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        assert isinstance(restored, CVLog)

    def test_roundtrip_entries_count_matches(self) -> None:
        """After roundtrip the number of entries must be identical."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        assert len(restored._entries) == len(log._entries)

    def test_roundtrip_pass_order_matches(self) -> None:
        """After roundtrip pass_order must be identical."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        assert restored.pass_order == log.pass_order

    def test_roundtrip_enabled_flag_preserved(self) -> None:
        """After roundtrip the enabled flag must be preserved."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        assert restored.enabled == log.enabled

    def test_roundtrip_each_entry_id_matches(self) -> None:
        """After roundtrip every CV ID must appear in the restored log."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        for cv_id in log._entries:
            assert restored.get(cv_id) is not None

    def test_roundtrip_contributions_preserved(self) -> None:
        """After roundtrip the contribution history of each entry must match."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        for cv_id, orig_entry in log._entries.items():
            rest_entry = restored.get(cv_id)
            assert rest_entry is not None
            assert len(rest_entry.contributions) == len(orig_entry.contributions)
            for oc, rc in zip(orig_entry.contributions, rest_entry.contributions):
                assert oc.source == rc.source
                assert oc.tag == rc.tag
                assert oc.meta == rc.meta

    def test_roundtrip_parent_ids_preserved(self) -> None:
        """After roundtrip parent_ids of each entry must match."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        for cv_id, orig_entry in log._entries.items():
            rest_entry = restored.get(cv_id)
            assert rest_entry is not None
            assert rest_entry.parent_ids == orig_entry.parent_ids

    def test_roundtrip_origin_preserved(self) -> None:
        """After roundtrip the origin field must be equivalent."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        for cv_id, orig_entry in log._entries.items():
            rest_entry = restored.get(cv_id)
            assert rest_entry is not None
            if orig_entry.origin is None:
                assert rest_entry.origin is None
            else:
                assert rest_entry.origin is not None
                assert rest_entry.origin.source == orig_entry.origin.source
                assert rest_entry.origin.location == orig_entry.origin.location
                assert rest_entry.origin.timestamp == orig_entry.origin.timestamp

    def test_roundtrip_deletion_preserved(self) -> None:
        """After roundtrip deleted entries must remain deleted."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        for cv_id, orig_entry in log._entries.items():
            rest_entry = restored.get(cv_id)
            assert rest_entry is not None
            if orig_entry.deleted is None:
                assert rest_entry.deleted is None
            else:
                assert rest_entry.deleted is not None
                assert rest_entry.deleted.source == orig_entry.deleted.source
                assert rest_entry.deleted.reason == orig_entry.deleted.reason

    def test_roundtrip_counters_allow_new_creates(self) -> None:
        """After roundtrip, calling create must not collide with existing IDs."""
        log = self._build_complex_log()
        restored = CVLog.from_json_string(log.to_json_string())
        existing_ids = set(restored._entries.keys())
        new_id = restored.create(make_origin())
        assert new_id not in existing_ids

    def test_deserialize_enabled_false_preserved(self) -> None:
        """Deserialising a disabled log must produce an enabled=False CVLog."""
        log = CVLog(enabled=False)
        cv_id = log.create(make_origin())
        restored = CVLog.from_json_string(log.to_json_string())
        assert restored.enabled is False


# ===========================================================================
# Group 7 — ID uniqueness
# ===========================================================================


class TestIdUniqueness:
    """Tests covering: 10,000 creates — all IDs unique."""

    def test_ten_thousand_creates_same_origin_all_unique(self) -> None:
        """Creating 10,000 CVs with the same origin must produce unique IDs."""
        log = CVLog()
        origin = make_origin("batch.ts", "0:0")
        ids = {log.create(origin) for _ in range(10_000)}
        assert len(ids) == 10_000, "Collision detected in 10,000 creates"

    def test_ten_thousand_creates_mixed_origins_all_unique(self) -> None:
        """Creating 10,000 CVs with mixed origins must produce unique IDs."""
        log = CVLog()
        ids = set()
        for i in range(10_000):
            origin = make_origin(f"file_{i % 100}.ts", f"{i}:0")
            ids.add(log.create(origin))
        assert len(ids) == 10_000, "Collision detected across mixed origins"

    def test_ten_thousand_derives_all_unique(self) -> None:
        """Deriving 10,000 children from one parent must produce unique IDs."""
        log = CVLog()
        parent = log.create(make_origin())
        ids = {log.derive(parent) for _ in range(10_000)}
        assert len(ids) == 10_000, "Collision detected in 10,000 derives"


# ===========================================================================
# Additional edge-case tests (boosts coverage above 95%)
# ===========================================================================


class TestEdgeCases:
    """Additional edge-case tests for full coverage."""

    def test_contribute_to_unknown_id_silently_ignored(self) -> None:
        """contribute() on a non-existent ID must not raise."""
        log = CVLog()
        # No entry for "deadbeef.1" — contribute should just do nothing.
        log.contribute("deadbeef.1", "parser", "created")  # must not raise

    def test_passthrough_to_unknown_id_silently_ignored(self) -> None:
        """passthrough() on a non-existent ID must not raise."""
        log = CVLog()
        log.passthrough("deadbeef.1", "checker")  # must not raise

    def test_delete_to_unknown_id_silently_ignored(self) -> None:
        """delete() on a non-existent ID must not raise."""
        log = CVLog()
        log.delete("deadbeef.1", "dce", "unknown")  # must not raise

    def test_ancestors_unknown_id_returns_empty(self) -> None:
        """ancestors() on a non-existent ID must return []."""
        log = CVLog()
        assert log.ancestors("deadbeef.1") == []

    def test_descendants_unknown_id_returns_empty(self) -> None:
        """descendants() on a non-existent ID must return []."""
        log = CVLog()
        assert log.descendants("deadbeef.1") == []

    def test_entry_dataclass_defaults(self) -> None:
        """CVEntry must have sensible defaults for all optional fields."""
        entry = CVEntry(id="abc.1")
        assert entry.parent_ids == []
        assert entry.origin is None
        assert entry.contributions == []
        assert entry.deleted is None

    def test_origin_dataclass_defaults(self) -> None:
        """Origin must default timestamp to None and meta to {}."""
        o = Origin(source="x", location="y")
        assert o.timestamp is None
        assert o.meta == {}

    def test_contribution_dataclass_defaults(self) -> None:
        """Contribution must default meta to {}."""
        c = Contribution(source="x", tag="y")
        assert c.meta == {}

    def test_deletion_record_dataclass_defaults(self) -> None:
        """DeletionRecord must default meta to {}."""
        d = DeletionRecord(source="x", reason="y")
        assert d.meta == {}

    def test_serialize_origin_none_is_null(self) -> None:
        """Serialised entry with no origin must have 'origin': None."""
        log = CVLog()
        cv_id = log.create()
        data = log.serialize()
        assert data["entries"][cv_id]["origin"] is None

    def test_serialize_deleted_none_is_null(self) -> None:
        """Serialised entry with no deletion must have 'deleted': None."""
        log = CVLog()
        cv_id = log.create(make_origin())
        data = log.serialize()
        assert data["entries"][cv_id]["deleted"] is None

    def test_serialize_with_deletion(self) -> None:
        """Serialised deletion record must include source, reason, and meta."""
        log = CVLog()
        cv_id = log.create(make_origin())
        log.delete(cv_id, "dce", "unused")
        data = log.serialize()
        deleted = data["entries"][cv_id]["deleted"]
        assert deleted is not None
        assert deleted["source"] == "dce"
        assert deleted["reason"] == "unused"

    def test_deserialize_with_no_meta_in_contributions(self) -> None:
        """deserialize must handle contributions with missing meta gracefully."""
        data = {
            "entries": {
                "abcdef01.1": {
                    "id": "abcdef01.1",
                    "parent_ids": [],
                    "origin": {"source": "x", "location": "y", "timestamp": None, "meta": {}},
                    "contributions": [{"source": "p", "tag": "t"}],
                    "deleted": None,
                }
            },
            "pass_order": [],
            "enabled": True,
        }
        log = CVLog.deserialize(data)
        entry = log.get("abcdef01.1")
        assert entry is not None
        assert entry.contributions[0].meta == {}

    def test_merge_with_empty_parent_list(self) -> None:
        """merge([]) must still return a valid CV ID."""
        log = CVLog()
        merged = log.merge([])
        assert_valid_id(merged)

    def test_create_multiple_bases_independent_counters(self) -> None:
        """Creates with different origins must have independent counters."""
        log = CVLog()
        a1 = log.create(make_origin("a.ts", "0:0"))
        a2 = log.create(make_origin("a.ts", "0:0"))
        b1 = log.create(make_origin("b.ts", "0:0"))
        assert a1.endswith(".1")
        assert a2.endswith(".2")
        assert b1.endswith(".1")

    def test_lineage_two_level_chain(self) -> None:
        """lineage of a 2-level chain (A→B) must be [A, B]."""
        log = CVLog()
        a = log.create(make_origin())
        b = log.derive(a)
        lin = log.lineage(b)
        assert [e.id for e in lin] == [a, b]

    def test_descendants_via_merge(self) -> None:
        """A merged CV must appear in descendants of all its parents."""
        log = CVLog()
        a = log.create(make_origin("a.ts", "0:0"))
        b = log.create(make_origin("b.ts", "0:0"))
        merged = log.merge([a, b])
        assert merged in log.descendants(a)
        assert merged in log.descendants(b)

    def test_to_json_string_is_valid_json(self) -> None:
        """to_json_string() must produce valid JSON (parseable by json_value)."""
        from json_value import parse_native

        log = CVLog()
        cv_id = log.create(make_origin("app.ts", "0:0"))
        log.contribute(cv_id, "parser", "created")
        s = log.to_json_string()
        parsed = parse_native(s)
        assert isinstance(parsed, dict)

    def test_enabled_false_in_serialised_output(self) -> None:
        """A disabled log serialised then deserialised must keep enabled=False."""
        log = CVLog(enabled=False)
        data = log.serialize()
        assert data["enabled"] is False
        restored = CVLog.deserialize(data)
        assert restored.enabled is False

    def test_origin_with_meta_roundtrip(self) -> None:
        """Origin.meta dict must survive a serialise/deserialise roundtrip."""
        log = CVLog()
        origin = Origin("src", "1:0", meta={"schema": "v2", "hash": "abc"})
        cv_id = log.create(origin)
        restored = CVLog.from_json_string(log.to_json_string())
        rest_origin = restored.get(cv_id).origin  # type: ignore[union-attr]
        assert rest_origin is not None
        assert rest_origin.meta == {"schema": "v2", "hash": "abc"}

    def test_origin_timestamp_roundtrip(self) -> None:
        """Origin.timestamp must survive a serialise/deserialise roundtrip."""
        log = CVLog()
        origin = Origin("src", "0:0", timestamp="2024-06-15T12:00:00Z")
        cv_id = log.create(origin)
        restored = CVLog.from_json_string(log.to_json_string())
        rest_origin = restored.get(cv_id).origin  # type: ignore[union-attr]
        assert rest_origin is not None
        assert rest_origin.timestamp == "2024-06-15T12:00:00Z"
