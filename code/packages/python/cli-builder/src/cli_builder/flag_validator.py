"""Flag constraint validation for CLI Builder.

=== What constraints do flags have? ===

After scanning argv, we have a dict of parsed flags. Before returning the
result to the application, we must verify that the combination of flags
the user provided is valid according to the spec. There are several kinds
of constraints:

1. **conflicts_with** — flag A and flag B cannot both be present.
   Example: ``grep -E`` (extended regexp) and ``-F`` (fixed strings) conflict.

2. **requires** (transitive) — if flag A is present, all flags that A requires
   (directly or transitively via G_flag) must also be present.
   Example: ``-h/--human-readable requires -l/--long-listing``.

3. **required** — a flag marked ``required: true`` must always be present,
   unless exempted by ``required_unless``.

4. **mutually_exclusive_groups** — at most one (or exactly one if ``required``)
   flag from a group may be used.

5. **duplicate_flag** — a non-``repeatable`` flag appeared more than once.
   (This is tracked during scanning in the parser, not here, but we handle
   the error type for completeness.)

=== Transitive requires via DirectedGraph ===

The ``requires`` constraint is transitive. If A requires B and B requires C,
then using A requires both B and C. We use ``DirectedGraph.transitive_closure``
to find all transitively required flags in O(V + E) time, where V is the
number of flags and E is the number of requires edges.

Recall from spec §4.2:
- G_flag has one node per flag in scope.
- An edge A → B means "A requires B".
- ``transitive_closure(A)`` returns all nodes reachable FROM A — i.e.,
  everything that A (directly or indirectly) requires.

=== Collecting all errors ===

The validator collects ALL constraint violations and returns them as a list.
This matches the spec's recommendation to report all errors at once (see §7,
"Collecting all errors is strongly preferred for usability").
"""

from __future__ import annotations

from typing import Any

from directed_graph import DirectedGraph
from directed_graph.algorithms import transitive_closure

from cli_builder.errors import ParseError


class FlagValidator:
    """Validates parsed flags against all spec constraints.

    Usage::

        validator = FlagValidator(active_flags, exclusive_groups)
        errors = validator.validate(parsed_flags, context=["git", "commit"])
        if errors:
            raise ParseErrors(errors)

    The validator checks conflicts, transitive requires, required flags,
    and mutually exclusive groups.
    """

    def __init__(
        self,
        active_flags: list[dict[str, Any]],
        exclusive_groups: list[dict[str, Any]],
    ) -> None:
        """Initialize the validator.

        Args:
            active_flags: All flag definitions in scope (global + command).
            exclusive_groups: Mutually exclusive group definitions for
                this scope.
        """
        self._flags = active_flags
        self._groups = exclusive_groups

        # Build a lookup dict: flag_id → flag_def
        self._by_id: dict[str, dict[str, Any]] = {
            f["id"]: f for f in active_flags
        }

        # Build the flag dependency graph G_flag.
        #
        # Nodes: one per flag in scope.
        # Edges: A → B when A.requires contains B.
        #
        # We build this once at validator construction time so the graph is
        # ready for transitive_closure() calls during validate().
        self._g_flag: DirectedGraph = DirectedGraph()
        for flag in active_flags:
            self._g_flag.add_node(flag["id"])
        for flag in active_flags:
            for req_id in flag.get("requires", []):
                if self._g_flag.has_node(req_id):
                    self._g_flag.add_edge(flag["id"], req_id)

    def validate(
        self,
        parsed_flags: dict[str, Any],
        context: list[str] | None = None,
    ) -> list[ParseError]:
        """Validate all flag constraints and return a list of errors.

        Checks (in order):
        1. conflicts_with — bilateral flag conflicts.
        2. requires (transitive) — missing dependency flags.
        3. required flags — flags that must be present.
        4. mutually_exclusive_groups — group violations.

        Args:
            parsed_flags: Dict mapping flag ID to parsed value. A flag is
                "present" if its value is not False and not None.
            context: The command_path (for error context).

        Returns:
            A list of ParseError objects. Empty if all constraints are satisfied.
        """
        ctx = context or []
        errors: list[ParseError] = []

        # Which flags are actually "present" (not False/None)?
        present: set[str] = {
            fid
            for fid, val in parsed_flags.items()
            if val is not False and val is not None
        }

        # --- Check 1: conflicts_with ---
        errors.extend(self._check_conflicts(present, ctx))

        # --- Check 2: transitive requires ---
        errors.extend(self._check_requires(present, ctx))

        # --- Check 3: required flags ---
        errors.extend(self._check_required(present, parsed_flags, ctx))

        # --- Check 4: mutually_exclusive_groups ---
        errors.extend(self._check_exclusive_groups(present, ctx))

        return errors

    # =========================================================================
    # Private checkers
    # =========================================================================

    def _check_conflicts(
        self,
        present: set[str],
        ctx: list[str],
    ) -> list[ParseError]:
        """Check conflicts_with constraints.

        For every present flag A, if any flag in A.conflicts_with is also
        present, record a conflicting_flags error.

        We use a seen_pairs set to avoid reporting the same conflict twice
        (once for A→B and once for B→A).

        Args:
            present: Set of flag IDs that are present.
            ctx: Command path context.

        Returns:
            List of ParseError objects for conflicts.
        """
        errors: list[ParseError] = []
        seen_pairs: set[frozenset[str]] = set()

        for fid in present:
            flag_def = self._by_id.get(fid)
            if flag_def is None:
                continue
            for other_id in flag_def.get("conflicts_with", []):
                if other_id in present:
                    pair = frozenset({fid, other_id})
                    if pair not in seen_pairs:
                        seen_pairs.add(pair)
                        a_display = self._flag_display(fid)
                        b_display = self._flag_display(other_id)
                        errors.append(
                            ParseError(
                                error_type="conflicting_flags",
                                message=(
                                    f"{a_display} and {b_display} "
                                    f"cannot be used together"
                                ),
                                context=ctx,
                            )
                        )

        return errors

    def _check_requires(
        self,
        present: set[str],
        ctx: list[str],
    ) -> list[ParseError]:
        """Check transitive requires constraints.

        For every present flag A, compute the transitive closure of A in
        G_flag (i.e., all flags that A transitively requires) and check
        that each is also present.

        Args:
            present: Set of flag IDs that are present.
            ctx: Command path context.

        Returns:
            List of ParseError objects for missing dependencies.
        """
        errors: list[ParseError] = []

        for fid in present:
            if not self._g_flag.has_node(fid):
                continue
            # transitive_closure returns all nodes reachable FROM fid.
            # These are all flags that fid (directly or indirectly) requires.
            required_ids: set[object] = transitive_closure(self._g_flag, fid)
            for req_id in required_ids:
                req_id_str = str(req_id)
                if req_id_str not in present:
                    errors.append(
                        ParseError(
                            error_type="missing_dependency_flag",
                            message=(
                                f"{self._flag_display(fid)} requires "
                                f"{self._flag_display(req_id_str)}"
                            ),
                            context=ctx,
                        )
                    )

        return errors

    def _check_required(
        self,
        present: set[str],
        parsed_flags: dict[str, Any],
        ctx: list[str],
    ) -> list[ParseError]:
        """Check that all required flags are present.

        A flag is exempt from being required if any flag in its
        ``required_unless`` list is present.

        Args:
            present: Set of flag IDs that are present.
            parsed_flags: Full parsed flags dict (for required_unless checks).
            ctx: Command path context.

        Returns:
            List of ParseError objects for missing required flags.
        """
        errors: list[ParseError] = []

        for flag_def in self._flags:
            if not flag_def.get("required", False):
                continue
            fid = flag_def["id"]
            if fid in present:
                continue

            # Check required_unless exemption
            exempt = False
            for unless_id in flag_def.get("required_unless", []):
                unless_val = parsed_flags.get(unless_id)
                if unless_val is not False and unless_val is not None:
                    exempt = True
                    break

            if not exempt:
                errors.append(
                    ParseError(
                        error_type="missing_required_flag",
                        message=(
                            f"{self._flag_display(fid)} is required"
                        ),
                        context=ctx,
                    )
                )

        return errors

    def _check_exclusive_groups(
        self,
        present: set[str],
        ctx: list[str],
    ) -> list[ParseError]:
        """Check mutually exclusive group constraints.

        For each group:
        - If more than one flag in the group is present → error.
        - If the group is required and no flags are present → error.

        Args:
            present: Set of flag IDs that are present.
            ctx: Command path context.

        Returns:
            List of ParseError objects for group violations.
        """
        errors: list[ParseError] = []

        for group in self._groups:
            flag_ids: list[str] = group.get("flag_ids", [])
            present_in_group = [fid for fid in flag_ids if fid in present]

            if len(present_in_group) > 1:
                # Build a nice display of all flags in the group.
                displays = ", ".join(
                    self._flag_display(fid) for fid in flag_ids
                )
                errors.append(
                    ParseError(
                        error_type="exclusive_group_violation",
                        message=(
                            f"Only one of {displays} may be used at a time"
                        ),
                        context=ctx,
                    )
                )

            elif group.get("required", False) and len(present_in_group) == 0:
                displays = ", ".join(
                    self._flag_display(fid) for fid in flag_ids
                )
                errors.append(
                    ParseError(
                        error_type="missing_exclusive_group",
                        message=(
                            f"One of {displays} is required"
                        ),
                        context=ctx,
                    )
                )

        return errors

    def _flag_display(self, flag_id: str) -> str:
        """Build a human-readable flag reference for error messages.

        Examples:
            -l/--long-listing
            --verbose
            -h

        Args:
            flag_id: The flag's ``id`` field.

        Returns:
            A string like ``-l/--long-listing`` or ``--verbose``.
        """
        flag_def = self._by_id.get(flag_id)
        if flag_def is None:
            return f"--{flag_id}"

        parts: list[str] = []
        if flag_def.get("short"):
            parts.append(f"-{flag_def['short']}")
        if flag_def.get("long"):
            parts.append(f"--{flag_def['long']}")
        if flag_def.get("single_dash_long"):
            parts.append(f"-{flag_def['single_dash_long']}")

        return "/".join(parts) if parts else f"--{flag_id}"
