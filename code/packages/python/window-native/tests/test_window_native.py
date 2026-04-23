from __future__ import annotations

import sys

import pytest

import window_native as window_native_module
from window_native import (
    LogicalSize,
    PhysicalSize,
    RenderTargetKind,
    SurfacePreference,
    Window,
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


@pytest.mark.parametrize(
    ("width", "height"),
    [
        (-1.0, 240.0),
        (320.0, -1.0),
        (float("inf"), 240.0),
        (320.0, float("nan")),
    ],
)
def test_create_window_rejects_invalid_sizes_before_native_boundary(
    width: float,
    height: float,
) -> None:
    with pytest.raises(WindowError, match="finite, non-negative"):
        create_window(width=width, height=height, visible=False)


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


def test_create_window_normalizes_arguments_before_crossing_native_boundary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}

    def fake_create_window(
        title: str,
        width: float,
        height: float,
        preferred_surface: int,
        visible: bool,
        resizable: bool,
        decorations: bool,
        transparent: bool,
    ) -> int:
        captured.update(
            title=title,
            width=width,
            height=height,
            preferred_surface=preferred_surface,
            visible=visible,
            resizable=resizable,
            decorations=decorations,
            transparent=transparent,
        )
        return 41

    monkeypatch.setattr(window_native_module, "_create_window", fake_create_window)
    monkeypatch.setattr(window_native_module, "_close_window", lambda handle: None)

    window = create_window(
        title="Normalized",
        width=320,
        height=240,
        preferred_surface=SurfacePreference.CAIRO,
        visible=1,
        resizable=0,
        decorations=7,
        transparent=[],
    )

    try:
        assert isinstance(window, Window)
        assert captured == {
            "title": "Normalized",
            "width": 320.0,
            "height": 240.0,
            "preferred_surface": int(SurfacePreference.CAIRO),
            "visible": True,
            "resizable": False,
            "decorations": True,
            "transparent": False,
        }
    finally:
        window.close()


@pytest.mark.parametrize("bad_width", ["wide", object(), None])
def test_create_window_rejects_non_numeric_widths(
    bad_width: object,
) -> None:
    with pytest.raises(TypeError, match="width must be numeric"):
        create_window(width=bad_width, height=120.0)


def test_window_methods_delegate_to_native_bindings(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls: list[tuple[object, ...]] = []

    monkeypatch.setattr(window_native_module, "_close_window", lambda handle: calls.append(("close", handle)))
    monkeypatch.setattr(window_native_module, "_window_id", lambda handle: 17)
    monkeypatch.setattr(window_native_module, "_window_logical_size", lambda handle: (640.0, 480.0))
    monkeypatch.setattr(window_native_module, "_window_physical_size", lambda handle: (1280, 960))
    monkeypatch.setattr(window_native_module, "_window_scale_factor", lambda handle: 2.0)
    monkeypatch.setattr(
        window_native_module,
        "_window_request_redraw",
        lambda handle: calls.append(("request_redraw", handle)),
    )
    monkeypatch.setattr(
        window_native_module,
        "_window_set_title",
        lambda handle, title: calls.append(("set_title", handle, title)),
    )
    monkeypatch.setattr(
        window_native_module,
        "_window_set_visible",
        lambda handle, visible: calls.append(("set_visible", handle, visible)),
    )
    monkeypatch.setattr(window_native_module, "_window_render_target_kind", lambda handle: "win32")

    window = Window(9)

    assert window.__enter__() is window
    assert window.id() == 17
    assert window.logical_size() == LogicalSize(640.0, 480.0)
    assert window.physical_size() == PhysicalSize(1280, 960)
    assert window.scale_factor() == 2.0
    assert window.render_target_kind() is RenderTargetKind.WIN32

    window.request_redraw()
    window.set_title("Delegated")
    window.set_visible(True)
    window.__exit__(None, None, None)
    window.close()

    assert calls == [
        ("request_redraw", 9),
        ("set_title", 9, "Delegated"),
        ("set_visible", 9, True),
        ("close", 9),
    ]

    with pytest.raises(WindowError, match="window handle is closed"):
        window.id()


def test_window_destructor_swallows_close_failures(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_close_window(handle: int) -> None:
        raise RuntimeError(f"close failed for {handle}")

    monkeypatch.setattr(window_native_module, "_close_window", fake_close_window)

    window = Window(5)
    window.__del__()


def test_module_exports_remain_stable() -> None:
    assert window_native_module.__all__ == [
        "LogicalSize",
        "PhysicalSize",
        "RenderTargetKind",
        "SurfacePreference",
        "Window",
        "WindowError",
        "create_window",
    ]
