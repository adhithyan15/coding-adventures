"""Validation helpers for BUILD/CI contract checks."""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path
import re

from build_tool.discovery import Package


CI_MANAGED_TOOLCHAIN_LANGUAGES = frozenset(
    {"python", "ruby", "typescript", "rust", "elixir", "lua", "perl"}
)


def validate_ci_full_build_toolchains(
    root: Path,
    packages: Iterable[Package],
) -> str | None:
    """Return an error message when CI full-build toolchains drift."""
    ci_path = root / ".github" / "workflows" / "ci.yml"
    try:
        workflow = ci_path.read_text(encoding="utf-8")
    except OSError:
        return None

    if "Full build on main merge" not in workflow:
        return None

    compact_workflow = "".join(workflow.split())
    missing_output_binding: list[str] = []
    missing_main_force: list[str] = []

    for lang in _languages_needing_ci_toolchains(packages):
        output_binding = (
            f"needs_{lang}:${{{{steps.toolchains.outputs.needs_{lang}}}}}"
        )
        if output_binding not in compact_workflow:
            missing_output_binding.append(lang)

        if f"needs_{lang}=true" not in compact_workflow:
            missing_main_force.append(lang)

    if not missing_output_binding and not missing_main_force:
        return None

    parts: list[str] = []
    if missing_output_binding:
        parts.append(
            "detect outputs for forced main full builds are not normalized through "
            f"steps.toolchains for: {', '.join(missing_output_binding)}"
        )
    if missing_main_force:
        parts.append(
            "forced main full-build path does not explicitly enable toolchains for: "
            f"{', '.join(missing_main_force)}"
        )

    return f"{ci_path.as_posix()}: {'; '.join(parts)}"


def validate_build_contracts(
    root: Path,
    packages: Iterable[Package],
) -> str | None:
    """Return combined BUILD/CI validation failures, if any."""
    package_list = list(packages)
    errors: list[str] = []

    ci_error = validate_ci_full_build_toolchains(root, package_list)
    if ci_error is not None:
        errors.append(ci_error)

    errors.extend(validate_lua_isolated_build_files(package_list))

    if not errors:
        return None
    return "\n  - ".join(errors)


def validate_lua_isolated_build_files(packages: Iterable[Package]) -> list[str]:
    """Validate Lua BUILD contracts needed for isolated LuaRocks builds."""
    errors: list[str] = []

    for pkg in packages:
        if pkg.language != "lua":
            continue

        self_rock = f"coding-adventures-{pkg.path.name.replace('_', '-')}"
        for build_path in sorted(pkg.path.glob("BUILD*")):
            lines = _read_build_lines(build_path)
            if not lines:
                continue

            foreign_remove = _first_foreign_lua_remove(lines, self_rock)
            if foreign_remove is not None:
                errors.append(
                    f"{build_path.as_posix()}: Lua BUILD removes unrelated rock "
                    f"{foreign_remove}; isolated package builds should only remove "
                    "the package they are rebuilding"
                )

            state_machine_index = _first_line_containing(
                lines, ("../state_machine", "..\\state_machine")
            )
            directed_graph_index = _first_line_containing(
                lines, ("../directed_graph", "..\\directed_graph")
            )
            if (
                state_machine_index is not None
                and directed_graph_index is not None
                and state_machine_index < directed_graph_index
            ):
                errors.append(
                    f"{build_path.as_posix()}: Lua BUILD installs state_machine "
                    "before directed_graph; isolated LuaRocks builds require "
                    "directed_graph first"
                )

            if _has_guarded_local_lua_install(lines) and not _self_install_disables_deps(
                lines, self_rock
            ):
                errors.append(
                    f"{build_path.as_posix()}: Lua BUILD uses guarded sibling rock "
                    "installs but the final self-install does not pass "
                    "--deps-mode=none or --no-manifest"
                )

    return errors


def _languages_needing_ci_toolchains(packages: Iterable[Package]) -> list[str]:
    return sorted(
        {
            pkg.language
            for pkg in packages
            if pkg.language in CI_MANAGED_TOOLCHAIN_LANGUAGES
        }
    )


def _read_build_lines(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []

    return [
        line.strip()
        for line in text.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def _first_foreign_lua_remove(lines: Iterable[str], self_rock: str) -> str | None:
    pattern = re.compile(r"\bluarocks remove --force ([^ \t]+)")
    for line in lines:
        match = pattern.search(line)
        if match is None:
            continue
        if match.group(1) != self_rock:
            return match.group(1)
    return None


def _first_line_containing(lines: list[str], needles: tuple[str, ...]) -> int | None:
    for index, line in enumerate(lines):
        if any(needle in line for needle in needles):
            return index
    return None


def _has_guarded_local_lua_install(lines: Iterable[str]) -> bool:
    return any(
        "luarocks show " in line and ("../" in line or "..\\" in line)
        for line in lines
    )


def _self_install_disables_deps(lines: Iterable[str], self_rock: str) -> bool:
    for line in lines:
        if "luarocks make" not in line or self_rock not in line:
            continue
        if (
            "--deps-mode=none" in line
            or "--deps-mode none" in line
            or "--no-manifest" in line
        ):
            return True
    return False
