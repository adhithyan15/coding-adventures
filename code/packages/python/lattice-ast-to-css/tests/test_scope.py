"""Tests for the ScopeChain — lexical scoping for Lattice.

The scope chain is the backbone of the Lattice transformer. Every variable
lookup, mixin parameter binding, and function evaluation depends on it.

These tests verify:
1. Basic get/set operations
2. Parent-chain lookup (lexical scoping)
3. Shadowing (child overrides parent's binding)
4. Isolation (child's set doesn't affect parent)
5. Depth tracking
6. has/has_local distinction
"""

from __future__ import annotations

from lattice_ast_to_css.scope import ScopeChain


class TestBasicOperations:
    """Test fundamental get/set behavior."""

    def test_set_and_get(self) -> None:
        scope = ScopeChain()
        scope.set("$color", "red")
        assert scope.get("$color") == "red"

    def test_get_undefined_returns_none(self) -> None:
        scope = ScopeChain()
        assert scope.get("$nonexistent") is None

    def test_overwrite_binding(self) -> None:
        scope = ScopeChain()
        scope.set("$color", "red")
        scope.set("$color", "blue")
        assert scope.get("$color") == "blue"

    def test_multiple_bindings(self) -> None:
        scope = ScopeChain()
        scope.set("$a", 1)
        scope.set("$b", 2)
        scope.set("$c", 3)
        assert scope.get("$a") == 1
        assert scope.get("$b") == 2
        assert scope.get("$c") == 3


class TestParentChainLookup:
    """Test lexical scope lookup through parent chain."""

    def test_child_sees_parent_bindings(self) -> None:
        """Variables declared in parent are visible in child."""
        parent = ScopeChain()
        parent.set("$color", "red")
        child = parent.child()
        assert child.get("$color") == "red"

    def test_grandchild_sees_grandparent(self) -> None:
        """Lookup walks all the way up the chain."""
        grandparent = ScopeChain()
        grandparent.set("$base", 16)
        parent = grandparent.child()
        child = parent.child()
        assert child.get("$base") == 16

    def test_child_undefined_in_parent(self) -> None:
        """If neither child nor parent has it, return None."""
        parent = ScopeChain()
        child = parent.child()
        assert child.get("$x") is None


class TestShadowing:
    """Test variable shadowing — child overrides parent's binding.

    This is essential for Lattice scoping:

        $color: red;         ← global
        .parent {
            $color: blue;    ← shadows global
            color: $color;   → blue
        }
        color: $color;       → red (global unchanged)
    """

    def test_child_shadows_parent(self) -> None:
        parent = ScopeChain()
        parent.set("$color", "red")
        child = parent.child()
        child.set("$color", "blue")
        assert child.get("$color") == "blue"

    def test_shadowing_doesnt_affect_parent(self) -> None:
        parent = ScopeChain()
        parent.set("$color", "red")
        child = parent.child()
        child.set("$color", "blue")
        assert parent.get("$color") == "red"

    def test_sibling_scopes_independent(self) -> None:
        """Two child scopes don't affect each other."""
        parent = ScopeChain()
        parent.set("$color", "red")

        child_a = parent.child()
        child_a.set("$color", "blue")

        child_b = parent.child()
        # child_b should see parent's value, not child_a's
        assert child_b.get("$color") == "red"
        assert child_a.get("$color") == "blue"


class TestIsolation:
    """Test that child set() doesn't modify parent."""

    def test_child_set_is_local(self) -> None:
        parent = ScopeChain()
        child = parent.child()
        child.set("$local", "value")
        assert parent.get("$local") is None

    def test_deep_chain_isolation(self) -> None:
        """Each level only affects its own bindings."""
        root = ScopeChain()
        mid = root.child()
        leaf = mid.child()
        leaf.set("$x", 42)
        assert mid.get("$x") is None
        assert root.get("$x") is None


class TestHasMethod:
    """Test has() and has_local() methods."""

    def test_has_in_current_scope(self) -> None:
        scope = ScopeChain()
        scope.set("$x", 1)
        assert scope.has("$x") is True

    def test_has_in_parent(self) -> None:
        parent = ScopeChain()
        parent.set("$x", 1)
        child = parent.child()
        assert child.has("$x") is True

    def test_has_returns_false(self) -> None:
        scope = ScopeChain()
        assert scope.has("$nope") is False

    def test_has_local_current_only(self) -> None:
        parent = ScopeChain()
        parent.set("$x", 1)
        child = parent.child()
        assert child.has_local("$x") is False
        child.set("$x", 2)
        assert child.has_local("$x") is True

    def test_has_local_doesnt_check_parent(self) -> None:
        parent = ScopeChain()
        parent.set("$inherited", True)
        child = parent.child()
        assert child.has("$inherited") is True
        assert child.has_local("$inherited") is False


class TestDepth:
    """Test depth tracking."""

    def test_global_depth_zero(self) -> None:
        assert ScopeChain().depth == 0

    def test_child_depth_one(self) -> None:
        parent = ScopeChain()
        child = parent.child()
        assert child.depth == 1

    def test_nested_depth(self) -> None:
        scope = ScopeChain()
        for i in range(5):
            scope = scope.child()
        assert scope.depth == 5


class TestRepr:
    """Test string representation."""

    def test_repr_empty(self) -> None:
        scope = ScopeChain()
        assert "depth=0" in repr(scope)
        assert "bindings=[]" in repr(scope)

    def test_repr_with_bindings(self) -> None:
        scope = ScopeChain()
        scope.set("$color", "red")
        r = repr(scope)
        assert "depth=0" in r
        assert "$color" in r

    def test_repr_child(self) -> None:
        parent = ScopeChain()
        child = parent.child()
        assert "depth=1" in repr(child)


class TestValueTypes:
    """Test that scope stores arbitrary value types correctly.

    The scope chain doesn't care about value types — it stores whatever
    the transformer gives it. This includes AST nodes, token lists,
    evaluated numbers, strings, etc.
    """

    def test_store_string(self) -> None:
        scope = ScopeChain()
        scope.set("$x", "hello")
        assert scope.get("$x") == "hello"

    def test_store_number(self) -> None:
        scope = ScopeChain()
        scope.set("$x", 42)
        assert scope.get("$x") == 42

    def test_store_list(self) -> None:
        scope = ScopeChain()
        scope.set("$x", [1, 2, 3])
        assert scope.get("$x") == [1, 2, 3]

    def test_store_none(self) -> None:
        """None is a valid value (Lattice 'null')."""
        scope = ScopeChain()
        scope.set("$x", None)
        # get() returns None for both "not found" and "bound to None"
        # has() distinguishes the two cases
        assert scope.has("$x") is True
        assert scope.get("$x") is None

    def test_store_dict(self) -> None:
        """Dicts can represent AST nodes or compound values."""
        scope = ScopeChain()
        scope.set("$mixin", {"params": ["$a", "$b"], "body": "..."})
        result = scope.get("$mixin")
        assert result["params"] == ["$a", "$b"]
