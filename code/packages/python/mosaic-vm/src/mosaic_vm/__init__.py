"""mosaic-vm — Generic tree-walking driver for Mosaic compiler backends.

The VM is the fourth stage of the Mosaic compiler pipeline:

    Source text → Lexer → Parser → Analyzer → MosaicIR → **VM** → Backend → Target code

The VM's responsibilities:

1. Traverse the ``MosaicIR`` tree depth-first.
2. Normalize every value into a ``ResolvedValue`` (hex → RGBA, dimension → {value, unit}).
3. Track a ``SlotContext`` (component slots + active each-loop scopes).
4. Call ``MosaicRenderer`` methods in strict open-before-close order.

What the VM Does NOT Do
-----------------------

The VM is agnostic about output format. It has no knowledge of React, Web
Components, SwiftUI, or any other platform. Backends own the output — the VM
only drives the traversal and normalizes values.

This separation mirrors the design of the JVM or CLR: the VM handles
execution mechanics; the platform code handles platform-specific behavior.

Traversal Order
---------------

The VM visits nodes depth-first, calling ``begin_node`` before children and
``end_node`` after. The exact call sequence for a component is::

    begin_component(name, slots)
      begin_node(root, is_primitive, resolved_props, ctx)
        [for each child of root in source order:]
          begin_node(child, ...) ... end_node(child)    ← child nodes
          render_slot_child(...)                         ← @slotName; children
          begin_when(...)                                ← when blocks
            [when children]
          end_when()
          begin_each(...)                                ← each blocks
            [each children — with loop scope pushed]
          end_each()
      end_node(root)
    end_component()
    emit() → list of {"filename": ..., "content": ...}

Color Parsing
-------------

Hex colors use these expansion rules:

+----------+----+----+----+-----+
| Source   | r  | g  | b  | a   |
+==========+====+====+====+=====+
| #rgb     | rr | gg | bb | 255 |
+----------+----+----+----+-----+
| #rrggbb  | rr | gg | bb | 255 |
+----------+----+----+----+-----+
| #rrggbbaa| rr | gg | bb | aa  |
+----------+----+----+----+-----+

Value Normalization
-------------------

+------------------+-----------------------+----------------------------------------+
| MosaicValue kind | → ResolvedValue kind  | What changes                           |
+==================+=======================+========================================+
| color_hex        | color                 | Parsed into r,g,b,a integers 0-255     |
+------------------+-----------------------+----------------------------------------+
| dimension        | dimension             | Split into numeric value + unit        |
+------------------+-----------------------+----------------------------------------+
| ident            | string                | Folded — no semantic change            |
+------------------+-----------------------+----------------------------------------+
| slot_ref         | slot_ref              | Gains slot_type and is_loop_var        |
+------------------+-----------------------+----------------------------------------+
| string/number/bool/enum | unchanged     | Passed through as-is                   |
+------------------+-----------------------+----------------------------------------+

Usage::

    from mosaic_analyzer import analyze
    from mosaic_vm import MosaicVM

    ir = analyze(source)
    vm = MosaicVM(ir)

    result = vm.run(my_renderer)
    for file_info in result:
        print(file_info["filename"], len(file_info["content"]), "chars")
"""

from __future__ import annotations

import re
from abc import ABC, abstractmethod
from dataclasses import dataclass, field

from mosaic_analyzer import (
    MosaicIR,
    MosaicNode,
    MosaicSlot,
)

__version__ = "0.1.0"

__all__ = [
    "MosaicVM",
    "MosaicVMError",
    "MosaicRenderer",
    "SlotContext",
    "LoopScope",
]

# ---------------------------------------------------------------------------
# Slot Context
# ---------------------------------------------------------------------------


@dataclass
class LoopScope:
    """A single active ``each`` loop scope.

    When the VM enters ``each @items as item { ... }``, it pushes a
    ``LoopScope`` onto the context's ``loop_scopes`` stack.

    Attributes:
        item_name: The loop variable name (e.g., ``"item"``).
        element_type: The MosaicType dict for each element (the ``T`` in ``list<T>``).
    """

    item_name: str
    element_type: dict


@dataclass
class SlotContext:
    """Scope tracking for slot resolution during tree traversal.

    The VM creates a root ``SlotContext`` from the component's slot list and
    maintains it throughout the walk. When entering an ``each`` block, the VM
    creates a new inner context with the loop variable pushed onto ``loop_scopes``.

    Resolution priority (innermost-first):

    1. ``loop_scopes`` — innermost loop variable wins
    2. ``component_slots`` — component-level slots
    3. Neither — ``MosaicVMError`` (should have been caught by analyzer)

    Attributes:
        component_slots: Mapping from slot name to ``MosaicSlot`` for the component.
        loop_scopes: Stack of active ``each`` loop variables, innermost last.
    """

    component_slots: dict[str, MosaicSlot] = field(default_factory=dict)
    loop_scopes: list[LoopScope] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Error Class
# ---------------------------------------------------------------------------


class MosaicVMError(Exception):
    """Raised when the VM encounters a runtime invariant violation.

    These are "should not happen" errors — they indicate that the analyzer
    failed to catch an undefined slot reference or an invalid type annotation.
    Syntactic and semantic errors should be caught earlier in the pipeline.
    """


# ---------------------------------------------------------------------------
# MosaicRenderer Protocol (ABC)
# ---------------------------------------------------------------------------


class MosaicRenderer(ABC):
    """The backend protocol — every code generator implements this class.

    The VM calls these methods in strict depth-first, open-before-close order.
    A backend typically accumulates generated source code in string buffers
    and flushes them in ``emit()``.

    Call order guarantee::

        begin_component
          begin_node (root)
            begin_node (child-1) ... end_node (child-1)
            begin_when ... end_when
            begin_each ... end_each
            render_slot_child (for @slotName; children)
          end_node (root)
        end_component
        emit  ← called by MosaicVM.run()
    """

    @abstractmethod
    def begin_component(self, name: str, slots: list[MosaicSlot]) -> None:
        """Called once before any tree traversal.

        Use this to initialize output buffers and emit file headers, import
        statements, class/function preambles, and prop type declarations.

        Args:
            name: The component name (PascalCase), e.g. ``"ProfileCard"``.
            slots: All declared slots, in source order.
        """

    @abstractmethod
    def end_component(self) -> None:
        """Called once after all tree traversal is complete.

        Use this to close any open structures (function bodies, class bodies)
        and finalize output buffers before ``emit()`` is called.
        """

    @abstractmethod
    def emit(self) -> list[dict]:
        """Called by ``MosaicVM.run()`` at the very end to collect generated files.

        Returns:
            A list of dicts, each with ``"filename"`` and ``"content"`` keys.
            Example: ``[{"filename": "Label.tsx", "content": "..."}]``
        """

    @abstractmethod
    def begin_node(
        self,
        tag: str,
        is_primitive: bool,
        properties: list[dict],
        context: SlotContext,
    ) -> None:
        """Called when entering a node element.

        Args:
            tag: Element type name, e.g. ``"Row"``, ``"Text"``, ``"Button"``.
            is_primitive: ``True`` for built-in elements; ``False`` for imports.
            properties: List of ``{"name": ..., "value": ResolvedValue}`` dicts.
            context: The ``SlotContext`` at this point in the tree.
        """

    @abstractmethod
    def end_node(self, tag: str) -> None:
        """Called when leaving a node element (after all children).

        Args:
            tag: The same tag passed to ``begin_node``.
        """

    @abstractmethod
    def render_slot_child(
        self, slot_name: str, slot_type: dict, context: SlotContext
    ) -> None:
        """Called when a slot reference appears as a child of a node (``@slotName;``).

        Args:
            slot_name: The slot being referenced, e.g. ``"action"``.
            slot_type: The declared ``MosaicType`` dict of that slot.
            context: The current ``SlotContext``.
        """

    @abstractmethod
    def begin_when(self, slot_name: str, context: SlotContext) -> None:
        """Called when entering a ``when @flag { ... }`` block.

        Args:
            slot_name: The bool slot that gates this block.
            context: The current ``SlotContext``.
        """

    @abstractmethod
    def end_when(self) -> None:
        """Called when leaving a ``when`` block (after all its children)."""

    @abstractmethod
    def begin_each(
        self,
        slot_name: str,
        item_name: str,
        element_type: dict,
        context: SlotContext,
    ) -> None:
        """Called when entering an ``each @items as item { ... }`` block.

        Args:
            slot_name: The list slot being iterated, e.g. ``"items"``.
            item_name: The loop variable name, e.g. ``"item"``.
            element_type: The ``MosaicType`` dict for each element.
            context: The current ``SlotContext`` (loop scope not yet pushed).
        """

    @abstractmethod
    def end_each(self) -> None:
        """Called when leaving an ``each`` block (after all its children)."""


# ---------------------------------------------------------------------------
# MosaicVM
# ---------------------------------------------------------------------------


class MosaicVM:
    """Generic tree-walking driver for Mosaic compiler backends.

    Construct a VM with a ``MosaicIR``, then call ``run(renderer)`` with any
    backend that implements ``MosaicRenderer``. The VM returns the list of
    generated files from ``renderer.emit()``.

    A single ``MosaicVM`` instance can be run against multiple renderers —
    one for React, another for Web Components, etc. The VM is stateless
    between ``run()`` calls.

    Example::

        ir = analyze(source)
        vm = MosaicVM(ir)

        react_files = vm.run(ReactRenderer())
        wc_files = vm.run(WebComponentRenderer())
    """

    def __init__(self, ir: MosaicIR) -> None:
        """Initialise the VM with a MosaicIR.

        Args:
            ir: The intermediate representation produced by the analyzer.
        """
        self._ir = ir

    def run(self, renderer: MosaicRenderer) -> list[dict]:
        """Traverse the IR tree, calling renderer methods in depth-first order.

        Args:
            renderer: A backend that implements ``MosaicRenderer``.

        Returns:
            The list of ``{"filename": ..., "content": ...}`` dicts from
            ``renderer.emit()``.

        Raises:
            MosaicVMError: If a slot reference cannot be resolved.
        """
        context = SlotContext(
            component_slots={s.name: s for s in self._ir.component.slots},
            loop_scopes=[],
        )
        renderer.begin_component(self._ir.component.name, self._ir.component.slots)
        self._walk_node(self._ir.component.root, context, renderer)
        renderer.end_component()
        return renderer.emit()

    # -------------------------------------------------------------------------
    # Tree traversal
    # -------------------------------------------------------------------------

    def _walk_node(
        self, node: MosaicNode, ctx: SlotContext, r: MosaicRenderer
    ) -> None:
        """Traverse a single node: resolve properties, call begin_node, walk children, call end_node."""
        resolved_props = [
            {"name": p.name, "value": self._resolve_value(p.value, ctx)}
            for p in node.properties
        ]
        r.begin_node(node.node_type, node.is_primitive, resolved_props, ctx)

        for child in node.children:
            self._walk_child(child, ctx, r)

        r.end_node(node.node_type)

    def _walk_child(self, child: dict, ctx: SlotContext, r: MosaicRenderer) -> None:
        """Dispatch a single child to the appropriate renderer method."""
        kind = child["kind"]

        if kind == "node":
            self._walk_node(child["node"], ctx, r)

        elif kind == "slot_ref":
            slot = self._resolve_slot(child["slot_name"], ctx)
            r.render_slot_child(child["slot_name"], slot.type, ctx)

        elif kind == "when":
            r.begin_when(child["slot_name"], ctx)
            for c in child["body"]:
                self._walk_child(c, ctx, r)
            r.end_when()

        elif kind == "each":
            # Resolve the list slot and extract element type
            list_slot = ctx.component_slots.get(child["slot_name"])
            if list_slot is None:
                raise MosaicVMError(f'Unknown list slot: @{child["slot_name"]}')
            if list_slot.type.get("kind") != "list":
                raise MosaicVMError(
                    f'each block references @{child["slot_name"]} but it is not a list type'
                )
            element_type: dict = list_slot.type["element_type"]

            r.begin_each(child["slot_name"], child["item_name"], element_type, ctx)

            inner_ctx = SlotContext(
                component_slots=ctx.component_slots,
                loop_scopes=[
                    *ctx.loop_scopes,
                    LoopScope(item_name=child["item_name"], element_type=element_type),
                ],
            )
            for c in child["body"]:
                self._walk_child(c, inner_ctx, r)

            r.end_each()

    # -------------------------------------------------------------------------
    # Value resolution
    # -------------------------------------------------------------------------

    def _resolve_value(self, v: dict, ctx: SlotContext) -> dict:
        """Normalize a ``MosaicValue`` dict into a ``ResolvedValue`` dict.

        Transformations:

        - ``color_hex`` → parsed RGBA integers (0-255 each)
        - ``dimension`` → already split into value + unit by the analyzer; passed through
        - ``ident`` → folded into ``string``
        - ``slot_ref`` → enriched with slot_type and is_loop_var flag
        - All others pass through unchanged
        """
        kind = v["kind"]

        if kind == "string":
            return {"kind": "string", "value": v["value"]}
        if kind == "number":
            return {"kind": "number", "value": v["value"]}
        if kind == "bool":
            return {"kind": "bool", "value": v["value"]}
        if kind == "ident":
            return {"kind": "string", "value": v["value"]}
        if kind == "dimension":
            return self._normalize_dimension(v["value"], v["unit"])
        if kind == "color_hex":
            return self._parse_color(v["value"])
        if kind == "enum":
            return {"kind": "enum", "namespace": v["namespace"], "member": v["member"]}
        if kind == "slot_ref":
            return self._resolve_slot_ref(v["slot_name"], ctx)

        raise MosaicVMError(f'Unknown value kind: "{kind}"')

    def _normalize_dimension(self, value: float, unit: str) -> dict:
        """Pass a dimension through as a resolved dimension dict.

        The analyzer has already split the DIMENSION token into value + unit,
        so we just validate the unit and pass it through.
        """
        # Permissive mode: unknown units are passed through as-is.
        return {"kind": "dimension", "value": value, "unit": unit}

    def _parse_color(self, hex_str: str) -> dict:
        """Parse a hex color string into RGBA integer components (0-255 each).

        Three-digit shorthand is expanded by doubling each digit::

            #rgb  → r = int(rr, 16), g = int(gg, 16), b = int(bb, 16)

        Examples::

            _parse_color("#fff")      → {"kind": "color", "r": 255, "g": 255, "b": 255, "a": 255}
            _parse_color("#2563eb")   → {"kind": "color", "r": 37,  "g": 99,  "b": 235, "a": 255}
            _parse_color("#00000080") → {"kind": "color", "r": 0,   "g": 0,   "b": 0,   "a": 128}
        """
        h = hex_str.lstrip("#")
        a = 255

        if len(h) == 3:
            r = int(h[0] * 2, 16)
            g = int(h[1] * 2, 16)
            b = int(h[2] * 2, 16)
        elif len(h) == 6:
            r = int(h[0:2], 16)
            g = int(h[2:4], 16)
            b = int(h[4:6], 16)
        elif len(h) == 8:
            r = int(h[0:2], 16)
            g = int(h[2:4], 16)
            b = int(h[4:6], 16)
            a = int(h[6:8], 16)
        else:
            raise MosaicVMError(f"Invalid color hex: {hex_str}")

        return {"kind": "color", "r": r, "g": g, "b": b, "a": a}

    def _resolve_slot_ref(self, slot_name: str, ctx: SlotContext) -> dict:
        """Resolve a slot reference to a typed ResolvedValue.

        Checks loop scopes innermost-first, then component slots.
        """
        # 1. Check active loop scopes, innermost first.
        for scope in reversed(ctx.loop_scopes):
            if scope.item_name == slot_name:
                return {
                    "kind": "slot_ref",
                    "slot_name": slot_name,
                    "slot_type": scope.element_type,
                    "is_loop_var": True,
                }

        # 2. Fall back to component slots.
        slot = ctx.component_slots.get(slot_name)
        if slot is None:
            raise MosaicVMError(f"Unresolved slot reference: @{slot_name}")

        return {
            "kind": "slot_ref",
            "slot_name": slot_name,
            "slot_type": slot.type,
            "is_loop_var": False,
        }

    def _resolve_slot(self, slot_name: str, ctx: SlotContext) -> MosaicSlot:
        """Look up a named slot for ``render_slot_child``.

        Called when a slot reference appears as a child (``@header;``), not
        as a property value. Must find a component slot (not a loop var).
        """
        slot = ctx.component_slots.get(slot_name)
        if slot is None:
            raise MosaicVMError(f"Unknown slot: @{slot_name}")
        return slot
