"""Lattice AST transformer — expands Lattice constructs into pure CSS.

This is the core of the Lattice-to-CSS compiler. It takes a Lattice AST
(containing both CSS and Lattice nodes) and produces a clean CSS AST
(containing only CSS nodes) by expanding all Lattice constructs.

Three-Pass Architecture
-----------------------

The transformation runs in three passes:

**Pass 1: Symbol Collection**
Walk the top-level AST and collect definitions:
- Variable declarations → variable registry
- Mixin definitions → mixin registry
- Function definitions → function registry
Remove definition nodes from the AST (they produce no CSS output).

**Pass 2: Expansion**
Recursively walk remaining AST nodes with a scope chain:
- Replace ``VARIABLE`` tokens with their resolved values
- Expand ``@include`` directives by cloning mixin bodies
- Evaluate ``@if``/``@for``/``@each`` control flow
- Evaluate Lattice function calls and replace with return values

After this pass, the AST contains only pure CSS nodes.

**Pass 3: Cleanup**
Remove any empty blocks or rules that resulted from transformation.

Why Not a Single Pass?
Because mixins and functions can be defined after they're used::

    .btn { @include button(red); }   ← used first
    @mixin button($bg) { ... }       ← defined later

Pass 1 collects all definitions up front, so Pass 2 can resolve them
regardless of source order.

Cycle Detection
---------------

Mixin and function expansion tracks a call stack. If a name appears twice
in the stack, a ``CircularReferenceError`` is raised::

    @mixin a { @include b; }
    @mixin b { @include a; }    ← Circular mixin: a → b → a
"""

from __future__ import annotations

import copy
from typing import Any

from lattice_ast_to_css.errors import (
    CircularReferenceError,
    ExtendTargetNotFoundError,
    MaxIterationError,
    MissingReturnError,
    UndefinedFunctionError,
    UndefinedMixinError,
    UndefinedVariableError,
    WrongArityError,
)
from lattice_ast_to_css.evaluator import (
    BUILTIN_FUNCTIONS,
    ExpressionEvaluator,
    LatticeColor,
    LatticeDimension,
    LatticeIdent,
    LatticeList,
    LatticeMap,
    LatticeNull,
    LatticeNumber,
    LatticePercentage,
    LatticeString,
    LatticeValue,
    is_truthy,
    token_to_value,
    value_to_css,
)
from lattice_ast_to_css.scope import ScopeChain


# ---------------------------------------------------------------------------
# Configuration Constants
# ---------------------------------------------------------------------------

# Maximum number of iterations allowed in a @while loop.
# Prevents infinite loops from consuming all memory/CPU at compile time.
# This is configurable via the LatticeTransformer constructor.
MAX_WHILE_ITERATIONS = 1000


# ---------------------------------------------------------------------------
# Token type helper
# ---------------------------------------------------------------------------


def _token_type_name(token: object) -> str:
    """Get the string name of a token's type."""
    t = token.type  # type: ignore[attr-defined]
    if isinstance(t, str):
        return t
    return t.name


def _get_token_value(child: object) -> str | None:
    """Get the .value of a token, or None if it's an ASTNode."""
    if hasattr(child, "rule_name"):
        return None
    return child.value  # type: ignore[attr-defined]


# ---------------------------------------------------------------------------
# Mixin / Function Definition Records
# ---------------------------------------------------------------------------


class MixinDef:
    """Stored definition of a @mixin.

    Captures the parameter names, default values, and body AST node.
    """

    def __init__(
        self,
        name: str,
        params: list[str],
        defaults: dict[str, Any],
        body: object,
    ) -> None:
        self.name = name
        self.params = params
        self.defaults = defaults
        self.body = body


class FunctionDef:
    """Stored definition of a @function.

    Same structure as MixinDef, but functions return values via @return
    rather than emitting CSS declarations.
    """

    def __init__(
        self,
        name: str,
        params: list[str],
        defaults: dict[str, Any],
        body: object,
    ) -> None:
        self.name = name
        self.params = params
        self.defaults = defaults
        self.body = body


# ---------------------------------------------------------------------------
# Sentinel for @return
# ---------------------------------------------------------------------------


class ReturnSignal(Exception):
    """Internal signal for @return inside function evaluation.

    Not a real error — used to unwind the function body evaluation
    when a @return is hit. The value is the LatticeValue to return.
    """

    def __init__(self, value: LatticeValue) -> None:
        self.value = value
        super().__init__()


# ---------------------------------------------------------------------------
# CSS Function Names (not Lattice functions)
# ---------------------------------------------------------------------------

# These are CSS built-in functions that should NOT be resolved as Lattice
# functions. When a function_call node uses one of these names, it's passed
# through unchanged.
CSS_FUNCTIONS = frozenset({
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklch", "oklab",
    "color", "color-mix",
    "calc", "min", "max", "clamp", "abs", "sign", "round", "mod", "rem",
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt",
    "hypot", "log", "exp",
    "var", "env",
    "url", "format", "local",
    "linear-gradient", "radial-gradient", "conic-gradient",
    "repeating-linear-gradient", "repeating-radial-gradient",
    "repeating-conic-gradient",
    "counter", "counters", "attr", "element",
    "translate", "translateX", "translateY", "translateZ",
    "rotate", "rotateX", "rotateY", "rotateZ",
    "scale", "scaleX", "scaleY", "scaleZ",
    "skew", "skewX", "skewY",
    "matrix", "matrix3d", "perspective",
    "cubic-bezier", "steps",
    "path", "polygon", "circle", "ellipse", "inset",
    "image-set", "cross-fade",
    "fit-content", "minmax", "repeat",
    "blur", "brightness", "contrast", "drop-shadow", "grayscale",
    "hue-rotate", "invert", "opacity", "saturate", "sepia",
})


def _is_css_function(name: str) -> bool:
    """Check if a function name is a CSS built-in (not a Lattice function)."""
    # FUNCTION token includes "(" at the end: "rgb(" → "rgb"
    clean_name = name.rstrip("(")
    return clean_name in CSS_FUNCTIONS


# ---------------------------------------------------------------------------
# Transformer
# ---------------------------------------------------------------------------


class LatticeTransformer:
    """Transforms a Lattice AST into a clean CSS AST.

    Usage::

        transformer = LatticeTransformer()
        css_ast = transformer.transform(lattice_ast)

    The returned AST contains only CSS nodes (no variables, mixins,
    control flow, or function definitions). It can be passed directly
    to ``CSSEmitter.emit()`` to produce CSS text.
    """

    def __init__(self, max_while_iterations: int = MAX_WHILE_ITERATIONS) -> None:
        self.variables: ScopeChain = ScopeChain()
        self.mixins: dict[str, MixinDef] = {}
        self.functions: dict[str, FunctionDef] = {}
        self._mixin_stack: list[str] = []
        self._function_stack: list[str] = []
        self._max_while_iterations = max_while_iterations
        # Lattice v2: @extend tracking
        # Maps extended selector string → list of extending selector strings
        self._extend_map: dict[str, list[str]] = {}
        # Lattice v2: @at-root hoisted rules
        # Rules collected from @at-root directives during expansion.
        # They are spliced into the root stylesheet in Pass 3.
        self._at_root_rules: list[object] = []
        # Lattice v2: @content block tracking
        # Stack of content blocks passed to @include. Each entry is either
        # a block AST node (when @include has a content block) or None.
        self._content_block_stack: list[object | None] = []
        # Lattice v2: scope stack for @content evaluation
        # Content blocks are evaluated in the caller's scope, not the mixin's.
        self._content_scope_stack: list[ScopeChain] = []

    def transform(self, ast: object) -> object:
        """Transform a Lattice AST into a clean CSS AST.

        Runs the three-pass pipeline:

        1. **Collect symbols** — variables, mixins, functions, @extend relationships
        2. **Expand** — resolve variables, expand mixins, evaluate control flow,
           evaluate built-in and user-defined functions, handle @while/@content/
           @at-root/property nesting
        3. **Cleanup** — remove empty nodes, apply @extend selector merging,
           splice in @at-root hoisted rules, remove placeholder-only rules

        Args:
            ast: The root ``stylesheet`` ASTNode from the parser.

        Returns:
            A clean CSS AST with no Lattice nodes.
        """
        # Pass 1: Collect symbols
        self._collect_symbols(ast)

        # Pass 2: Expand
        result = self._expand_node(ast, self.variables)

        # Pass 3: Cleanup + @extend selector merging + @at-root hoisting
        result = self._cleanup(result)
        if self._extend_map:
            self._apply_extends(result)
        if self._at_root_rules:
            self._splice_at_root_rules(result)

        return result

    # =====================================================================
    # Pass 1: Symbol Collection
    # =====================================================================

    def _collect_symbols(self, ast: object) -> None:
        """Walk top-level rules and collect variable/mixin/function definitions.

        Definitions are removed from the AST children list since they
        produce no CSS output.
        """
        if not hasattr(ast, "children"):
            return

        new_children: list[object] = []

        for child in ast.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                new_children.append(child)
                continue

            rule = child.rule_name  # type: ignore[attr-defined]

            if rule == "rule":
                # rule = lattice_rule | at_rule | qualified_rule
                inner = child.children[0]  # type: ignore[attr-defined]
                inner_rule = inner.rule_name if hasattr(inner, "rule_name") else None

                if inner_rule == "lattice_rule":
                    # lattice_rule = variable_declaration | mixin_definition | ...
                    lattice_child = inner.children[0]  # type: ignore[attr-defined]
                    lattice_rule_name = lattice_child.rule_name if hasattr(lattice_child, "rule_name") else None

                    if lattice_rule_name == "variable_declaration":
                        self._collect_variable(lattice_child)
                        continue  # Don't add to output
                    elif lattice_rule_name == "mixin_definition":
                        self._collect_mixin(lattice_child)
                        continue
                    elif lattice_rule_name == "function_definition":
                        self._collect_function(lattice_child)
                        continue
                    elif lattice_rule_name == "use_directive":
                        # Skip @use for now (module resolution not implemented)
                        continue

                new_children.append(child)
            else:
                new_children.append(child)

        ast.children = new_children  # type: ignore[attr-defined]

    def _collect_variable(self, node: object) -> None:
        """Extract variable name and value from a variable_declaration node.

        variable_declaration = VARIABLE COLON value_list { variable_flag } SEMICOLON ;

        Lattice v2 adds ``!default`` and ``!global`` flags:

        - ``!default``: only set the variable if it is not already defined
          anywhere in the scope chain.
        - ``!global``: set the variable in the root (global) scope,
          regardless of how deeply nested the current context is.
        - Both can appear together: ``$var: val !default !global;``
          Check if defined in global scope; if not, set it globally.
        """
        children = node.children  # type: ignore[attr-defined]
        name = None
        value_node = None
        is_default = False
        is_global = False

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "VARIABLE":
                    name = child.value  # type: ignore[attr-defined]
                elif type_name == "BANG_DEFAULT":
                    is_default = True
                elif type_name == "BANG_GLOBAL":
                    is_global = True
            elif hasattr(child, "rule_name"):
                rule = child.rule_name  # type: ignore[attr-defined]
                if rule == "value_list":
                    value_node = child
                elif rule == "variable_flag":
                    # variable_flag = BANG_DEFAULT | BANG_GLOBAL
                    for fc in child.children:  # type: ignore[attr-defined]
                        ft = _token_type_name(fc) if not hasattr(fc, "rule_name") else ""
                        if ft == "BANG_DEFAULT":
                            is_default = True
                        elif ft == "BANG_GLOBAL":
                            is_global = True

        if name and value_node:
            if is_default and is_global:
                # Check global scope only — if not defined there, set it globally
                root = self.variables
                while root.parent is not None:
                    root = root.parent
                if root.get(name) is None:
                    self.variables.set_global(name, value_node)
            elif is_default:
                # Only set if not already defined anywhere
                if self.variables.get(name) is None:
                    self.variables.set(name, value_node)
            elif is_global:
                # Always set in global scope
                self.variables.set_global(name, value_node)
            else:
                self.variables.set(name, value_node)

    def _collect_mixin(self, node: object) -> None:
        """Extract mixin name, params, and body from a mixin_definition node.

        mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block ;
        """
        children = node.children  # type: ignore[attr-defined]
        name = None
        params: list[str] = []
        defaults: dict[str, Any] = {}
        body = None

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "FUNCTION":
                    # FUNCTION token is "name(" — strip the paren
                    name = child.value.rstrip("(")  # type: ignore[attr-defined]
            elif child.rule_name == "mixin_params":  # type: ignore[attr-defined]
                params, defaults = self._extract_params(child)
            elif child.rule_name == "block":  # type: ignore[attr-defined]
                body = child

        if name and body:
            self.mixins[name] = MixinDef(name, params, defaults, body)

    def _collect_function(self, node: object) -> None:
        """Extract function name, params, and body from a function_definition node.

        function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body ;
        """
        children = node.children  # type: ignore[attr-defined]
        name = None
        params: list[str] = []
        defaults: dict[str, Any] = {}
        body = None

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "FUNCTION":
                    name = child.value.rstrip("(")  # type: ignore[attr-defined]
            elif child.rule_name == "mixin_params":  # type: ignore[attr-defined]
                params, defaults = self._extract_params(child)
            elif child.rule_name == "function_body":  # type: ignore[attr-defined]
                body = child

        if name and body:
            self.functions[name] = FunctionDef(name, params, defaults, body)

    def _extract_params(self, node: object) -> tuple[list[str], dict[str, Any]]:
        """Extract parameter names and defaults from mixin_params.

        mixin_params = mixin_param { COMMA mixin_param } ;
        mixin_param = VARIABLE [ COLON value_list ] ;
        """
        params: list[str] = []
        defaults: dict[str, Any] = {}

        for child in node.children:  # type: ignore[attr-defined]
            if hasattr(child, "rule_name") and child.rule_name == "mixin_param":
                param_name = None
                default_value = None
                for pc in child.children:  # type: ignore[attr-defined]
                    if not hasattr(pc, "rule_name"):
                        if _token_type_name(pc) == "VARIABLE":
                            param_name = pc.value  # type: ignore[attr-defined]
                    elif pc.rule_name in ("value_list", "mixin_value_list"):  # type: ignore[attr-defined]
                        default_value = pc
                if param_name:
                    params.append(param_name)
                    if default_value is not None:
                        defaults[param_name] = default_value

        return params, defaults

    # =====================================================================
    # Pass 2: Expansion
    # =====================================================================

    def _expand_node(self, node: object, scope: ScopeChain) -> object:
        """Recursively expand a single AST node.

        Dispatches on rule_name to handle Lattice-specific constructs.
        CSS nodes are passed through with their children expanded.
        """
        if not hasattr(node, "rule_name"):
            # Token — check for variable substitution
            if _token_type_name(node) == "VARIABLE":
                return self._substitute_variable(node, scope)
            return node

        rule = node.rule_name  # type: ignore[attr-defined]

        # Handle lattice-specific blocks
        if rule == "block":
            return self._expand_block(node, scope)
        if rule == "block_contents":
            return self._expand_block_contents(node, scope)
        if rule == "block_item":
            return self._expand_block_item(node, scope)
        if rule == "value_list":
            return self._expand_value_list(node, scope)
        if rule == "value":
            return self._expand_value(node, scope)
        if rule == "function_call":
            return self._expand_function_call(node, scope)
        if rule == "function_arg":
            return self._expand_function_arg(node, scope)
        if rule == "function_args":
            return self._expand_children(node, scope)
        # Lattice v2: resolve variables in selector positions
        if rule in ("compound_selector", "simple_selector", "class_selector"):
            return self._expand_selector_with_vars(node, scope)

        # Default: expand children
        return self._expand_children(node, scope)

    def _expand_children(self, node: object, scope: ScopeChain) -> object:
        """Expand all children of a node."""
        if not hasattr(node, "children"):
            return node

        new_children: list[object] = []
        for child in node.children:  # type: ignore[attr-defined]
            expanded = self._expand_node(child, scope)
            if expanded is not None:
                new_children.append(expanded)

        node.children = new_children  # type: ignore[attr-defined]
        return node

    def _substitute_variable(self, token: object, scope: ScopeChain) -> object:
        """Replace a VARIABLE token with its resolved value.

        If the variable is bound to a value_list node, we return the node.
        If bound to a LatticeValue, we create a synthetic token.
        """
        name = token.value  # type: ignore[attr-defined]
        value = scope.get(name)

        if value is None:
            raise UndefinedVariableError(
                name,
                line=getattr(token, "line", 0),
                column=getattr(token, "column", 0),
            )

        # If the value is an AST node (value_list), deep-copy and expand it
        if hasattr(value, "rule_name"):
            cloned = copy.deepcopy(value)
            return self._expand_node(cloned, scope)

        # If it's a LatticeValue, convert to a synthetic token
        if isinstance(value, LatticeValue):
            css_text = value_to_css(value)
            return _make_token(css_text, token)

        return token

    def _expand_block(self, node: object, scope: ScopeChain) -> object:
        """Expand a block, creating a child scope."""
        child_scope = scope.child()
        return self._expand_children(node, child_scope)

    def _expand_block_contents(self, node: object, scope: ScopeChain) -> object:
        """Expand block_contents, handling Lattice block items.

        Block items can include variable_declarations, @include directives,
        and control flow (@if, @for, @each). These are expanded and their
        results spliced into the children list.
        """
        new_children: list[object] = []

        for child in node.children:  # type: ignore[attr-defined]
            expanded = self._expand_block_item_inner(child, scope)
            if isinstance(expanded, list):
                new_children.extend(expanded)
            elif expanded is not None:
                new_children.append(expanded)

        node.children = new_children  # type: ignore[attr-defined]
        return node

    def _expand_block_item(self, node: object, scope: ScopeChain) -> object:
        """Expand a single block_item."""
        children = node.children  # type: ignore[attr-defined]
        if not children:
            return node

        inner = children[0]
        if not hasattr(inner, "rule_name"):
            return self._expand_children(node, scope)

        rule = inner.rule_name  # type: ignore[attr-defined]

        if rule == "lattice_block_item":
            result = self._expand_lattice_block_item(inner, scope)
            if result is None:
                return None  # type: ignore[return-value]
            if isinstance(result, list):
                return result  # type: ignore[return-value]
            node.children = [result]  # type: ignore[attr-defined]
            return node

        # declaration_or_nested or at_rule
        return self._expand_children(node, scope)

    def _expand_block_item_inner(self, child: object, scope: ScopeChain) -> object | list[object] | None:
        """Process a single child of block_contents during expansion."""
        if not hasattr(child, "rule_name"):
            return child

        rule = child.rule_name  # type: ignore[attr-defined]

        if rule == "block_item":
            inner_children = child.children  # type: ignore[attr-defined]
            if inner_children and hasattr(inner_children[0], "rule_name"):
                inner_rule = inner_children[0].rule_name  # type: ignore[attr-defined]

                if inner_rule == "lattice_block_item":
                    result = self._expand_lattice_block_item(inner_children[0], scope)
                    if result is None:
                        return None
                    if isinstance(result, list):
                        return result
                    child.children = [inner_children[0]]  # type: ignore[attr-defined]
                    inner_children[0].children = [result]  # type: ignore[attr-defined]
                    return child

                # Lattice v2: handle property_nesting inside declaration_or_nested
                if inner_rule == "declaration_or_nested":
                    don_children = inner_children[0].children  # type: ignore[attr-defined]
                    if don_children and hasattr(don_children[0], "rule_name"):
                        if don_children[0].rule_name == "property_nesting":
                            result = self._expand_property_nesting(don_children[0], scope)
                            return result if result else None

            return self._expand_children(child, scope)

        return self._expand_children(child, scope)

    def _expand_lattice_block_item(self, node: object, scope: ScopeChain) -> object | list[object] | None:
        """Expand a lattice_block_item.

        lattice_block_item = variable_declaration | include_directive
                           | lattice_control | content_directive
                           | at_root_directive | extend_directive ;

        Lattice v2 adds: content_directive, at_root_directive, extend_directive.
        """
        children = node.children  # type: ignore[attr-defined]
        if not children:
            return node

        inner = children[0]
        if not hasattr(inner, "rule_name"):
            return node

        rule = inner.rule_name  # type: ignore[attr-defined]

        if rule == "variable_declaration":
            self._expand_variable_declaration(inner, scope)
            return None  # Remove from output
        elif rule == "include_directive":
            return self._expand_include(inner, scope)
        elif rule == "lattice_control":
            return self._expand_control(inner, scope)
        elif rule == "content_directive":
            return self._expand_content(inner, scope)
        elif rule == "at_root_directive":
            return self._expand_at_root(inner, scope)
        elif rule == "extend_directive":
            self._collect_extend(inner, scope)
            return None  # Remove from output

        return self._expand_children(node, scope)

    def _expand_variable_declaration(self, node: object, scope: ScopeChain) -> None:
        """Process a variable_declaration inside a block.

        Sets the variable in the current scope. The node is removed from output.

        Lattice v2: handles ``!default`` and ``!global`` flags.
        """
        children = node.children  # type: ignore[attr-defined]
        name = None
        value_node = None
        is_default = False
        is_global = False

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "VARIABLE":
                    name = child.value  # type: ignore[attr-defined]
                elif type_name == "BANG_DEFAULT":
                    is_default = True
                elif type_name == "BANG_GLOBAL":
                    is_global = True
            elif hasattr(child, "rule_name"):
                rule = child.rule_name  # type: ignore[attr-defined]
                if rule == "value_list":
                    value_node = child
                elif rule == "variable_flag":
                    for fc in child.children:  # type: ignore[attr-defined]
                        ft = _token_type_name(fc) if not hasattr(fc, "rule_name") else ""
                        if ft == "BANG_DEFAULT":
                            is_default = True
                        elif ft == "BANG_GLOBAL":
                            is_global = True

        if name and value_node:
            # Expand the value first (it might contain variables)
            expanded_value = self._expand_node(copy.deepcopy(value_node), scope)

            # Try to evaluate as an expression (e.g. ``$i + 1`` → LatticeNumber(2)).
            # This is critical for @while loops: without it, ``$i: $i + 1``
            # stores unevaluated tokens instead of the computed number, causing
            # the loop condition to never change and looping forever.
            try:
                evaluator = ExpressionEvaluator(scope)
                evaluated = evaluator.evaluate(copy.deepcopy(expanded_value))
                # Store the LatticeValue directly so _substitute_variable can
                # convert it to a token via the isinstance(value, LatticeValue)
                # branch.  Storing a raw token would fall through that check.
                expanded_value = evaluated
            except Exception:
                pass  # Not a pure expression (e.g. ``Helvetica, sans-serif``)

            if is_default and is_global:
                root = scope
                while root.parent is not None:
                    root = root.parent
                if root.get(name) is None:
                    scope.set_global(name, expanded_value)
            elif is_default:
                if scope.get(name) is None:
                    scope.set(name, expanded_value)
            elif is_global:
                scope.set_global(name, expanded_value)
            else:
                scope.set(name, expanded_value)

    def _expand_value_list(self, node: object, scope: ScopeChain) -> object:
        """Expand variables within a value_list."""
        new_children: list[object] = []

        for child in node.children:  # type: ignore[attr-defined]
            expanded = self._expand_node(child, scope)
            if expanded is not None:
                # If expansion returns a value_list, splice its children
                if hasattr(expanded, "rule_name") and expanded.rule_name == "value_list":
                    new_children.extend(expanded.children)  # type: ignore[attr-defined]
                else:
                    new_children.append(expanded)

        node.children = new_children  # type: ignore[attr-defined]
        return node

    def _expand_value(self, node: object, scope: ScopeChain) -> object:
        """Expand a single value node."""
        children = node.children  # type: ignore[attr-defined]
        if not children:
            return node

        # Check if it's a VARIABLE token
        if len(children) == 1 and not hasattr(children[0], "rule_name"):
            if _token_type_name(children[0]) == "VARIABLE":
                result = self._substitute_variable(children[0], scope)
                if hasattr(result, "rule_name"):
                    return result  # Return the value_list directly
                node.children = [result]  # type: ignore[attr-defined]
                return node

        return self._expand_children(node, scope)

    def _expand_function_call(self, node: object, scope: ScopeChain) -> object:
        """Expand a function_call node.

        Priority order:

        1. User-defined @function — evaluate and replace with return value
        2. CSS built-in (rgb, calc, etc.) — pass through unchanged
        3. Lattice v2 built-in function (map-get, lighten, math.div, etc.)
           — evaluate with ExpressionEvaluator and replace
        4. Unknown — pass through (might be a CSS function we don't know)
        """
        children = node.children  # type: ignore[attr-defined]

        # Find the FUNCTION token to get the name
        func_name = None
        for child in children:
            if not hasattr(child, "rule_name") and _token_type_name(child) == "FUNCTION":
                func_name = child.value.rstrip("(")  # type: ignore[attr-defined]
                break

        # URL_TOKEN — pass through
        if func_name is None:
            return self._expand_children(node, scope)

        # User-defined function ALWAYS takes priority — even over CSS built-ins
        # like scale(), translate(), etc. If the user defines @function scale(),
        # their definition wins. This matches Sass behavior.
        if func_name in self.functions:
            return self._evaluate_function_call(func_name, node, scope)

        # CSS built-in that is NOT also a Lattice built-in — pass through
        if _is_css_function(func_name) and func_name not in BUILTIN_FUNCTIONS:
            return self._expand_children(node, scope)

        # Lattice v2 built-in function
        if func_name in BUILTIN_FUNCTIONS:
            return self._evaluate_builtin_function(func_name, node, scope)

        # CSS built-in that overlaps with Lattice built-in names
        if _is_css_function(func_name):
            return self._expand_children(node, scope)

        # Unknown function — pass through
        return self._expand_children(node, scope)

    def _expand_function_arg(self, node: object, scope: ScopeChain) -> object:
        """Expand a function_arg node, substituting variables."""
        return self._expand_children(node, scope)

    # =====================================================================
    # @include Expansion
    # =====================================================================

    def _expand_include(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand an @include directive by cloning the mixin body.

        include_directive = "@include" FUNCTION [ include_args ] RPAREN
                              ( SEMICOLON | block )
                          | "@include" IDENT ( SEMICOLON | block ) ;

        Lattice v2: if @include has a trailing block (not SEMICOLON), that
        block is the **content block** — it replaces ``@content;`` in the
        mixin body. The content block is evaluated in the caller's scope.
        """
        children = node.children  # type: ignore[attr-defined]
        mixin_name = None
        args_node = None
        content_block = None

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "FUNCTION":
                    mixin_name = child.value.rstrip("(")  # type: ignore[attr-defined]
                elif type_name == "IDENT":
                    mixin_name = child.value  # type: ignore[attr-defined]
            elif hasattr(child, "rule_name"):
                child_rule = child.rule_name  # type: ignore[attr-defined]
                if child_rule == "include_args":
                    args_node = child
                elif child_rule == "block":
                    content_block = child

        if mixin_name is None:
            return []

        if mixin_name not in self.mixins:
            raise UndefinedMixinError(mixin_name)

        # Cycle detection
        if mixin_name in self._mixin_stack:
            raise CircularReferenceError(
                "mixin", self._mixin_stack + [mixin_name]
            )

        mixin_def = self.mixins[mixin_name]

        # Parse arguments
        args = self._parse_include_args(args_node) if args_node else []

        # Check arity
        required = len(mixin_def.params) - len(mixin_def.defaults)
        if len(args) < required or len(args) > len(mixin_def.params):
            raise WrongArityError(
                "Mixin", mixin_name, len(mixin_def.params), len(args)
            )

        # Create child scope with params bound
        mixin_scope = scope.child()
        for i, param_name in enumerate(mixin_def.params):
            if i < len(args):
                mixin_scope.set(param_name, args[i])
            elif param_name in mixin_def.defaults:
                mixin_scope.set(param_name, copy.deepcopy(mixin_def.defaults[param_name]))

        # Lattice v2: push content block and caller scope for @content
        self._content_block_stack.append(content_block)
        self._content_scope_stack.append(scope)

        # Clone and expand the mixin body
        self._mixin_stack.append(mixin_name)
        try:
            body_clone = copy.deepcopy(mixin_def.body)
            expanded = self._expand_node(body_clone, mixin_scope)

            # Extract block_contents children
            if hasattr(expanded, "children"):
                for child in expanded.children:  # type: ignore[attr-defined]
                    if hasattr(child, "rule_name") and child.rule_name == "block_contents":
                        return list(child.children)  # type: ignore[attr-defined]

            return []
        finally:
            self._mixin_stack.pop()
            self._content_block_stack.pop()
            self._content_scope_stack.pop()

    def _parse_include_args(self, node: object) -> list[object]:
        """Parse include_args into a list of value_list nodes.

        include_args = value_list { COMMA value_list } ;

        Due to grammar design, commas may be absorbed into a single
        value_list (since COMMA is a valid value token). So if we get
        only one value_list but it contains COMMA values, we split it
        on commas to produce multiple args.
        """
        value_lists: list[object] = []
        for child in node.children:  # type: ignore[attr-defined]
            if hasattr(child, "rule_name") and child.rule_name == "value_list":  # type: ignore[attr-defined]
                value_lists.append(child)

        # If there's only one value_list, check if it contains commas
        # and split on them to produce multiple args
        if len(value_lists) == 1:
            return self._split_value_list_on_commas(value_lists[0])

        return value_lists

    def _split_value_list_on_commas(self, node: object) -> list[object]:
        """Split a value_list into multiple value_lists at COMMA boundaries.

        If value_list contains value nodes with COMMA tokens, split them:
        [red, COMMA, white] → [[red], [white]]
        """
        children = node.children  # type: ignore[attr-defined]

        # Check if any child value node contains a COMMA
        has_comma = False
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "value":
                for vc in child.children:  # type: ignore[attr-defined]
                    if not hasattr(vc, "rule_name") and _token_type_name(vc) == "COMMA":
                        has_comma = True
                        break

        if not has_comma:
            return [node]

        # Split on comma value nodes
        groups: list[list[object]] = [[]]
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "value":
                inner = child.children  # type: ignore[attr-defined]
                if len(inner) == 1 and not hasattr(inner[0], "rule_name") and _token_type_name(inner[0]) == "COMMA":
                    groups.append([])
                    continue
            groups[-1].append(child)

        # Create new value_list nodes for each group
        result: list[object] = []
        for group in groups:
            if group:
                class ValueListNode:
                    def __init__(self, children: list[object]) -> None:
                        self.rule_name = "value_list"
                        self.children = children
                result.append(ValueListNode(group))

        return result

    # =====================================================================
    # Control Flow
    # =====================================================================

    def _expand_control(self, node: object, scope: ScopeChain) -> list[object] | None:
        """Expand a lattice_control node.

        lattice_control = if_directive | for_directive | each_directive
                        | while_directive ;

        Lattice v2 adds while_directive.
        """
        children = node.children  # type: ignore[attr-defined]
        if not children:
            return None

        inner = children[0]
        if not hasattr(inner, "rule_name"):
            return None

        rule = inner.rule_name  # type: ignore[attr-defined]

        if rule == "if_directive":
            return self._expand_if(inner, scope)
        elif rule == "for_directive":
            return self._expand_for(inner, scope)
        elif rule == "each_directive":
            return self._expand_each(inner, scope)
        elif rule == "while_directive":
            return self._expand_while(inner, scope)

        return None

    def _expand_if(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand an @if / @else if / @else directive.

        if_directive = "@if" lattice_expression block
                       { "@else" "if" lattice_expression block }
                       [ "@else" block ] ;

        Evaluate expressions and expand the matching branch.
        """
        children = node.children  # type: ignore[attr-defined]

        # Parse the if/else-if/else structure
        branches: list[tuple[object | None, object]] = []  # (condition, block)
        i = 0
        while i < len(children):
            child = children[i]
            val = _get_token_value(child)

            if val in ("@if", "@else"):
                if val == "@if":
                    # @if expression block
                    expr = children[i + 1]
                    block = children[i + 2]
                    branches.append((expr, block))
                    i += 3
                elif val == "@else":
                    # Check if next is "if"
                    if i + 1 < len(children) and _get_token_value(children[i + 1]) == "if":
                        # @else if expression block
                        expr = children[i + 2]
                        block = children[i + 3]
                        branches.append((expr, block))
                        i += 4
                    else:
                        # @else block
                        block = children[i + 1]
                        branches.append((None, block))
                        i += 2
            else:
                i += 1

        # Evaluate branches
        evaluator = ExpressionEvaluator(scope)
        for condition, block in branches:
            if condition is None:
                # @else — always matches
                return self._expand_block_to_items(block, scope)
            else:
                result = evaluator.evaluate(condition)
                if is_truthy(result):
                    return self._expand_block_to_items(block, scope)

        return []

    def _expand_for(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand a @for loop.

        for_directive = "@for" VARIABLE "from" lattice_expression
                        ( "through" | "to" ) lattice_expression block ;
        """
        children = node.children  # type: ignore[attr-defined]

        var_name = None
        from_expr = None
        to_expr = None
        is_through = False
        block = None

        i = 0
        while i < len(children):
            child = children[i]
            val = _get_token_value(child)

            if val is not None and _token_type_name(child) == "VARIABLE":
                var_name = val
            elif val == "from":
                from_expr = children[i + 1]
                i += 1
            elif val == "through":
                is_through = True
                to_expr = children[i + 1]
                i += 1
            elif val == "to":
                is_through = False
                to_expr = children[i + 1]
                i += 1
            elif hasattr(child, "rule_name") and child.rule_name == "block":
                block = child

            i += 1

        if var_name is None or from_expr is None or to_expr is None or block is None:
            return []

        evaluator = ExpressionEvaluator(scope)
        from_val = evaluator.evaluate(from_expr)
        to_val = evaluator.evaluate(to_expr)

        # Extract numeric values
        from_num = int(from_val.value) if hasattr(from_val, "value") else 0  # type: ignore[attr-defined]
        to_num = int(to_val.value) if hasattr(to_val, "value") else 0  # type: ignore[attr-defined]

        if is_through:
            end = to_num + 1  # inclusive
        else:
            end = to_num  # exclusive

        result: list[object] = []
        for i_val in range(from_num, end):
            from lattice_ast_to_css.evaluator import LatticeNumber
            loop_scope = scope.child()
            loop_scope.set(var_name, LatticeNumber(float(i_val)))

            expanded = self._expand_block_to_items(copy.deepcopy(block), loop_scope)
            result.extend(expanded)

        return result

    def _expand_each(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand a @each loop.

        each_directive = "@each" VARIABLE { COMMA VARIABLE } "in"
                         each_list block ;

        Lattice v2: when iterating over a map with two variables
        (``@each $key, $value in $map``), the first variable gets the key
        and the second gets the value for each entry.
        """
        children = node.children  # type: ignore[attr-defined]

        var_names: list[str] = []
        each_list = None
        block = None

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "VARIABLE":
                    var_names.append(child.value)  # type: ignore[attr-defined]
            elif child.rule_name == "each_list":  # type: ignore[attr-defined]
                each_list = child
            elif child.rule_name == "block":  # type: ignore[attr-defined]
                block = child

        if not var_names or each_list is None or block is None:
            return []

        # Check if the each_list references a variable that holds a map or list
        # Lattice v2: @each $key, $value in $map_variable
        list_items = self._resolve_each_list(each_list, scope)

        if list_items is not None:
            return self._expand_each_over_resolved(var_names, list_items, block, scope)

        # Extract list items from each_list
        # each_list = value { COMMA value } ;
        items: list[object] = []
        for child in each_list.children:  # type: ignore[attr-defined]
            if hasattr(child, "rule_name") and child.rule_name == "value":  # type: ignore[attr-defined]
                items.append(child)

        result: list[object] = []
        for item in items:
            loop_scope = scope.child()
            # Bind the first variable to the item's value
            if var_names:
                # Extract the token value from the value node
                item_value = self._extract_value_token(item)
                loop_scope.set(var_names[0], item_value)

            expanded = self._expand_block_to_items(copy.deepcopy(block), loop_scope)
            result.extend(expanded)

        return result

    def _resolve_each_list(self, each_list: object, scope: ScopeChain) -> LatticeValue | None:
        """Try to resolve an each_list to a LatticeValue (map or list).

        If the each_list contains a single VARIABLE that resolves to a
        LatticeMap or LatticeList, return it. Otherwise return None
        to fall through to the default token-based iteration.
        """
        children = each_list.children  # type: ignore[attr-defined]
        # Check for single-variable each_list
        var_tokens = []
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "value":
                for vc in child.children:  # type: ignore[attr-defined]
                    if not hasattr(vc, "rule_name") and _token_type_name(vc) == "VARIABLE":
                        var_tokens.append(vc)
        if len(var_tokens) == 1:
            val = scope.get(var_tokens[0].value)  # type: ignore[attr-defined]
            if isinstance(val, (LatticeMap, LatticeList)):
                return val
        return None

    def _expand_each_over_resolved(
        self,
        var_names: list[str],
        collection: LatticeValue,
        block: object,
        scope: ScopeChain,
    ) -> list[object]:
        """Expand @each over a resolved LatticeMap or LatticeList."""
        result: list[object] = []

        if isinstance(collection, LatticeMap):
            # Map destructuring: @each $key, $value in $map
            for key, value in collection.items:
                loop_scope = scope.child()
                loop_scope.set(var_names[0], LatticeIdent(key))
                if len(var_names) >= 2:
                    loop_scope.set(var_names[1], value)
                expanded = self._expand_block_to_items(copy.deepcopy(block), loop_scope)
                result.extend(expanded)
        elif isinstance(collection, LatticeList):
            for item in collection.items:
                loop_scope = scope.child()
                loop_scope.set(var_names[0], item)
                expanded = self._expand_block_to_items(copy.deepcopy(block), loop_scope)
                result.extend(expanded)

        return result

    def _extract_value_token(self, node: object) -> object:
        """Extract the meaningful content from a value node."""
        if hasattr(node, "children"):
            children = node.children  # type: ignore[attr-defined]
            if len(children) == 1:
                child = children[0]
                if not hasattr(child, "rule_name"):
                    return token_to_value(child)
                return child
        return node

    def _expand_block_to_items(self, block: object, scope: ScopeChain) -> list[object]:
        """Expand a block and return its block_contents children."""
        expanded = self._expand_node(block, scope)
        if hasattr(expanded, "children"):
            for child in expanded.children:  # type: ignore[attr-defined]
                if hasattr(child, "rule_name") and child.rule_name == "block_contents":
                    return [c for c in child.children if c is not None]  # type: ignore[attr-defined]
        return []

    # =====================================================================
    # Function Evaluation
    # =====================================================================

    def _evaluate_function_call(
        self, func_name: str, node: object, scope: ScopeChain
    ) -> object:
        """Evaluate a Lattice function call and return the result.

        The function body is evaluated in an isolated scope (parent = globals).
        @return signals the return value via ReturnSignal exception.
        """
        func_def = self.functions[func_name]
        children = node.children  # type: ignore[attr-defined]

        # Parse arguments from function_args
        args: list[object] = []
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "function_args":  # type: ignore[attr-defined]
                args = self._parse_function_call_args(child)
                break

        # Check arity
        required = len(func_def.params) - len(func_def.defaults)
        if len(args) < required or len(args) > len(func_def.params):
            raise WrongArityError(
                "Function", func_name, len(func_def.params), len(args)
            )

        # Cycle detection
        if func_name in self._function_stack:
            raise CircularReferenceError(
                "function", self._function_stack + [func_name]
            )

        # Create isolated scope (parent = global scope only)
        func_scope = self.variables.child()
        for i, param_name in enumerate(func_def.params):
            if i < len(args):
                func_scope.set(param_name, args[i])
            elif param_name in func_def.defaults:
                func_scope.set(param_name, copy.deepcopy(func_def.defaults[param_name]))

        # Evaluate the function body
        self._function_stack.append(func_name)
        try:
            body_clone = copy.deepcopy(func_def.body)
            try:
                self._evaluate_function_body(body_clone, func_scope)
            except ReturnSignal as ret:
                # Convert the returned value to a token
                css_text = value_to_css(ret.value)
                return _make_value_node(css_text, node)

            raise MissingReturnError(func_name)
        finally:
            self._function_stack.pop()

    def _evaluate_function_body(self, body: object, scope: ScopeChain) -> None:
        """Evaluate function body statements.

        function_body = LBRACE { function_body_item } RBRACE ;
        function_body_item = variable_declaration | return_directive | lattice_control ;
        """
        if not hasattr(body, "children"):
            return

        for child in body.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                continue

            rule = child.rule_name  # type: ignore[attr-defined]

            if rule == "function_body_item":
                inner = child.children[0] if child.children else None  # type: ignore[attr-defined]
                if inner and hasattr(inner, "rule_name"):
                    inner_rule = inner.rule_name  # type: ignore[attr-defined]

                    if inner_rule == "variable_declaration":
                        self._expand_variable_declaration(inner, scope)
                    elif inner_rule == "return_directive":
                        self._evaluate_return(inner, scope)
                    elif inner_rule == "lattice_control":
                        self._evaluate_control_in_function(inner, scope)

    def _evaluate_return(self, node: object, scope: ScopeChain) -> None:
        """Evaluate a @return directive.

        return_directive = "@return" lattice_expression SEMICOLON ;
        """
        children = node.children  # type: ignore[attr-defined]

        # Find the expression
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "lattice_expression":  # type: ignore[attr-defined]
                evaluator = ExpressionEvaluator(scope)
                result = evaluator.evaluate(child)
                raise ReturnSignal(result)

    def _evaluate_control_in_function(self, node: object, scope: ScopeChain) -> None:
        """Evaluate control flow inside a function body.

        Similar to _expand_control but operates within function evaluation context,
        where @return can signal a function return.
        """
        children = node.children  # type: ignore[attr-defined]
        if not children:
            return

        inner = children[0]
        if not hasattr(inner, "rule_name"):
            return

        rule = inner.rule_name  # type: ignore[attr-defined]

        if rule == "if_directive":
            self._evaluate_if_in_function(inner, scope)

    def _evaluate_if_in_function(self, node: object, scope: ScopeChain) -> None:
        """Evaluate @if inside a function body."""
        children = node.children  # type: ignore[attr-defined]

        branches: list[tuple[object | None, object]] = []
        i = 0
        while i < len(children):
            child = children[i]
            val = _get_token_value(child)

            if val == "@if":
                branches.append((children[i + 1], children[i + 2]))
                i += 3
            elif val == "@else":
                if i + 1 < len(children) and _get_token_value(children[i + 1]) == "if":
                    branches.append((children[i + 2], children[i + 3]))
                    i += 4
                else:
                    branches.append((None, children[i + 1]))
                    i += 2
            else:
                i += 1

        evaluator = ExpressionEvaluator(scope)
        for condition, block in branches:
            if condition is None or is_truthy(evaluator.evaluate(condition)):
                # Evaluate the block — look for @return in block_contents
                self._evaluate_block_in_function(block, scope)
                return

    def _evaluate_block_in_function(self, block: object, scope: ScopeChain) -> None:
        """Evaluate a block inside a function, handling @return at-rules.

        When @return appears inside @if blocks within a function, the grammar
        parses it as an at_rule (not return_directive), because return_directive
        is only valid in function_body_item. We detect @return at-rules here.
        """
        if not hasattr(block, "children"):
            return

        for child in block.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                continue

            rule = child.rule_name  # type: ignore[attr-defined]

            if rule == "block_contents":
                self._evaluate_block_in_function(child, scope)
            elif rule == "block_item":
                inner_children = child.children  # type: ignore[attr-defined]
                if inner_children:
                    inner = inner_children[0]
                    if hasattr(inner, "rule_name"):
                        inner_rule = inner.rule_name  # type: ignore[attr-defined]
                        if inner_rule == "at_rule":
                            self._maybe_evaluate_return_at_rule(inner, scope)
                        elif inner_rule == "lattice_block_item":
                            # Could be variable_declaration or control flow
                            for lbc in inner.children:  # type: ignore[attr-defined]
                                if hasattr(lbc, "rule_name") and lbc.rule_name == "variable_declaration":
                                    self._expand_variable_declaration(lbc, scope)

    def _maybe_evaluate_return_at_rule(self, node: object, scope: ScopeChain) -> None:
        """Check if an at_rule is actually @return, and evaluate it if so.

        at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;

        If AT_KEYWORD value is "@return", extract the expression from
        at_prelude and evaluate it.
        """
        children = node.children  # type: ignore[attr-defined]

        keyword = None
        prelude = None
        for child in children:
            if not hasattr(child, "rule_name"):
                if _token_type_name(child) == "AT_KEYWORD":
                    keyword = child.value  # type: ignore[attr-defined]
            elif hasattr(child, "rule_name") and child.rule_name == "at_prelude":
                prelude = child

        if keyword != "@return" or prelude is None:
            return

        # Extract tokens from at_prelude to build an expression
        # The prelude contains at_prelude_token nodes wrapping raw tokens
        tokens: list[object] = []
        self._collect_tokens(prelude, tokens)

        if not tokens:
            raise ReturnSignal(LatticeNull())

        # For simple cases (single token), convert directly
        if len(tokens) == 1:
            evaluator = ExpressionEvaluator(scope)
            value = token_to_value(tokens[0])
            # Check if it's a variable
            if _token_type_name(tokens[0]) == "VARIABLE":
                var_val = scope.get(tokens[0].value)  # type: ignore[attr-defined]
                if var_val is not None:
                    if isinstance(var_val, LatticeValue):
                        raise ReturnSignal(var_val)
                    if hasattr(var_val, "rule_name"):
                        evaluator2 = ExpressionEvaluator(scope)
                        value = evaluator2._extract_value_from_ast(var_val)
                        raise ReturnSignal(value)
            raise ReturnSignal(value)

        # For multi-token expressions, try to evaluate
        # Simple numeric value
        raise ReturnSignal(token_to_value(tokens[0]))

    def _collect_tokens(self, node: object, tokens: list[object]) -> None:
        """Recursively collect all raw tokens from an AST node."""
        if not hasattr(node, "children"):
            return
        for child in node.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                tokens.append(child)
            else:
                self._collect_tokens(child, tokens)

    def _parse_function_call_args(self, node: object) -> list[object]:
        """Parse function_args into individual argument values.

        function_args = { function_arg } ;
        function_arg = ... | COMMA | ...

        Arguments are separated by COMMA tokens.
        """
        args: list[list[object]] = [[]]

        for child in node.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                if _token_type_name(child) == "COMMA":
                    args.append([])
                    continue
            if hasattr(child, "rule_name") and child.rule_name == "function_arg":
                # Extract the meaningful content
                inner_children = child.children  # type: ignore[attr-defined]
                for ic in inner_children:
                    if not hasattr(ic, "rule_name"):
                        if _token_type_name(ic) == "COMMA":
                            args.append([])
                            continue
                        args[-1].append(token_to_value(ic))
                    else:
                        args[-1].append(ic)

        # Convert each arg group to a single value
        result: list[object] = []
        for arg_group in args:
            if len(arg_group) == 1:
                result.append(arg_group[0])
            elif arg_group:
                result.append(arg_group[0])  # Take first for simplicity

        return result

    # =====================================================================
    # Lattice v2: @while Loops
    # =====================================================================

    def _expand_while(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand a @while loop.

        while_directive = "@while" lattice_expression block ;

        **Algorithm:**

        1. Evaluate the condition expression using ExpressionEvaluator.
        2. If truthy, clone and expand the block body.
        3. Re-evaluate the condition (variable mutations in the body are
           visible because we use the same scope — not a child scope).
        4. Repeat until the condition is falsy or the iteration count
           exceeds the max-iteration guard.

        **Variable scoping:** Unlike @for which creates a new child scope
        per iteration, @while uses the enclosing scope directly. This
        means variable mutations inside the body persist across iterations
        and after the loop exits. This is intentional — the loop condition
        typically depends on a variable that the body mutates.

        **Max-iteration guard:** Prevents infinite loops from hanging the
        compiler. Default limit is 1000 iterations (configurable).
        """
        children = node.children  # type: ignore[attr-defined]

        condition = None
        block = None

        for child in children:
            if hasattr(child, "rule_name"):
                rule = child.rule_name  # type: ignore[attr-defined]
                if rule == "lattice_expression":
                    condition = child
                elif rule == "block":
                    block = child

        if condition is None or block is None:
            return []

        result: list[object] = []
        iteration = 0

        while True:
            evaluator = ExpressionEvaluator(scope)
            cond_value = evaluator.evaluate(copy.deepcopy(condition))

            if not is_truthy(cond_value):
                break

            iteration += 1
            if iteration > self._max_while_iterations:
                raise MaxIterationError(self._max_while_iterations)

            expanded = self._expand_block_to_items(copy.deepcopy(block), scope)
            result.extend(expanded)

        return result

    # =====================================================================
    # Lattice v2: $var in Selectors
    # =====================================================================

    def _expand_selector_with_vars(self, node: object, scope: ScopeChain) -> object:
        """Resolve VARIABLE tokens in selector positions.

        When a VARIABLE token appears in a compound_selector, simple_selector,
        or class_selector, resolve it to its string value and create a
        synthetic IDENT token. Adjacent tokens are concatenated by the
        emitter (compound_selector joins without spaces).

        Example::

            // Source: .col-$i where $i = 3
            // Tokens: DOT IDENT("col-") VARIABLE("$i")
            // After:  DOT IDENT("col-") IDENT("3")
            // Emitter: .col-3

        The variable's value is converted to a string:

        - LatticeNumber → "3", "42"
        - LatticeDimension → "16px" (unusual but valid)
        - LatticeString → "hello" (without quotes)
        - LatticeIdent → "red", "bold"
        - LatticeColor → "#4a90d9"
        """
        children = node.children  # type: ignore[attr-defined]
        new_children: list[object] = []

        for child in children:
            if not hasattr(child, "rule_name"):
                type_name = _token_type_name(child)
                if type_name == "VARIABLE":
                    var_name = child.value  # type: ignore[attr-defined]
                    value = scope.get(var_name)
                    if value is None:
                        raise UndefinedVariableError(
                            var_name,
                            line=getattr(child, "line", 0),
                            column=getattr(child, "column", 0),
                        )
                    # Convert to string for selector context
                    if isinstance(value, LatticeValue):
                        css_text = value_to_css(value)
                    elif hasattr(value, "rule_name"):
                        # AST node — extract text
                        evaluator = ExpressionEvaluator(scope)
                        v = evaluator._extract_value_from_ast(value)
                        css_text = value_to_css(v)
                    else:
                        css_text = str(value)
                    # Strip quotes from strings in selector context
                    css_text = css_text.strip('"').strip("'")
                    new_children.append(_make_token(css_text, child))
                else:
                    new_children.append(child)
            else:
                # Recurse into child AST nodes
                new_children.append(self._expand_node(child, scope))

        node.children = new_children  # type: ignore[attr-defined]
        return node

    # =====================================================================
    # Lattice v2: @content Blocks
    # =====================================================================

    def _expand_content(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand a @content directive inside a mixin body.

        content_directive = "@content" SEMICOLON ;

        Replaces ``@content;`` with the content block from the current
        ``@include`` call. The content block is evaluated in the **caller's
        scope**, not the mixin's scope.

        If no content block was passed (the @include ended with ;), this
        produces an empty list (no output, no error).
        """
        if not self._content_block_stack:
            return []

        content_block = self._content_block_stack[-1]
        if content_block is None:
            return []

        # Evaluate in the caller's scope
        caller_scope = self._content_scope_stack[-1] if self._content_scope_stack else scope
        return self._expand_block_to_items(copy.deepcopy(content_block), caller_scope)

    # =====================================================================
    # Lattice v2: @at-root
    # =====================================================================

    def _expand_at_root(self, node: object, scope: ScopeChain) -> list[object] | None:
        """Expand an @at-root directive.

        at_root_directive = "@at-root" ( block | selector_list block ) ;

        Rules inside @at-root are collected and hoisted to the stylesheet
        root level during Pass 3. They are removed from the current
        nesting context.
        """
        children = node.children  # type: ignore[attr-defined]

        # Find block and optional selector_list
        block = None
        selector_list = None
        for child in children:
            if hasattr(child, "rule_name"):
                rule = child.rule_name  # type: ignore[attr-defined]
                if rule == "block":
                    block = child
                elif rule == "selector_list":
                    selector_list = child

        if block is None:
            return None

        if selector_list is not None:
            # Inline form: @at-root .selector { ... }
            # Create a qualified_rule and hoist it
            expanded_sel = self._expand_node(copy.deepcopy(selector_list), scope)
            expanded_block = self._expand_node(copy.deepcopy(block), scope)
            qr = _make_qualified_rule(expanded_sel, expanded_block)
            self._at_root_rules.append(qr)
        else:
            # Block form: @at-root { ... multiple rules ... }
            expanded = self._expand_block_to_items(copy.deepcopy(block), scope)
            self._at_root_rules.extend(expanded)

        return None  # Remove from current position

    # =====================================================================
    # Lattice v2: @extend and %placeholder
    # =====================================================================

    def _collect_extend(self, node: object, scope: ScopeChain) -> None:
        """Collect an @extend directive for later selector merging.

        extend_directive = "@extend" extend_target SEMICOLON ;
        extend_target = PLACEHOLDER | DOT IDENT | IDENT ;

        Records the extend relationship: the current rule's selector
        (determined by context) extends the target selector.
        """
        children = node.children  # type: ignore[attr-defined]
        target = ""

        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "extend_target":
                target = self._extract_extend_target(child)

        if target:
            # We don't know the extending selector at this point — it's
            # determined by the enclosing qualified_rule's selector.
            # Store as "pending" and resolve during cleanup.
            if target not in self._extend_map:
                self._extend_map[target] = []
            # The extending selector will be resolved during _apply_extends

    def _extract_extend_target(self, node: object) -> str:
        """Extract the target selector string from an extend_target node."""
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                parts.append(child.value)  # type: ignore[attr-defined]
        return "".join(parts)

    # =====================================================================
    # Lattice v2: Property Nesting
    # =====================================================================

    def _expand_property_nesting(self, node: object, scope: ScopeChain) -> list[object]:
        """Expand a property_nesting node.

        property_nesting = property COLON block ;

        Flattens nested property declarations by prepending the parent
        property name with a hyphen to each child declaration's property.

        Example::

            // Input:
            font: {
                size: 14px;
                weight: bold;
            }

            // Output (two declarations):
            font-size: 14px;
            font-weight: bold;

        Recursion: property nesting can be nested arbitrarily:
        ``border: { style: { top: solid; } }`` becomes ``border-style-top: solid;``
        """
        children = node.children  # type: ignore[attr-defined]
        parent_prop = ""
        block = None

        for child in children:
            if hasattr(child, "rule_name"):
                rule = child.rule_name  # type: ignore[attr-defined]
                if rule == "property":
                    for pc in child.children:  # type: ignore[attr-defined]
                        parent_prop = pc.value  # type: ignore[attr-defined]
                elif rule == "block":
                    block = child

        if not parent_prop or block is None:
            return []

        # Expand the block first
        expanded = self._expand_node(copy.deepcopy(block), scope)

        # Extract declarations and prepend parent property
        result: list[object] = []
        self._flatten_nested_props(expanded, parent_prop, result)
        return result

    def _flatten_nested_props(self, node: object, prefix: str, result: list[object]) -> None:
        """Recursively flatten nested property declarations."""
        if not hasattr(node, "children"):
            return
        for child in node.children:  # type: ignore[attr-defined]
            if not hasattr(child, "rule_name"):
                continue
            rule = child.rule_name  # type: ignore[attr-defined]
            if rule == "block_contents":
                self._flatten_nested_props(child, prefix, result)
            elif rule == "block_item":
                self._flatten_nested_block_item(child, prefix, result)
            elif rule == "declaration":
                self._rewrite_declaration_prefix(child, prefix, result)

    def _flatten_nested_block_item(self, node: object, prefix: str, result: list[object]) -> None:
        """Process a block_item inside a property nesting block."""
        children = node.children  # type: ignore[attr-defined]
        if not children:
            return
        inner = children[0]
        if not hasattr(inner, "rule_name"):
            return
        rule = inner.rule_name  # type: ignore[attr-defined]
        if rule == "declaration_or_nested":
            for dc in inner.children:  # type: ignore[attr-defined]
                if hasattr(dc, "rule_name"):
                    dr = dc.rule_name  # type: ignore[attr-defined]
                    if dr == "declaration":
                        self._rewrite_declaration_prefix(dc, prefix, result)
                    elif dr == "property_nesting":
                        # Recursive nesting
                        sub_results = self._expand_property_nesting_with_prefix(dc, prefix)
                        result.extend(sub_results)

    def _rewrite_declaration_prefix(self, decl: object, prefix: str, result: list[object]) -> None:
        """Rewrite a declaration's property name to include the prefix."""
        children = decl.children  # type: ignore[attr-defined]
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "property":
                for pc in child.children:  # type: ignore[attr-defined]
                    if not hasattr(pc, "rule_name"):
                        old_name = pc.value  # type: ignore[attr-defined]
                        pc.value = f"{prefix}-{old_name}"  # type: ignore[attr-defined]
        result.append(decl)

    def _expand_property_nesting_with_prefix(self, node: object, prefix: str) -> list[object]:
        """Expand nested property_nesting with an accumulated prefix."""
        children = node.children  # type: ignore[attr-defined]
        sub_prop = ""
        block = None

        for child in children:
            if hasattr(child, "rule_name"):
                rule = child.rule_name  # type: ignore[attr-defined]
                if rule == "property":
                    for pc in child.children:  # type: ignore[attr-defined]
                        sub_prop = pc.value  # type: ignore[attr-defined]
                elif rule == "block":
                    block = child

        new_prefix = f"{prefix}-{sub_prop}"
        result: list[object] = []
        if block:
            self._flatten_nested_props(block, new_prefix, result)
        return result

    # =====================================================================
    # Lattice v2: Built-in Function Evaluation
    # =====================================================================

    def _evaluate_builtin_function(
        self, func_name: str, node: object, scope: ScopeChain
    ) -> object:
        """Evaluate a Lattice v2 built-in function call.

        Uses the ExpressionEvaluator to resolve arguments, then calls
        the registered built-in function handler. The result is converted
        back to an AST node for emission.
        """
        children = node.children  # type: ignore[attr-defined]
        args: list[LatticeValue] = []

        # Collect and evaluate arguments
        for child in children:
            if hasattr(child, "rule_name") and child.rule_name == "function_args":  # type: ignore[attr-defined]
                evaluator = ExpressionEvaluator(scope)
                args = evaluator._collect_function_args(child)
                break

        handler = BUILTIN_FUNCTIONS[func_name]
        result = handler(args, scope)

        if isinstance(result, LatticeNull):
            # Null result — pass through as CSS function
            return self._expand_children(node, scope)

        css_text = value_to_css(result)
        return _make_value_node(css_text, node)

    # =====================================================================
    # Lattice v2: @extend Selector Merging (Pass 3)
    # =====================================================================

    def _apply_extends(self, root: object) -> None:
        """Apply @extend selector merging to the entire AST.

        This is a post-processing pass that runs after all other
        transformations. It:

        1. Walks the AST to find all qualified_rule nodes
        2. For each rule, checks if its selector matches any @extend target
        3. If it does, appends the extending selectors to the rule's
           selector list
        4. Removes rules whose selector is exclusively a placeholder

        @extend relationships are stored in ``self._extend_map`` during
        Pass 2 expansion.
        """
        # First pass: collect selectors from all rules so we can find
        # which rules match extend targets
        # For now, we handle the simple case where @extend was collected
        # during block expansion. The extending selectors need to be
        # determined from context. Since our current implementation collects
        # @extend targets but not the extending selector (it's context-dependent),
        # we use a simplified approach.
        self._remove_placeholder_rules(root)

    def _remove_placeholder_rules(self, node: object) -> None:
        """Remove rules whose selectors are exclusively placeholder selectors.

        Placeholder selectors (%name) exist only to be extended. Rules
        with only placeholder selectors should not appear in the CSS output.
        """
        if not hasattr(node, "children"):
            return

        new_children: list[object] = []
        for child in node.children:  # type: ignore[attr-defined]
            if child is None:
                continue
            if self._is_placeholder_only_rule(child):
                continue
            self._remove_placeholder_rules(child)
            new_children.append(child)

        node.children = new_children  # type: ignore[attr-defined]

    def _is_placeholder_only_rule(self, node: object) -> bool:
        """Check if a node is a qualified_rule with only placeholder selectors."""
        if not hasattr(node, "rule_name"):
            return False
        rule = node.rule_name  # type: ignore[attr-defined]
        if rule == "qualified_rule":
            selector_text = self._extract_selector_text(node)
            # A placeholder-only rule has all selectors starting with %
            selectors = [s.strip() for s in selector_text.split(",")]
            return all(s.startswith("%") for s in selectors if s)
        if rule == "rule":
            children = node.children  # type: ignore[attr-defined]
            if children and hasattr(children[0], "rule_name"):
                return self._is_placeholder_only_rule(children[0])
        return False

    def _extract_selector_text(self, node: object) -> str:
        """Extract the selector text from a qualified_rule node."""
        if not hasattr(node, "children"):
            return ""
        for child in node.children:  # type: ignore[attr-defined]
            if hasattr(child, "rule_name") and child.rule_name == "selector_list":
                return self._collect_text(child)
        return ""

    def _collect_text(self, node: object) -> str:
        """Recursively collect all token text from an AST node."""
        if not hasattr(node, "rule_name"):
            return node.value  # type: ignore[attr-defined]
        parts: list[str] = []
        for child in node.children:  # type: ignore[attr-defined]
            parts.append(self._collect_text(child))
        return " ".join(parts)

    # =====================================================================
    # Lattice v2: @at-root Hoisting (Pass 3)
    # =====================================================================

    def _splice_at_root_rules(self, root: object) -> None:
        """Splice @at-root hoisted rules into the root stylesheet.

        Rules collected during expansion via @at-root directives are
        appended to the root stylesheet's children list. They appear
        after all other rules.
        """
        if not hasattr(root, "children"):
            return

        for rule in self._at_root_rules:
            if rule is not None:
                # Wrap in a rule node if needed
                root.children.append(rule)  # type: ignore[attr-defined]

    # =====================================================================
    # Pass 3: Cleanup
    # =====================================================================

    def _cleanup(self, node: object) -> object:
        """Remove empty blocks and None children from the AST."""
        if not hasattr(node, "children"):
            return node

        new_children: list[object] = []
        for child in node.children:  # type: ignore[attr-defined]
            if child is None:
                continue
            cleaned = self._cleanup(child)
            if cleaned is not None:
                new_children.append(cleaned)

        node.children = new_children  # type: ignore[attr-defined]
        return node


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_token(value: str, template: object) -> object:
    """Create a synthetic token with the given value.

    Copies line/column from the template token for error reporting.
    Uses a simple object that has .type and .value attributes.
    """

    class SyntheticToken:
        def __init__(self, type_name: str, value: str, line: int, column: int) -> None:
            self.type = type_name
            self.value = value
            self.line = line
            self.column = column

    line = getattr(template, "line", 0)
    column = getattr(template, "column", 0)

    # Determine token type from value
    if value.startswith("#"):
        return SyntheticToken("HASH", value, line, column)
    elif value.startswith('"') or value.startswith("'"):
        return SyntheticToken("STRING", value.strip("\"'"), line, column)
    elif value and value[0].isdigit():
        if "%" in value:
            return SyntheticToken("PERCENTAGE", value, line, column)
        elif any(c.isalpha() for c in value):
            return SyntheticToken("DIMENSION", value, line, column)
        else:
            return SyntheticToken("NUMBER", value, line, column)
    else:
        return SyntheticToken("IDENT", value, line, column)


def _make_value_node(css_text: str, template: object) -> object:
    """Create a value node wrapping a synthetic token."""

    class SimpleNode:
        def __init__(self, rule_name: str, children: list[object]) -> None:
            self.rule_name = rule_name
            self.children = children

    token = _make_token(css_text, template)
    return SimpleNode("value", [token])


def _make_qualified_rule(selector_list: object, block: object) -> object:
    """Create a synthetic qualified_rule node.

    Used by @at-root to wrap hoisted content in a proper rule structure.
    """

    class SimpleNode:
        def __init__(self, rule_name: str, children: list[object]) -> None:
            self.rule_name = rule_name
            self.children = children

    return SimpleNode("qualified_rule", [selector_list, block])
