from __future__ import annotations

import sys

import pytest

from window_native import (
    LogicalSize,
    PhysicalSize,
    RenderTargetKind,
    SurfacePreference,
    WindowError,
    create_window,
)


def test_surface_preference_values_match_shared_contract() -> None:
    assert SurfacePreference.DEFAULT == 0
    assert SurfacePreference.METAL == 1
    assert SurfacePreference.DIRECT2D == 2
    assert SurfacePreference.CAIRO == 3
    assert SurfacePreference.CANVAS2D == 4


def test_render_target_kind_strings_are_stable() -> None:
    assert RenderTargetKind.APPKIT.value == "appkit"
    assert RenderTargetKind.WIN32.value == "win32"
    assert RenderTargetKind.BROWSER_CANVAS.value == "browser-canvas"


@pytest.mark.skipif(sys.platform != "darwin", reason="macOS-only smoke path")
def test_create_hidden_window_and_query_basic_state() -> None:
    window = create_window(
        title="Python window-native smoke test",
        width=320.0,
        height=240.0,
        preferred_surface=SurfacePreference.METAL,
        visible=False,
    )

    try:
        assert window.id() >= 1
        assert window.logical_size() == LogicalSize(320.0, 240.0)
        assert window.physical_size() == PhysicalSize(320, 240)
        assert window.scale_factor() == 1.0
        assert window.render_target_kind() is RenderTargetKind.APPKIT
        window.set_title("Updated from pytest")
        window.request_redraw()
        window.set_visible(False)
        window.close()
        window.close()
    finally:
        window.close()


@pytest.mark.skipif(sys.platform == "darwin", reason="non-macOS unsupported-path check")
def test_create_window_raises_on_unsupported_platforms() -> None:
    with pytest.raises(WindowError, match="only wired for AppKit|only available on Apple"):
        create_window(title="unsupported")
