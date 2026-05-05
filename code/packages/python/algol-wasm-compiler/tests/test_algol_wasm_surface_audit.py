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
        name="forward-procedure-declaration-visibility",
        source="""
            begin
              procedure first; begin second end;
              integer result;
              switch route := done;
              procedure second; begin result := 12; goto route[1] end;
              first;
              result := 0;
            done:
              print('FORWARD ', result)
            end
        """,
        result=[12],
        stdout="FORWARD 12",
    ),
    SurfaceCase(
        name="forward-read-only-by-name-expression-actual",
        source="""
            begin
              integer result;
              procedure relay(x); integer x; begin emit(x) end;
              procedure emit(y); integer y; begin result := y end;
              relay(3 + 4);
              print(result)
            end
        """,
        result=[7],
        stdout="7",
    ),
    SurfaceCase(
        name="forward-switch-declaration-visibility",
        source="""
            begin
              integer result;
              switch outer := if ready() then inner[1] else fail;
              switch inner := done;
              boolean procedure ready; begin ready := true end;
              goto outer[1];
            fail:
              result := 0;
              goto finish;
            done:
              result := 13;
            finish:
              print('SWITCH ', result)
            end
        """,
        result=[13],
        stdout="SWITCH 13",
    ),
    SurfaceCase(
        name="forward-array-bound-visibility",
        source="""
            begin
              integer result;
              integer array a[lower():upper()];
              integer procedure lower; begin lower := 0 end;
              integer procedure upper; begin upper := 1 end;
              a[0] := 5;
              a[1] := 8;
              result := a[0] + a[1];
              print('ARRAY ', result)
            end
        """,
        result=[13],
        stdout="ARRAY 13",
    ),
    SurfaceCase(
        name="array-bound-declaration-order",
        source="""
            begin
              integer result;
              integer array b[0:0];
              integer array a[b[0]:b[0]];
              a[0] := 21;
              result := a[0];
              print('ORDER ', result)
            end
        """,
        result=[21],
        stdout="ORDER 21",
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
        name="typed-by-name-expression-actuals",
        source="""
            begin
              integer result;
              real r;
              boolean flag;
              string word;
              real procedure readreal(x); real x;
                begin r := r + 1.0; readreal := x end;
              boolean procedure readbool(x); boolean x;
                begin flag := not flag; readbool := x end;
              string procedure readstring(x); string x;
                begin word := 'LAZY'; readstring := x end;
              result := 0;
              r := 2.0;
              if readreal(r + 0.5) = 3.5 then result := result + 3 else result := 0;
              flag := false;
              if readbool(flag) then result := result + 5 else result := 0;
              word := 'EARLY';
              if readstring(if true then word else 'NO') = 'LAZY' then
                result := result + 7
              else
                result := 0;
              print(result)
            end
        """,
        result=[15],
        stdout="15",
    ),
    SurfaceCase(
        name="multi-argument-output",
        source="""
            begin
              integer result;
              real x;
              boolean ok;
              string msg;
              x := 1.5;
              ok := true;
              msg := 'IO';
              result := 21;
              print(msg, ' ', result, ' ', ok, ' ', x)
            end
        """,
        result=[21],
        stdout="IO 21 true 1.500",
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
