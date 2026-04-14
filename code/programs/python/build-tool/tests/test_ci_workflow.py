from __future__ import annotations

from pathlib import Path

from build_tool.ci_workflow import analyze_ci_workflow_patch
from build_tool.cli import _compute_languages_needed
from build_tool.discovery import Package


def test_analyze_ci_workflow_patch_allows_toolchain_scoped_dotnet_changes():
    change = analyze_ci_workflow_patch(
        """
@@ -312,0 +313,6 @@
+      - name: Set up .NET
+        if: needs.detect.outputs.needs_dotnet == 'true'
+        uses: actions/setup-dotnet@v4
+        with:
+          dotnet-version: '9.0.x'
"""
    )

    assert not change.requires_full_rebuild
    assert change.toolchains == frozenset({"dotnet"})


def test_analyze_ci_workflow_patch_ignores_comment_only_changes():
    change = analyze_ci_workflow_patch(
        """
@@ -316,2 +316,2 @@
-          # .NET 8 is the current LTS release.
+          # .NET 9 is the current LTS release.
"""
    )

    assert not change.requires_full_rebuild
    assert change.toolchains == frozenset()


def test_analyze_ci_workflow_patch_requires_full_rebuild_for_build_command_changes():
    change = analyze_ci_workflow_patch(
        """
@@ -404,1 +404,1 @@
-          $BT -root . -validate-build-files -language all
+          $BT -root . -force -validate-build-files -language all
"""
    )

    assert change.requires_full_rebuild


def test_compute_languages_needed_includes_safe_ci_workflow_toolchains():
    packages = [
        Package(
            name="python/logic-gates",
            path=Path("/repo/code/packages/python/logic-gates"),
            build_commands=[],
            language="python",
        )
    ]

    needed = _compute_languages_needed(
        packages,
        {"python/logic-gates"},
        False,
        frozenset({"dotnet"}),
    )

    assert needed["go"] is True
    assert needed["python"] is True
    assert needed["dotnet"] is True
    assert needed["rust"] is False
