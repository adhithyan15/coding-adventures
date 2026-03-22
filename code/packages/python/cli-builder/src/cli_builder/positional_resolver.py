"""Positional argument resolution for CLI Builder.

=== The positional resolution problem ===

Given a list of raw positional tokens (strings that are not flags or
subcommands) and a list of argument definitions, we need to assign each
token to an argument slot. This is straightforward when all arguments are
non-variadic: token[0] → arg[0], token[1] → arg[1], etc.

The hard case is when one argument is *variadic* — it can absorb zero or more
tokens. Consider ``cp``:

    cp a.txt b.txt c.txt /dest/

Here the spec has two arguments:
    - ``source`` (variadic=True, type=path): accepts one or more source files
    - ``dest`` (variadic=False, type=path): the destination directory

We cannot simply assign left-to-right, because that would give "dest" to the
variadic argument and leave "/dest/" unassigned.

=== The last-wins algorithm (spec §6.4.1) ===

The spec calls this the "last-wins" algorithm, implemented as:

1. Split arg_defs into three groups:
   - ``leading_defs``: args before the variadic
   - ``variadic_def``: the single variadic arg (if any)
   - ``trailing_defs``: args after the variadic

2. Assign tokens from the LEFT to leading_defs.
3. Assign tokens from the RIGHT to trailing_defs.
4. Everything in the middle goes to the variadic.

This naturally handles the ``cp``/``mv`` pattern without any ambiguity:

    positional_tokens = ["a.txt", "b.txt", "c.txt", "/dest/"]
    leading_defs  = []          (variadic is first)
    variadic_def  = "source"
    trailing_defs = ["dest"]    (after variadic)

    trailing_start = 4 - 1 = 3
    dest     = tokens[3] = "/dest/"
    variadic = tokens[0:3] = ["a.txt", "b.txt", "c.txt"]

=== Type coercion ===

Each token is coerced to the argument's declared type. Coercion errors produce
``invalid_value`` ParseErrors. See ``coerce_value()`` for the full type table.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cli_builder.errors import ParseError


def coerce_value(
    raw: str,
    arg_type: str,
    enum_values: list[str],
    context: list[str],
    arg_name: str,
) -> tuple[Any, ParseError | None]:
    """Coerce a raw string token to the specified type.

    Returns a (coerced_value, error_or_None) tuple. On success, error is None.
    On failure, coerced_value is None and error is a ParseError.

    Args:
        raw: The raw string token from argv.
        arg_type: The type string from the spec.
        enum_values: Valid values when type is "enum".
        context: The command_path (for error context).
        arg_name: The argument name/id (for error messages).

    Returns:
        A tuple of (coerced value or None, ParseError or None).
    """
    # boolean arguments don't really come through the positional path
    # (booleans are flags), but handle it defensively.
    if arg_type == "boolean":
        return raw.lower() in ("true", "1", "yes"), None

    if arg_type == "integer":
        try:
            return int(raw), None
        except ValueError:
            return None, ParseError(
                error_type="invalid_value",
                message=f"Invalid integer for argument '{arg_name}': '{raw}'",
                context=context,
            )

    if arg_type == "float":
        try:
            return float(raw), None
        except ValueError:
            return None, ParseError(
                error_type="invalid_value",
                message=f"Invalid float for argument '{arg_name}': '{raw}'",
                context=context,
            )

    if arg_type == "enum":
        if raw not in enum_values:
            return None, ParseError(
                error_type="invalid_enum_value",
                message=(
                    f"Invalid value '{raw}' for argument '{arg_name}'. "
                    f"Must be one of: {', '.join(enum_values)}"
                ),
                context=context,
            )
        return raw, None

    if arg_type == "string":
        if not raw:
            return None, ParseError(
                error_type="invalid_value",
                message=f"Argument '{arg_name}' must be a non-empty string",
                context=context,
            )
        return raw, None

    if arg_type == "path":
        # Paths are syntactically validated only — no filesystem check.
        # The spec explicitly says existence is NOT checked for "path" type.
        return raw, None

    if arg_type == "file":
        # "file" type: must refer to an existing, readable file.
        try:
            p = Path(raw)
            if not p.is_file():
                return None, ParseError(
                    error_type="invalid_value",
                    message=f"Argument '{arg_name}': '{raw}' is not an existing file",
                    context=context,
                )
        except (OSError, ValueError):
            return None, ParseError(
                error_type="invalid_value",
                message=f"Argument '{arg_name}': cannot access '{raw}'",
                context=context,
            )
        return raw, None

    if arg_type == "directory":
        # "directory" type: must refer to an existing directory.
        try:
            p = Path(raw)
            if not p.is_dir():
                return None, ParseError(
                    error_type="invalid_value",
                    message=f"Argument '{arg_name}': '{raw}' is not an existing directory",
                    context=context,
                )
        except (OSError, ValueError):
            return None, ParseError(
                error_type="invalid_value",
                message=f"Argument '{arg_name}': cannot access '{raw}'",
                context=context,
            )
        return raw, None

    # Unknown type — pass through as string (defensive default)
    return raw, None


class PositionalResolver:
    """Resolves positional tokens against an argument definition list.

    Implements the last-wins partitioning algorithm from spec §6.4.1:
    leading arguments are assigned left-to-right, trailing arguments are
    assigned right-to-left, and the variadic argument (if any) absorbs
    the middle.

    Usage::

        resolver = PositionalResolver(arg_defs)
        result = resolver.resolve(["a.txt", "b.txt", "/dest/"], parsed_flags)
        # result == {"source": ["a.txt", "b.txt"], "dest": "/dest/"}
    """

    def __init__(self, arg_defs: list[dict[str, Any]]) -> None:
        """Initialize with the argument definitions for the current scope.

        Args:
            arg_defs: List of argument definition dicts (per spec §2.3).
        """
        self._arg_defs = arg_defs

    def resolve(
        self,
        tokens: list[str],
        parsed_flags: dict[str, Any],
        context: list[str] | None = None,
    ) -> tuple[dict[str, Any], list[ParseError]]:
        """Assign positional tokens to argument slots.

        Args:
            tokens: List of raw positional string tokens from argv.
            parsed_flags: Already-parsed flags (used to check
                ``required_unless_flag`` exemptions).
            context: The command_path (for error context in ParseErrors).

        Returns:
            A tuple of (result_dict, errors_list). The result_dict maps
            argument IDs to their coerced values. errors_list contains any
            ParseErrors encountered. Both are always returned — callers check
            if errors_list is non-empty.
        """
        ctx = context or []
        errors: list[ParseError] = []
        result: dict[str, Any] = {}

        if not self._arg_defs:
            # No argument definitions: any tokens are unexpected.
            if tokens:
                errors.append(
                    ParseError(
                        error_type="too_many_arguments",
                        message=(
                            f"Expected no positional arguments, "
                            f"but got {len(tokens)}: {tokens!r}"
                        ),
                        context=ctx,
                    )
                )
            return result, errors

        # Find the variadic argument (if any).
        variadic_idx = -1
        for i, adef in enumerate(self._arg_defs):
            if adef.get("variadic"):
                variadic_idx = i
                break

        if variadic_idx == -1:
            # --- No variadic: simple one-to-one assignment ---
            result, errors = self._resolve_fixed(tokens, ctx, parsed_flags)
        else:
            # --- Variadic case: partition and assign ---
            result, errors = self._resolve_variadic(
                tokens, variadic_idx, ctx, parsed_flags
            )

        # Fill in defaults for absent optional arguments
        for adef in self._arg_defs:
            aid = adef["id"]
            if aid not in result:
                default = adef.get("default")
                result[aid] = default if default is not None else ([] if adef.get("variadic") else None)

        return result, errors

    def _is_required(
        self,
        adef: dict[str, Any],
        parsed_flags: dict[str, Any],
    ) -> bool:
        """Check if an argument is required given the current parsed flags.

        An argument is NOT required if any of its ``required_unless_flag``
        IDs is present in ``parsed_flags``.

        Args:
            adef: The argument definition.
            parsed_flags: Currently parsed flags.

        Returns:
            True if the argument must be provided.
        """
        if not adef.get("required", True):
            return False
        exempt_flags = adef.get("required_unless_flag", [])
        for flag_id in exempt_flags:
            val = parsed_flags.get(flag_id)
            if val is not None and val is not False:
                return False
        return True

    def _coerce_arg(
        self,
        raw: str,
        adef: dict[str, Any],
        ctx: list[str],
    ) -> tuple[Any, ParseError | None]:
        """Coerce a single token for a given argument definition.

        Args:
            raw: Raw string token.
            adef: Argument definition dict.
            ctx: Command path context.

        Returns:
            (coerced_value, error_or_None).
        """
        return coerce_value(
            raw=raw,
            arg_type=adef["type"],
            enum_values=adef.get("enum_values", []),
            context=ctx,
            arg_name=adef.get("display_name", adef.get("name", adef["id"])),
        )

    def _resolve_fixed(
        self,
        tokens: list[str],
        ctx: list[str],
        parsed_flags: dict[str, Any],
    ) -> tuple[dict[str, Any], list[ParseError]]:
        """Resolve tokens against a fixed (non-variadic) argument list.

        One-to-one assignment in definition order.

        Args:
            tokens: Positional tokens.
            ctx: Command path context.
            parsed_flags: Parsed flags (for required_unless_flag checks).

        Returns:
            (result_dict, errors_list).
        """
        errors: list[ParseError] = []
        result: dict[str, Any] = {}

        for i, adef in enumerate(self._arg_defs):
            if i < len(tokens):
                coerced, err = self._coerce_arg(tokens[i], adef, ctx)
                if err:
                    errors.append(err)
                else:
                    result[adef["id"]] = coerced
            elif self._is_required(adef, parsed_flags):
                errors.append(
                    ParseError(
                        error_type="missing_required_argument",
                        message=(
                            f"Missing required argument: "
                            f"<{adef.get('display_name', adef.get('name', adef['id']))}>"
                        ),
                        context=ctx,
                    )
                )

        if len(tokens) > len(self._arg_defs):
            errors.append(
                ParseError(
                    error_type="too_many_arguments",
                    message=(
                        f"Expected at most {len(self._arg_defs)} positional "
                        f"argument(s), but got {len(tokens)}"
                    ),
                    context=ctx,
                )
            )

        return result, errors

    def _resolve_variadic(
        self,
        tokens: list[str],
        variadic_idx: int,
        ctx: list[str],
        parsed_flags: dict[str, Any],
    ) -> tuple[dict[str, Any], list[ParseError]]:
        """Resolve tokens against an argument list with one variadic argument.

        Partitions tokens into leading / variadic / trailing segments.

        Args:
            tokens: Positional tokens.
            variadic_idx: Index of the variadic argument in self._arg_defs.
            ctx: Command path context.
            parsed_flags: Parsed flags.

        Returns:
            (result_dict, errors_list).
        """
        errors: list[ParseError] = []
        result: dict[str, Any] = {}

        leading_defs = self._arg_defs[:variadic_idx]
        variadic_def = self._arg_defs[variadic_idx]
        trailing_defs = self._arg_defs[variadic_idx + 1 :]

        # --- Assign leading arguments (left-to-right) ---
        for i, adef in enumerate(leading_defs):
            if i < len(tokens):
                coerced, err = self._coerce_arg(tokens[i], adef, ctx)
                if err:
                    errors.append(err)
                else:
                    result[adef["id"]] = coerced
            elif self._is_required(adef, parsed_flags):
                errors.append(
                    ParseError(
                        error_type="missing_required_argument",
                        message=(
                            f"Missing required argument: "
                            f"<{adef.get('display_name', adef.get('name', adef['id']))}>"
                        ),
                        context=ctx,
                    )
                )

        # --- Assign trailing arguments (right-to-left) ---
        trailing_start = len(tokens) - len(trailing_defs)
        for i, adef in enumerate(trailing_defs):
            token_idx = trailing_start + i
            if 0 <= token_idx < len(tokens):
                coerced, err = self._coerce_arg(tokens[token_idx], adef, ctx)
                if err:
                    errors.append(err)
                else:
                    result[adef["id"]] = coerced
            elif self._is_required(adef, parsed_flags):
                errors.append(
                    ParseError(
                        error_type="missing_required_argument",
                        message=(
                            f"Missing required argument: "
                            f"<{adef.get('display_name', adef.get('name', adef['id']))}>"
                        ),
                        context=ctx,
                    )
                )

        # --- Variadic gets everything in between ---
        variadic_end = max(len(leading_defs), trailing_start)
        variadic_tokens = tokens[len(leading_defs) : variadic_end]
        count = len(variadic_tokens)

        v_min = variadic_def.get("variadic_min", 1 if variadic_def.get("required", True) else 0)
        v_max = variadic_def.get("variadic_max", None)

        if count < v_min:
            errors.append(
                ParseError(
                    error_type="too_few_arguments",
                    message=(
                        f"Expected at least {v_min} "
                        f"<{variadic_def.get('display_name', variadic_def.get('name', variadic_def['id']))}>, "
                        f"got {count}"
                    ),
                    context=ctx,
                )
            )
        elif v_max is not None and count > v_max:
            errors.append(
                ParseError(
                    error_type="too_many_arguments",
                    message=(
                        f"Expected at most {v_max} "
                        f"<{variadic_def.get('display_name', variadic_def.get('name', variadic_def['id']))}>, "
                        f"got {count}"
                    ),
                    context=ctx,
                )
            )

        coerced_variadic = []
        for raw in variadic_tokens:
            coerced, err = self._coerce_arg(raw, variadic_def, ctx)
            if err:
                errors.append(err)
            else:
                coerced_variadic.append(coerced)

        result[variadic_def["id"]] = coerced_variadic

        return result, errors
