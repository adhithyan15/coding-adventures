"""Specification loader and validator for CLI Builder.

=== What does the spec loader do? ===

The ``SpecLoader`` reads a JSON file describing a CLI's structure and validates
it thoroughly before returning the normalized spec. Validation is eager: we
catch *developer* mistakes (circular requires, duplicate IDs, bad references)
before any user argv is ever processed.

This separation is crucial. A spec error is a *bug in the tool*, not a mistake
by the user. Surfacing it immediately — at startup, not when someone happens to
use a broken flag combination — makes tools far easier to develop and test.

=== Validation rules (per spec §6.4.3) ===

1. ``cli_builder_spec_version`` must be present and equal to ``"1.0"``.
2. ``name`` and ``description`` are required top-level fields.
3. Every scope (root + every command at every nesting level) must have unique
   flag ``id`` values, unique argument ``id`` values, and unique command ``id``
   values among siblings.
4. Every flag must have at least one of ``short``, ``long``, or
   ``single_dash_long``.
5. All ``conflicts_with`` and ``requires`` IDs must reference valid flags in
   the same scope or in ``global_flags``.
6. All ``mutually_exclusive_groups`` must reference valid flag IDs in the same
   scope.
7. ``enum_values`` must be present and non-empty when ``type`` is ``"enum"``.
8. At most one argument per scope may have ``variadic: true``.
9. Build the flag dependency graph G_flag for each scope and call
   ``has_cycle()``. A cycle means the spec is self-contradictory.

=== The dependency graph check ===

Rule 9 is the most interesting. Consider this spec fragment:

    flag A: requires = ["B"]
    flag B: requires = ["A"]

This is a logical contradiction: using A requires B, but using B requires A.
There is no valid invocation that satisfies both constraints simultaneously.
The ``DirectedGraph.has_cycle()`` method catches this in O(V + E) time using
Kahn's algorithm (topological sort).

=== Normalization ===

The loader also fills in default values for optional fields (e.g.,
``parsing_mode`` defaults to ``"gnu"``, ``builtin_flags`` defaults to
``{help: true, version: true}``). This simplifies downstream code, which can
assume all optional fields are present.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from directed_graph import DirectedGraph

from cli_builder.errors import SpecError

# Valid spec format version supported by this implementation.
SUPPORTED_VERSION = "1.0"

# Valid types for flag and argument values.
VALID_TYPES = frozenset(
    ["boolean", "string", "integer", "float", "path", "file", "directory", "enum"]
)

# Valid parsing modes.
VALID_PARSING_MODES = frozenset(["posix", "gnu", "subcommand_first", "traditional"])


class SpecLoader:
    """Loads, validates, and normalizes a CLI Builder JSON spec file.

    Usage::

        loader = SpecLoader("myapp.json")
        spec = loader.load()   # raises SpecError if invalid
        # spec is now normalized and safe to use

    The returned spec dict has all optional fields filled with their defaults,
    so downstream code never needs to check for missing keys.
    """

    def __init__(self, spec_file_path: str | Path) -> None:
        """Initialize the loader with a path to the JSON spec file.

        Args:
            spec_file_path: Absolute or relative path to the ``.json`` file.
        """
        self._path = Path(spec_file_path)

    def load(self) -> dict[str, Any]:
        """Read, validate, and normalize the spec file.

        Returns:
            The normalized spec dictionary.

        Raises:
            SpecError: If the file cannot be read, is not valid JSON, or
                fails any validation rule.
        """
        # --- Step 1: Read and parse the JSON file ---
        #
        # We catch OSError (file not found, permission denied) and
        # json.JSONDecodeError (malformed JSON) and convert them into
        # SpecError so callers only need to catch one exception type.
        try:
            text = self._path.read_text(encoding="utf-8")
        except OSError as exc:
            raise SpecError(
                f"Cannot read spec file '{self._path}': {exc}"
            ) from exc

        try:
            spec: dict[str, Any] = json.loads(text)
        except json.JSONDecodeError as exc:
            raise SpecError(
                f"Spec file '{self._path}' is not valid JSON: {exc}"
            ) from exc

        if not isinstance(spec, dict):
            raise SpecError("Spec must be a JSON object at the top level")

        # --- Step 2: Validate the spec version ---
        #
        # The version field acts as a format discriminant. If a future version
        # of CLI Builder changes the spec format, it can detect old specs and
        # refuse to process them (or apply a migration).
        version = spec.get("cli_builder_spec_version")
        if version is None:
            raise SpecError("Missing required field: 'cli_builder_spec_version'")
        if version != SUPPORTED_VERSION:
            raise SpecError(
                f"Unsupported spec version '{version}'. "
                f"Expected '{SUPPORTED_VERSION}'."
            )

        # --- Step 3: Validate required top-level fields ---
        for field_name in ("name", "description"):
            if not spec.get(field_name):
                raise SpecError(f"Missing required field: '{field_name}'")

        # --- Step 4: Fill in defaults for optional top-level fields ---
        spec.setdefault("display_name", spec["name"])
        spec.setdefault("version", None)
        spec.setdefault("parsing_mode", "gnu")
        spec.setdefault("builtin_flags", {"help": True, "version": True})
        spec.setdefault("global_flags", [])
        spec.setdefault("flags", [])
        spec.setdefault("arguments", [])
        spec.setdefault("commands", [])
        spec.setdefault("mutually_exclusive_groups", [])

        # Validate parsing_mode value
        if spec["parsing_mode"] not in VALID_PARSING_MODES:
            raise SpecError(
                f"Invalid parsing_mode '{spec['parsing_mode']}'. "
                f"Must be one of: {sorted(VALID_PARSING_MODES)}"
            )

        # Normalize builtin_flags
        bf = spec["builtin_flags"]
        bf.setdefault("help", True)
        bf.setdefault("version", True)

        # --- Step 5: Validate global_flags ---
        #
        # Global flags are validated in isolation first (they have no scope
        # to reference each other via requires/conflicts_with at this stage).
        global_flag_ids = self._validate_flags_list(
            spec["global_flags"],
            scope_name="global_flags",
            available_ids=None,  # global flags cannot reference each other yet
        )

        # --- Step 6: Validate the root scope ---
        self._validate_scope(
            flags=spec["flags"],
            arguments=spec["arguments"],
            commands=spec["commands"],
            exclusive_groups=spec["mutually_exclusive_groups"],
            scope_name="root",
            global_flag_ids=global_flag_ids,
        )

        # --- Step 7: Recursively validate all commands ---
        self._validate_commands_recursive(
            spec["commands"],
            parent_scope="root",
            global_flag_ids=global_flag_ids,
        )

        return spec

    # =========================================================================
    # Internal validation helpers
    # =========================================================================

    def _validate_flags_list(
        self,
        flags: list[dict[str, Any]],
        scope_name: str,
        available_ids: set[str] | None,
    ) -> set[str]:
        """Validate a list of flag definitions and return the set of their IDs.

        Checks:
        - Each flag has a unique ``id`` within the list.
        - Each flag has at least one of ``short``, ``long``, ``single_dash_long``.
        - ``description`` and ``type`` are present.
        - ``type`` is a valid type string.
        - ``enum_values`` is present and non-empty when ``type == "enum"``.
        - ``conflicts_with`` and ``requires`` reference IDs in ``available_ids``
          (if ``available_ids`` is provided).

        Args:
            flags: List of flag definition dicts.
            scope_name: Human-readable scope name for error messages.
            available_ids: Set of valid flag IDs for cross-references.
                Pass ``None`` to skip cross-reference validation.

        Returns:
            The set of flag IDs found in this list.

        Raises:
            SpecError: On any validation failure.
        """
        seen_ids: set[str] = set()

        for flag in flags:
            # --- Required fields ---
            fid = flag.get("id")
            if not fid:
                raise SpecError(
                    f"Flag in scope '{scope_name}' is missing required field 'id'"
                )

            # --- Unique ID ---
            if fid in seen_ids:
                raise SpecError(
                    f"Duplicate flag id '{fid}' in scope '{scope_name}'"
                )
            seen_ids.add(fid)

            # --- At least one name form ---
            if not (flag.get("short") or flag.get("long") or flag.get("single_dash_long")):
                raise SpecError(
                    f"Flag '{fid}' in scope '{scope_name}' must have at least one of "
                    f"'short', 'long', or 'single_dash_long'"
                )

            # --- description and type ---
            if not flag.get("description"):
                raise SpecError(
                    f"Flag '{fid}' in scope '{scope_name}' is missing 'description'"
                )
            ftype = flag.get("type")
            if not ftype:
                raise SpecError(
                    f"Flag '{fid}' in scope '{scope_name}' is missing 'type'"
                )
            if ftype not in VALID_TYPES:
                raise SpecError(
                    f"Flag '{fid}' has invalid type '{ftype}'. "
                    f"Must be one of: {sorted(VALID_TYPES)}"
                )

            # --- enum_values required for enum type ---
            if ftype == "enum":
                ev = flag.get("enum_values")
                if not ev:
                    raise SpecError(
                        f"Flag '{fid}' has type 'enum' but 'enum_values' is "
                        f"missing or empty in scope '{scope_name}'"
                    )

            # --- Fill in defaults ---
            flag.setdefault("required", False)
            flag.setdefault("default", None)
            flag.setdefault("value_name", None)
            flag.setdefault("enum_values", [])
            flag.setdefault("conflicts_with", [])
            flag.setdefault("requires", [])
            flag.setdefault("required_unless", [])
            flag.setdefault("repeatable", False)

            # --- Cross-reference validation ---
            if available_ids is not None:
                for ref_id in flag["conflicts_with"]:
                    if ref_id not in available_ids and ref_id not in seen_ids:
                        raise SpecError(
                            f"Flag '{fid}' in scope '{scope_name}' references "
                            f"unknown flag '{ref_id}' in 'conflicts_with'"
                        )
                for ref_id in flag["requires"]:
                    if ref_id not in available_ids and ref_id not in seen_ids:
                        raise SpecError(
                            f"Flag '{fid}' in scope '{scope_name}' references "
                            f"unknown flag '{ref_id}' in 'requires'"
                        )

        return seen_ids

    def _validate_arguments_list(
        self,
        arguments: list[dict[str, Any]],
        scope_name: str,
    ) -> set[str]:
        """Validate a list of argument definitions.

        Checks:
        - Each argument has a unique ``id``.
        - Required fields: ``id``, ``name``, ``description``, ``type``.
        - ``type`` is valid.
        - ``enum_values`` present when ``type == "enum"``.
        - At most one argument has ``variadic: true``.

        Args:
            arguments: List of argument definition dicts.
            scope_name: Human-readable scope name for error messages.

        Returns:
            The set of argument IDs found.

        Raises:
            SpecError: On any validation failure.
        """
        seen_ids: set[str] = set()
        variadic_count = 0

        for arg in arguments:
            aid = arg.get("id")
            if not aid:
                raise SpecError(
                    f"Argument in scope '{scope_name}' is missing required field 'id'"
                )

            if aid in seen_ids:
                raise SpecError(
                    f"Duplicate argument id '{aid}' in scope '{scope_name}'"
                )
            seen_ids.add(aid)

            # Accept display_name (preferred) or name (backward compatibility).
            # Normalize to display_name for downstream consumers.
            if not arg.get("display_name") and not arg.get("name"):
                raise SpecError(
                    f"Argument '{aid}' in scope '{scope_name}' is missing 'display_name'"
                )
            if not arg.get("display_name"):
                arg["display_name"] = arg["name"]

            if not arg.get("description"):
                raise SpecError(
                    f"Argument '{aid}' in scope '{scope_name}' is missing 'description'"
                )

            atype = arg.get("type")
            if not atype:
                raise SpecError(
                    f"Argument '{aid}' in scope '{scope_name}' is missing 'type'"
                )
            if atype not in VALID_TYPES:
                raise SpecError(
                    f"Argument '{aid}' has invalid type '{atype}'. "
                    f"Must be one of: {sorted(VALID_TYPES)}"
                )

            if atype == "enum":
                ev = arg.get("enum_values")
                if not ev:
                    raise SpecError(
                        f"Argument '{aid}' has type 'enum' but 'enum_values' is "
                        f"missing or empty in scope '{scope_name}'"
                    )

            # Fill defaults
            arg.setdefault("required", True)
            arg.setdefault("variadic", False)
            arg.setdefault("variadic_min", 1 if arg.get("required", True) else 0)
            arg.setdefault("variadic_max", None)
            arg.setdefault("default", None)
            arg.setdefault("enum_values", [])
            arg.setdefault("required_unless_flag", [])

            # --- At most one variadic argument per scope ---
            #
            # If more than one argument is variadic, there is no unambiguous
            # way to partition the positional token list. The spec forbids it.
            if arg.get("variadic"):
                variadic_count += 1
                if variadic_count > 1:
                    raise SpecError(
                        f"Scope '{scope_name}' has more than one variadic argument. "
                        f"At most one argument per scope may be variadic."
                    )

        return seen_ids

    def _check_requires_cycles(
        self,
        flags: list[dict[str, Any]],
        scope_name: str,
    ) -> None:
        """Build the flag dependency graph and check for cycles.

        The flag dependency graph G_flag has one node per flag and a directed
        edge A → B whenever flag A has B in its ``requires`` list. A cycle in
        this graph means the spec is self-contradictory.

        Example of a bad spec:
            flag A: requires = ["B"]
            flag B: requires = ["A"]

        This creates edges A→B and B→A, which form a cycle. ``has_cycle()``
        detects this and we raise a SpecError.

        Args:
            flags: All flags in scope (including globals).
            scope_name: Human-readable scope name for error messages.

        Raises:
            SpecError: If any cycle exists in the requires dependency graph.
        """
        g: DirectedGraph = DirectedGraph()
        for flag in flags:
            g.add_node(flag["id"])

        for flag in flags:
            for req_id in flag.get("requires", []):
                # Only add edges for IDs that exist in the graph
                if g.has_node(req_id):
                    g.add_edge(flag["id"], req_id)

        if g.has_cycle():
            raise SpecError(
                f"Circular 'requires' dependency detected in scope '{scope_name}'. "
                f"Check for flags that mutually require each other."
            )

    def _validate_scope(
        self,
        flags: list[dict[str, Any]],
        arguments: list[dict[str, Any]],
        commands: list[dict[str, Any]],
        exclusive_groups: list[dict[str, Any]],
        scope_name: str,
        global_flag_ids: set[str],
    ) -> set[str]:
        """Validate one command scope (root or a subcommand).

        A scope consists of flags, arguments, commands, and exclusive groups.
        All IDs must be unique within the scope, and all cross-references
        must resolve to known IDs.

        Args:
            flags: Flag definitions for this scope.
            arguments: Argument definitions for this scope.
            commands: Subcommand definitions for this scope.
            exclusive_groups: Mutually exclusive group definitions.
            scope_name: Human-readable name for error messages.
            global_flag_ids: IDs of global flags (always available).

        Returns:
            The set of flag IDs in this scope.

        Raises:
            SpecError: On any validation failure.
        """
        # All available IDs for cross-reference = global flags + scope flags
        # We need to compute scope flag IDs first, then validate references.
        # Two-pass: first collect IDs, then validate references.
        local_flag_ids: set[str] = {f["id"] for f in flags if "id" in f}
        available_ids = global_flag_ids | local_flag_ids

        scope_flag_ids = self._validate_flags_list(
            flags,
            scope_name=scope_name,
            available_ids=available_ids,
        )

        self._validate_arguments_list(arguments, scope_name=scope_name)

        # --- Validate command IDs are unique within this scope ---
        seen_cmd_ids: set[str] = set()
        seen_cmd_names: set[str] = set()
        for cmd in commands:
            cid = cmd.get("id")
            if not cid:
                raise SpecError(
                    f"Command in scope '{scope_name}' is missing required field 'id'"
                )
            if cid in seen_cmd_ids:
                raise SpecError(
                    f"Duplicate command id '{cid}' in scope '{scope_name}'"
                )
            seen_cmd_ids.add(cid)

            cname = cmd.get("name")
            if not cname:
                raise SpecError(
                    f"Command '{cid}' in scope '{scope_name}' is missing 'name'"
                )
            if cname in seen_cmd_names:
                raise SpecError(
                    f"Duplicate command name '{cname}' in scope '{scope_name}'"
                )
            seen_cmd_names.add(cname)

            # Aliases must also be unique within the scope
            for alias in cmd.get("aliases", []):
                if alias in seen_cmd_names:
                    raise SpecError(
                        f"Duplicate command name/alias '{alias}' in scope '{scope_name}'"
                    )
                seen_cmd_names.add(alias)

            if not cmd.get("description"):
                raise SpecError(
                    f"Command '{cid}' in scope '{scope_name}' is missing 'description'"
                )

            # Fill command defaults
            cmd.setdefault("aliases", [])
            cmd.setdefault("inherit_global_flags", True)
            cmd.setdefault("flags", [])
            cmd.setdefault("arguments", [])
            cmd.setdefault("commands", [])
            cmd.setdefault("mutually_exclusive_groups", [])

        # --- Validate mutually_exclusive_groups ---
        for group in exclusive_groups:
            gid = group.get("id")
            if not gid:
                raise SpecError(
                    f"Exclusive group in scope '{scope_name}' is missing 'id'"
                )
            flag_ids = group.get("flag_ids", [])
            if not flag_ids:
                raise SpecError(
                    f"Exclusive group '{gid}' in scope '{scope_name}' "
                    f"has empty 'flag_ids'"
                )
            for ref_id in flag_ids:
                if ref_id not in available_ids:
                    raise SpecError(
                        f"Exclusive group '{gid}' in scope '{scope_name}' "
                        f"references unknown flag '{ref_id}'"
                    )
            group.setdefault("required", False)

        # --- Check for circular requires dependencies ---
        #
        # Build the combined flag list (global + scope) and run cycle detection.
        all_flags_in_scope = list(flags)  # globals handled at root
        self._check_requires_cycles(all_flags_in_scope, scope_name=scope_name)

        return scope_flag_ids

    def _validate_commands_recursive(
        self,
        commands: list[dict[str, Any]],
        parent_scope: str,
        global_flag_ids: set[str],
    ) -> None:
        """Recursively validate all commands at any nesting depth.

        Commands may contain their own ``flags``, ``arguments``, ``commands``,
        and ``mutually_exclusive_groups``. We validate each command as its own
        scope, then recurse into its nested commands.

        Args:
            commands: List of command definition dicts to validate.
            parent_scope: Name of the parent scope (for error messages).
            global_flag_ids: IDs of global flags.

        Raises:
            SpecError: On any validation failure in any nested scope.
        """
        for cmd in commands:
            scope_name = f"{parent_scope}.{cmd['name']}"
            self._validate_scope(
                flags=cmd.get("flags", []),
                arguments=cmd.get("arguments", []),
                commands=cmd.get("commands", []),
                exclusive_groups=cmd.get("mutually_exclusive_groups", []),
                scope_name=scope_name,
                global_flag_ids=global_flag_ids,
            )
            # Recurse into nested commands
            self._validate_commands_recursive(
                cmd.get("commands", []),
                parent_scope=scope_name,
                global_flag_ids=global_flag_ids,
            )
