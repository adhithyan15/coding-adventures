"""Scope chain — tracks variable and function declarations during type checking.

=============================================================================
WHAT IS A SCOPE?
=============================================================================

A **scope** is a region of a program in which a name (variable, function)
is visible. Most languages support *nested scopes*: inner blocks can see
names declared in outer blocks, but not vice-versa.

In Nib::

    fn main() {                  // ← scope A (function body)
        let x: u4 = 5;           // x declared in scope A
        if true {                // ← scope B (if-block), nested inside A
            let y: u4 = x + 1;  // y declared in B; x visible from A
        }
        // y is NOT visible here — it was declared inside scope B
    }

The ``ScopeChain`` class models this with a *stack of dictionaries*. Each
dictionary maps a name (string) to its ``Symbol`` record. When we enter a
new block (``{``), we ``push()`` a new empty dict; when we leave (``}``),
we ``pop()`` it.

Looking up a name traverses the stack from innermost to outermost, returning
the first match found. This correctly models shadowing: an inner declaration
can hide an outer one with the same name.

=============================================================================
SYMBOL TABLE ENTRIES
=============================================================================

A ``Symbol`` records everything the type checker needs to know about a name:

- ``name``:       The identifier string (e.g., ``"x"``).
- ``nib_type``:   The declared ``NibType`` (e.g., ``NibType.U4``).
- ``is_const``:   True if declared with ``const`` (compile-time constant).
- ``is_static``:  True if declared with ``static`` (RAM-mapped variable).

``is_const`` is especially important for the ``for``-loop bounds check:
the start and end of a range must be either integer literals or ``const``-
declared names (not variables whose value is unknown at compile time).

=============================================================================
GLOBAL SCOPE VS LOCAL SCOPE
=============================================================================

The ``ScopeChain`` is initialised with a single empty scope (the *global
scope*). Top-level ``const`` and ``static`` declarations are added there.
When the type checker enters a function body, it ``push()``es a scope for
the parameters; when it enters a block (``{...}``), it ``push()``es another.

This mirrors how C, Rust, and most block-structured languages work.

=============================================================================
FUNCTION SIGNATURES IN THE GLOBAL SCOPE
=============================================================================

Function declarations are *also* stored in the global scope, but as
``Symbol`` objects with the special ``fn_params`` and ``fn_return_type``
fields set. This lets the type checker resolve function calls before it
has walked the function body — essential for supporting forward references
(calling a function before its definition appears in the source text).

Actually — Nib does a *two-pass* approach:

  Pass 1: Collect all function signatures, const/static types into global
          scope. Build the call graph.
  Pass 2: Type-check each function body using those signatures.

This means every function can call every other function (declared order
doesn't matter), just like in C with a header file.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from nib_type_checker.types import NibType


@dataclass
class Symbol:
    """A single entry in the symbol table.

    Attributes
    ----------
    name:
        The source-level identifier string.
    nib_type:
        The ``NibType`` of this variable, constant, or static. For
        function symbols, this is the *return* type (or ``None`` if void).
    is_const:
        True if declared with the ``const`` keyword. Const names may
        appear as for-loop bounds.
    is_static:
        True if declared with the ``static`` keyword. Static names live
        in the 4004's RAM.
    is_fn:
        True if this symbol is a function declaration.
    fn_params:
        For function symbols: ordered list of ``(param_name, param_type)``
        pairs. Empty list for functions with no parameters.
    fn_return_type:
        For function symbols: the declared return type, or ``None`` if the
        function is void (returns nothing).

    Examples
    --------
    >>> s = Symbol(name="x", nib_type=NibType.U4)
    >>> s.is_const
    False
    >>> s.is_fn
    False
    >>> fn = Symbol(
    ...     name="add",
    ...     nib_type=NibType.U4,
    ...     is_fn=True,
    ...     fn_params=[("a", NibType.U4), ("b", NibType.U4)],
    ...     fn_return_type=NibType.U4,
    ... )
    >>> fn.is_fn
    True
    """

    name: str
    nib_type: NibType | None
    is_const: bool = False
    is_static: bool = False
    is_fn: bool = False
    fn_params: list[tuple[str, NibType]] = field(default_factory=list)
    fn_return_type: NibType | None = None


class ScopeChain:
    """A stack of scopes implementing lexical (static) scoping.

    The chain is modelled as a list of dictionaries. The *last* element is
    the innermost (currently active) scope.

    How name resolution works:

    1. ``define(name, sym)`` inserts into the *current* (innermost) scope.
    2. ``lookup(name)`` searches from innermost to outermost, returning the
       first match.  This correctly implements shadowing.
    3. ``push()`` appends a new empty scope (called on block entry).
    4. ``pop()`` removes the innermost scope (called on block exit).

    The chain always contains at least one scope (the global scope). Popping
    the global scope raises a ``RuntimeError`` to prevent bugs.

    Examples
    --------
    >>> sc = ScopeChain()
    >>> sc.define("x", Symbol("x", NibType.U4))
    >>> sc.lookup("x").nib_type
    <NibType.U4: 'u4'>
    >>> sc.lookup("y") is None
    True
    >>> sc.push()
    >>> sc.define("y", Symbol("y", NibType.U8))
    >>> sc.lookup("y").nib_type  # found in inner scope
    <NibType.U8: 'u8'>
    >>> sc.lookup("x").nib_type  # found in outer scope
    <NibType.U4: 'u4'>
    >>> sc.pop()
    >>> sc.lookup("y") is None  # gone after pop
    True
    """

    def __init__(self) -> None:
        # Start with one scope: the global scope.
        self._scopes: list[dict[str, Symbol]] = [{}]

    def push(self) -> None:
        """Enter a new nested scope (e.g., on encountering a ``{`` block).

        Called whenever the type checker descends into a block, function
        body, or for-loop body.
        """
        self._scopes.append({})

    def pop(self) -> None:
        """Exit the current scope (e.g., on encountering a ``}``).

        Raises
        ------
        RuntimeError
            If an attempt is made to pop the global scope (which would
            leave the chain empty — a type-checker bug, not a user error).
        """
        if len(self._scopes) <= 1:
            raise RuntimeError("Cannot pop the global scope — type checker bug")
        self._scopes.pop()

    def define(self, name: str, sym: Symbol) -> None:
        """Declare ``name`` in the current (innermost) scope.

        If the name is already declared in the current scope, it is
        silently overwritten. (Shadowing an outer-scope name is allowed;
        re-declaring in the same scope is currently not an error in the
        type checker — a future version may add a duplicate-declaration
        warning.)

        Parameters
        ----------
        name:
            The identifier string to declare.
        sym:
            The ``Symbol`` record to associate with ``name``.
        """
        self._scopes[-1][name] = sym

    def lookup(self, name: str) -> Symbol | None:
        """Search for ``name`` from innermost scope outward.

        Traverses the scope stack from the current (innermost) scope
        toward the global (outermost) scope, returning the first ``Symbol``
        found.

        Parameters
        ----------
        name:
            The identifier string to look up.

        Returns
        -------
        Symbol | None
            The ``Symbol`` if found; ``None`` if not declared in any
            accessible scope.

        Examples
        --------
        >>> sc = ScopeChain()
        >>> sc.define("outer", Symbol("outer", NibType.U4))
        >>> sc.push()
        >>> sc.define("inner", Symbol("inner", NibType.U8))
        >>> sc.lookup("outer").name
        'outer'
        >>> sc.lookup("inner").name
        'inner'
        >>> sc.lookup("missing") is None
        True
        """
        for scope in reversed(self._scopes):
            if name in scope:
                return scope[name]
        return None

    def define_global(self, name: str, sym: Symbol) -> None:
        """Declare ``name`` in the *global* (outermost) scope.

        Used during Pass 1 (signature collection) to add function
        signatures, consts, and statics into the global scope regardless
        of how deeply nested the current scope is.

        Parameters
        ----------
        name:
            The identifier string to declare.
        sym:
            The ``Symbol`` record to associate with ``name``.
        """
        self._scopes[0][name] = sym
