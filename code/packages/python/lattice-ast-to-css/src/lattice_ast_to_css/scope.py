"""Scope chain — lexical scoping for Lattice variables, mixins, and functions.

Why Lexical Scoping?
--------------------

CSS has no concept of scope — everything is global. But Lattice adds
variables, mixins, and functions, which need scoping rules to prevent
name collisions and enable local reasoning.

Lattice uses **lexical (static) scoping**, meaning a variable's scope is
determined by where it appears in the source text, not by runtime call order.
This is the same model used by JavaScript, Python, and most modern languages.

How It Works
------------

Each ``{ }`` block in the source creates a new child scope. Variables declared
inside a block are local to that scope and its descendants. Looking up a
variable walks up the parent chain until the name is found::

    $color: red;              ← global scope (depth 0)
    .parent {                 ← child scope (depth 1)
        $color: blue;         ← shadows the global $color
        color: $color;        → blue (found at depth 1)
        .child {              ← grandchild scope (depth 2)
            color: $color;    → blue (inherited from depth 1)
        }
    }
    .sibling {                ← another child scope (depth 1)
        color: $color;        → red (global scope, not affected by .parent)
    }

This is implemented as a linked list of scope nodes. Each node has a ``parent``
pointer and a ``bindings`` dict. Looking up a name walks the chain upward.

Special Scoping Rules
---------------------

- **Mixin expansion**: creates a child scope of the caller's scope. This lets
  mixins see the caller's variables (like closures in JavaScript).

- **Function evaluation**: creates an **isolated** scope whose parent is the
  definition-site global scope, NOT the caller's scope. This prevents functions
  from accidentally depending on where they're called from — they only see
  their own parameters and global definitions.
"""

from __future__ import annotations

from typing import Any


class ScopeChain:
    """A lexical scope node in the scope chain.

    Each scope has:

    - ``bindings``: A dict mapping names to values (AST nodes, tokens, or
      evaluated values — the scope doesn't care about the type).
    - ``parent``: The enclosing scope, or ``None`` for the global scope.

    Operations:

    - ``get(name)`` — look up a name, walking up the parent chain.
    - ``set(name, value)`` — bind a name in *this* scope (not parent's).
    - ``has(name)`` — check if a name exists anywhere in the chain.
    - ``child()`` — create a new child scope with ``self`` as parent.
    - ``depth`` — how many levels deep this scope is (0 = global).

    Why a class and not just a dict?
    Because nested scopes need parent-chain lookups, and we need to
    distinguish between "not found anywhere" (error) and "not found
    locally but found in parent" (normal lexical lookup).

    Example::

        global_scope = ScopeChain()
        global_scope.set("$color", "red")

        block_scope = global_scope.child()
        block_scope.set("$color", "blue")

        block_scope.get("$color")    # → "blue" (local)
        global_scope.get("$color")   # → "red" (unchanged)

        nested = block_scope.child()
        nested.get("$color")         # → "blue" (inherited from parent)
    """

    def __init__(self, parent: ScopeChain | None = None) -> None:
        self.bindings: dict[str, Any] = {}
        self.parent = parent

    def get(self, name: str) -> Any:
        """Look up a name in this scope or any ancestor scope.

        Walks up the parent chain until the name is found. If the name
        isn't found anywhere, returns ``None``.

        This is the core of lexical scoping — a variable declared in an
        outer scope is visible in all inner scopes unless shadowed.

        Args:
            name: The variable/mixin/function name to look up.

        Returns:
            The bound value, or ``None`` if not found anywhere.
        """
        if name in self.bindings:
            return self.bindings[name]
        if self.parent is not None:
            return self.parent.get(name)
        return None

    def set(self, name: str, value: Any) -> None:
        """Bind a name to a value in this scope.

        Always binds in the *current* scope, never in a parent scope.
        This means a child scope can shadow a parent's binding without
        modifying the parent.

        Args:
            name: The variable/mixin/function name to bind.
            value: The value to associate with the name.
        """
        self.bindings[name] = value

    def has(self, name: str) -> bool:
        """Check whether a name exists in this scope or any ancestor.

        Like ``get``, walks up the parent chain. Returns ``True`` if the
        name is bound anywhere, ``False`` otherwise.

        Args:
            name: The name to check.

        Returns:
            ``True`` if the name is bound, ``False`` otherwise.
        """
        if name in self.bindings:
            return True
        if self.parent is not None:
            return self.parent.has(name)
        return False

    def has_local(self, name: str) -> bool:
        """Check whether a name exists in *this* scope only (not parents).

        Useful for detecting re-declarations and shadowing.

        Args:
            name: The name to check.

        Returns:
            ``True`` if the name is bound in this scope, ``False`` otherwise.
        """
        return name in self.bindings

    def child(self) -> ScopeChain:
        """Create a new child scope with ``self`` as parent.

        The child inherits all bindings from the parent chain via ``get``,
        but any ``set`` calls on the child only affect the child.

        Returns:
            A new ``ScopeChain`` whose parent is ``self``.
        """
        return ScopeChain(parent=self)

    @property
    def depth(self) -> int:
        """How many levels deep this scope is.

        The global scope has depth 0. Each ``child()`` call adds 1.

        Returns:
            The depth of this scope in the chain.
        """
        if self.parent is None:
            return 0
        return 1 + self.parent.depth

    def __repr__(self) -> str:
        names = list(self.bindings.keys())
        return f"ScopeChain(depth={self.depth}, bindings={names})"
