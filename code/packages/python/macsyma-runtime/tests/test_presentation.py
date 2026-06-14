"""Presentation helpers for MACSYMA runtime results."""

from __future__ import annotations

from symbolic_ir import ADD, DIV, IRApply, IRInteger, IRSymbol

from macsyma_runtime import EV, has_ev_flag, output_text_for


def test_has_ev_flag_detects_display2d_case_insensitively() -> None:
    expr = IRApply(EV, (IRSymbol("x"), IRSymbol("Display2D")))

    assert has_ev_flag(expr, "display2d") is True
    assert has_ev_flag(IRSymbol("x"), "display2d") is False


def test_output_text_for_routes_display2d_through_box_pretty_printer() -> None:
    x = IRSymbol("x")
    output = IRApply(DIV, (IRInteger(1), IRApply(ADD, (x, IRInteger(1)))))
    input_expr = IRApply(EV, (output, IRSymbol("display2d")))

    rendered = output_text_for(input_expr, output)

    assert "\n" in rendered
    assert "─" in rendered
    assert "x + 1" in rendered


def test_output_text_for_defaults_to_linear_macsyma_pretty_text() -> None:
    x = IRSymbol("x")
    output = IRApply(ADD, (x, IRInteger(1)))

    assert output_text_for(output, output) == "x + 1"
