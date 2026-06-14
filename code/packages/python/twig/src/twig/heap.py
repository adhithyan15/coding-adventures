"""Twig's host-side heap with refcounted GC primitives.

Why this exists
===============
``vm-core`` itself is heap-free — its register file holds Python
primitives and that's it.  Twig needs three kinds of heap-allocated
objects (cons cells, symbols, closures), all of which need lifetime
management.  TW00's design puts that machinery here, on the host
side, exposed to ``vm-core`` only via :func:`call_builtin` boundaries
(``cons``, ``car``, ``cdr``, ``apply_closure``, …).

Why reference counting first?
=============================
Three reasons:

1. **Visibility.**  Refcounting is the simplest correct GC algorithm.
   Reading this file teaches the algorithm directly — every alloc
   bumps a counter, every release drops one, freeing happens at zero.
   Mark-sweep (TW01) hides allocation costs and amortises collection;
   visible refcounting is more pedagogical for a starting point.
2. **Correctness for v1.**  TW00 has no ``letrec`` and no nested
   closures that mutually reference each other after construction, so
   programs cannot construct cycles in the v1 surface.  Refcounting
   suffices.
3. **Easy to swap.**  ``Heap`` is the only place that knows about
   refcounts.  TW01's mark-sweep replacement is a drop-in.

Tagged values
=============
Twig values are one of:

* ``int`` — Twig integers (``0``, ``42``, ``-7``)
* ``bool`` — Twig booleans (``#t`` / ``#f``)
* :data:`NIL` — a singleton sentinel for Twig's ``nil``
* :class:`HeapHandle` — an opaque integer handle to a heap object

Predicates use ``isinstance`` checks.  ``bool`` must be checked
*before* ``int`` because ``bool`` is a subclass of ``int`` in Python.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

# ---------------------------------------------------------------------------
# Sentinel: nil
# ---------------------------------------------------------------------------


class _Nil:
    """The unique value of Twig's ``nil``.

    Made a class (not just ``None``) so the empty list is observably
    distinct from the absence of a value at the host boundary.  Twig
    code that returns ``nil`` is *returning a value*; Python ``None``
    means "no return value at all".
    """

    __slots__ = ()
    _instance: _Nil | None = None

    def __new__(cls) -> _Nil:
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __repr__(self) -> str:
        return "nil"

    def __bool__(self) -> bool:
        return False  # nil is falsy in Twig


NIL: _Nil = _Nil()


# ---------------------------------------------------------------------------
# Heap handle
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class HeapHandle:
    """An opaque handle into the :class:`Heap`.

    The wrapped integer is *not* a Twig value.  Callers should never
    perform arithmetic on it; they pass handles around through
    ``vm-core``'s register file as Python objects, and only the
    :class:`Heap` interprets them.
    """

    id: int

    def __repr__(self) -> str:
        return f"<handle:{self.id}>"


# ---------------------------------------------------------------------------
# Internal object kinds
# ---------------------------------------------------------------------------


@dataclass
class _Cons:
    """Cons cell: a 2-slot heap object with arbitrary Twig values."""

    car: Any
    cdr: Any


@dataclass
class _Symbol:
    """Interned symbol — one record per unique name in a heap."""

    name: str


@dataclass
class _Closure:
    """First-class function value.

    ``fn_name`` is a top-level IIR function name (gensym'd for
    ``lambda`` expressions, the user-supplied name for ``define``).
    ``captured`` is the snapshot of free-variable values taken at
    closure-construction time, in fixed order matching the IIR
    function's leading parameters.
    """

    fn_name: str
    captured: list[Any]


# ---------------------------------------------------------------------------
# Heap statistics
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class HeapStats:
    """A snapshot of a :class:`Heap`'s lifetime counters.

    The counters are aggregate (never reset between programs), so
    successive snapshots show monotonic growth.  Use
    ``Heap.reset_stats()`` to zero them between independent runs.
    """

    live_objects: int
    peak_objects: int
    total_allocs: int
    total_releases: int


# ---------------------------------------------------------------------------
# Heap
# ---------------------------------------------------------------------------


class Heap:
    """A host-side refcounted heap for Twig.

    All allocation routes through one of:

    * :meth:`alloc_cons` — for cons cells (returns a fresh handle).
    * :meth:`make_symbol` — for symbols (interns by name; same name
      always returns the same handle).
    * :meth:`alloc_closure` — for first-class functions.

    Read-only access:

    * :meth:`car` / :meth:`cdr` for cons cells.
    * :meth:`symbol_name` for symbols.
    * :meth:`closure_fn` / :meth:`closure_captured` for closures.

    Lifetime:

    * :meth:`incref` bumps a handle's refcount.
    * :meth:`decref` drops it; reaching zero frees the object and
      decrefs any inner handles.
    * Symbols are deliberately *not* refcounted — they're interned
      for the lifetime of the heap and shared by every reference.

    Bound checks: every method that accepts a handle validates the
    handle is live.  Out-of-bounds access raises
    :class:`TwigRuntimeError` rather than silently returning garbage.
    """

    def __init__(self) -> None:
        # Storage: one Python dict mapping handle ID → object.
        # We use a dict (not a list) so we can leave gaps when an
        # object is freed; allocations always advance ``_next_id``
        # rather than reusing slots.  Slot reuse is a TW01 concern
        # tied to mark-sweep compaction.
        self._objects: dict[int, Any] = {}
        self._refcounts: dict[int, int] = {}
        self._symbols: dict[str, int] = {}
        self._next_id: int = 1  # 0 reserved as a "null handle" sentinel

        # Lifetime statistics — useful for tests asserting GC correctness
        # ("after a clean program, ``live_objects`` returns to 0").
        self._peak_objects: int = 0
        self._total_allocs: int = 0
        self._total_releases: int = 0

    # ------------------------------------------------------------------
    # Allocation
    # ------------------------------------------------------------------

    def alloc_cons(self, car: Any, cdr: Any) -> HeapHandle:
        """Allocate a fresh cons cell.

        Both ``car`` and ``cdr`` may be any Twig value.  If either is a
        :class:`HeapHandle` we ``incref`` it before storing — the cons
        cell now owns an additional reference, and freeing the cons
        cell will release them.
        """
        handle = self._fresh_handle()
        self._objects[handle.id] = _Cons(car=car, cdr=cdr)
        self._refcounts[handle.id] = 1
        self._note_alloc()
        if isinstance(car, HeapHandle):
            self.incref(car)
        if isinstance(cdr, HeapHandle):
            self.incref(cdr)
        return handle

    def make_symbol(self, name: str) -> HeapHandle:
        """Return the interned symbol handle for ``name``.

        Symbols are interned by name (Scheme semantics).  Two
        ``make_symbol("foo")`` calls return the same handle.  Symbols
        do *not* participate in refcounting — they live for the
        lifetime of the heap.
        """
        existing = self._symbols.get(name)
        if existing is not None:
            return HeapHandle(existing)
        handle = self._fresh_handle()
        self._objects[handle.id] = _Symbol(name=name)
        # Symbol refcount stays "infinite" — represented here by
        # absence from ``_refcounts``.  decref() treats missing keys
        # as no-ops, so we never accidentally free an interned symbol.
        self._symbols[name] = handle.id
        self._note_alloc()
        return handle

    def alloc_closure(
        self, fn_name: str, captured: list[Any]
    ) -> HeapHandle:
        """Allocate a closure.

        ``captured`` is the list of free-variable values to be passed
        as the leading arguments at apply time.  Any handles in
        ``captured`` are increfed so the closure owns them.
        """
        handle = self._fresh_handle()
        # Defensive copy: the caller may keep mutating their list.
        self._objects[handle.id] = _Closure(
            fn_name=fn_name, captured=list(captured)
        )
        self._refcounts[handle.id] = 1
        self._note_alloc()
        for v in captured:
            if isinstance(v, HeapHandle):
                self.incref(v)
        return handle

    # ------------------------------------------------------------------
    # Inspection
    # ------------------------------------------------------------------

    def car(self, handle: HeapHandle) -> Any:
        cell = self._require_cons(handle)
        return cell.car

    def cdr(self, handle: HeapHandle) -> Any:
        cell = self._require_cons(handle)
        return cell.cdr

    def symbol_name(self, handle: HeapHandle) -> str:
        obj = self._require_object(handle)
        if not isinstance(obj, _Symbol):
            from twig.errors import TwigRuntimeError

            raise TwigRuntimeError(f"{handle!r} is not a symbol")
        return obj.name

    def closure_fn(self, handle: HeapHandle) -> str:
        return self._require_closure(handle).fn_name

    def closure_captured(self, handle: HeapHandle) -> list[Any]:
        # Return a copy: callers shouldn't mutate the heap's storage.
        return list(self._require_closure(handle).captured)

    # ------------------------------------------------------------------
    # Predicates
    # ------------------------------------------------------------------

    def is_cons(self, value: Any) -> bool:
        return (
            isinstance(value, HeapHandle)
            and isinstance(self._objects.get(value.id), _Cons)
        )

    def is_symbol(self, value: Any) -> bool:
        return (
            isinstance(value, HeapHandle)
            and isinstance(self._objects.get(value.id), _Symbol)
        )

    def is_closure(self, value: Any) -> bool:
        return (
            isinstance(value, HeapHandle)
            and isinstance(self._objects.get(value.id), _Closure)
        )

    # ------------------------------------------------------------------
    # GC primitives
    # ------------------------------------------------------------------

    def incref(self, handle: HeapHandle) -> None:
        """Increment the refcount.  No-op for symbols (interned)."""
        if handle.id not in self._refcounts:
            return
        self._refcounts[handle.id] += 1

    def decref(self, handle: HeapHandle) -> None:
        """Decrement the refcount.  Frees the object on reaching 0.

        Freeing a cons cell or closure recursively decrefs any nested
        handles, so dropping the head of a list correctly releases
        the entire spine.
        """
        if handle.id not in self._refcounts:
            return  # symbol or already-freed: no-op
        self._refcounts[handle.id] -= 1
        if self._refcounts[handle.id] <= 0:
            self._free(handle.id)

    # ------------------------------------------------------------------
    # Stats
    # ------------------------------------------------------------------

    def stats(self) -> HeapStats:
        return HeapStats(
            live_objects=len(self._objects),
            peak_objects=self._peak_objects,
            total_allocs=self._total_allocs,
            total_releases=self._total_releases,
        )

    def reset_stats(self) -> None:
        self._peak_objects = len(self._objects)
        self._total_allocs = 0
        self._total_releases = 0

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _fresh_handle(self) -> HeapHandle:
        handle = HeapHandle(self._next_id)
        self._next_id += 1
        return handle

    def _note_alloc(self) -> None:
        self._total_allocs += 1
        live = len(self._objects)
        if live > self._peak_objects:
            self._peak_objects = live

    def _require_object(self, handle: HeapHandle) -> Any:
        obj = self._objects.get(handle.id)
        if obj is None:
            from twig.errors import TwigRuntimeError

            raise TwigRuntimeError(f"{handle!r} is not a live heap handle")
        return obj

    def _require_cons(self, handle: HeapHandle) -> _Cons:
        obj = self._require_object(handle)
        if not isinstance(obj, _Cons):
            from twig.errors import TwigRuntimeError

            raise TwigRuntimeError(f"{handle!r} is not a cons cell")
        return obj

    def _require_closure(self, handle: HeapHandle) -> _Closure:
        obj = self._require_object(handle)
        if not isinstance(obj, _Closure):
            from twig.errors import TwigRuntimeError

            raise TwigRuntimeError(f"{handle!r} is not a closure")
        return obj

    def _free(self, handle_id: int) -> None:
        obj = self._objects.pop(handle_id, None)
        self._refcounts.pop(handle_id, None)
        self._total_releases += 1
        if obj is None:
            return
        # Recursively decref any nested handles.  We must do this
        # *after* removing the object from ``_objects`` so a cycle
        # that somehow got built does not infinite-loop the
        # refcounter.  TW00 doesn't construct cycles, but defending
        # against them is cheap.
        if isinstance(obj, _Cons):
            for inner in (obj.car, obj.cdr):
                if isinstance(inner, HeapHandle):
                    self.decref(inner)
        elif isinstance(obj, _Closure):
            for inner in obj.captured:
                if isinstance(inner, HeapHandle):
                    self.decref(inner)
