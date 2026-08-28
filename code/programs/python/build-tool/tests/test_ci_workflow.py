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


def test_analyze_ci_workflow_patch_allows_shared_jvm_toolchain_changes():
    change = analyze_ci_workflow_patch(
        """
@@ -314,0 +315,17 @@
+      - name: Set up JDK 21
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: actions/setup-java@v4
+        with:
+          distribution: 'temurin'
+          java-version: '21'
+      - name: Set up Gradle
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: gradle/actions/setup-gradle@v6.3.0
+      - name: Disable long-lived Gradle services on Windows CI
+        if: (needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true') && runner.os == 'Windows'
+        shell: bash
+        run: |
+          {
+            echo 'GRADLE_OPTS=-Dorg.gradle.daemon=false -Dorg.gradle.vfs.watch=false'
+          } >> "$GITHUB_ENV"
"""
    )

    assert not change.requires_full_rebuild
    assert change.toolchains == frozenset({"java", "kotlin"})


def test_analyze_ci_workflow_patch_allows_dart_toolchain_changes():
    change = analyze_ci_workflow_patch(
        """
@@ -345,0 +346,6 @@
+      - name: Set up Dart
+        if: needs.detect.outputs.needs_dart == 'true'
+        uses: dart-lang/setup-dart@v1
+      - name: Verify Dart
+        run: dart --version
"""
    )

    assert not change.requires_full_rebuild
    assert change.toolchains == frozenset({"dart"})


def test_analyze_ci_workflow_patch_allows_verified_lua_cache_and_fallback():
    change = analyze_ci_workflow_patch(
        """
@@ -409,0 +410,14 @@
+      - name: Restore pinned Lua 5.4.7 cache
+        id: lua-cache
+        uses: actions/cache@v4
+        with:
-          luaVersion: "5.4"
+          path: .lua
+          key: lua-5.4.7-${{ runner.os }}-${{ runner.arch }}
+      - name: Install pinned Lua 5.4.7 from verified mirrors
+        shell: bash
+        run: |
+          if [ "$RUNNER_OS" = "macOS" ]; then
+            brew install readline ncurses
+          else
+            sudo apt-get install -q libreadline-dev libncurses-dev
+          fi
+          python3 code/scripts/setup_lua.py --prefix .lua
"""
    )

    assert not change.requires_full_rebuild
    assert change.toolchains == frozenset({"lua"})


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


def test_compute_languages_needed_includes_safe_jvm_ci_workflow_toolchains():
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
        frozenset({"java", "kotlin"}),
    )

    assert needed["go"] is True
    assert needed["python"] is True
    assert needed["java"] is True
    assert needed["kotlin"] is True
    assert needed["dotnet"] is False


def test_compute_languages_needed_collapses_csharp_and_fsharp_to_dotnet():
    packages = [
        Package(
            name=f"{language}/demo",
            path=Path(f"/repo/code/packages/{language}/demo"),
            build_commands=[],
            language=language,
        )
        for language in ("csharp", "fsharp")
    ]

    needed = _compute_languages_needed(
        packages,
        {"csharp/demo", "fsharp/demo"},
        False,
        frozenset(),
    )

    assert needed["dotnet"] is True
    assert "csharp" not in needed
    assert "fsharp" not in needed


def test_compute_languages_needed_includes_affected_dart_toolchain():
    package = Package(
        name="dart/demo",
        path=Path("/repo/code/packages/dart/demo"),
        build_commands=[],
        language="dart",
    )

    needed = _compute_languages_needed(
        [package], {package.name}, False, frozenset()
    )

    assert needed["dart"] is True
