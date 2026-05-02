"""Executable surface audit for the grammar-backed ALGOL 60 WASM lane."""

from __future__ import annotations

from dataclasses import dataclass
from textwrap import dedent

import pytest
from wasm_runtime import WasiConfig, WasiHost, WasmRuntime

from algol_wasm_compiler import compile_source


@dataclass(frozen=True)
class SurfaceCase:
    """A grammar-surface program with observable end-to-end behavior."""

    name: str
    source: str
    result: list[int]
    stdout: str = ""


_SURFACE_CASES = (
    SurfaceCase(
        name="publication-notation-and-comments",
        source="""
            BEGIN
              INTEGER result;
              BOOLEAN ok;
              COMMENT publication symbols and mixed-case keywords;
              result := 2 × 3 + entier(7.0 ÷ 2.0);
              ok := (result ≥ 9) ∧ (result ≤ 10) ∧ (result ≠ 11) ∧ ¬ false;
              if ok ⊃ true then result := result + 1 else result := 0;
              if ok ≡ true then result := result + 1 else result := 0
            END
        """,
        result=[11],
    ),
    SurfaceCase(
        name="single-statement-procedures-and-bare-calls",
        source="""
            begin
              integer result;
              procedure bump; result := result + 1;
              integer procedure seven(); seven := 7;
              result := 0;
              bump;
              result := result + seven
            end
        """,
        result=[8],
    ),
    SurfaceCase(
        name="for-list-scalar-and-array-control",
        source="""
            begin
              integer result, i;
              integer array a[1:3];
              result := 0;
              for i := 1 step 1 until 3, 5 do result := result + i;
              for a[2] := 1 step 1 until 3 do result := result + a[2];
              i := 0;
              for i := i + 1 while i < 4 do result := result + i
            end
        """,
        result=[23],
    ),
    SurfaceCase(
        name="typed-formal-procedure-dispatch",
        source="""
            begin
              integer result;
              string msg;
              boolean procedure yes(x); value x; boolean x; begin yes := not x end;
              string procedure echo(s); value s; string s; begin echo := s end;
              procedure use(bp, sp); boolean procedure bp; string procedure sp;
                begin
                  if bp(false) then result := result + 3 else result := 0;
                  msg := sp("OK")
                end;
              result := 0;
              use(yes, echo);
              print(msg);
              output(' ');
              print(result)
            end
        """,
        result=[3],
        stdout="OK 3",
    ),
    SurfaceCase(
        name="boolean-string-value-array-copies",
        source="""
            begin
              integer result;
              string array words[1:2];
              boolean array flags[1:2];
              procedure mutate(w, f); value w, f; string array w; boolean array f;
                begin w[1] := 'NO'; f[1] := false end;
              words[1] := 'YES';
              flags[1] := true;
              mutate(words, flags);
              if flags[1] and words[1] = 'YES' then result := 17 else result := 0;
              print(words[1]);
              output(' ');
              print(result)
            end
        """,
        result=[17],
        stdout="YES 17",
    ),
    SurfaceCase(
        name="nonlocal-switch-goto-unwinds-dynamic-storage",
        source="""
            begin
              integer result;
              switch route := left, right;
              procedure escape(s); switch s; begin goto s[2] end;
              result := 0;
              begin
                integer array temp[1:2];
                temp[1] := 4;
                escape(route);
                result := 99
              end;
            left:
              result := 1;
              goto done;
            right:
              result := result + 6;
            done:
            end
        """,
        result=[6],
    ),
    SurfaceCase(
        name="program-without-result-returns-zero-after-output",
        source="""
            begin
              string msg;
              msg := 'NORESULT';
              print(msg)
            end
        """,
        result=[0],
        stdout="NORESULT",
    ),
)


@pytest.mark.parametrize("case", _SURFACE_CASES, ids=lambda case: case.name)
def test_algol_wasm_surface_cases_execute_end_to_end(case: SurfaceCase) -> None:
    compiled = compile_source(dedent(case.source).strip())
    captured: list[str] = []
    runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=captured.append)))

    assert runtime.load_and_run(compiled.binary, "_start", []) == case.result
    assert "".join(captured) == case.stdout
