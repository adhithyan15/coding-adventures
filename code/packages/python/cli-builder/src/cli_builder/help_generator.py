"""Help text generator for CLI Builder.

=== Auto-generated help from spec ===

One of CLI Builder's key features is that help text is generated automatically
from the spec. The developer never writes help text manually — it is derived
from the ``description``, ``flags``, ``arguments``, and ``commands`` fields
in the JSON spec. This guarantees that help text is always accurate.

=== Format overview (spec §9) ===

For a root-level command::

    USAGE
      <name> [OPTIONS] [COMMAND] [ARGS...]

    DESCRIPTION
      <description>

    COMMANDS
      subcommand    Description of the subcommand.

    OPTIONS
      -s, --long <VALUE>    Description. [default: val]
      -b, --boolean         Boolean flag description.

    GLOBAL OPTIONS
      -h, --help     Show this help message and exit.
      --version      Show version and exit.

For a subcommand::

    USAGE
      <program> <subcommand> [OPTIONS] <ARG> [ARG...]

    DESCRIPTION
      <subcommand description>

    OPTIONS
      ...

    ARGUMENTS
      <ARG>      Description. Required.
      [ARG...]   Description. Optional, repeatable.

=== Formatting rules ===

- Required positional args: ``<NAME>``
- Optional positional args: ``[NAME]``
- Variadic required: ``<NAME>...``
- Variadic optional: ``[NAME...]``
- Non-boolean flags: ``-s, --long <VALUE>``
- Boolean flags: ``-s, --long``
- ``single_dash_long`` flags: ``-classpath <VALUE>``
- Default values appended as ``[default: X]`` when set and required=false
"""

from __future__ import annotations

from typing import Any

# Width for the left column in FLAGS and ARGUMENTS tables.
# Flags/args with names longer than this get a line break.
COLUMN_WIDTH = 28


class HelpGenerator:
    """Generates formatted help text from a CLI spec.

    Usage::

        gen = HelpGenerator(spec, command_path=["git", "remote"])
        text = gen.generate()
        print(text)
    """

    def __init__(
        self,
        spec: dict[str, Any],
        command_path: list[str],
    ) -> None:
        """Initialize the generator.

        Args:
            spec: The normalized spec dict (from SpecLoader.load()).
            command_path: The command path to generate help for.
                E.g., ``["git"]`` for root help, ``["git", "remote"]``
                for subcommand help.
        """
        self._spec = spec
        self._command_path = command_path

        # Resolve the command node for this path.
        self._node = self._resolve_node()

    def generate(self) -> str:
        """Generate and return the formatted help text.

        Returns:
            A multi-line string with the help text.
        """
        sections: list[str] = []

        # --- USAGE line ---
        sections.append(self._usage_section())

        # --- DESCRIPTION ---
        desc = self._node.get("description") or self._spec.get("description", "")
        if desc:
            sections.append("DESCRIPTION\n  " + desc)

        # --- COMMANDS (if this node has subcommands) ---
        commands = self._node.get("commands", [])
        if commands:
            sections.append(self._commands_section(commands))

        # --- OPTIONS (scope-specific flags) ---
        flags = self._node.get("flags", [])
        if flags:
            sections.append(self._options_section("OPTIONS", flags))

        # --- GLOBAL OPTIONS ---
        global_flags = self._spec.get("global_flags", [])
        builtin_flags = self._spec.get("builtin_flags", {})
        builtin_list = self._builtin_flags_list(builtin_flags)
        all_global = list(global_flags) + builtin_list
        if all_global:
            sections.append(self._options_section("GLOBAL OPTIONS", all_global))

        # --- ARGUMENTS ---
        arguments = self._node.get("arguments", [])
        if arguments:
            sections.append(self._arguments_section(arguments))

        return "\n\n".join(sections)

    # =========================================================================
    # Section builders
    # =========================================================================

    def _usage_section(self) -> str:
        """Build the USAGE line.

        Examples:
            USAGE
              git [OPTIONS] [COMMAND]

            USAGE
              git remote add [OPTIONS] <NAME> <URL>
        """
        parts = list(self._command_path)

        # Determine if this node has subcommands
        has_subcommands = bool(self._node.get("commands"))
        # Determine if this node has flags (or inherited globals)
        has_flags = bool(
            self._node.get("flags")
            or self._spec.get("global_flags")
            or self._spec.get("builtin_flags")
        )
        # Determine if this node has arguments
        arguments = self._node.get("arguments", [])

        if has_flags:
            parts.append("[OPTIONS]")
        if has_subcommands:
            parts.append("[COMMAND]")
        for arg in arguments:
            parts.append(self._arg_usage(arg))

        return "USAGE\n  " + " ".join(parts)

    def _commands_section(self, commands: list[dict[str, Any]]) -> str:
        """Build the COMMANDS section.

        Each command is listed with its name and description, aligned in
        two columns.

        Args:
            commands: List of command definition dicts.

        Returns:
            Formatted COMMANDS section string.
        """
        lines = ["COMMANDS"]
        # Find the longest name for column alignment.
        max_name = max((len(c["name"]) for c in commands), default=0)
        col_width = max(max_name + 2, 12)

        for cmd in commands:
            name = cmd["name"]
            desc = cmd.get("description", "")
            padding = " " * (col_width - len(name))
            lines.append(f"  {name}{padding}{desc}")

        return "\n".join(lines)

    def _options_section(
        self,
        heading: str,
        flags: list[dict[str, Any]],
    ) -> str:
        """Build an OPTIONS or GLOBAL OPTIONS section.

        Each flag is shown with its short/long forms and description.
        Non-boolean flags include a VALUE placeholder.

        Args:
            heading: Section heading ("OPTIONS" or "GLOBAL OPTIONS").
            flags: List of flag definition dicts.

        Returns:
            Formatted options section string.
        """
        lines = [heading]
        for flag in flags:
            flag_str = self._flag_signature(flag)
            desc = flag.get("description", "")
            default = flag.get("default")
            if default is not None and not flag.get("required", False):
                desc = f"{desc} [default: {default}]"

            # Two-column layout: left=signature, right=description
            if len(flag_str) < COLUMN_WIDTH - 2:
                padding = " " * (COLUMN_WIDTH - len(flag_str))
                lines.append(f"  {flag_str}{padding}{desc}")
            else:
                # Signature too long: put description on next line
                lines.append(f"  {flag_str}")
                lines.append(f"  {' ' * COLUMN_WIDTH}{desc}")

        return "\n".join(lines)

    def _arguments_section(self, arguments: list[dict[str, Any]]) -> str:
        """Build the ARGUMENTS section.

        Each argument is shown with its usage form and description.

        Args:
            arguments: List of argument definition dicts.

        Returns:
            Formatted ARGUMENTS section string.
        """
        lines = ["ARGUMENTS"]
        for arg in arguments:
            arg_str = self._arg_usage(arg)
            desc = arg.get("description", "")
            required = arg.get("required", True)
            qualifier = "Required." if required else "Optional."
            full_desc = f"{desc} {qualifier}".strip()

            if len(arg_str) < COLUMN_WIDTH - 2:
                padding = " " * (COLUMN_WIDTH - len(arg_str))
                lines.append(f"  {arg_str}{padding}{full_desc}")
            else:
                lines.append(f"  {arg_str}")
                lines.append(f"  {' ' * COLUMN_WIDTH}{full_desc}")

        return "\n".join(lines)

    # =========================================================================
    # Formatting helpers
    # =========================================================================

    def _flag_signature(self, flag: dict[str, Any]) -> str:
        """Build the flag's usage signature for display.

        Examples:
            -l, --long-listing
            -o, --output <FILE>
            -classpath <PATH>
            --verbose

        Args:
            flag: Flag definition dict.

        Returns:
            Formatted flag signature string.
        """
        parts: list[str] = []

        if flag.get("short"):
            parts.append(f"-{flag['short']}")
        if flag.get("long"):
            parts.append(f"--{flag['long']}")
        if flag.get("single_dash_long"):
            parts.append(f"-{flag['single_dash_long']}")

        sig = ", ".join(parts)

        # Append value placeholder for non-boolean flags
        flag_type = flag.get("type", "string")
        if flag_type != "boolean":
            value_name = flag.get("value_name") or flag_type.upper()
            sig = f"{sig} <{value_name}>"

        return sig

    def _arg_usage(self, arg: dict[str, Any]) -> str:
        """Build the argument's usage form for help text.

        Examples:
            <FILE>       (required, non-variadic)
            [FILE]       (optional, non-variadic)
            <FILE>...    (required, variadic)
            [FILE...]    (optional, variadic)

        Args:
            arg: Argument definition dict.

        Returns:
            Formatted argument usage string.
        """
        name = arg.get("name", arg.get("id", "ARG"))
        required = arg.get("required", True)
        variadic = arg.get("variadic", False)

        if variadic:
            if required:
                return f"<{name}...>"
            else:
                return f"[{name}...]"
        else:
            if required:
                return f"<{name}>"
            else:
                return f"[{name}]"

    def _builtin_flags_list(
        self,
        builtin_flags: dict[str, bool],
    ) -> list[dict[str, Any]]:
        """Generate pseudo-flag-defs for the builtin --help and --version flags.

        Args:
            builtin_flags: Dict with "help" and "version" keys.

        Returns:
            List of synthetic flag definition dicts.
        """
        builtins: list[dict[str, Any]] = []

        if builtin_flags.get("help", True):
            builtins.append(
                {
                    "id": "__help__",
                    "short": "h",
                    "long": "help",
                    "description": "Show this help message and exit.",
                    "type": "boolean",
                }
            )

        if builtin_flags.get("version", True) and self._spec.get("version"):
            builtins.append(
                {
                    "id": "__version__",
                    "long": "version",
                    "description": "Show version and exit.",
                    "type": "boolean",
                }
            )

        return builtins

    # =========================================================================
    # Node resolution
    # =========================================================================

    def _resolve_node(self) -> dict[str, Any]:
        """Resolve the spec node corresponding to self._command_path.

        The command_path starts with the program name. Each subsequent
        element is a subcommand name. We walk down the ``commands`` arrays
        to find the target node.

        Returns:
            The spec dict for the target node (root spec or subcommand dict).
        """
        # Start at root
        node: dict[str, Any] = self._spec

        # Skip the program name (first element)
        for cmd_name in self._command_path[1:]:
            found = False
            for cmd in node.get("commands", []):
                if cmd["name"] == cmd_name or cmd_name in cmd.get("aliases", []):
                    node = cmd
                    found = True
                    break
            if not found:
                # Couldn't resolve — fall back to root
                return self._spec

        return node
