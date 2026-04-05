"""mosaic-analyzer — Validates the Mosaic AST and produces a typed MosaicIR.

The analyzer is the third stage of the Mosaic compiler pipeline:

    Source text → Lexer → Tokens → Parser → ASTNode → **Analyzer** → MosaicIR

What the Analyzer Does
----------------------

The AST produced by the parser is a faithful, unvalidated reflection of the
source text. Every keyword, semicolon, and brace is present as a token leaf.
The analyzer's job is to:

1. **Strip syntax noise** — discard KEYWORD, SEMICOLON, LBRACE/RBRACE and
   keep only semantically meaningful tokens.
2. **Resolve types** — convert ``"text"``, ``"bool"``, etc. to typed
   ``MosaicType`` dict values like ``{"kind": "text"}``.
3. **Normalize values** — parse ``"16dp"`` → ``{"kind": "dimension", "value": 16, "unit": "dp"}``,
   ``"#2563eb"`` → ``{"kind": "color_hex", "value": "#2563eb"}``, etc.
4. **Determine required/optional** — slots with defaults are optional;
   slots without are required.
5. **Identify primitives** — classify nodes as primitive (Row, Column, Text…)
   or component (imported types).

IR Data Model
-------------

All IR types are Python ``dataclass`` objects:

- ``MosaicIR`` — top-level result: one component + list of imports
- ``MosaicComponent`` — name, slots, root node tree
- ``MosaicSlot`` — name, type dict, optional default value dict, required flag
- ``MosaicNode`` — tag name, is_primitive flag, properties list, children list
- ``MosaicProperty`` — name and value dict
- ``MosaicImport`` — component name, optional alias, path string

Types are represented as dicts with a ``"kind"`` key:
    ``{"kind": "text"}``
    ``{"kind": "list", "element_type": {"kind": "number"}}``
    ``{"kind": "component", "name": "Button"}``

Values use a similar ``"kind"``-keyed structure:
    ``{"kind": "string", "value": "hello"}``
    ``{"kind": "dimension", "value": 16, "unit": "dp"}``
    ``{"kind": "color_hex", "value": "#2563eb"}``
    ``{"kind": "slot_ref", "slot_name": "title"}``
    ``{"kind": "bool", "value": True}``
    ``{"kind": "number", "value": 42}``
    ``{"kind": "ident", "value": "center"}``
    ``{"kind": "enum", "namespace": "heading", "member": "large"}``

Children are dicts with a ``"kind"`` key:
    ``{"kind": "node", "node": MosaicNode(...)}``
    ``{"kind": "slot_ref", "slot_name": "header"}``
    ``{"kind": "when", "slot_name": "show", "body": [...]}``
    ``{"kind": "each", "slot_name": "items", "item_name": "item", "body": [...]}``

Usage::

    from mosaic_analyzer import analyze

    ir = analyze('''
        component Label {
            slot text: text;
            Text { content: @text; }
        }
    ''')
    print(ir.component.name)           # "Label"
    print(ir.component.slots[0].type)  # {"kind": "text"}
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from lang_parser import ASTNode
from mosaic_parser import parse as parse_mosaic

__version__ = "0.1.0"

__all__ = [
    "analyze",
    "AnalysisError",
    "MosaicIR",
    "MosaicComponent",
    "MosaicSlot",
    "MosaicNode",
    "MosaicProperty",
    "MosaicImport",
    "PRIMITIVE_NODES",
]

# ---------------------------------------------------------------------------
# Primitive node registry
# ---------------------------------------------------------------------------

#: The set of built-in layout and display elements.
#: All other tag names are treated as composite (imported) components.
PRIMITIVE_NODES: frozenset[str] = frozenset([
    "Row", "Column", "Box", "Stack",
    "Text", "Image", "Icon",
    "Spacer", "Divider", "Scroll",
])

# ---------------------------------------------------------------------------
# IR Data Classes
# ---------------------------------------------------------------------------


@dataclass
class MosaicImport:
    """An ``import X [as Y] from "path";`` declaration.

    Attributes:
        name: The exported component name (``X`` in ``import X from …``).
        alias: Optional local alias (``Y`` in ``import X as Y from …``).
        path: Relative or absolute path to the ``.mosaic`` source file.

    Example — ``import Button from "./button.mosaic"``::

        MosaicImport(name="Button", alias=None, path="./button.mosaic")
    """

    name: str
    alias: str | None = None
    path: str = ""


@dataclass
class MosaicSlot:
    """A typed data slot — the "props API" of a Mosaic component.

    Attributes:
        name: Slot name (kebab-case), e.g. ``"avatar-url"``.
        type: Type dict with ``"kind"`` key, e.g. ``{"kind": "text"}``.
        default_value: Optional default value dict. ``None`` means required.
        required: ``True`` when ``default_value`` is ``None``.

    Example — ``slot title: text;``::

        MosaicSlot(name="title", type={"kind": "text"},
                   default_value=None, required=True)
    """

    name: str
    type: dict  # noqa: A003
    default_value: dict | None = None
    required: bool = True


@dataclass
class MosaicProperty:
    """A single property assignment on a node (``name: value;``).

    Attributes:
        name: Property name, kebab-case (e.g., ``"corner-radius"``).
        value: Resolved value dict with ``"kind"`` key.
    """

    name: str
    value: dict


@dataclass
class MosaicNode:
    """A visual node in the component tree.

    Nodes correspond to platform-native elements. Primitive nodes are built-in
    layout/display elements (Row, Column, Text, etc.). Non-primitive nodes are
    imported component types.

    Attributes:
        node_type: Element tag name, e.g. ``"Column"``, ``"Text"``.
        is_primitive: ``True`` for built-in elements.
        properties: List of ``MosaicProperty`` objects.
        children: List of child dicts (``"kind"`` ∈ ``{"node","slot_ref","when","each"}``).
    """

    node_type: str
    is_primitive: bool = False
    properties: list[MosaicProperty] = field(default_factory=list)
    children: list[dict] = field(default_factory=list)


@dataclass
class MosaicComponent:
    """A Mosaic component — the unit of UI composition.

    Attributes:
        name: PascalCase component name (e.g., ``"ProfileCard"``).
        slots: Ordered list of ``MosaicSlot`` declarations.
        imports: All import declarations from the file.
        root: Root node of the visual tree.
    """

    name: str
    slots: list[MosaicSlot] = field(default_factory=list)
    imports: list[MosaicImport] = field(default_factory=list)
    root: MosaicNode | None = None


@dataclass
class MosaicIR:
    """The root of the intermediate representation.

    Contains one component (the one declared in this ``.mosaic`` file) and
    all import declarations.

    Attributes:
        component: The single component declared in the file.
        imports: All ``import X from "..."`` declarations.
    """

    component: MosaicComponent
    imports: list[MosaicImport] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Error Class
# ---------------------------------------------------------------------------


class AnalysisError(Exception):
    """Raised when the analyzer encounters a structural problem in the AST.

    These are "should not happen" errors — they indicate either a bug in the
    parser or a malformed AST. Syntactic errors are caught earlier by the
    parser.
    """


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def analyze(source: str) -> MosaicIR:
    """Analyze Mosaic source text and return a typed ``MosaicIR``.

    This is the main entry point. It parses the source, then analyzes the
    resulting AST to produce a validated intermediate representation.

    Args:
        source: The ``.mosaic`` source text.

    Returns:
        A ``MosaicIR`` ready for code generation.

    Raises:
        LexerError: If the source contains invalid tokens.
        GrammarParseError: If the source is syntactically invalid.
        AnalysisError: If the AST contains structural errors.

    Example::

        ir = analyze('''
            component Label {
                slot text: text;
                Text { content: @text; }
            }
        ''')
        print(ir.component.name)  # "Label"
    """
    ast = parse_mosaic(source)
    return _analyze_file(ast)


# ---------------------------------------------------------------------------
# File-level analysis
# ---------------------------------------------------------------------------


def _analyze_file(ast: ASTNode) -> MosaicIR:
    if ast.rule_name != "file":
        raise AnalysisError(f'Expected root rule "file", got "{ast.rule_name}"')

    imports: list[MosaicImport] = []
    component_decl = None

    for child in ast.children:
        if isinstance(child, ASTNode):
            if child.rule_name == "import_decl":
                imports.append(_analyze_import(child))
            elif child.rule_name == "component_decl":
                component_decl = child

    if component_decl is None:
        raise AnalysisError("No component declaration found in file")

    component = _analyze_component(component_decl, imports)
    return MosaicIR(component=component, imports=imports)


# ---------------------------------------------------------------------------
# Import analysis
# ---------------------------------------------------------------------------


def _analyze_import(node: ASTNode) -> MosaicImport:
    # import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON
    # Tokens: "import"  NAME  ["as" NAME]  "from"  STRING  ";"
    names = _token_values(node, "NAME")
    strings = _token_values(node, "STRING")

    if not names:
        raise AnalysisError("import_decl missing component name")
    if not strings:
        raise AnalysisError("import_decl missing path")

    name = names[0]
    alias = names[1] if len(names) >= 2 else None
    path = strings[0]
    return MosaicImport(name=name, alias=alias, path=path)


# ---------------------------------------------------------------------------
# Component analysis
# ---------------------------------------------------------------------------


def _analyze_component(node: ASTNode, imports: list[MosaicImport]) -> MosaicComponent:
    # component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE
    names = _token_values(node, "NAME")
    if not names:
        raise AnalysisError("component_decl missing name")

    name = names[0]
    slots: list[MosaicSlot] = []
    tree_node = None

    for child in node.children:
        if isinstance(child, ASTNode):
            if child.rule_name == "slot_decl":
                slots.append(_analyze_slot(child))
            elif child.rule_name == "node_tree":
                tree_node = child

    if tree_node is None:
        raise AnalysisError(f'component "{name}" has no node tree')

    root = _analyze_node_tree(tree_node)
    return MosaicComponent(name=name, slots=slots, imports=imports, root=root)


# ---------------------------------------------------------------------------
# Slot analysis
# ---------------------------------------------------------------------------


def _analyze_slot(node: ASTNode) -> MosaicSlot:
    # slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
    names = _token_values(node, "NAME")
    if not names:
        raise AnalysisError("slot_decl missing name")

    name = names[0]
    slot_type_node = _find_child(node, "slot_type")
    if slot_type_node is None:
        raise AnalysisError(f'slot "{name}" missing type')

    slot_type = _analyze_slot_type(slot_type_node)
    default_value_node = _find_child(node, "default_value")
    default_value = _analyze_default_value(default_value_node) if default_value_node else None
    required = default_value is None

    return MosaicSlot(name=name, type=slot_type, default_value=default_value, required=required)


def _analyze_slot_type(node: ASTNode) -> dict:
    # slot_type = list_type | KEYWORD | NAME
    list_type_node = _find_child(node, "list_type")
    if list_type_node is not None:
        return _analyze_list_type(list_type_node)

    keyword = _first_token_value(node, "KEYWORD")
    if keyword:
        return _parse_primitive_type(keyword)

    name = _first_token_value(node, "NAME")
    if name:
        return {"kind": "component", "name": name}

    raise AnalysisError("slot_type has no recognizable content")


def _analyze_list_type(node: ASTNode) -> dict:
    # list_type = KEYWORD LANGLE slot_type RANGLE
    element_type_node = _find_child(node, "slot_type")
    if element_type_node is None:
        raise AnalysisError("list_type missing element type")

    element_type = _analyze_slot_type(element_type_node)
    return {"kind": "list", "element_type": element_type}


def _parse_primitive_type(keyword: str) -> dict:
    """Convert a type keyword to a MosaicType dict.

    Recognized keywords: ``text``, ``number``, ``bool``, ``image``,
    ``color``, ``node``.

    Raises:
        AnalysisError: If the keyword is not a recognized primitive type.
    """
    primitives = {"text", "number", "bool", "image", "color", "node"}
    if keyword in primitives:
        return {"kind": keyword}
    raise AnalysisError(f'Unknown primitive type keyword: "{keyword}"')


def _analyze_default_value(node: ASTNode) -> dict:
    # default_value = STRING | DIMENSION | NUMBER | COLOR_HEX | KEYWORD
    s = _first_token_value(node, "STRING")
    if s is not None:
        return {"kind": "string", "value": s}

    dim = _first_token_value(node, "DIMENSION")
    if dim is not None:
        return _parse_dimension(dim)

    num = _first_token_value(node, "NUMBER")
    if num is not None:
        return {"kind": "number", "value": float(num)}

    color = _first_token_value(node, "COLOR_HEX")
    if color is not None:
        return {"kind": "color_hex", "value": color}

    kw = _first_token_value(node, "KEYWORD")
    if kw == "true":
        return {"kind": "bool", "value": True}
    if kw == "false":
        return {"kind": "bool", "value": False}

    raise AnalysisError("default_value has no recognizable content")


# ---------------------------------------------------------------------------
# Node tree analysis
# ---------------------------------------------------------------------------


def _analyze_node_tree(node: ASTNode) -> MosaicNode:
    # node_tree = node_element
    element = _find_child(node, "node_element")
    if element is None:
        raise AnalysisError("node_tree missing node_element")
    return _analyze_node_element(element)


def _analyze_node_element(node: ASTNode) -> MosaicNode:
    # node_element = NAME LBRACE { node_content } RBRACE
    tag = _first_token_value(node, "NAME")
    if not tag:
        raise AnalysisError("node_element missing tag name")

    is_primitive = tag in PRIMITIVE_NODES
    properties: list[MosaicProperty] = []
    children: list[dict] = []

    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == "node_content":
            prop, child_item = _analyze_node_content(child)
            if prop is not None:
                properties.append(prop)
            if child_item is not None:
                children.append(child_item)

    return MosaicNode(node_type=tag, is_primitive=is_primitive, properties=properties, children=children)


def _analyze_node_content(node: ASTNode) -> tuple[MosaicProperty | None, dict | None]:
    # node_content = when_block | each_block | slot_reference | child_node | property_assignment
    for child in node.children:
        if not isinstance(child, ASTNode):
            continue

        if child.rule_name == "property_assignment":
            return _analyze_property_assignment(child), None

        if child.rule_name == "child_node":
            element = _find_child(child, "node_element")
            if element:
                return None, {"kind": "node", "node": _analyze_node_element(element)}

        if child.rule_name == "slot_reference":
            name = _first_token_value(child, "NAME")
            if name:
                return None, {"kind": "slot_ref", "slot_name": name}

        if child.rule_name == "when_block":
            return None, _analyze_when_block(child)

        if child.rule_name == "each_block":
            return None, _analyze_each_block(child)

    return None, None


# ---------------------------------------------------------------------------
# Property analysis
# ---------------------------------------------------------------------------


def _analyze_property_assignment(node: ASTNode) -> MosaicProperty:
    # property_assignment = ( NAME | KEYWORD ) COLON property_value SEMICOLON
    name = _first_token_value(node, "NAME") or _first_token_value(node, "KEYWORD")
    if not name:
        raise AnalysisError("property_assignment missing name")

    value_node = _find_child(node, "property_value")
    if value_node is None:
        raise AnalysisError(f'property "{name}" missing value')

    value = _analyze_property_value(value_node)
    return MosaicProperty(name=name, value=value)


def _analyze_property_value(node: ASTNode) -> dict:
    # property_value = slot_ref | enum_value | STRING | DIMENSION | NUMBER
    #                | COLOR_HEX | KEYWORD | NAME
    for child in node.children:
        if not isinstance(child, ASTNode):
            continue

        if child.rule_name == "slot_ref":
            slot_name = _first_token_value(child, "NAME")
            if slot_name:
                return {"kind": "slot_ref", "slot_name": slot_name}

        if child.rule_name == "enum_value":
            names = _token_values(child, "NAME")
            if len(names) >= 2:
                return {"kind": "enum", "namespace": names[0], "member": names[1]}

    s = _first_token_value(node, "STRING")
    if s is not None:
        return {"kind": "string", "value": s}

    dim = _first_token_value(node, "DIMENSION")
    if dim is not None:
        return _parse_dimension(dim)

    num = _first_token_value(node, "NUMBER")
    if num is not None:
        return {"kind": "number", "value": float(num)}

    color = _first_token_value(node, "COLOR_HEX")
    if color is not None:
        return {"kind": "color_hex", "value": color}

    kw = _first_token_value(node, "KEYWORD")
    if kw == "true":
        return {"kind": "bool", "value": True}
    if kw == "false":
        return {"kind": "bool", "value": False}
    if kw is not None:
        return {"kind": "ident", "value": kw}

    ident = _first_token_value(node, "NAME")
    if ident is not None:
        return {"kind": "ident", "value": ident}

    raise AnalysisError("property_value has no recognizable content")


# ---------------------------------------------------------------------------
# When / Each analysis
# ---------------------------------------------------------------------------


def _analyze_when_block(node: ASTNode) -> dict:
    # when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE
    slot_ref_node = _find_child(node, "slot_ref")
    if slot_ref_node is None:
        raise AnalysisError("when_block missing slot_ref")

    slot_name = _first_token_value(slot_ref_node, "NAME")
    if not slot_name:
        raise AnalysisError("when_block slot_ref missing name")

    body = _collect_node_contents(node)
    return {"kind": "when", "slot_name": slot_name, "body": body}


def _analyze_each_block(node: ASTNode) -> dict:
    # each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
    slot_ref_node = _find_child(node, "slot_ref")
    if slot_ref_node is None:
        raise AnalysisError("each_block missing slot_ref")

    slot_name = _first_token_value(slot_ref_node, "NAME")
    if not slot_name:
        raise AnalysisError("each_block slot_ref missing name")

    # Find the loop variable: the NAME token that is a *direct child* of the
    # each_block (not inside slot_ref). It comes after the "as" keyword.
    item_name = _find_loop_variable(node)
    if not item_name:
        raise AnalysisError("each_block missing loop variable")

    body = _collect_node_contents(node)
    return {"kind": "each", "slot_name": slot_name, "item_name": item_name, "body": body}


def _find_loop_variable(each_block: ASTNode) -> str | None:
    """Find the loop variable NAME in an each_block.

    The structure is: KEYWORD(each) slot_ref KEYWORD(as) NAME(item) LBRACE...
    We need the NAME that is a direct child of each_block (not inside slot_ref).
    """
    after_as = False
    for child in each_block.children:
        if isinstance(child, ASTNode):
            # Skip ASTNode subtrees (including slot_ref)
            continue
        # Direct-child token
        if child.type_name == "KEYWORD" and child.value == "as":
            after_as = True
            continue
        if after_as and child.type_name == "NAME":
            return child.value
    return None


def _collect_node_contents(node: ASTNode) -> list[dict]:
    """Collect all child dicts from node_content children of a block."""
    children: list[dict] = []
    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == "node_content":
            _prop, child_item = _analyze_node_content(child)
            if child_item is not None:
                children.append(child_item)
    return children


# ---------------------------------------------------------------------------
# Value parsing helpers
# ---------------------------------------------------------------------------


_DIMENSION_RE = re.compile(r"^(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)$")


def _parse_dimension(raw: str) -> dict:
    """Parse a DIMENSION token like ``"16dp"`` into a structured dict.

    Examples::

        _parse_dimension("16dp")   → {"kind": "dimension", "value": 16.0, "unit": "dp"}
        _parse_dimension("1.5sp")  → {"kind": "dimension", "value": 1.5,  "unit": "sp"}
        _parse_dimension("100%")   → {"kind": "dimension", "value": 100.0, "unit": "%"}
    """
    m = _DIMENSION_RE.match(raw)
    if not m:
        raise AnalysisError(f'Invalid DIMENSION token: "{raw}"')
    return {"kind": "dimension", "value": float(m.group(1)), "unit": m.group(2)}


# ---------------------------------------------------------------------------
# AST traversal helpers
# ---------------------------------------------------------------------------


def _find_child(node: ASTNode, rule_name: str) -> ASTNode | None:
    """Find the first direct child ASTNode with the given rule_name."""
    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == rule_name:
            return child
    return None


def _token_values(node: ASTNode, type_name: str) -> list[str]:
    """Collect all direct-child token values with the given type_name."""
    return [
        child.value
        for child in node.children
        if not isinstance(child, ASTNode) and child.type_name == type_name
    ]


def _first_token_value(node: ASTNode, type_name: str) -> str | None:
    """Get the first direct-child token value with the given type_name."""
    for child in node.children:
        if not isinstance(child, ASTNode) and child.type_name == type_name:
            return child.value
    return None
