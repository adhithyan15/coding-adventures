#!/usr/bin/env python3
"""Generate native Scytale tests from the language-neutral cipher fixtures.

The generated tests intentionally contain no JSON loader.  The bounded, strict
loader lives here, and every implementation lane receives ordinary source code
that its existing test runner can compile without a new runtime dependency.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import Any

from package_parity_report import IMPLEMENTATION_LANGUAGES

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_PATH = REPO_ROOT / "code/specs/fixtures/classical-ciphers-v1/cases.json"
MAX_FIXTURE_BYTES = 131_072
MAX_FIXTURE_DEPTH = 8
SCYTALE_OPERATIONS = {
    "scytale-encrypt",
    "scytale-decrypt",
    "scytale-brute-force",
}
IDENTIFIER_PATTERN = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")

TARGETS = {
    "csharp": Path(
        "code/packages/csharp/scytale-cipher/tests/CodingAdventures.ScytaleCipher.Tests/GeneratedClassicalCipherFixtureTests.cs"
    ),
    "dart": Path(
        "code/packages/dart/scytale-cipher/test/generated_classical_cipher_fixture_test.dart"
    ),
    "elixir": Path(
        "code/packages/elixir/scytale_cipher/test/generated_classical_cipher_fixture_test.exs"
    ),
    "fsharp": Path(
        "code/packages/fsharp/scytale-cipher/tests/CodingAdventures.ScytaleCipher.Tests/GeneratedClassicalCipherFixtureTests.fs"
    ),
    "go": Path(
        "code/packages/go/scytale-cipher/generated_classical_cipher_fixture_test.go"
    ),
    "haskell": Path(
        "code/packages/haskell/scytale-cipher/test/GeneratedClassicalCipherFixtureSpec.hs"
    ),
    "java": Path(
        "code/packages/java/scytale-cipher/src/test/java/com/codingadventures/scytalecipher/GeneratedClassicalCipherFixtureTest.java"
    ),
    "kotlin": Path(
        "code/packages/kotlin/scytale-cipher/src/test/kotlin/com/codingadventures/scytalecipher/GeneratedClassicalCipherFixtureTest.kt"
    ),
    "lua": Path(
        "code/packages/lua/scytale_cipher/tests/test_generated_classical_cipher_fixture.lua"
    ),
    "perl": Path(
        "code/packages/perl/scytale-cipher/t/02-generated-classical-cipher-fixture.t"
    ),
    "python": Path(
        "code/packages/python/scytale-cipher/tests/test_generated_classical_cipher_fixture.py"
    ),
    "ruby": Path(
        "code/packages/ruby/scytale_cipher/test/test_generated_classical_cipher_fixture.rb"
    ),
    "rust": Path(
        "code/packages/rust/scytale-cipher/tests/generated_classical_cipher_fixture.rs"
    ),
    "swift": Path(
        "code/packages/swift/scytale-cipher/Tests/ScytaleCipherTests/GeneratedClassicalCipherFixtureTests.swift"
    ),
    "typescript": Path(
        "code/packages/typescript/scytale-cipher/tests/generated-classical-cipher-fixture.test.ts"
    ),
}

EXPECTED_LIMITS = {
    "max_cases": 64,
    "max_fixture_bytes": MAX_FIXTURE_BYTES,
    "max_fixture_nesting_depth": MAX_FIXTURE_DEPTH,
    "max_fixture_text_scalars": 8193,
    "max_scytale_brute_force_scalars": 4096,
    "max_vigenere_analysis_scalars": 8192,
    "max_vigenere_key_length": 40,
}


def _reject_duplicate_names(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("fixture-invalid-json: duplicate object name")
        result[key] = value
    return result


def _reject_nonfinite(token: str) -> None:
    raise ValueError(f"fixture-invalid-json: non-finite number {token}")


def _check_raw_depth(raw: bytes) -> None:
    depth = 0
    in_string = False
    escaped = False
    for byte in raw:
        if in_string:
            if escaped:
                escaped = False
            elif byte == 0x5C:
                escaped = True
            elif byte == 0x22:
                in_string = False
        elif byte == 0x22:
            in_string = True
        elif byte in (0x7B, 0x5B):
            depth += 1
            if depth > MAX_FIXTURE_DEPTH:
                raise ValueError("fixture-depth-limit")
        elif byte in (0x7D, 0x5D):
            depth -= 1
            if depth < 0:
                raise ValueError("fixture-invalid-json: unbalanced container")


def _check_scalars(value: Any) -> None:
    pending = [value]
    while pending:
        current = pending.pop()
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                raise ValueError("fixture-invalid-scalar")
        elif isinstance(current, float) and not math.isfinite(current):
            raise ValueError("fixture-invalid-json: non-finite number")
        elif isinstance(current, dict):
            pending.extend(current.keys())
            pending.extend(current.values())
        elif isinstance(current, list):
            pending.extend(current)


def load_cases_bytes(raw: bytes) -> tuple[list[dict[str, Any]], str]:
    """Strictly load, validate, and select every normative Scytale case."""
    if len(raw) > MAX_FIXTURE_BYTES:
        raise ValueError("fixture-size-limit")
    _check_raw_depth(raw)
    try:
        text = raw.decode("utf-8", errors="strict")
        document = json.loads(
            text,
            object_pairs_hook=_reject_duplicate_names,
            parse_constant=_reject_nonfinite,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("fixture-invalid-json") from error
    _check_scalars(document)
    if not isinstance(document, dict):
        raise ValueError("fixture-invalid-profile")
    if document.get("schema_version") != 1:
        raise ValueError("fixture-invalid-profile")
    if document.get("profile") != "cr01-cr03-portable-v1":
        raise ValueError("fixture-invalid-profile")
    if document.get("limits") != EXPECTED_LIMITS:
        raise ValueError("fixture-invalid-profile")
    all_cases = document.get("cases")
    if not isinstance(all_cases, list) or len(all_cases) > EXPECTED_LIMITS["max_cases"]:
        raise ValueError("fixture-invalid-profile")

    selected: list[dict[str, Any]] = []
    seen_ids: set[str] = set()
    for case in all_cases:
        if not isinstance(case, dict) or set(case) != {
            "id",
            "operation",
            "input",
            "expected",
        }:
            raise ValueError("fixture-invalid-case")
        case_id = case.get("id")
        operation = case.get("operation")
        if (
            not isinstance(case_id, str)
            or IDENTIFIER_PATTERN.fullmatch(case_id) is None
            or case_id in seen_ids
        ):
            raise ValueError("fixture-invalid-case")
        seen_ids.add(case_id)
        if operation not in SCYTALE_OPERATIONS:
            continue
        _validate_scytale_case(case)
        selected.append(case)
    if (
        len(selected) != 18
        or {case["operation"] for case in selected} != SCYTALE_OPERATIONS
    ):
        raise ValueError("fixture-invalid-scytale-roster")
    return selected, hashlib.sha256(raw).hexdigest()


def _validate_scytale_case(case: dict[str, Any]) -> None:
    operation = case["operation"]
    input_value = case["input"]
    expected = case["expected"]
    if not isinstance(input_value, dict) or not isinstance(expected, dict):
        raise ValueError("fixture-invalid-case")
    if operation in {"scytale-encrypt", "scytale-decrypt"}:
        if set(input_value) != {"text", "key"}:
            raise ValueError("fixture-invalid-case")
        if (
            not isinstance(input_value["text"], str)
            or type(input_value["key"]) is not int
        ):
            raise ValueError("fixture-invalid-case")
        if set(expected) not in ({"text"}, {"error_id"}):
            raise ValueError("fixture-invalid-case")
    else:
        if set(input_value) not in ({"text"}, {"repeat_scalar", "repeat_count"}):
            raise ValueError("fixture-invalid-case")
        if "text" in input_value and not isinstance(input_value["text"], str):
            raise ValueError("fixture-invalid-case")
        if "repeat_scalar" in input_value and (
            not isinstance(input_value["repeat_scalar"], str)
            or len(input_value["repeat_scalar"]) != 1
            or type(input_value["repeat_count"]) is not int
        ):
            raise ValueError("fixture-invalid-case")
        if set(expected) not in ({"candidates"}, {"error_id"}):
            raise ValueError("fixture-invalid-case")
        if "candidates" in expected and (
            not isinstance(expected["candidates"], list)
            or any(
                not isinstance(candidate, dict)
                or set(candidate) != {"key", "text"}
                or type(candidate["key"]) is not int
                or not isinstance(candidate["text"], str)
                for candidate in expected["candidates"]
            )
        ):
            raise ValueError("fixture-invalid-case")
    if "text" in expected and not isinstance(expected["text"], str):
        raise ValueError("fixture-invalid-case")
    if "error_id" in expected and (
        not isinstance(expected["error_id"], str)
        or IDENTIFIER_PATTERN.fullmatch(expected["error_id"]) is None
    ):
        raise ValueError("fixture-invalid-case")


def load_cases(path: Path) -> tuple[list[dict[str, Any]], str]:
    with path.open("rb") as stream:
        raw = stream.read(MAX_FIXTURE_BYTES + 1)
    return load_cases_bytes(raw)


def _scalar_values(value: str) -> str:
    return ", ".join(str(ord(character)) for character in value)


def _quote_csharp(value: str) -> str:
    return f"Scalars({_scalar_values(value)})"


def _quote_dart(value: str) -> str:
    return json.dumps(value, ensure_ascii=False).replace("$", r"\$")


def _quote_elixir(value: str) -> str:
    return f"List.to_string([{_scalar_values(value)}])"


def _quote_fsharp(value: str) -> str:
    return f"(scalars [{_scalar_values(value).replace(',', ';')}])"


def _quote_go(value: str) -> str:
    return f"string([]rune{{{_scalar_values(value)}}})"


def _quote_haskell(value: str) -> str:
    return f"(map chr [{_scalar_values(value)}])"


def _quote_java(value: str) -> str:
    values = _scalar_values(value)
    return f"new String(new int[] {{{values}}}, 0, {len(value)})"


def _quote_kotlin(value: str) -> str:
    return f"scalars({_scalar_values(value)})"


def _quote_lua(value: str) -> str:
    return f"utf8.char({_scalar_values(value)})"


def _quote_perl(value: str) -> str:
    scalars = _scalar_values(value)
    suffix = f", {scalars}" if scalars else ""
    return f'pack("U*"{suffix})'


def _quote_ruby(value: str) -> str:
    return f'[{_scalar_values(value)}].pack("U*")'


def _quote_python(value: str) -> str:
    values = _scalar_values(value)
    if len(value) == 1:
        values += ","
    return f'"".join(map(chr, ({values})))'


def _quote_rust(value: str) -> str:
    escaped = "".join(f"\\u{{{ord(character):x}}}" for character in value)
    return f'"{escaped}"'


def _rust_assignment(name: str, value: str) -> list[str]:
    literal = _quote_rust(value)
    compact = f"    let {name} = {literal};"
    if len(compact) <= 100:
        return [compact]
    return [f"    let {name} =", f"        {literal};"]


def _quote_swift(value: str) -> str:
    return f"scalars([{_scalar_values(value)}])"


def _quote_typescript(value: str) -> str:
    return f"String.fromCodePoint({_scalar_values(value)})"


def _repeat_descriptor(case: dict[str, Any]) -> tuple[str, int]:
    input_value = case["input"]
    return input_value["repeat_scalar"], input_value["repeat_count"]


def _header(prefix: str, digest: str) -> str:
    return (
        f"{prefix} GENERATED by code/scripts/generate_scytale_fixture_consumers.py.\n"
        f"{prefix} Source SHA-256: {digest}\n"
        f"{prefix} Do not edit by hand.\n"
    )


def _text_cases(cases: Iterable[dict[str, Any]]) -> Iterable[dict[str, Any]]:
    return (case for case in cases if "text" in case["expected"])


def _invalid_cases(cases: Iterable[dict[str, Any]]) -> Iterable[dict[str, Any]]:
    return (
        case
        for case in cases
        if case["operation"] in {"scytale-encrypt", "scytale-decrypt"}
        and "error_id" in case["expected"]
    )


def _brute_case(cases: Iterable[dict[str, Any]], suffix: str) -> dict[str, Any]:
    return next(case for case in cases if case["id"].endswith(suffix))


def render_csharp(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "namespace CodingAdventures.ScytaleCipher.Tests;\n",
        "public sealed class GeneratedClassicalCipherFixtureTests\n{",
        "    private static string Scalars(params int[] values) => string.Concat(values.Select(char.ConvertFromUtf32));\n",
        "    [Fact]",
        "    public void MatchesAllNormativeScytaleCases()\n    {",
    ]
    for case in _text_cases(cases):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"        // {case['id']}",
            f"        Assert.Equal({_quote_csharp(case['expected']['text'])}, ScytaleCipher.{fn}({_quote_csharp(case['input']['text'])}, {case['input']['key']}));",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"        // {case['id']}",
            f"        var invalidError{index} = Assert.Throws<ArgumentOutOfRangeException>(() => ScytaleCipher.{fn}({_quote_csharp(case['input']['text'])}, {case['input']['key']}));",
            f"        Assert.Equal({_quote_csharp(case['expected']['error_id'])}, invalidError{index}.Message.Contains({_quote_csharp('Key must')}, StringComparison.Ordinal) ? {_quote_csharp('scytale-invalid-key')} : {_quote_csharp('unexpected-error')});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    candidates = ", ".join(
        f"new BruteForceResult({c['key']}, {_quote_csharp(c['text'])})"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"        // {brute['id']}",
        f"        Assert.Equal(new[] {{ {candidates} }}, ScytaleCipher.BruteForce({_quote_csharp(brute['input']['text'])}));",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"        // {short['id']}",
        f"        Assert.Empty(ScytaleCipher.BruteForce({_quote_csharp(short['input']['text'])}));",
        f"        // {limit['id']}",
        f"        var limitError = Assert.Throws<ArgumentOutOfRangeException>(() => ScytaleCipher.BruteForce(string.Concat(Enumerable.Repeat({_quote_csharp(repeat_scalar)}, {repeat_count}))));",
        f"        Assert.Equal({_quote_csharp(limit['expected']['error_id'])}, limitError.GetType() == typeof(ArgumentOutOfRangeException) ? {_quote_csharp('scytale-brute-force-limit')} : {_quote_csharp('unexpected-error')});",
        "    }",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_dart(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "import 'package:coding_adventures_scytale_cipher/scytale_cipher.dart';",
        "import 'package:test/test.dart';\n",
        "void main() {",
        "  test('generated classical-cipher Scytale fixtures', () {",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    expect({fn}({_quote_dart(case['input']['text'])}, {case['input']['key']}), {_quote_dart(case['expected']['text'])});",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            "    expect(",
            f"        () => {fn}({_quote_dart(case['input']['text'])}, {case['input']['key']}),",
            "        throwsA(isA<ArgumentError>().having(",
            f"            (error) => error.message.toString().startsWith({_quote_dart('key must')}),",
            "            'message',",
            "            isTrue)));",
            f"    expect({_quote_dart('scytale-invalid-key')}, {_quote_dart(case['expected']['error_id'])});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = [
        f"      [{candidate['key']}, {_quote_dart(candidate['text'])}]"
        for candidate in brute["expected"]["candidates"]
    ]
    lines += [
        f"    // {brute['id']}",
        f"    expect(bruteForce({_quote_dart(brute['input']['text'])}).map((c) => [c.key, c.text]).toList(), [",
        *[f"{line}," for line in expected[:-1]],
        expected[-1],
        "    ]);",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    // {short['id']}",
        f"    expect(bruteForce({_quote_dart(short['input']['text'])}), isEmpty);",
        f"    // {limit['id']}",
        "    expect(",
        f"        () => bruteForce(List.filled({repeat_count}, {_quote_dart(repeat_scalar)}).join()),",
        "        throwsA(isA<RangeError>().having(",
        f"            (error) => error.message, 'message', {_quote_dart(limit['expected']['error_id'])})));",
        "  });",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_elixir(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("#", digest),
        "defmodule CodingAdventures.GeneratedClassicalCipherFixtureTest do",
        "  use ExUnit.Case, async: true",
        "  alias CodingAdventures.ScytaleCipher\n",
        '  test "all normative Scytale cases" do',
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    assert ScytaleCipher.{fn}({_quote_elixir(case['input']['text'])}, {case['input']['key']}) == {_quote_elixir(case['expected']['text'])}",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    invalid_error_{index} = assert_raise ArgumentError, fn -> ScytaleCipher.{fn}({_quote_elixir(case['input']['text'])}, {case['input']['key']}) end",
            f"    assert String.starts_with?(Exception.message(invalid_error_{index}), {_quote_elixir('Key must')})",
            f"    assert {_quote_elixir('scytale-invalid-key')} == {_quote_elixir(case['expected']['error_id'])}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"%{{key: {c['key']}, text: {_quote_elixir(c['text'])}}}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    # {brute['id']}",
        f"    assert ScytaleCipher.brute_force({_quote_elixir(brute['input']['text'])}) == [{expected}]",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    # {short['id']}",
        f"    assert ScytaleCipher.brute_force({_quote_elixir(short['input']['text'])}) == []",
        f"    # {limit['id']}",
        f"    assert_raise ArgumentError, {_quote_elixir(limit['expected']['error_id'])}, fn ->",
        f"      ScytaleCipher.brute_force(String.duplicate({_quote_elixir(repeat_scalar)}, {repeat_count}))",
        "    end",
        "  end",
        "end",
    ]
    return "\n".join(lines) + "\n"


def render_fsharp(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "namespace CodingAdventures.ScytaleCipher.Tests\n",
        "open System",
        "open Xunit",
        "open CodingAdventures.ScytaleCipher\n",
        "type GeneratedClassicalCipherFixtureTests() =",
        "    [<Fact>]",
        "    member _.``all normative Scytale cases``() =",
        '        let scalars values = values |> List.map Char.ConvertFromUtf32 |> String.concat ""',
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        Assert.Equal({_quote_fsharp(case['expected']['text'])}, ScytaleCipher.{fn} {_quote_fsharp(case['input']['text'])} {case['input']['key']})",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        let invalidError{index} = Assert.Throws<ArgumentException>(fun () -> ScytaleCipher.{fn} {_quote_fsharp(case['input']['text'])} {case['input']['key']} |> ignore)",
            f"        Assert.Equal({_quote_fsharp(case['expected']['error_id'])}, if invalidError{index}.Message.Contains({_quote_fsharp('Key must')}, StringComparison.Ordinal) then {_quote_fsharp('scytale-invalid-key')} else {_quote_fsharp('unexpected-error')})",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = "; ".join(
        f"{{ Key = {c['key']}; Text = {_quote_fsharp(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"        // {brute['id']}",
        f"        Assert.Equal<BruteForceResult list>([ {expected} ], ScytaleCipher.bruteForce {_quote_fsharp(brute['input']['text'])})",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"        // {short['id']}",
        f"        Assert.Empty(ScytaleCipher.bruteForce {_quote_fsharp(short['input']['text'])})",
        f"        // {limit['id']}",
        f"        let limitError = Assert.Throws<ArgumentException>(fun () -> ScytaleCipher.bruteForce (String.replicate {repeat_count} {_quote_fsharp(repeat_scalar)}) |> ignore)",
        f"        Assert.Equal({_quote_fsharp(limit['expected']['error_id'])}, if limitError.GetType() = typeof<ArgumentException> then {_quote_fsharp('scytale-brute-force-limit')} else {_quote_fsharp('unexpected-error')})",
    ]
    return "\n".join(lines) + "\n"


def render_go(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "package scytalecipher\n",
        "import (",
        '\t"errors"',
        '\t"reflect"',
        '\t"strings"',
        '\t"testing"',
        ")\n",
        "func TestGeneratedClassicalCipherFixture(t *testing.T) {",
    ]
    for index, case in enumerate(_text_cases(cases)):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"\t// {case['id']}",
            f"\tgot{index}, err{index} := {fn}({_quote_go(case['input']['text'])}, {case['input']['key']})",
            f"\tif err{index} != nil || got{index} != {_quote_go(case['expected']['text'])} {{",
            f'\t\tt.Fatalf("{case["id"]}: got %q, %v", got{index}, err{index})',
            "\t}",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"\t// {case['id']}",
            f"\t_, invalidErr{index} := {fn}({_quote_go(case['input']['text'])}, {case['input']['key']})",
            f"\tinvalidID{index} := {_quote_go('unexpected-error')}",
            f"\tif errors.Is(invalidErr{index}, ErrInvalidKey) {{",
            f"\t\tinvalidID{index} = {_quote_go('scytale-invalid-key')}",
            "\t}",
            f"\tif invalidID{index} != {_quote_go(case['expected']['error_id'])} {{",
            f'\t\tt.Fatalf("{case["id"]}: %s, %v", invalidID{index}, invalidErr{index})',
            "\t}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{Key: {c['key']}, Text: {_quote_go(c['text'])}}}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"\t// {brute['id']}",
        f"\tbrute, err := BruteForce({_quote_go(brute['input']['text'])})",
        f"\tif err != nil || !reflect.DeepEqual(brute, []BruteForceResult{{{expected}}}) {{",
        f'\t\tt.Fatalf("{brute["id"]}: %#v, %v", brute, err)',
        "\t}",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"\t// {short['id']}",
        f"\tshort, err := BruteForce({_quote_go(short['input']['text'])})",
        "\tif err != nil || len(short) != 0 {",
        f'\t\tt.Fatalf("{short["id"]}: %#v, %v", short, err)',
        "\t}",
        f"\t// {limit['id']}",
        f"\t_, limitErr := BruteForce(strings.Repeat({_quote_go(repeat_scalar)}, {repeat_count}))",
        f"\tif !errors.Is(limitErr, ErrBruteForceLimit) || limitErr.Error() != {_quote_go(limit['expected']['error_id'])} {{",
        f'\t\tt.Fatalf("{limit["id"]}: %v", limitErr)',
        "\t}",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_haskell(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("--", digest),
        "module GeneratedClassicalCipherFixtureSpec (spec) where\n",
        "import Data.Char (chr)",
        "import Data.List (isPrefixOf)",
        "import ScytaleCipher",
        "import Test.Hspec\n",
        "spec :: Spec",
        'spec = describe "generated classical-cipher Scytale fixtures" $ do',
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    -- {case['id']}",
            f"    it {_quote_haskell(case['id'])} $ {fn} {_quote_haskell(case['input']['text'])} ({case['input']['key']}) `shouldBe` Right {_quote_haskell(case['expected']['text'])}",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    -- {case['id']}",
            f"    it {_quote_haskell(case['id'])} $ either (\\err -> if {_quote_haskell('Key must')} `isPrefixOf` err then {_quote_haskell('scytale-invalid-key')} else {_quote_haskell('unexpected-error')}) (const {_quote_haskell('unexpected-success')}) ({fn} {_quote_haskell(case['input']['text'])} ({case['input']['key']})) `shouldBe` {_quote_haskell(case['expected']['error_id'])}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"BruteForceResult {c['key']} {_quote_haskell(c['text'])}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    -- {brute['id']}",
        f"    it {_quote_haskell(brute['id'])} $ bruteForce {_quote_haskell(brute['input']['text'])} `shouldBe` Right [{expected}]",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    -- {short['id']}",
        f"    it {_quote_haskell(short['id'])} $ bruteForce {_quote_haskell(short['input']['text'])} `shouldBe` Right []",
        f"    -- {limit['id']}",
        f"    it {_quote_haskell(limit['id'])} $ bruteForce (concat (replicate {repeat_count} {_quote_haskell(repeat_scalar)})) `shouldBe` Left {_quote_haskell(limit['expected']['error_id'])}",
    ]
    return "\n".join(lines) + "\n"


def render_java(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "package com.codingadventures.scytalecipher;\n",
        "import java.util.List;",
        "import org.junit.jupiter.api.Test;",
        "import static org.junit.jupiter.api.Assertions.*;\n",
        "class GeneratedClassicalCipherFixtureTest {",
        "    @Test",
        "    void matchesAllNormativeScytaleCases() {",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        assertEquals({_quote_java(case['expected']['text'])}, ScytaleCipher.{fn}({_quote_java(case['input']['text'])}, {case['input']['key']}));",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        var invalidError{index} = assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.{fn}({_quote_java(case['input']['text'])}, {case['input']['key']}));",
            f"        assertEquals({_quote_java(case['expected']['error_id'])}, invalidError{index}.getMessage().startsWith({_quote_java('key')}) && invalidError{index}.getMessage().contains({_quote_java('must')}) ? {_quote_java('scytale-invalid-key')} : {_quote_java('unexpected-error')});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    keys = ", ".join(str(c["key"]) for c in brute["expected"]["candidates"])
    texts = ", ".join(_quote_java(c["text"]) for c in brute["expected"]["candidates"])
    lines += [
        f"        // {brute['id']}",
        f"        var brute = ScytaleCipher.bruteForce({_quote_java(brute['input']['text'])});",
        f"        assertEquals(List.of({keys}), brute.stream().map(result -> result.key).toList());",
        f"        assertEquals(List.of({texts}), brute.stream().map(result -> result.text).toList());",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"        // {short['id']}",
        f"        assertTrue(ScytaleCipher.bruteForce({_quote_java(short['input']['text'])}).isEmpty());",
        f"        // {limit['id']}",
        f"        var limitError = assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.bruteForce({_quote_java(repeat_scalar)}.repeat({repeat_count})));",
        f"        assertEquals({_quote_java(limit['expected']['error_id'])}, limitError.getMessage());",
        "    }",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_kotlin(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "package com.codingadventures.scytalecipher\n",
        "import org.junit.jupiter.api.Test",
        "import org.junit.jupiter.api.assertThrows",
        "import kotlin.test.assertEquals\n",
        "class GeneratedClassicalCipherFixtureTest {",
        '    private fun scalars(vararg values: Int): String = values.joinToString("") { String(java.lang.Character.toChars(it)) }\n',
        "    @Test",
        "    fun matchesAllNormativeScytaleCases() {",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        assertEquals({_quote_kotlin(case['expected']['text'])}, ScytaleCipher.{fn}({_quote_kotlin(case['input']['text'])}, {case['input']['key']}))",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        val invalidError{index} = assertThrows<IllegalArgumentException> {{ ScytaleCipher.{fn}({_quote_kotlin(case['input']['text'])}, {case['input']['key']}) }}",
            f"        assertEquals({_quote_kotlin(case['expected']['error_id'])}, if (invalidError{index}.message?.let {{ it.startsWith({_quote_kotlin('key')}) && it.contains({_quote_kotlin('must')}) }} == true) {_quote_kotlin('scytale-invalid-key')} else {_quote_kotlin('unexpected-error')})",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"ScytaleCipher.BruteForceResult({c['key']}, {_quote_kotlin(c['text'])})"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"        // {brute['id']}",
        f"        assertEquals(listOf({expected}), ScytaleCipher.bruteForce({_quote_kotlin(brute['input']['text'])}))",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"        // {short['id']}",
        f"        assertEquals(emptyList(), ScytaleCipher.bruteForce({_quote_kotlin(short['input']['text'])}))",
        f"        // {limit['id']}",
        f"        val limitError = assertThrows<IllegalArgumentException> {{ ScytaleCipher.bruteForce({_quote_kotlin(repeat_scalar)}.repeat({repeat_count})) }}",
        f"        assertEquals({_quote_kotlin(limit['expected']['error_id'])}, limitError.message)",
        "    }",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_lua(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("--", digest),
        'package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path',
        'local scytale = require("coding_adventures.scytale_cipher")\n',
        'describe("generated classical-cipher Scytale fixtures", function()',
        '    it("matches every normative case", function()',
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        -- {case['id']}",
            f"        assert.equals({_quote_lua(case['expected']['text'])}, scytale.{fn}({_quote_lua(case['input']['text'])}, {case['input']['key']}))",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        -- {case['id']}",
            f"        local invalid_ok{index}, invalid_error{index} = pcall(function() scytale.{fn}({_quote_lua(case['input']['text'])}, {case['input']['key']}) end)",
            f"        local invalid_id{index} = not invalid_ok{index} and tostring(invalid_error{index}):find({_quote_lua('Key must be')}, 1, true) and {_quote_lua('scytale-invalid-key')} or {_quote_lua('unexpected-error')}",
            f"        assert.equals({_quote_lua(case['expected']['error_id'])}, invalid_id{index})",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    lines += [
        f"        -- {brute['id']}",
        f"        local brute = scytale.brute_force({_quote_lua(brute['input']['text'])})",
        f"        assert.equals({len(brute['expected']['candidates'])}, #brute)",
    ]
    for index, candidate in enumerate(brute["expected"]["candidates"], 1):
        lines += [
            f"        assert.same({{ key = {candidate['key']}, text = {_quote_lua(candidate['text'])} }}, brute[{index}])"
        ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"        -- {short['id']}",
        f"        assert.equals(0, #scytale.brute_force({_quote_lua(short['input']['text'])}))",
        f"        -- {limit['id']}",
        f"        assert.has_error(function() scytale.brute_force(string.rep({_quote_lua(repeat_scalar)}, {repeat_count})) end, {_quote_lua(limit['expected']['error_id'])})",
        "    end)",
        "end)",
    ]
    return "\n".join(lines) + "\n"


def render_perl(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("#", digest),
        "use Test2::V0;",
        "use utf8;",
        "use CodingAdventures::ScytaleCipher qw(encrypt decrypt brute_force);\n",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"# {case['id']}",
            f"is({fn}({_quote_perl(case['input']['text'])}, {case['input']['key']}), {_quote_perl(case['expected']['text'])}, {_quote_perl(case['id'])});",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"# {case['id']}",
            f"my $invalid_error_{index} = dies {{ {fn}({_quote_perl(case['input']['text'])}, {case['input']['key']}) }};",
            f"like($invalid_error_{index}, qr/Key must be/, {_quote_perl(case['id'])});",
            f"is($invalid_error_{index} =~ /Key must be/ ? {_quote_perl('scytale-invalid-key')} : {_quote_perl('unexpected-error')}, {_quote_perl(case['expected']['error_id'])}, {_quote_perl(case['id'] + '-error-id')});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{ key => {c['key']}, text => {_quote_perl(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"# {brute['id']}",
        f"is([brute_force({_quote_perl(brute['input']['text'])})], [{expected}], {_quote_perl(brute['id'])});",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"# {short['id']}",
        f"is([brute_force({_quote_perl(short['input']['text'])})], [], {_quote_perl(short['id'])});",
        f"# {limit['id']}",
        f"like(dies {{ brute_force({_quote_perl(repeat_scalar)} x {repeat_count}) }}, qr/{limit['expected']['error_id']}/, {_quote_perl(limit['id'])});",
        "done_testing;",
    ]
    return "\n".join(lines) + "\n"


def render_python(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("#", digest),
        "# ruff: noqa: E501, I001",
        "# fmt: off",
        '"""Generated native consumer for the normative Scytale fixture cases."""\n',
        "import pytest",
        "from scytale_cipher import brute_force, decrypt, encrypt\n",
        "def test_generated_classical_cipher_scytale_cases() -> None:",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    assert {fn}({_quote_python(case['input']['text'])}, {case['input']['key']}) == {_quote_python(case['expected']['text'])}",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    with pytest.raises(ValueError) as invalid_error{index}:",
            f"        {fn}({_quote_python(case['input']['text'])}, {case['input']['key']})",
            f"    assert ({_quote_python('scytale-invalid-key')} if str(invalid_error{index}.value).startswith({_quote_python('Key must')}) else {_quote_python('unexpected-error')}) == {_quote_python(case['expected']['error_id'])}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f'{{"key": {c["key"]}, "text": {_quote_python(c["text"])}}}'
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    # {brute['id']}",
        f"    assert brute_force({_quote_python(brute['input']['text'])}) == [{expected}]",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    # {short['id']}",
        f"    assert brute_force({_quote_python(short['input']['text'])}) == []",
        f"    # {limit['id']}",
        f"    with pytest.raises(ValueError, match={_quote_python(limit['expected']['error_id'])}):",
        f"        brute_force({_quote_python(repeat_scalar)} * {repeat_count})",
        "# fmt: on",
    ]
    return "\n".join(lines) + "\n"


def render_ruby(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("#", digest),
        "# frozen_string_literal: true\n",
        'require "minitest/autorun"',
        'require "coding_adventures_scytale_cipher"\n',
        "class TestGeneratedClassicalCipherFixture < Minitest::Test",
        "  def test_all_normative_scytale_cases",
        "    cipher = CodingAdventures::ScytaleCipher",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    assert_equal {_quote_ruby(case['expected']['text'])}, cipher.{fn}({_quote_ruby(case['input']['text'])}, {case['input']['key']})",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    invalid_error_{index} = assert_raises(ArgumentError) {{ cipher.{fn}({_quote_ruby(case['input']['text'])}, {case['input']['key']}) }}",
            f"    assert_equal {_quote_ruby(case['expected']['error_id'])}, invalid_error_{index}.message.start_with?({_quote_ruby('Key must')}) ? {_quote_ruby('scytale-invalid-key')} : {_quote_ruby('unexpected-error')}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{ key: {c['key']}, text: {_quote_ruby(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    # {brute['id']}",
        f"    assert_equal [{expected}], cipher.brute_force({_quote_ruby(brute['input']['text'])})",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    # {short['id']}",
        f"    assert_equal [], cipher.brute_force({_quote_ruby(short['input']['text'])})",
        f"    # {limit['id']}",
        f"    error = assert_raises(ArgumentError) {{ cipher.brute_force({_quote_ruby(repeat_scalar)} * {repeat_count}) }}",
        f"    assert_equal {_quote_ruby(limit['expected']['error_id'])}, error.message",
        "  end",
        "end",
    ]
    return "\n".join(lines) + "\n"


def render_rust(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "use scytale_cipher::{brute_force, decrypt, encrypt, BruteForceResult};\n",
        "#[test]",
        "fn generated_classical_cipher_scytale_cases() {",
    ]
    for index, case in enumerate(_text_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            *_rust_assignment(f"text_input_{index}", case["input"]["text"]),
            *_rust_assignment(f"text_expected_{index}", case["expected"]["text"]),
            f"    assert_eq!({fn}(text_input_{index}, {case['input']['key']}).unwrap(), text_expected_{index});",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    let invalid_id_{index} = match {fn}({_quote_rust(case['input']['text'])}, {case['input']['key']}) {{",
            f"        Err(error) if error.starts_with({_quote_rust('Key must be')}) => {_quote_rust('scytale-invalid-key')},",
            f"        _ => {_quote_rust('unexpected-error')},",
            "    };",
            f"    assert_eq!(invalid_id_{index}, {_quote_rust(case['expected']['error_id'])});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected: list[str] = []
    for index, candidate in enumerate(brute["expected"]["candidates"]):
        lines += _rust_assignment(f"brute_text_{index}", candidate["text"])
        expected += [
            "            BruteForceResult {",
            f"                key: {candidate['key']},",
            f"                text: brute_text_{index}.to_string()",
            "            },",
        ]
    lines += [
        f"    // {brute['id']}",
        *_rust_assignment("brute_input", brute["input"]["text"]),
        "    assert_eq!(",
        "        brute_force(brute_input).unwrap(),",
        "        vec![",
        *expected,
        "        ]",
        "    );",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    // {short['id']}",
        f"    assert!(brute_force({_quote_rust(short['input']['text'])}).unwrap().is_empty());",
        f"    // {limit['id']}",
        "    assert_eq!(",
        f"        brute_force(&{_quote_rust(repeat_scalar)}.repeat({repeat_count})),",
        f"        Err({_quote_rust(limit['expected']['error_id'])}.to_string())",
        "    );",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_swift(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "import XCTest",
        "@testable import ScytaleCipher\n",
        "final class GeneratedClassicalCipherFixtureTests: XCTestCase {",
        "    private func scalars(_ values: [UInt32]) -> String { String(values.compactMap(UnicodeScalar.init).map(Character.init)) }\n",
        "    func testAllNormativeScytaleCases() throws {",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        XCTAssertEqual(try ScytaleCipher.{fn}({_quote_swift(case['input']['text'])}, key: {case['input']['key']}), {_quote_swift(case['expected']['text'])})",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        XCTAssertThrowsError(try ScytaleCipher.{fn}({_quote_swift(case['input']['text'])}, key: {case['input']['key']})) {{ error in",
            "            let actualID: String",
            "            switch error {",
            f"            case ScytaleCipherError.keyTooSmall(_), ScytaleCipherError.keyTooLarge(_, textLength: _): actualID = {_quote_swift('scytale-invalid-key')}",
            f"            default: actualID = {_quote_swift('unexpected-error')}",
            "            }",
            f"            XCTAssertEqual(actualID, {_quote_swift(case['expected']['error_id'])})",
            "        }",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    keys = ", ".join(str(c["key"]) for c in brute["expected"]["candidates"])
    texts = ", ".join(_quote_swift(c["text"]) for c in brute["expected"]["candidates"])
    lines += [
        f"        // {brute['id']}",
        f"        let brute = try ScytaleCipher.bruteForce({_quote_swift(brute['input']['text'])})",
        f"        XCTAssertEqual(brute.map(\\.key), [{keys}])",
        f"        XCTAssertEqual(brute.map(\\.text), [{texts}])",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"        // {short['id']}",
        f"        XCTAssertTrue(try ScytaleCipher.bruteForce({_quote_swift(short['input']['text'])}).isEmpty)",
        f"        // {limit['id']}",
        f"        XCTAssertThrowsError(try ScytaleCipher.bruteForce(String(repeating: {_quote_swift(repeat_scalar)}, count: {repeat_count}))) {{ error in",
        "            guard case ScytaleCipherError.bruteForceLimit = error else {",
        f"                return XCTFail({_quote_swift('expected ' + limit['expected']['error_id'])})",
        "            }",
        f"            XCTAssertEqual({_quote_swift('scytale-brute-force-limit')}, {_quote_swift(limit['expected']['error_id'])})",
        "        }",
        "    }",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_typescript(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        'import { describe, expect, it } from "vitest";',
        'import { bruteForce, decrypt, encrypt } from "../src/index.js";\n',
        'describe("generated classical-cipher Scytale fixtures", () => {',
        '  it("matches every normative case", () => {',
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    expect({fn}({_quote_typescript(case['input']['text'])}, {case['input']['key']})).toBe({_quote_typescript(case['expected']['text'])});",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    let invalidID{index} = {_quote_typescript('unexpected-error')};",
            f"    try {{ {fn}({_quote_typescript(case['input']['text'])}, {case['input']['key']}); }} catch (error) {{",
            f"      if (error instanceof Error && error.message.startsWith({_quote_typescript('Key must be')})) invalidID{index} = {_quote_typescript('scytale-invalid-key')};",
            "    }",
            f"    expect(invalidID{index}).toBe({_quote_typescript(case['expected']['error_id'])});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{ key: {c['key']}, text: {_quote_typescript(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    // {brute['id']}",
        f"    expect(bruteForce({_quote_typescript(brute['input']['text'])})).toEqual([{expected}]);",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    repeat_scalar, repeat_count = _repeat_descriptor(limit)
    lines += [
        f"    // {short['id']}",
        f"    expect(bruteForce({_quote_typescript(short['input']['text'])})).toEqual([]);",
        f"    // {limit['id']}",
        f"    expect(() => bruteForce({_quote_typescript(repeat_scalar)}.repeat({repeat_count}))).toThrow({_quote_typescript(limit['expected']['error_id'])});",
        "  });",
        "});",
    ]
    return "\n".join(lines) + "\n"


RENDERERS: dict[str, Callable[[list[dict[str, Any]], str], str]] = {
    "csharp": render_csharp,
    "dart": render_dart,
    "elixir": render_elixir,
    "fsharp": render_fsharp,
    "go": render_go,
    "haskell": render_haskell,
    "java": render_java,
    "kotlin": render_kotlin,
    "lua": render_lua,
    "perl": render_perl,
    "python": render_python,
    "ruby": render_ruby,
    "rust": render_rust,
    "swift": render_swift,
    "typescript": render_typescript,
}


def render_all(cases: list[dict[str, Any]], digest: str) -> dict[Path, str]:
    if tuple(TARGETS) != IMPLEMENTATION_LANGUAGES or set(RENDERERS) != set(TARGETS):
        raise ValueError("established-language-roster-drift")
    return {
        TARGETS[language]: RENDERERS[language](cases, digest)
        for language in IMPLEMENTATION_LANGUAGES
    }


def check_outputs(outputs: dict[Path, str], root: Path = REPO_ROOT) -> list[str]:
    failures: list[str] = []
    for relative_path, expected in outputs.items():
        path = root / relative_path
        expected_bytes = expected.encode("utf-8")
        try:
            if path.stat().st_size != len(expected_bytes):
                failures.append(relative_path.as_posix())
                continue
            with path.open("rb") as stream:
                actual = stream.read(len(expected_bytes) + 1)
        except (FileNotFoundError, OSError):
            actual = None
        if actual != expected_bytes:
            failures.append(relative_path.as_posix())
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail if generated files are stale"
    )
    args = parser.parse_args()
    cases, digest = load_cases(FIXTURE_PATH)
    outputs = render_all(cases, digest)
    if args.check:
        failures = check_outputs(outputs)
        if failures:
            for failure in failures:
                print(
                    f"stale generated Scytale fixture consumer: {failure}",
                    file=sys.stderr,
                )
            return 1
        print(
            f"Scytale fixture consumers are current ({len(cases)} cases, {len(outputs)} lanes)."
        )
        return 0
    for relative_path, source in outputs.items():
        path = REPO_ROOT / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8", newline="\n")
    print(
        f"Generated {len(outputs)} Scytale fixture consumers from {len(cases)} cases."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
