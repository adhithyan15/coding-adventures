"""Validation helpers for BUILD/CI contract checks."""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path

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


def _languages_needing_ci_toolchains(packages: Iterable[Package]) -> list[str]:
    return sorted(
        {
            pkg.language
            for pkg in packages
            if pkg.language in CI_MANAGED_TOOLCHAIN_LANGUAGES
        }
    )
