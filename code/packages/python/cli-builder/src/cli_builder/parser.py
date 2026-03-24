"""The main CLI parsing engine.

=== Overview ===

The ``Parser`` class is the top-level entry point for CLI Builder. It ties
together all the sub-components:

- ``SpecLoader`` — validates and normalizes the JSON spec at construction time
- ``DirectedGraph`` — drives command routing (Phase 1)
- ``ModalStateMachine`` — drives token scanning (Phase 2)
- ``TokenClassifier`` — classifies each argv token during scanning
- ``FlagValidator`` — validates all flag constraints (Phase 3)
- ``PositionalResolver`` — assigns positional tokens to argument slots (Phase 3)
- ``HelpGenerator`` — produces help text when ``--help`` is encountered

=== Three-phase parse algorithm ===

Phase 1 — Routing (directed graph):

    The command routing graph G_cmd is a directed graph where each node is
    a command (or the root program) and each edge is labeled by the command
    name/alias that triggers the transition.

    We walk argv left-to-right. Each non-flag token that matches an outgoing
    edge label advances us deeper into the command tree. When we hit a token
    that doesn't match, routing ends and scanning begins. Flags are skipped
    during routing — they belong to Phase 2.

    After routing, we know the "leaf command" whose flag and argument schema
    will be used for the rest of the parse.

Phase 2 — Scanning (modal state machine):

    The ModalStateMachine has four modes:
    - SCANNING: the normal state. Tokens are classified and dispatched.
    - FLAG_VALUE: a non-boolean flag was just seen; next token is its value.
    - END_OF_FLAGS: ``--`` was seen; all remaining tokens are positional.
    - (ROUTING is handled in Phase 1, not by the modal machine.)

    We re-walk argv (skipping tokens consumed during routing) and process
    each one according to the current mode and the token's classification.

    Help and version flags cause immediate returns (no further processing).

Phase 3 — Validation:

    3a. Positional argument resolution: assign positional tokens to slots
        using the last-wins algorithm (PositionalResolver).
    3b. Flag constraint validation: check conflicts, requires, required flags,
        exclusive groups (FlagValidator).

=== Traditional mode (tar-style) ===

When ``parsing_mode`` is ``"traditional"``, the first non-subcommand token
(if it doesn't start with ``-``) is interpreted as a stack of short flag
characters without the leading dash. This allows ``tar xvf archive.tar``
to mean the same as ``tar -x -v -f archive.tar``.

=== Fuzzy matching ===

For unknown flag and unknown command errors, we compute Levenshtein edit
distance between the unknown token and all valid tokens at that scope.
If the closest match has distance ≤ 2, it is included as a ``suggestion``.

=== Design note: re-walking argv ===

We walk argv twice: once for routing (Phase 1) and once for scanning (Phase 2).
This is deliberate. Routing must see the full token stream to correctly
identify where subcommands end and flags/args begin, before we know which
flags are in scope. An alternative "single-pass" design would require the
router to also classify tokens before the active flag set is known — which
is circular. Two passes eliminate the circularity.
"""

from __future__ import annotations

from typing import Any

from directed_graph import DirectedGraph
from state_machine import DFA, ModalStateMachine

from cli_builder.errors import ParseError, ParseErrors, SpecError
from cli_builder.flag_validator import FlagValidator
from cli_builder.help_generator import HelpGenerator
from cli_builder.positional_resolver import PositionalResolver, coerce_value
from cli_builder.spec_loader import SpecLoader
from cli_builder.token_classifier import TokenClassifier, _is_valueless_type
from cli_builder.types import HelpResult, ParseResult, VersionResult


# =========================================================================
# Levenshtein distance for fuzzy suggestions
# =========================================================================


def _levenshtein(a: str, b: str) -> int:
    """Compute the Levenshtein edit distance between two strings.

    The edit distance is the minimum number of single-character edits
    (insertions, deletions, substitutions) needed to transform ``a``
    into ``b``.

    We use dynamic programming (Wagner–Fischer algorithm) with O(m*n)
    time and O(min(m,n)) space.

    Args:
        a: First string.
        b: Second string.

    Returns:
        Non-negative integer edit distance.
    """
    if a == b:
        return 0
    if not a:
        return len(b)
    if not b:
        return len(a)

    # Ensure a is the shorter string for space efficiency
    if len(a) > len(b):
        a, b = b, a

    prev_row = list(range(len(a) + 1))
    for j, char_b in enumerate(b):
        curr_row = [j + 1]
        for i, char_a in enumerate(a):
            insert = curr_row[i] + 1
            delete = prev_row[i + 1] + 1
            replace = prev_row[i] + (0 if char_a == char_b else 1)
            curr_row.append(min(insert, delete, replace))
        prev_row = curr_row

    return prev_row[len(a)]


def _fuzzy_suggest(token: str, candidates: list[str]) -> str | None:
    """Find the closest candidate within edit distance 2.

    Args:
        token: The unknown token entered by the user.
        candidates: Valid options at this scope.

    Returns:
        The closest candidate if distance ≤ 2, else None.
    """
    best: str | None = None
    best_dist = 3  # threshold: only suggest if distance ≤ 2

    for candidate in candidates:
        dist = _levenshtein(token, candidate)
        if dist < best_dist:
            best_dist = dist
            best = candidate

    return best


# =========================================================================
# Parser
# =========================================================================


class Parser:
    """Three-phase CLI argument parser driven by a JSON spec file.

    Usage::

        result = Parser("myapp.json", ["myapp", "--verbose", "file.txt"]).parse()

        if isinstance(result, HelpResult):
            print(result.text)
            raise SystemExit(0)
        elif isinstance(result, VersionResult):
            print(result.version)
            raise SystemExit(0)
        else:
            # result is ParseResult
            use(result.flags, result.arguments)

    Raises:
        SpecError: At construction time if the spec file is invalid.
        ParseErrors: From parse() if the argv is invalid.
    """

    def __init__(
        self,
        spec_file_path: str,
        argv: list[str],
    ) -> None:
        """Initialize the parser and validate the spec.

        The spec is loaded and validated eagerly. If it is invalid, a
        ``SpecError`` is raised here (at construction time), not during
        ``parse()``. This ensures that spec errors surface immediately,
        regardless of what argv is passed.

        Args:
            spec_file_path: Path to the JSON spec file.
            argv: The full argv list including ``argv[0]`` (program name).

        Raises:
            SpecError: If the spec file is invalid.
        """
        loader = SpecLoader(spec_file_path)
        self._spec: dict[str, Any] = loader.load()
        self._argv = list(argv)

        # Build the command routing graph G_cmd.
        #
        # Nodes: program root + one node per command at any nesting depth.
        # Edges: parent → child, labeled by command name and aliases.
        # We use a flat dict: node_id → command_def to look up command metadata.
        self._g_cmd: DirectedGraph = DirectedGraph()
        self._cmd_map: dict[str, dict[str, Any]] = {}  # node_id → cmd_def
        self._build_routing_graph()

    # =========================================================================
    # Public API
    # =========================================================================

    def parse(self) -> ParseResult | HelpResult | VersionResult:
        """Parse argv against the spec.

        Returns:
            One of ``ParseResult``, ``HelpResult``, or ``VersionResult``.

        Raises:
            ParseErrors: If the argv contains errors.
        """
        argv = self._argv

        if not argv:
            raise ParseErrors(
                [
                    ParseError(
                        error_type="missing_required_argument",
                        message="argv must not be empty (must include program name)",
                        context=[],
                    )
                ]
            )

        program = argv[0]
        tokens = argv[1:]  # Strip argv[0] per spec §6.1

        # =====================================================================
        # Phase 1: Routing
        # =====================================================================
        command_path, leaf_node_id, remaining_tokens, routing_errors, consumed_indices = (
            self._phase1_routing(program, tokens)
        )

        # =====================================================================
        # Phase 2: Scanning
        # =====================================================================
        # Collect all flags in scope: global + all flags from command path.
        active_flags = self._collect_active_flags(command_path, leaf_node_id)

        # Check for --help / -h or --version early (before full scan).
        # This is allowed even if the spec has routing errors.
        quick_help = self._check_quick_help_version(
            tokens, active_flags, command_path, program
        )
        if quick_help is not None:
            return quick_help

        if routing_errors:
            raise ParseErrors(routing_errors)

        parsed_flags, positional_tokens, scan_errors, explicit_flags = self._phase2_scanning(
            tokens, command_path, leaf_node_id, active_flags, consumed_indices
        )

        # Check help/version again (they may have been detected in scan)
        if isinstance(parsed_flags, HelpResult) or isinstance(
            parsed_flags, VersionResult
        ):
            return parsed_flags  # type: ignore[return-value]

        # =====================================================================
        # Phase 3: Validation
        # =====================================================================
        all_errors: list[ParseError] = list(scan_errors)

        # 3a: Resolve positional arguments
        leaf_def = self._get_node_def(leaf_node_id)
        arg_defs = leaf_def.get("arguments", [])
        resolver = PositionalResolver(arg_defs)
        arguments, pos_errors = resolver.resolve(
            positional_tokens, parsed_flags, context=command_path
        )
        all_errors.extend(pos_errors)

        # 3b: Validate flag constraints
        leaf_flags = active_flags
        leaf_exclusive_groups = leaf_def.get("mutually_exclusive_groups", [])
        validator = FlagValidator(leaf_flags, leaf_exclusive_groups)
        flag_errors = validator.validate(parsed_flags, context=command_path)
        all_errors.extend(flag_errors)

        if all_errors:
            raise ParseErrors(all_errors)

        # Apply default values for absent flags
        final_flags = self._apply_flag_defaults(active_flags, parsed_flags)

        return ParseResult(
            program=program,
            command_path=command_path,
            flags=final_flags,
            arguments=arguments,
            explicit_flags=explicit_flags,
        )

    # =========================================================================
    # Phase 1: Routing
    # =========================================================================

    def _phase1_routing(
        self,
        program: str,
        tokens: list[str],
    ) -> tuple[list[str], str, list[str], list[ParseError], set[int]]:
        """Route tokens through the command tree.

        Walks tokens left-to-right. Non-flag tokens that match outgoing
        edges advance the current node. Flags and unknown tokens are skipped.

        Args:
            program: The program name (argv[0]).
            tokens: All tokens after argv[0].

        Returns:
            A 5-tuple:
            - command_path: List of canonical command names from root to leaf.
            - leaf_node_id: ID of the resolved leaf node.
            - remaining_tokens: Tokens NOT consumed by routing.
            - routing_errors: Any errors encountered during routing.
            - consumed_indices: Set of token indices (into tokens[]) consumed
              as subcommand names. Uses indices so that alias tokens (e.g. "a"
              for "add") are correctly identified regardless of canonical name.
        """
        command_path = [program]
        current_node_id = "__root__"
        errors: list[ParseError] = []
        consumed_indices: set[int] = set()  # indices into tokens[] consumed as subcommands

        # In subcommand_first mode, we require the first non-flag token to
        # be a subcommand. We track whether we've seen a non-subcommand
        # non-flag token to emit the right error.
        parsing_mode = self._spec.get("parsing_mode", "gnu")

        # Build the active flag set at root (for skip-flag logic)
        root_flags = self._collect_active_flags([program], "__root__")
        flag_lookup = self._build_flag_lookup(root_flags)

        i = 0
        while i < len(tokens):
            token = tokens[i]

            # End-of-flags stops routing
            if token == "--":
                break

            # Skip flags (they're handled in Phase 2)
            if token.startswith("-"):
                # If this flag takes a value, skip the next token too
                i += self._skip_flag(token, i, tokens, flag_lookup)
                continue

            # Check if this token is a valid subcommand
            successors = list(self._g_cmd.successors(current_node_id))
            canonical = self._resolve_command_name(token, successors)

            if canonical is not None:
                command_path.append(canonical)
                current_node_id = canonical
                consumed_indices.add(i)  # record actual token index, not canonical name
                # Update flag lookup for new scope
                scope_flags = self._collect_active_flags(command_path, current_node_id)
                flag_lookup = self._build_flag_lookup(scope_flags)
                i += 1
            else:
                # Token is not a known subcommand.
                # In subcommand_first mode, this is an error.
                if parsing_mode == "subcommand_first":
                    valid_names = self._get_subcommand_names(successors)
                    suggestion = _fuzzy_suggest(token, valid_names)
                    errors.append(
                        ParseError(
                            error_type="unknown_command",
                            message=f"Unknown command '{token}'",
                            suggestion=f"Did you mean '{suggestion}'?" if suggestion else None,
                            context=list(command_path),
                        )
                    )
                break

        return command_path, current_node_id, tokens, errors, consumed_indices

    # =========================================================================
    # Phase 2: Scanning
    # =========================================================================

    def _phase2_scanning(
        self,
        tokens: list[str],
        command_path: list[str],
        leaf_node_id: str,
        active_flags: list[dict[str, Any]],
        consumed_indices: set[int] | None = None,
    ) -> tuple[dict[str, Any] | HelpResult | VersionResult, list[str], list[ParseError], list[str]]:
        """Scan tokens and produce parsed flags and positional tokens.

        Uses a ModalStateMachine with three modes:
        - SCANNING: normal token processing
        - FLAG_VALUE: consuming the value for a non-boolean flag
        - END_OF_FLAGS: all remaining tokens are positional

        The routing was done in Phase 1; here we re-walk the full token list
        and skip tokens that were consumed as subcommand names.

        Args:
            tokens: All tokens after argv[0].
            command_path: The resolved command path (including program name).
            leaf_node_id: The resolved leaf command node ID.
            active_flags: All flags in scope.
            consumed_indices: Set of token indices consumed as subcommand names
                during Phase 1. Used to correctly skip alias tokens. If None,
                falls back to name-based matching (legacy behaviour).

        Returns:
            A 4-tuple of (parsed_flags_or_special_result, positional_tokens,
            errors, explicit_flags). The explicit_flags list tracks which flag
            IDs were explicitly set by the user (v1.1 feature).
        """
        # Build the modal state machine.
        #
        # We use a minimal DFA for each mode. The DFA doesn't actually
        # classify tokens — that's the TokenClassifier's job. The DFA just
        # tracks the current mode state (scanning vs waiting for flag value
        # vs end-of-flags).
        #
        # SCANNING mode: the main loop
        # FLAG_VALUE mode: the next token is a value for pending_flag
        # END_OF_FLAGS mode: all remaining tokens are positional
        scanning_dfa = DFA(
            states={"scanning"},
            alphabet={"token"},
            transitions={("scanning", "token"): "scanning"},
            initial="scanning",
            accepting={"scanning"},
        )
        flag_value_dfa = DFA(
            states={"await_value"},
            alphabet={"token"},
            transitions={("await_value", "token"): "await_value"},
            initial="await_value",
            accepting={"await_value"},
        )
        end_of_flags_dfa = DFA(
            states={"positional"},
            alphabet={"token"},
            transitions={("positional", "token"): "positional"},
            initial="positional",
            accepting={"positional"},
        )
        modal = ModalStateMachine(
            modes={
                "SCANNING": scanning_dfa,
                "FLAG_VALUE": flag_value_dfa,
                "END_OF_FLAGS": end_of_flags_dfa,
            },
            mode_transitions={
                ("SCANNING", "to_flag_value"): "FLAG_VALUE",
                ("FLAG_VALUE", "to_scanning"): "SCANNING",
                ("SCANNING", "to_end_of_flags"): "END_OF_FLAGS",
                ("END_OF_FLAGS", "to_end_of_flags"): "END_OF_FLAGS",
            },
            initial_mode="SCANNING",
        )

        classifier = TokenClassifier(active_flags)
        flag_lookup = self._build_flag_lookup(active_flags)

        parsed_flags: dict[str, Any] = {}
        positional_tokens: list[str] = []
        errors: list[ParseError] = []
        pending_flag: dict[str, Any] | None = None
        flag_counts: dict[str, int] = {}  # for duplicate detection

        # --- explicit_flags tracking (v1.1) ---
        #
        # Every time a flag is consumed from argv, its ID is appended here.
        # This lets callers distinguish "flag was explicitly passed" from
        # "flag has its default value". A flag that appears N times will
        # appear N times in this list.
        explicit_flags: list[str] = []

        # Which tokens were consumed as subcommand names (to skip)?
        # Prefer the index-based set from Phase 1, which correctly handles
        # alias tokens (e.g. "a" as alias for "add"). Fall back to name
        # matching for callers that don't pass consumed_indices.
        _consumed_indices: set[int] = consumed_indices if consumed_indices is not None else set()

        parsing_mode = self._spec.get("parsing_mode", "gnu")

        # Handle traditional mode: first non-subcommand non-flag token
        # may be a bare flag stack.
        traditional_first_done = parsing_mode != "traditional"

        for tok_idx, token in enumerate(tokens):
            # Skip tokens that were consumed as subcommand names during routing.
            # Use index-based matching so alias tokens (e.g. "a" for "add") are
            # correctly identified rather than matching by canonical name.
            if tok_idx in _consumed_indices:
                continue

            mode = modal.current_mode

            if mode == "END_OF_FLAGS":
                positional_tokens.append(token)
                continue

            if mode == "FLAG_VALUE":
                # This entire token is the value for pending_flag.
                # pending_flag is guaranteed non-None when in FLAG_VALUE mode
                # because we only switch to FLAG_VALUE after setting pending_flag.
                if pending_flag is None:  # pragma: no cover — defensive guard
                    modal.switch_mode("to_scanning")
                    continue
                coerced, err = coerce_value(
                    raw=token,
                    arg_type=pending_flag["type"],
                    enum_values=pending_flag.get("enum_values", []),
                    context=command_path,
                    arg_name=pending_flag.get("long") or pending_flag.get("short") or pending_flag["id"],
                )
                if err:
                    errors.append(err)
                else:
                    self._set_flag(parsed_flags, flag_counts, pending_flag, coerced, errors, command_path, explicit_flags)
                pending_flag = None
                modal.switch_mode("to_scanning")
                continue

            # mode == "SCANNING"

            # Traditional mode: treat the first non-flag non-subcommand token
            # as stacked flags (tar xvf style).
            if not traditional_first_done and not token.startswith("-"):
                traditional_first_done = True
                # Try to classify as stacked flags without the leading dash.
                stacked_result = self._try_traditional(token, active_flags, command_path)
                if stacked_result is not None:
                    for idx, (char, fdef) in enumerate(stacked_result):
                        is_last = idx == len(stacked_result) - 1
                        if _is_valueless_type(fdef.get("type")):
                            self._set_flag(parsed_flags, flag_counts, fdef, True, errors, command_path, explicit_flags)
                        elif is_last:
                            # Non-boolean last flag: consume next token as its value.
                            pending_flag = fdef
                            modal.switch_mode("to_flag_value")
                        else:
                            # Non-boolean flag in the middle of the stack.
                            # This is technically invalid (non-boolean flags must
                            # be last) but we record an error and continue.
                            errors.append(
                                ParseError(
                                    error_type="missing_flag_value",
                                    message=(
                                        f"Flag -{char} requires a value but is "
                                        f"not at the end of the traditional flag stack '{token}'"
                                    ),
                                    context=command_path,
                                )
                            )
                    continue
                # Falls through to normal positional handling below.

            if not token.startswith("-") or token == "-":
                # Positional token
                if parsing_mode == "posix":
                    # In POSIX mode, the first positional ends flag scanning.
                    modal.switch_mode("to_end_of_flags")
                positional_tokens.append(token)
                continue

            # Classify the token
            event = classifier.classify(token)
            event_type = event["type"]

            if event_type == "end_of_flags":
                modal.switch_mode("to_end_of_flags")
                continue

            if event_type == "long_flag":
                flag_def = event["flag_def"]
                # Check for --help
                result = self._handle_builtin(
                    flag_def["id"], flag_def.get("long"), flag_def.get("short"),
                    command_path, parsed_flags
                )
                if result is not None:
                    return result, [], [], []
                if _is_valueless_type(flag_def.get("type")):
                    # Boolean flags set to True; count flags increment via _set_flag.
                    self._set_flag(parsed_flags, flag_counts, flag_def, True, errors, command_path, explicit_flags)
                elif flag_def.get("default_when_present") is not None:
                    # --- default_when_present disambiguation (v1.1) ---
                    #
                    # When an enum flag has ``default_when_present`` and appears
                    # as ``--flag`` (no ``=value``), we must decide: does the
                    # NEXT token serve as the flag's value, or is it a separate
                    # argument?
                    #
                    # Rule: peek at the next unprocessed token. If it is a valid
                    # enum value for this flag, consume it. Otherwise, use the
                    # ``default_when_present`` value.
                    #
                    # We handle this by storing the flag def as pending and
                    # switching to a special mode. However, since our modal
                    # machine doesn't have a dedicated mode for this, we use
                    # FLAG_VALUE mode but set a marker so we can disambiguate.
                    #
                    # Simpler approach: peek ahead in the token list directly.
                    next_idx = tok_idx + 1
                    # Find next unprocessed token (skip consumed subcommands)
                    while next_idx < len(tokens) and next_idx in _consumed_indices:
                        next_idx += 1
                    if next_idx < len(tokens):
                        next_token = tokens[next_idx]
                        enum_values = flag_def.get("enum_values", [])
                        if next_token in enum_values:
                            # Consume the next token as the enum value.
                            # Mark it so the main loop skips it.
                            _consumed_indices.add(next_idx)
                            self._set_flag(parsed_flags, flag_counts, flag_def, next_token, errors, command_path, explicit_flags)
                        else:
                            # Next token is not a valid enum value — use default.
                            self._set_flag(
                                parsed_flags, flag_counts, flag_def,
                                flag_def["default_when_present"],
                                errors, command_path, explicit_flags,
                            )
                    else:
                        # No more tokens — use default_when_present.
                        self._set_flag(
                            parsed_flags, flag_counts, flag_def,
                            flag_def["default_when_present"],
                            errors, command_path, explicit_flags,
                        )
                else:
                    pending_flag = flag_def
                    modal.switch_mode("to_flag_value")
                continue

            if event_type == "long_flag_with_value":
                flag_def = event["flag_def"]
                coerced, err = coerce_value(
                    raw=event["value"],
                    arg_type=flag_def["type"],
                    enum_values=flag_def.get("enum_values", []),
                    context=command_path,
                    arg_name=flag_def.get("long") or flag_def["id"],
                )
                if err:
                    errors.append(err)
                else:
                    self._set_flag(parsed_flags, flag_counts, flag_def, coerced, errors, command_path, explicit_flags)
                continue

            if event_type == "single_dash_long":
                flag_def = event["flag_def"]
                result = self._handle_builtin(
                    flag_def["id"], flag_def.get("long"), flag_def.get("short"),
                    command_path, parsed_flags
                )
                if result is not None:
                    return result, [], [], []
                if _is_valueless_type(flag_def.get("type")):
                    self._set_flag(parsed_flags, flag_counts, flag_def, True, errors, command_path, explicit_flags)
                else:
                    pending_flag = flag_def
                    modal.switch_mode("to_flag_value")
                continue

            if event_type == "short_flag":
                flag_def = event["flag_def"]
                result = self._handle_builtin(
                    flag_def["id"], flag_def.get("long"), flag_def.get("short"),
                    command_path, parsed_flags
                )
                if result is not None:
                    return result, [], [], []
                if _is_valueless_type(flag_def.get("type")):
                    self._set_flag(parsed_flags, flag_counts, flag_def, True, errors, command_path, explicit_flags)
                else:
                    pending_flag = flag_def
                    modal.switch_mode("to_flag_value")
                continue

            if event_type == "short_flag_with_value":
                flag_def = event["flag_def"]
                coerced, err = coerce_value(
                    raw=event["value"],
                    arg_type=flag_def["type"],
                    enum_values=flag_def.get("enum_values", []),
                    context=command_path,
                    arg_name=flag_def.get("short") or flag_def["id"],
                )
                if err:
                    errors.append(err)
                else:
                    self._set_flag(parsed_flags, flag_counts, flag_def, coerced, errors, command_path, explicit_flags)
                continue

            if event_type == "stacked_flags":
                chars = event["chars"]
                flag_defs = event["flag_defs"]
                trailing_value = event.get("trailing_value")
                for idx, (char, fdef) in enumerate(zip(chars, flag_defs)):
                    is_last = idx == len(chars) - 1
                    if _is_valueless_type(fdef.get("type")):
                        # Boolean sets True; count increments via _set_flag.
                        self._set_flag(parsed_flags, flag_counts, fdef, True, errors, command_path, explicit_flags)
                    elif is_last and trailing_value:
                        coerced, err = coerce_value(
                            raw=trailing_value,
                            arg_type=fdef["type"],
                            enum_values=fdef.get("enum_values", []),
                            context=command_path,
                            arg_name=fdef.get("short") or fdef["id"],
                        )
                        if err:
                            errors.append(err)
                        else:
                            self._set_flag(parsed_flags, flag_counts, fdef, coerced, errors, command_path, explicit_flags)
                    else:
                        # Value-taking flag at end without inline value → next token is value
                        pending_flag = fdef
                        modal.switch_mode("to_flag_value")
                continue

            if event_type == "unknown_flag":
                # Compute a fuzzy suggestion
                valid_flags = self._all_flag_names(active_flags)
                unknown_tok = event["token"]
                # Strip leading dashes for distance computation
                clean = unknown_tok.lstrip("-")
                suggestion = _fuzzy_suggest(clean, valid_flags)
                if suggestion:
                    # Re-add appropriate dashes
                    suggestion = (
                        f"--{suggestion}"
                        if len(suggestion) > 1
                        else f"-{suggestion}"
                    )
                errors.append(
                    ParseError(
                        error_type="unknown_flag",
                        message=f"Unknown flag '{unknown_tok}'",
                        suggestion=suggestion,
                        context=command_path,
                    )
                )
                continue

            # Positional token classified as positional
            if event_type == "positional":
                if parsing_mode == "posix":
                    modal.switch_mode("to_end_of_flags")
                positional_tokens.append(event["value"])
                continue

        return parsed_flags, positional_tokens, errors, explicit_flags

    # =========================================================================
    # Helper methods
    # =========================================================================

    def _build_routing_graph(self) -> None:
        """Build the command routing graph G_cmd from the spec.

        Each command (at any nesting depth) becomes a node. The root program
        is a special node ``"__root__"``. Edges go from parent to child,
        keyed by the child's canonical name.

        We also populate ``self._cmd_map`` to look up command definitions
        by their canonical name ID.
        """
        self._g_cmd.add_node("__root__")
        self._cmd_map["__root__"] = self._spec

        def add_commands(
            parent_id: str,
            commands: list[dict[str, Any]],
        ) -> None:
            for cmd in commands:
                node_id = cmd["name"]
                self._g_cmd.add_node(node_id)
                self._g_cmd.add_edge(parent_id, node_id)
                self._cmd_map[node_id] = cmd
                # Recurse into nested commands
                add_commands(node_id, cmd.get("commands", []))

        add_commands("__root__", self._spec.get("commands", []))

    def _get_node_def(self, node_id: str) -> dict[str, Any]:
        """Return the spec dict for a given node ID.

        Args:
            node_id: The command name or ``"__root__"``.

        Returns:
            The command definition dict (or root spec dict).
        """
        return self._cmd_map.get(node_id, self._spec)

    def _collect_active_flags(
        self,
        command_path: list[str],
        leaf_node_id: str,
    ) -> list[dict[str, Any]]:
        """Collect all flags active in the current command scope.

        Active flags = global_flags + flags from each node in command_path.

        The leaf node's ``inherit_global_flags`` setting controls whether
        global flags are included (default: True).

        Args:
            command_path: Sequence of command names from root to leaf.
            leaf_node_id: The current leaf node ID.

        Returns:
            Deduplicated list of flag defs, in priority order:
            global flags first, then command-specific flags.
        """
        leaf_def = self._get_node_def(leaf_node_id)
        inherit = leaf_def.get("inherit_global_flags", True)

        active: list[dict[str, Any]] = []
        seen_ids: set[str] = set()

        def add_flags(flags: list[dict[str, Any]]) -> None:
            for f in flags:
                if f["id"] not in seen_ids:
                    seen_ids.add(f["id"])
                    active.append(f)

        if inherit:
            add_flags(self._spec.get("global_flags", []))

        # Add flags from each command node in the path (skip program name)
        for cmd_name in command_path[1:]:
            node_def = self._get_node_def(cmd_name)
            add_flags(node_def.get("flags", []))

        # Root-level flags
        if leaf_node_id == "__root__":
            add_flags(self._spec.get("flags", []))
        elif leaf_node_id in self._cmd_map:
            add_flags(self._cmd_map[leaf_node_id].get("flags", []))

        return active

    def _build_flag_lookup(
        self,
        flags: list[dict[str, Any]],
    ) -> dict[str, dict[str, Any]]:
        """Build a dict of token → flag_def for routing-phase skip logic.

        Maps long names and short chars to their flag defs.

        Args:
            flags: List of active flag defs.

        Returns:
            Dict mapping flag name/char to flag def.
        """
        lookup: dict[str, dict[str, Any]] = {}
        for f in flags:
            if f.get("long"):
                lookup[f["long"]] = f
            if f.get("short"):
                lookup[f["short"]] = f
            if f.get("single_dash_long"):
                lookup[f["single_dash_long"]] = f
        return lookup

    def _skip_flag(
        self,
        token: str,
        _idx: int,
        _tokens: list[str],
        flag_lookup: dict[str, dict[str, Any]],
    ) -> int:
        """Return the number of tokens to advance when skipping a flag.

        During routing (Phase 1), we skip flags but need to know how many
        tokens each flag consumes. A boolean flag consumes 1 token; a
        value-taking flag may consume 2 (flag + value) unless the value is
        inline (--output=file or -fvalue).

        Args:
            token: The flag token.
            _idx: Current index (unused).
            _tokens: All tokens (unused — value skipping is handled here).
            flag_lookup: Dict of flag name → flag def.

        Returns:
            Number of extra tokens to skip (0 or 1 beyond the flag itself).
        """
        # --name=value → inline value, skip 1 token total
        if token.startswith("--") and "=" in token:
            return 1

        # --name → look up whether it takes a value
        if token.startswith("--"):
            name = token[2:]
            fdef = flag_lookup.get(name)
            if fdef and not _is_valueless_type(fdef.get("type")):
                # Enum flags with default_when_present might NOT consume
                # a value token, but during routing we conservatively skip 1
                # (the flag itself). The value ambiguity is resolved in Phase 2.
                if fdef.get("default_when_present") is not None:
                    return 1
                return 2  # skip flag + value token
            return 1

        # -x → single char flag
        if token.startswith("-") and len(token) == 2:
            char = token[1]
            fdef = flag_lookup.get(char)
            if fdef and not _is_valueless_type(fdef.get("type")):
                return 2
            return 1

        # -xVALUE or stacked → skip just the token
        return 1

    def _resolve_command_name(
        self,
        token: str,
        node_ids: list[object],
    ) -> str | None:
        """Check if token matches the name or alias of any successor node.

        Args:
            token: The candidate subcommand token.
            node_ids: List of successor node IDs.

        Returns:
            The canonical command name if matched, else None.
        """
        for node_id in node_ids:
            node_id_str = str(node_id)
            cmd_def = self._cmd_map.get(node_id_str)
            if cmd_def is None:
                continue
            if cmd_def.get("name") == token:
                return node_id_str
            if token in cmd_def.get("aliases", []):
                return node_id_str
        return None

    def _get_subcommand_names(self, node_ids: list[object]) -> list[str]:
        """Return all valid subcommand names and aliases for a set of node IDs.

        Args:
            node_ids: Successor node IDs.

        Returns:
            List of all valid name strings.
        """
        names: list[str] = []
        for node_id in node_ids:
            cmd_def = self._cmd_map.get(str(node_id))
            if cmd_def:
                names.append(cmd_def["name"])
                names.extend(cmd_def.get("aliases", []))
        return names

    def _set_flag(
        self,
        parsed_flags: dict[str, Any],
        flag_counts: dict[str, int],
        flag_def: dict[str, Any],
        value: Any,
        errors: list[ParseError],
        context: list[str],
        explicit_flags: list[str] | None = None,
    ) -> None:
        """Set a flag value in parsed_flags, handling repeatable, count, and duplicate.

        === Count type (v1.1) ===

        Count flags behave differently from all other types. Instead of storing
        a value directly, each occurrence increments an integer counter. The
        ``value`` parameter is ignored for count flags — what matters is how
        many times the flag appears.

        Example: ``-vvv`` produces three calls to ``_set_flag`` for the same
        flag def, resulting in ``parsed_flags["verbose"] = 3``.

        === Explicit flags tracking (v1.1) ===

        Every call to ``_set_flag`` records the flag's ID in ``explicit_flags``
        (if provided). This enables callers to distinguish user-supplied flags
        from defaults.

        Args:
            parsed_flags: The accumulating parsed flags dict.
            flag_counts: Counter tracking how many times each flag was seen.
            flag_def: The flag's definition dict.
            value: The coerced value to set (ignored for count flags).
            errors: Error list to append to on duplicate.
            context: Command path context.
            explicit_flags: List to append flag IDs to (v1.1 tracking).
        """
        fid = flag_def["id"]
        count = flag_counts.get(fid, 0) + 1
        flag_counts[fid] = count

        # Track explicit usage (v1.1)
        if explicit_flags is not None:
            explicit_flags.append(fid)

        # --- Count type: increment counter, no duplicate error ---
        #
        # Count flags are inherently repeatable — each occurrence adds 1.
        # There is no "duplicate" error for count flags; that is their
        # intended use pattern.
        if flag_def.get("type") == "count":
            parsed_flags[fid] = parsed_flags.get(fid, 0) + 1
            return

        if flag_def.get("repeatable", False):
            if fid not in parsed_flags:
                parsed_flags[fid] = []
            parsed_flags[fid].append(value)  # type: ignore[union-attr]
        elif count > 1:
            errors.append(
                ParseError(
                    error_type="duplicate_flag",
                    message=(
                        f"{self._flag_display_from_def(flag_def)} "
                        f"specified more than once"
                    ),
                    context=context,
                )
            )
        else:
            parsed_flags[fid] = value

    def _flag_display_from_def(self, flag_def: dict[str, Any]) -> str:
        """Build a display string for a flag from its definition.

        Args:
            flag_def: Flag definition dict.

        Returns:
            String like ``-l/--long-listing`` or ``--verbose``.
        """
        parts: list[str] = []
        if flag_def.get("short"):
            parts.append(f"-{flag_def['short']}")
        if flag_def.get("long"):
            parts.append(f"--{flag_def['long']}")
        if flag_def.get("single_dash_long"):
            parts.append(f"-{flag_def['single_dash_long']}")
        return "/".join(parts) or f"--{flag_def['id']}"

    def _handle_builtin(
        self,
        flag_id: str,
        long_name: str | None,
        short_name: str | None,
        command_path: list[str],
        parsed_flags: dict[str, Any],
    ) -> HelpResult | VersionResult | None:
        """Check if a flag triggers a builtin response (help or version).

        Args:
            flag_id: The flag's ID.
            long_name: The flag's long name (if any).
            short_name: The flag's short name (if any).
            command_path: Current command path.
            parsed_flags: Currently parsed flags (unused here).

        Returns:
            HelpResult, VersionResult, or None if not a builtin.
        """
        builtin_flags = self._spec.get("builtin_flags", {})

        # Help trigger: --help or (builtin) -h
        #
        # We check flag_id == "__help__" for the builtin sentinel, and also
        # long_name == "help" to handle specs that define --help explicitly.
        # We do NOT check short_name == "h" here because a user-defined flag
        # with short="h" (e.g. --human-readable) would incorrectly trigger
        # help. The "-h" builtin shortcut is handled by _check_quick_help_version
        # before Phase 2 runs, using the user_defined_short_h guard.
        if builtin_flags.get("help", True):
            if long_name == "help" or flag_id == "__help__":
                gen = HelpGenerator(self._spec, command_path)
                return HelpResult(text=gen.generate(), command_path=list(command_path))

        # Version trigger: --version
        if builtin_flags.get("version", True) and self._spec.get("version"):
            if long_name == "version" or flag_id == "__version__":
                return VersionResult(version=self._spec["version"])

        return None

    def _check_quick_help_version(
        self,
        tokens: list[str],
        active_flags: list[dict[str, Any]],
        command_path: list[str],
        program: str,
    ) -> HelpResult | VersionResult | None:
        """Quick scan for --help/-h/--version before full parsing.

        This allows help to work even if there are routing errors.

        Args:
            tokens: All tokens after argv[0].
            active_flags: Active flags in scope.
            command_path: Current command path.
            program: Program name.

        Returns:
            HelpResult or VersionResult if found, else None.
        """
        builtin_flags = self._spec.get("builtin_flags", {})
        help_enabled = builtin_flags.get("help", True)
        version_enabled = builtin_flags.get("version", True) and self._spec.get("version")

        # Check whether the user has defined a flag with short="h". If so,
        # "-h" is NOT the builtin help trigger — it is the user's own flag.
        # We detect this by checking whether any active flag has short="h"
        # and an id that is not the builtin sentinel "__help__".
        user_defined_short_h = any(
            f.get("short") == "h" and f.get("id") != "__help__"
            for f in active_flags
        )

        for token in tokens:
            if help_enabled and token == "--help":
                gen = HelpGenerator(self._spec, command_path)
                return HelpResult(text=gen.generate(), command_path=list(command_path))
            if help_enabled and token == "-h" and not user_defined_short_h:
                gen = HelpGenerator(self._spec, command_path)
                return HelpResult(text=gen.generate(), command_path=list(command_path))
            if version_enabled and token == "--version":
                return VersionResult(version=self._spec["version"])  # type: ignore[arg-type]

        return None

    def _apply_flag_defaults(
        self,
        active_flags: list[dict[str, Any]],
        parsed_flags: dict[str, Any],
    ) -> dict[str, Any]:
        """Apply default values for all absent flags.

        All flags in scope should be present in the result dict. Absent
        optional flags get:
        - False for boolean flags
        - None for value-taking flags (or the spec's ``default`` value)

        Args:
            active_flags: All flags in scope.
            parsed_flags: Currently parsed flags (may be incomplete).

        Returns:
            A complete flags dict with defaults filled in.
        """
        result = dict(parsed_flags)
        for flag in active_flags:
            fid = flag["id"]
            if fid not in result:
                default = flag.get("default")
                if flag.get("type") == "boolean":
                    result[fid] = default if default is not None else False
                elif flag.get("type") == "count":
                    # Count flags default to 0 when absent.
                    result[fid] = default if default is not None else 0
                else:
                    result[fid] = default
        return result

    def _all_flag_names(self, active_flags: list[dict[str, Any]]) -> list[str]:
        """Collect all valid flag names for fuzzy matching suggestions.

        Args:
            active_flags: Flags in scope.

        Returns:
            List of long names (without --) and short chars.
        """
        names: list[str] = []
        for f in active_flags:
            if f.get("long"):
                names.append(f["long"])
            if f.get("short"):
                names.append(f["short"])
            if f.get("single_dash_long"):
                names.append(f["single_dash_long"])
        return names

    def _try_traditional(
        self,
        token: str,
        active_flags: list[dict[str, Any]],
        context: list[str],
    ) -> list[tuple[str, dict[str, Any]]] | None:
        """Try to classify a bare token as stacked flags (tar-style).

        In traditional mode (spec §5.3), argv[1] (the first non-flag,
        non-subcommand token) is treated as a sequence of short flag
        characters without a leading dash.

        Args:
            token: The token to try as a traditional flag stack.
            active_flags: Active flags in scope.
            context: Command path (for error messages, not used here).

        Returns:
            List of (char, flag_def) pairs if all chars are valid boolean
            short flags. None if any char is unknown (falls back to positional).
        """
        by_short = {f["short"]: f for f in active_flags if f.get("short")}
        result: list[tuple[str, dict[str, Any]]] = []

        for char in token:
            fdef = by_short.get(char)
            if fdef is None:
                return None  # Unknown char: fall back to positional
            result.append((char, fdef))

        return result
