"""CSS emitter — reconstructs CSS text from a clean AST.

After the transformer has expanded all Lattice nodes (variables, mixins,
control flow, functions), the AST contains only pure CSS nodes:

- ``stylesheet`` — the root
- ``qualified_rule`` — selector + block (e.g., ``h1 { color: red; }``)
- ``at_rule`` — @-rules (e.g., ``@media``, ``@import``)
- ``selector_list`` — comma-separated selectors
- ``complex_selector`` — compound selectors with combinators
- ``compound_selector`` — type/class/id/pseudo selectors
- ``block`` — ``{ declarations }``
- ``declaration`` — ``property: value;``
- ``value_list`` — space-separated values
- ``function_call`` — ``rgb(255, 0, 0)``
- ``priority`` — ``!important``

The emitter walks this tree and produces formatted CSS text.

How It Works
------------

The emitter dispatches on ``rule_name``. Each rule has a handler method
that knows how to format that particular CSS construct. Unknown rules
fall through to a default handler that recurses into children.

Two formatting modes are supported:

- **Pretty-print** (default): 2-space indentation, newlines between
  declarations, blank lines between rules. This is the human-readable
  output you'd want during development.

- **Minified**: No unnecessary whitespace. Every byte counts for
  production CSS.

Example::

    emitter = CSSEmitter()
    css_text = emitter.emit(clean_ast)
    # h1 {
    #   color: red;
    #   font-size: 16px;
    # }

Design Note
-----------

The emitter assumes the AST is clean — no Lattice nodes remain. If a
Lattice node is encountered, it's silently skipped. The transformer is
responsible for removing all Lattice nodes before the emitter runs.
"""

from __future__ import annotations


class CSSEmitter:
    """Emits CSS text from a clean AST.

    The emitter walks the AST recursively, dispatching on ``rule_name``
    to produce properly formatted CSS output.

    Args:
        indent: The indentation string per level (default: 2 spaces).
        minified: If True, emit minified CSS with no unnecessary whitespace.
    """

    def __init__(self, indent: str = "  ", minified: bool = False) -> None:
        self.indent = indent
        self.minified = minified

    def emit(self, node: object) -> str:
        """Emit CSS text from an AST node.

        This is the main entry point. Pass the root ``stylesheet`` node
        and get back a complete CSS string.

        Args:
            node: An ASTNode (typically the root ``stylesheet``).

        Returns:
            Formatted CSS text.
        """
        result = self._emit_node(node, depth=0)
        return result.strip() + "\n" if result.strip() else ""

    def _emit_node(self, node: object, depth: int = 0) -> str:
        """Dispatch to the appropriate handler based on rule_name.

        If the node is a token (no rule_name), return its text value.
        If the rule_name has a specific handler, use it. Otherwise,
        fall through to the default handler.
        """
        # Raw token — return its value
        if not hasattr(node, "rule_name"):
            return node.value  # type: ignore[attr-defined]

        rule = node.rule_name  # type: ignore[attr-defined]

        # Dispatch to specific handler
        handler = getattr(self, f"_emit_{rule}", None)
        if handler:
            return handler(node, depth)

        # Default: recurse children and concatenate
        return self._emit_default(node, depth)

    # -----------------------------------------------------------------
    # Top-Level Structure
    # -----------------------------------------------------------------

    def _emit_stylesheet(self, node: object, depth: int) -> str:
        """stylesheet = { rule } ;

        Join rules with blank lines (pretty) or nothing (minified).
        """
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            text = self._emit_node(child, depth)
            if text.strip():
                parts.append(text)

        if self.minified:
            return "".join(parts)
        return "\n\n".join(parts)

    def _emit_rule(self, node: object, depth: int) -> str:
        """rule = lattice_rule | at_rule | qualified_rule ;

        A rule is a wrapper — just emit the single child.
        """
        children = node.children  # type: ignore[attr-defined]
        if children:
            return self._emit_node(children[0], depth)
        return ""

    # -----------------------------------------------------------------
    # Qualified Rules (selector + block)
    # -----------------------------------------------------------------

    def _emit_qualified_rule(self, node: object, depth: int) -> str:
        """qualified_rule = selector_list block ;

        Emits: selector_list {
                 declarations...
               }
        """
        children = node.children  # type: ignore[attr-defined]
        parts: list[str] = []

        for child in children:
            if hasattr(child, "rule_name"):
                rule = child.rule_name
                if rule == "selector_list":
                    parts.append(self._emit_node(child, depth))
                elif rule == "block":
                    parts.append(self._emit_block(child, depth))
                else:
                    parts.append(self._emit_node(child, depth))
            else:
                parts.append(child.value)

        if self.minified:
            return "".join(parts)

        # Pretty: selector_list + space + block
        selector = ""
        block = ""
        for child in children:
            if hasattr(child, "rule_name"):
                if child.rule_name == "selector_list":
                    selector = self._emit_node(child, depth)
                elif child.rule_name == "block":
                    block = self._emit_block(child, depth)
        return f"{selector} {block}" if selector else block

    # -----------------------------------------------------------------
    # At-Rules
    # -----------------------------------------------------------------

    def _emit_at_rule(self, node: object, depth: int) -> str:
        """at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;

        Two forms:
        - @import url("style.css");
        - @media (max-width: 768px) { ... }
        """
        children = node.children  # type: ignore[attr-defined]
        keyword = ""
        prelude = ""
        block_text = ""
        has_semicolon = False

        for child in children:
            if not hasattr(child, "rule_name"):
                # Token
                type_name = self._token_type(child)
                if type_name == "AT_KEYWORD":
                    keyword = child.value
                elif type_name == "SEMICOLON":
                    has_semicolon = True
            else:
                if child.rule_name == "at_prelude":
                    prelude = self._emit_at_prelude(child, depth)
                elif child.rule_name == "block":
                    block_text = self._emit_block(child, depth)

        if self.minified:
            if has_semicolon:
                return f"{keyword}{prelude};"
            return f"{keyword}{prelude}{block_text}"

        if has_semicolon:
            prelude_part = f" {prelude}" if prelude.strip() else ""
            return f"{keyword}{prelude_part};"
        prelude_part = f" {prelude}" if prelude.strip() else ""
        return f"{keyword}{prelude_part} {block_text}"

    def _emit_at_prelude(self, node: object, depth: int) -> str:
        """at_prelude = { at_prelude_token } ;

        Collect tokens and nodes, space-separate them.
        """
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        return " ".join(parts) if parts else ""

    def _emit_at_prelude_token(self, node: object, depth: int) -> str:
        """Single token in an at-rule prelude."""
        return self._emit_default(node, depth)

    def _emit_at_prelude_tokens(self, node: object, depth: int) -> str:
        """at_prelude_tokens = { at_prelude_token } ;"""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        return " ".join(parts)

    def _emit_function_in_prelude(self, node: object, depth: int) -> str:
        """function_in_prelude = FUNCTION at_prelude_tokens RPAREN ;"""
        children = node.children  # type: ignore[attr-defined]
        parts: list[str] = []
        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = self._token_type(child)
                if type_name == "RPAREN":
                    parts.append(")")
                else:
                    parts.append(child.value)
            else:
                parts.append(self._emit_node(child, depth))
        return "".join(parts)

    def _emit_paren_block(self, node: object, depth: int) -> str:
        """paren_block = LPAREN at_prelude_tokens RPAREN ;"""
        children = node.children  # type: ignore[attr-defined]
        parts: list[str] = []
        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = self._token_type(child)
                if type_name == "LPAREN":
                    parts.append("(")
                elif type_name == "RPAREN":
                    parts.append(")")
                else:
                    parts.append(child.value)
            else:
                parts.append(self._emit_node(child, depth))
        return "".join(parts)

    # -----------------------------------------------------------------
    # Selectors
    # -----------------------------------------------------------------

    def _emit_selector_list(self, node: object, depth: int) -> str:
        """selector_list = complex_selector { COMMA complex_selector } ;

        Comma-separate selectors.
        """
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                type_name = self._token_type(child)
                if type_name == "COMMA":
                    continue  # We add commas ourselves
            else:
                parts.append(self._emit_node(child, depth))
        sep = "," if self.minified else ", "
        return sep.join(parts)

    def _emit_complex_selector(self, node: object, depth: int) -> str:
        """complex_selector = compound_selector { [ combinator ] compound_selector } ;"""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        return " ".join(parts)

    def _emit_combinator(self, node: object, depth: int) -> str:
        """combinator = GREATER | PLUS | TILDE ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return children[0].value
        return ""

    def _emit_compound_selector(self, node: object, depth: int) -> str:
        """compound_selector = simple_selector { subclass_selector } | ... ;

        Concatenate without spaces: h1.classname#id
        """
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        return "".join(parts)

    def _emit_simple_selector(self, node: object, depth: int) -> str:
        """simple_selector = IDENT | STAR | AMPERSAND ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return children[0].value
        return ""

    def _emit_subclass_selector(self, node: object, depth: int) -> str:
        """subclass_selector — dispatch to child."""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return self._emit_node(children[0], depth)
        return ""

    def _emit_class_selector(self, node: object, depth: int) -> str:
        """class_selector = DOT IDENT ;"""
        children = node.children  # type: ignore[attr-defined]
        # DOT + IDENT → .classname
        parts = [child.value for child in children if not hasattr(child, "rule_name")]
        return "".join(parts)

    def _emit_id_selector(self, node: object, depth: int) -> str:
        """id_selector = HASH ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return children[0].value
        return ""

    def _emit_attribute_selector(self, node: object, depth: int) -> str:
        """attribute_selector = LBRACKET IDENT [ attr_matcher attr_value [ IDENT ] ] RBRACKET ;"""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            if hasattr(child, "rule_name"):
                parts.append(self._emit_node(child, depth))
            else:
                type_name = self._token_type(child)
                if type_name == "LBRACKET":
                    parts.append("[")
                elif type_name == "RBRACKET":
                    parts.append("]")
                else:
                    parts.append(child.value)
        return "".join(parts)

    def _emit_attr_matcher(self, node: object, depth: int) -> str:
        """attr_matcher = EQUALS | TILDE_EQUALS | ... ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return children[0].value
        return ""

    def _emit_attr_value(self, node: object, depth: int) -> str:
        """attr_value = IDENT | STRING ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            child = children[0]
            type_name = self._token_type(child)
            if type_name == "STRING":
                return f'"{child.value}"'
            return child.value
        return ""

    def _emit_pseudo_class(self, node: object, depth: int) -> str:
        """pseudo_class = COLON FUNCTION pseudo_class_args RPAREN | COLON IDENT ;"""
        children = node.children  # type: ignore[attr-defined]
        parts: list[str] = []
        for child in children:
            if hasattr(child, "rule_name"):
                parts.append(self._emit_node(child, depth))
            else:
                type_name = self._token_type(child)
                if type_name == "COLON":
                    parts.append(":")
                elif type_name == "RPAREN":
                    parts.append(")")
                else:
                    parts.append(child.value)
        return "".join(parts)

    def _emit_pseudo_class_args(self, node: object, depth: int) -> str:
        """pseudo_class_args = { pseudo_class_arg } ;"""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        return "".join(parts)

    def _emit_pseudo_class_arg(self, node: object, depth: int) -> str:
        """Single arg in pseudo-class."""
        return self._emit_default(node, depth)

    def _emit_pseudo_element(self, node: object, depth: int) -> str:
        """pseudo_element = COLON_COLON IDENT ;"""
        children = node.children  # type: ignore[attr-defined]
        parts: list[str] = []
        for child in children:
            type_name = self._token_type(child)
            if type_name == "COLON_COLON":
                parts.append("::")
            else:
                parts.append(child.value)
        return "".join(parts)

    # -----------------------------------------------------------------
    # Blocks and Declarations
    # -----------------------------------------------------------------

    def _emit_block(self, node: object, depth: int) -> str:
        """block = LBRACE block_contents RBRACE ;

        Emits { declarations } with proper indentation.
        """
        children = node.children  # type: ignore[attr-defined]

        # Find block_contents node
        contents = None
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "block_contents":
                contents = child
                break

        if self.minified:
            if contents is None:
                return "{}"
            inner = self._emit_block_contents(contents, depth + 1)
            return "{" + inner + "}"

        if contents is None:
            return "{\n" + self.indent * depth + "}"

        inner = self._emit_block_contents(contents, depth + 1)
        if not inner.strip():
            return "{\n" + self.indent * depth + "}"
        return "{\n" + inner + "\n" + self.indent * depth + "}"

    def _emit_block_contents(self, node: object, depth: int) -> str:
        """block_contents = { block_item } ;"""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            text = self._emit_node(child, depth)
            if text.strip():
                parts.append(text)

        if self.minified:
            return "".join(parts)

        prefix = self.indent * depth
        return "\n".join(f"{prefix}{part}" for part in parts)

    def _emit_block_item(self, node: object, depth: int) -> str:
        """block_item = lattice_block_item | at_rule | declaration_or_nested ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return self._emit_node(children[0], depth)
        return ""

    def _emit_declaration_or_nested(self, node: object, depth: int) -> str:
        """declaration_or_nested = declaration | qualified_rule ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return self._emit_node(children[0], depth)
        return ""

    def _emit_declaration(self, node: object, depth: int) -> str:
        """declaration = property COLON value_list [ priority ] SEMICOLON ;

        Emits: property: value_list;
        """
        children = node.children  # type: ignore[attr-defined]
        prop = ""
        value = ""
        priority = ""

        for child in children:
            if hasattr(child, "rule_name"):
                if child.rule_name == "property":
                    prop = self._emit_property(child, depth)
                elif child.rule_name == "value_list":
                    value = self._emit_value_list(child, depth)
                elif child.rule_name == "priority":
                    priority = " !important"
            # Skip COLON and SEMICOLON tokens

        if self.minified:
            return f"{prop}:{value}{priority};"
        return f"{prop}: {value}{priority};"

    def _emit_property(self, node: object, depth: int) -> str:
        """property = IDENT | CUSTOM_PROPERTY ;"""
        children = node.children  # type: ignore[attr-defined]
        if children:
            return children[0].value
        return ""

    def _emit_priority(self, node: object, depth: int) -> str:
        """priority = BANG "important" ;"""
        return "!important"

    # -----------------------------------------------------------------
    # Values
    # -----------------------------------------------------------------

    def _emit_value_list(self, node: object, depth: int) -> str:
        """value_list = value { value } ;

        Space-separate values, but commas don't need extra spaces.
        """
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            text = self._emit_node(child, depth)
            parts.append(text)

        # Collapse spaces around commas
        result = " ".join(parts)
        result = result.replace(" , ", ", ").replace(" ,", ",")
        return result

    def _emit_value(self, node: object, depth: int) -> str:
        """value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH | ... ;"""
        children = node.children  # type: ignore[attr-defined]
        if len(children) == 1:
            child = children[0]
            if hasattr(child, "rule_name"):
                return self._emit_node(child, depth)
            type_name = self._token_type(child)
            if type_name == "STRING":
                return f'"{child.value}"'
            return child.value
        return self._emit_default(node, depth)

    def _emit_function_call(self, node: object, depth: int) -> str:
        """function_call = FUNCTION function_args RPAREN | URL_TOKEN ;"""
        children = node.children  # type: ignore[attr-defined]

        if len(children) == 1:
            # URL_TOKEN
            return children[0].value

        parts: list[str] = []
        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = self._token_type(child)
                if type_name == "FUNCTION":
                    parts.append(child.value)  # Includes "("
                elif type_name == "RPAREN":
                    parts.append(")")
                else:
                    parts.append(child.value)
            else:
                parts.append(self._emit_node(child, depth))
        return "".join(parts)

    def _emit_function_args(self, node: object, depth: int) -> str:
        """function_args = { function_arg } ;"""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        result = " ".join(parts)
        result = result.replace(" , ", ", ").replace(" ,", ",")
        return result

    def _emit_function_arg(self, node: object, depth: int) -> str:
        """Single argument in a function call."""
        children = node.children  # type: ignore[attr-defined]
        if len(children) == 1:
            child = children[0]
            if hasattr(child, "rule_name"):
                return self._emit_node(child, depth)
            return child.value
        return self._emit_default(node, depth)

    # -----------------------------------------------------------------
    # Default and Utilities
    # -----------------------------------------------------------------

    def _emit_default(self, node: object, depth: int) -> str:
        """Default handler: concatenate children with spaces."""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._emit_node(child, depth))
        return " ".join(parts)

    def _token_type(self, token: object) -> str:
        """Get the string name of a token's type."""
        t = token.type  # type: ignore[attr-defined]
        if isinstance(t, str):
            return t
        return t.name
