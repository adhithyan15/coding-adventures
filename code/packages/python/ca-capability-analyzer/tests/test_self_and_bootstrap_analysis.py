"""Integration tests running ca-capability-analyzer on test-bootstrap and itself."""

from pathlib import Path

from coding_adventures_ca_capability_analyzer.analyzer import analyze_package
from coding_adventures_ca_capability_analyzer.types import CAP_FS_READ


def test_analyze_test_bootstrap_is_pure() -> None:
    """Verify test-bootstrap package requires ZERO capabilities and passes audit."""
    this_dir = Path(__file__).resolve().parent
    test_bootstrap_dir = this_dir.parent.parent / "test-bootstrap"

    assert test_bootstrap_dir.is_dir(), f"Expected directory {test_bootstrap_dir}"

    report = analyze_package(test_bootstrap_dir)
    assert report.passed, f"test-bootstrap audit failed: {report.render_text()}"
    assert len(report.capabilities_detected) == 0, (
        f"Expected pure computation, detected: {report.capabilities_detected}"
    )
    assert len(report.undeclared_capabilities) == 0


def test_analyze_self_passes_check() -> None:
    """Verify ca-capability-analyzer self-analysis matches declared capabilities."""
    this_dir = Path(__file__).resolve().parent
    self_dir = this_dir.parent

    assert (self_dir / "required_capabilities.json").is_file()

    report = analyze_package(self_dir)
    assert report.passed, f"Self-analysis audit failed: {report.render_text()}"
    assert CAP_FS_READ in report.capabilities_detected
    assert len(report.undeclared_capabilities) == 0
