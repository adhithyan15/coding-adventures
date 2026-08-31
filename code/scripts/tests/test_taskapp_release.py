from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "release-task-app.yml"
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
    NATIVE_TARGETS,
    archive_native,
    archive_web,
    artifact_names,
    build_manifest,
    render_notes,
    validate_identifiers,
)

COMMIT = "0123456789abcdef0123456789abcdef01234567"


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


def test_manifest_requires_the_exact_release_payload_set(tmp_path: Path) -> None:
    for name in artifact_names("0.1.0"):
        (tmp_path / name).write_bytes(b"payload")

    manifest = build_manifest("0.1.0", "task-app-v0.1.0", COMMIT, tmp_path)

    assert manifest["sourceCommit"] == COMMIT
    assert [artifact["name"] for artifact in manifest["artifacts"]] == artifact_names(
        "0.1.0"
    )
    assert all(artifact["installable"] is False for artifact in manifest["artifacts"])
    assert {artifact.get("toolkit") for artifact in manifest["artifacts"][1:]} == {
        target["toolkit"] for target in NATIVE_TARGETS.values()
    }

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
    assert "issues/13522" in notes
    assert "SHA256SUMS" in notes


def test_workflow_validates_before_building_and_has_one_publisher() -> None:
    workflow = WORKFLOW.read_text(encoding="utf-8")

    assert "workflow_dispatch:" in workflow
    assert "version:" in workflow
    assert "tag:" in workflow
    assert "TaskApp releases must be dispatched from main" in workflow
    assert "git ls-remote --exit-code --tags" in workflow
    assert "Release $RELEASE_TAG is already published" in workflow
    assert "needs: validate" in workflow
    assert "needs: [validate, build-web, build-native]" in workflow
    assert workflow.count('gh release create "$RELEASE_TAG"') == 1
    assert "sha256sum --check SHA256SUMS" in workflow
    assert "--latest=false" in workflow
    assert 'RUST_VERSION: "1.97.0"' in workflow
    assert "git diff --exit-code" in workflow
    assert workflow.count("sudo apt-get install -y libcairo2-dev") == 2
    assert "cmp" in workflow
    assert "code/packages/rust/task-wasm/pkg/task_engine.wasm" in workflow
    assert "host/web/public/task_engine.wasm" in workflow
    assert "':(exclude)code/packages/rust/task-wasm/pkg/task_engine.wasm'" in workflow
    assert WEB_LOCK.is_file()
