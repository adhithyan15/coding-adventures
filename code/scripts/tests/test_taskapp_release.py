from __future__ import annotations

import json
import plistlib
import struct
import sys
import tarfile
import zipfile
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "release-task-app.yml"
WINDOWS_SMOKE = REPOSITORY_ROOT / "code" / "scripts" / "taskapp-xaml-smoke.ps1"
WEB_LOCK = (
    REPOSITORY_ROOT
    / "code"
    / "programs"
    / "mosaic"
    / "task-app"
    / "host"
    / "web"
    / "package-lock.json"
)
sys.path.insert(0, str(SCRIPTS))

from taskapp_release import (
    LINUX_BUNDLES,
    NATIVE_TARGETS,
    archive_linux_bundle,
    archive_macos_app,
    archive_native,
    archive_web,
    archive_windows_app,
    artifact_names,
    build_manifest,
    materialize_upgrade_fixture,
    render_notes,
    validate_identifiers,
    verify_upgrade_state,
    write_windows_icon,
)

COMMIT = "0123456789abcdef0123456789abcdef01234567"
UPGRADE_FIXTURE = (
    REPOSITORY_ROOT
    / "code"
    / "programs"
    / "mosaic"
    / "task-app"
    / "fixtures"
    / "release-upgrade-v0.1.0.json"
)


@pytest.mark.parametrize(
    "version",
    ("0.1.0", "1.2.3-alpha.1", "2.0.0-rc.1+build.7"),
)
def test_accepts_strict_semver_and_matching_product_tag(version: str) -> None:
    validate_identifiers(version, f"task-app-v{version}", COMMIT)


@pytest.mark.parametrize(
    "version",
    ("v0.1.0", "0.1", "01.2.3", "1.02.3", "1.2.03", "1.2.3-01", "1.2.3 "),
)
def test_rejects_invalid_semver(version: str) -> None:
    with pytest.raises(ValueError, match="strict SemVer"):
        validate_identifiers(version, f"task-app-v{version}", COMMIT)


def test_rejects_mismatched_tag_and_short_commit() -> None:
    with pytest.raises(ValueError, match="tag must be"):
        validate_identifiers("0.1.0", "task-app-v0.1.1", COMMIT)
    with pytest.raises(ValueError, match="40-character"):
        validate_identifiers("0.1.0", "task-app-v0.1.0", "abc123")


def test_archives_verified_web_and_native_payloads(tmp_path: Path) -> None:
    web = tmp_path / "web"
    (web / "assets").mkdir(parents=True)
    (web / "index.html").write_text("<main>TaskApp</main>", encoding="utf-8")
    (web / "task_engine.wasm").write_bytes(b"wasm")
    (web / "assets" / "app.js").write_text("export {};", encoding="utf-8")

    web_archive = archive_web("0.1.0", COMMIT, web, tmp_path / "assets")
    with zipfile.ZipFile(web_archive) as archive:
        names = archive.namelist()
    assert "task-app-web-v0.1.0/SOURCE_COMMIT" in names
    assert "task-app-web-v0.1.0/task_engine.wasm" in names

    native = tmp_path / "qt"
    native.mkdir()
    (native / "mosaic-degradations.json").write_text(
        json.dumps({"nativeComplete": True, "degradations": []}),
        encoding="utf-8",
    )
    (native / "TaskApp.qml").write_text("ApplicationWindow {}", encoding="utf-8")
    native_archive = archive_native(
        "0.1.0",
        COMMIT,
        "qt",
        native,
        tmp_path / "assets",
    )
    with zipfile.ZipFile(native_archive) as archive:
        names = archive.namelist()
    assert "task-app-qt-linux-project-v0.1.0/SOURCE_COMMIT" in names
    assert "task-app-qt-linux-project-v0.1.0/TaskApp.qml" in names


def test_native_archive_rejects_a_degraded_project(tmp_path: Path) -> None:
    (tmp_path / "mosaic-degradations.json").write_text(
        json.dumps(
            {
                "nativeComplete": False,
                "degradations": [{"code": "runtime.sample-fallback"}],
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="not strict native-complete"):
        archive_native("0.1.0", COMMIT, "qt", tmp_path, tmp_path / "assets")


def test_archives_verified_linux_bundle_with_launcher_contract(tmp_path: Path) -> None:
    source = tmp_path / "bundle"
    (source / "bin").mkdir(parents=True)
    (source / "lib").mkdir()
    executable = source / "bin" / "trestle"
    executable.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    executable.chmod(0o755)
    runtime = source / "lib" / "libmosaic_app.so"
    runtime.write_bytes(b"rust-runtime")
    expected_runtime = tmp_path / "libtask_mosaic_app.so"
    expected_runtime.write_bytes(b"rust-runtime")

    bundle = archive_linux_bundle(
        "0.2.0",
        COMMIT,
        "compose",
        source,
        executable,
        runtime,
        expected_runtime,
        tmp_path / "assets",
    )

    root = "task-app-compose-linux-bundle-v0.2.0"
    with tarfile.open(bundle, "r:gz") as archive:
        names = archive.getnames()
        metadata = json.load(archive.extractfile(f"{root}/BUNDLE.json"))
        launcher = archive.extractfile(f"{root}/launch-trestle").read().decode()
        operations = archive.extractfile(f"{root}/LOCAL-DATA.txt").read().decode()
        launcher_mode = archive.getmember(f"{root}/launch-trestle").mode
    assert f"{root}/bin/trestle" in names
    assert metadata["applicationId"] == "task-app"
    assert metadata["rustRuntime"] == "lib/libmosaic_app.so"
    assert "$XDG_DATA_HOME/task-app/mosaic-state.v1.json" == metadata["statePath"]
    assert "pre-v0.2.0-compose.json" in launcher
    assert "Application identifier: task-app" in operations
    assert "mosaic-state.v1.json.corrupt" in operations
    assert launcher_mode & 0o111


def test_materializes_and_verifies_v0_1_0_upgrade_fixture(tmp_path: Path) -> None:
    state = materialize_upgrade_fixture(
        UPGRADE_FIXTURE,
        tmp_path / "task-app" / "mosaic-state.v1.json",
    )

    snapshot = json.loads(state.read_text(encoding="utf-8"))
    assert snapshot["schema"] == "task-mosaic-app/state"
    assert snapshot["version"] == 1
    assert isinstance(snapshot["bytes"], list)
    verify_upgrade_state(UPGRADE_FIXTURE, state)

    Path(f"{state}.corrupt").write_text("damaged", encoding="utf-8")
    with pytest.raises(ValueError, match="quarantined a compatible"):
        verify_upgrade_state(UPGRADE_FIXTURE, state)


def test_linux_bundle_rejects_runtime_mismatch_and_external_paths(
    tmp_path: Path,
) -> None:
    source = tmp_path / "bundle"
    source.mkdir()
    executable = source / "trestle"
    executable.write_bytes(b"app")
    runtime = source / "libmosaic_app.so"
    runtime.write_bytes(b"wrong")
    expected_runtime = tmp_path / "expected.so"
    expected_runtime.write_bytes(b"expected")

    with pytest.raises(ValueError, match="does not match"):
        archive_linux_bundle(
            "0.2.0",
            COMMIT,
            "qt",
            source,
            executable,
            runtime,
            expected_runtime,
            tmp_path / "assets",
        )
    with pytest.raises(ValueError, match="executable must be inside"):
        archive_linux_bundle(
            "0.2.0",
            COMMIT,
            "qt",
            source,
            expected_runtime,
            runtime,
            runtime,
            tmp_path / "assets",
        )


def test_archives_unsigned_macos_app_with_stable_identity(tmp_path: Path) -> None:
    executable = tmp_path / "App"
    executable.write_bytes(b"mach-o")
    executable.chmod(0o755)
    resources = tmp_path / "App_App.bundle"
    runtime = resources / "Runtime" / "libmosaic_app.dylib"
    runtime.parent.mkdir(parents=True)
    runtime.write_bytes(b"rust-runtime")
    expected_runtime = tmp_path / "libtask_mosaic_app.dylib"
    expected_runtime.write_bytes(b"rust-runtime")

    payload = archive_macos_app(
        "0.2.0",
        COMMIT,
        "arm64",
        executable,
        resources,
        runtime,
        expected_runtime,
        tmp_path / "assets",
    )

    with zipfile.ZipFile(payload) as archive:
        plist = plistlib.loads(archive.read("Trestle.app/Contents/Info.plist"))
        metadata = json.loads(
            archive.read("Trestle.app/Contents/Resources/BUNDLE.json")
        )
        operations = archive.read(
            "Trestle.app/Contents/Resources/LOCAL-DATA.txt"
        ).decode()
        icon = archive.read("Trestle.app/Contents/Resources/Trestle.icns")
        executable_mode = (
            archive.getinfo("Trestle.app/Contents/MacOS/Trestle").external_attr >> 16
        )
    assert plist["CFBundleDisplayName"] == "Trestle"
    assert plist["CFBundleIdentifier"] == "org.codingadventures.trestle"
    assert plist["CFBundleShortVersionString"] == "0.2.0"
    assert plist["CFBundleIconFile"] == "Trestle"
    assert metadata["architecture"] == "arm64"
    assert metadata["statePath"] == "~/Library/Application Support/task-app/mosaic-state.v1.json"
    assert metadata["signed"] is False
    assert metadata["iosArtifact"] is False
    assert icon.startswith(b"icns")
    assert "Uninstall while retaining data" in operations
    assert executable_mode & 0o111

    with pytest.raises(ValueError, match="unsupported macOS architecture"):
        archive_macos_app(
            "0.2.0",
            COMMIT,
            "universal",
            executable,
            resources,
            runtime,
            expected_runtime,
            tmp_path / "assets",
        )


def test_archives_self_contained_windows_app_with_stable_identity(tmp_path: Path) -> None:
    source = tmp_path / "publish"
    source.mkdir()
    executable = source / "Trestle.exe"
    executable.write_bytes(b"pe-app")
    runtime = source / "mosaic_app.dll"
    runtime.write_bytes(b"rust-runtime")
    expected_runtime = tmp_path / "task_mosaic_app.dll"
    expected_runtime.write_bytes(b"rust-runtime")
    (source / "Trestle.dll").write_bytes(b"managed-app")
    for name in ("Trestle.pri", "App.xbf", "MainWindow.xbf", "TaskApp.xbf"):
        (source / name).write_bytes(b"winui-resource")
    icon_path = write_windows_icon(tmp_path / "Trestle.ico")

    payload = archive_windows_app(
        "0.2.0",
        COMMIT,
        source,
        executable,
        runtime,
        expected_runtime,
        tmp_path / "assets",
    )

    root = "Trestle-windows-x64-v0.2.0"
    with zipfile.ZipFile(payload) as archive:
        metadata = json.loads(archive.read(f"{root}/BUNDLE.json"))
        icon = archive.read(f"{root}/Trestle.ico")
        operations = archive.read(f"{root}/LOCAL-DATA.txt").decode()
        names = archive.namelist()
    assert f"{root}/Trestle.exe" in names
    assert f"{root}/mosaic_app.dll" in names
    assert f"{root}/Trestle.pri" in names
    assert f"{root}/TaskApp.xbf" in names
    assert metadata["applicationIdentity"] == "org.codingadventures.trestle"
    assert metadata["statePath"] == "%LOCALAPPDATA%\\task-app\\mosaic-state.v1.json"
    assert metadata["dotnetSelfContained"] is True
    assert metadata["msix"] is False
    assert "%LOCALAPPDATA%\\task-app\\mosaic-state.v1.json" in operations
    assert icon == icon_path.read_bytes()
    assert icon[:6] == struct.pack("<HHH", 0, 1, 6)


def test_windows_archive_rejects_publish_without_application_resources(tmp_path: Path) -> None:
    source = tmp_path / "publish"
    source.mkdir()
    executable = source / "Trestle.exe"
    executable.write_bytes(b"pe-app")
    runtime = source / "mosaic_app.dll"
    runtime.write_bytes(b"rust-runtime")
    expected_runtime = tmp_path / "task_mosaic_app.dll"
    expected_runtime.write_bytes(b"rust-runtime")

    with pytest.raises(ValueError, match="missing required WinUI resources"):
        archive_windows_app(
            "0.2.0",
            COMMIT,
            source,
            executable,
            runtime,
            expected_runtime,
            tmp_path / "assets",
        )


def test_manifest_requires_the_exact_release_payload_set(tmp_path: Path) -> None:
    for name in artifact_names("0.1.0"):
        (tmp_path / name).write_bytes(b"payload")

    manifest = build_manifest("0.1.0", "task-app-v0.1.0", COMMIT, tmp_path)

    assert manifest["sourceCommit"] == COMMIT
    assert [artifact["name"] for artifact in manifest["artifacts"]] == artifact_names(
        "0.1.0"
    )
    project_artifacts = [
        artifact
        for artifact in manifest["artifacts"]
        if artifact["kind"] == "generated-native-project"
    ]
    bundle_artifacts = [
        artifact
        for artifact in manifest["artifacts"]
        if artifact["kind"] == "portable-linux-bundle"
    ]
    assert manifest["artifacts"][0]["installable"] is False
    assert all(artifact["installable"] is False for artifact in project_artifacts)
    assert all(artifact["installable"] is False for artifact in bundle_artifacts)
    assert all(artifact["runnable"] is True for artifact in bundle_artifacts)
    assert {artifact.get("toolkit") for artifact in project_artifacts} == {
        target["toolkit"] for target in NATIVE_TARGETS.values()
    }
    assert {artifact["backend"] for artifact in bundle_artifacts} == set(LINUX_BUNDLES)
    macos_artifact = manifest["artifacts"][-2]
    assert macos_artifact["kind"] == "unsigned-macos-application"
    assert macos_artifact["runnable"] is True
    assert macos_artifact["signed"] is False
    windows_artifact = manifest["artifacts"][-1]
    assert windows_artifact["kind"] == "portable-windows-application"
    assert windows_artifact["runnable"] is True
    assert windows_artifact["signed"] is False
    assert windows_artifact["msix"] is False

    (tmp_path / "unexpected.zip").write_bytes(b"unexpected")
    with pytest.raises(ValueError, match="payload mismatch"):
        build_manifest("0.1.0", "task-app-v0.1.0", COMMIT, tmp_path)


def test_release_notes_are_product_scoped_and_filter_previous_history() -> None:
    history = [
        {
            "number": 13575,
            "title": "Prove native TaskApp scheduling lifecycle",
            "url": "https://github.com/adhithyan15/coding-adventures/pull/13575",
            "mergedAt": "2026-08-31T04:19:55Z",
        },
        {
            "number": 13542,
            "title": "Persist TaskApp locally",
            "url": "https://github.com/adhithyan15/coding-adventures/pull/13542",
            "mergedAt": "2026-08-30T20:00:00Z",
        },
    ]

    notes = render_notes(
        "0.1.0",
        "task-app-v0.1.0",
        COMMIT,
        "adhithyan15/coding-adventures",
        history,
        "2026-08-31T00:00:00Z",
    )

    assert "# TaskApp v0.1.0" in notes
    assert "(#13575)" in notes
    assert "(#13542)" not in notes
    assert "no installer" in notes.lower()
    assert "portable bundle" in notes.lower()
    assert "task-app-compose-linux-bundle-v0.1.0.tar.gz" in notes
    assert "task-app-swiftui-macos-bundle-v0.1.0.zip" in notes
    assert "task-app-xaml-windows-bundle-v0.1.0.zip" in notes
    assert "not notarized" in notes.lower()
    assert "issues/13522" in notes
    assert "SHA256SUMS" in notes


def test_workflow_validates_before_building_and_has_one_publisher() -> None:
    workflow = WORKFLOW.read_text(encoding="utf-8")

    assert "workflow_dispatch:" in workflow
    assert "pull_request:" in workflow
    assert "task-app-v0.0.0-ci" in workflow
    assert "version:" in workflow
    assert "tag:" in workflow
    assert "TaskApp releases must be dispatched from main" in workflow
    assert "git ls-remote --exit-code --tags" in workflow
    assert "Release $RELEASE_TAG is already published" in workflow
    assert "needs: validate" in workflow
    assert "needs: [validate, build-web, build-native]" in workflow
    assert "if: github.event_name == 'workflow_dispatch'" in workflow
    assert workflow.count('gh release create "$RELEASE_TAG"') == 1
    assert "sha256sum --check SHA256SUMS" in workflow
    assert "--latest=false" in workflow
    assert 'RUST_VERSION: "1.97.0"' in workflow
    assert "git diff --exit-code" in workflow
    assert workflow.count("sudo apt-get install -y libcairo2-dev") == 2
    assert "cmp" in workflow
    assert "archive-linux-bundle" in workflow
    assert "createDistributable" in workflow
    assert "flutter build linux --release" in workflow
    assert 'staged_aot="$generated/build/lib/libapp.so"' in workflow
    assert "cmake --install" in workflow
    assert "launch-trestle" in workflow
    assert "*.tar.gz" in workflow
    assert "archive-macos-app" in workflow
    assert "Trestle.app/Contents/Info.plist" in workflow
    assert "archive-windows-app" in workflow
    assert "write-windows-icon" in workflow
    assert "-p:SelfContained=true" in workflow
    assert "-p:AssemblyName=Trestle" in workflow
    assert "$taskAppExecutable" not in workflow
    assert "@('Trestle.pri', 'App.xbf', 'MainWindow.xbf', 'TaskApp.xbf')" in workflow
    assert "-RestartExePath $replacementExecutable" in workflow
    assert workflow.count("materialize-upgrade-fixture") == 5
    assert workflow.count("verify-upgrade-state") == 5
    assert workflow.count("release-upgrade-v0.1.0.json") >= 10
    assert "windows-corrupt-probe" in workflow
    assert "macos-corrupt-probe" in workflow
    assert "code/packages/rust/task-wasm/pkg/task_engine.wasm" in workflow
    assert "host/web/public/task_engine.wasm" in workflow
    assert "':(exclude)code/packages/rust/task-wasm/pkg/task_engine.wasm'" in workflow
    assert WEB_LOCK.is_file()


def test_windows_ui_smoke_can_restart_through_a_replacement_package() -> None:
    smoke = WINDOWS_SMOKE.read_text(encoding="utf-8")

    assert "[string]$RestartExePath = ''" in smoke
    assert "$effectiveRestartExePath" in smoke
    assert "Start-Process -FilePath $ExePath" in smoke
    assert "Start-Process -FilePath $effectiveRestartExePath" in smoke
