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
        if not isinstance(case_id, str) or case_id in seen_ids:
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
    if "error_id" in expected and not isinstance(expected["error_id"], str):
        raise ValueError("fixture-invalid-case")


def load_cases(path: Path) -> tuple[list[dict[str, Any]], str]:
    raw = path.read_bytes()
    return load_cases_bytes(raw)


def _quote(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


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
        if case["expected"].get("error_id") == "scytale-invalid-key"
    )


def _brute_case(cases: Iterable[dict[str, Any]], suffix: str) -> dict[str, Any]:
    return next(case for case in cases if case["id"].endswith(suffix))


def render_csharp(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("//", digest),
        "namespace CodingAdventures.ScytaleCipher.Tests;\n",
        "public sealed class GeneratedClassicalCipherFixtureTests\n{",
        "    [Fact]",
        "    public void MatchesAllNormativeScytaleCases()\n    {",
    ]
    for case in _text_cases(cases):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"        // {case['id']}",
            f"        Assert.Equal({_quote(case['expected']['text'])}, ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}));",
        ]
    for case in _invalid_cases(cases):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"        // {case['id']}",
            f"        Assert.Throws<ArgumentOutOfRangeException>(() => ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}));",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    candidates = ", ".join(
        f"new BruteForceResult({c['key']}, {_quote(c['text'])})"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"        // {brute['id']}",
        f"        Assert.Equal(new[] {{ {candidates} }}, ScytaleCipher.BruteForce({_quote(brute['input']['text'])}));",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"        // {short['id']}",
        f"        Assert.Empty(ScytaleCipher.BruteForce({_quote(short['input']['text'])}));",
        f"        // {limit['id']}",
        "        Assert.Throws<ArgumentOutOfRangeException>(() => ScytaleCipher.BruteForce(new string('A', 4097)));",
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
            f"    expect({fn}({_quote(case['input']['text'])}, {case['input']['key']}), {_quote(case['expected']['text'])});",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    expect(() => {fn}({_quote(case['input']['text'])}, {case['input']['key']}), throwsArgumentError);",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = [
        f"      [{candidate['key']}, {_quote(candidate['text'])}]"
        for candidate in brute["expected"]["candidates"]
    ]
    lines += [
        f"    // {brute['id']}",
        f"    expect(bruteForce({_quote(brute['input']['text'])}).map((c) => [c.key, c.text]).toList(), [",
        *[f"{line}," for line in expected[:-1]],
        expected[-1],
        "    ]);",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    // {short['id']}",
        f"    expect(bruteForce({_quote(short['input']['text'])}), isEmpty);",
        f"    // {limit['id']}",
        "    expect(() => bruteForce(List.filled(4097, 'A').join()), throwsRangeError);",
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
            f"    assert ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}) == {_quote(case['expected']['text'])}",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    assert_raise ArgumentError, fn -> ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}) end",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"%{{key: {c['key']}, text: {_quote(c['text'])}}}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    # {brute['id']}",
        f"    assert ScytaleCipher.brute_force({_quote(brute['input']['text'])}) == [{expected}]",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    # {short['id']}",
        f"    assert ScytaleCipher.brute_force({_quote(short['input']['text'])}) == []",
        f"    # {limit['id']}",
        '    assert_raise ArgumentError, "scytale-brute-force-limit", fn ->',
        '      ScytaleCipher.brute_force(String.duplicate("A", 4097))',
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
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        Assert.Equal({_quote(case['expected']['text'])}, ScytaleCipher.{fn} {_quote(case['input']['text'])} {case['input']['key']})",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        Assert.Throws<ArgumentException>(fun () -> ScytaleCipher.{fn} {_quote(case['input']['text'])} {case['input']['key']} |> ignore) |> ignore",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = "; ".join(
        f"{{ Key = {c['key']}; Text = {_quote(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"        // {brute['id']}",
        f"        Assert.Equal<BruteForceResult list>([ {expected} ], ScytaleCipher.bruteForce {_quote(brute['input']['text'])})",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"        // {short['id']}",
        f"        Assert.Empty(ScytaleCipher.bruteForce {_quote(short['input']['text'])})",
        f"        // {limit['id']}",
        "        Assert.Throws<ArgumentException>(fun () -> ScytaleCipher.bruteForce (String('A', 4097)) |> ignore) |> ignore",
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
            f"\tgot{index}, err{index} := {fn}({_quote(case['input']['text'])}, {case['input']['key']})",
            f"\tif err{index} != nil || got{index} != {_quote(case['expected']['text'])} {{",
            f'\t\tt.Fatalf("{case["id"]}: got %q, %v", got{index}, err{index})',
            "\t}",
        ]
    for index, case in enumerate(_invalid_cases(cases)):
        fn = "Encrypt" if case["operation"].endswith("encrypt") else "Decrypt"
        lines += [
            f"\t// {case['id']}",
            f"\t_, invalidErr{index} := {fn}({_quote(case['input']['text'])}, {case['input']['key']})",
            f"\tif !errors.Is(invalidErr{index}, ErrInvalidKey) {{",
            f'\t\tt.Fatalf("{case["id"]}: %v", invalidErr{index})',
            "\t}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{Key: {c['key']}, Text: {_quote(c['text'])}}}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"\t// {brute['id']}",
        f"\tbrute, err := BruteForce({_quote(brute['input']['text'])})",
        f"\tif err != nil || !reflect.DeepEqual(brute, []BruteForceResult{{{expected}}}) {{",
        f'\t\tt.Fatalf("{brute["id"]}: %#v, %v", brute, err)',
        "\t}",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"\t// {short['id']}",
        f"\tshort, err := BruteForce({_quote(short['input']['text'])})",
        "\tif err != nil || len(short) != 0 {",
        f'\t\tt.Fatalf("{short["id"]}: %#v, %v", short, err)',
        "\t}",
        f"\t// {limit['id']}",
        '\t_, limitErr := BruteForce(strings.Repeat("A", 4097))',
        "\tif !errors.Is(limitErr, ErrBruteForceLimit) {",
        f'\t\tt.Fatalf("{limit["id"]}: %v", limitErr)',
        "\t}",
        "}",
    ]
    return "\n".join(lines) + "\n"


def render_haskell(cases: list[dict[str, Any]], digest: str) -> str:
    lines = [
        _header("--", digest),
        "module GeneratedClassicalCipherFixtureSpec (spec) where\n",
        "import ScytaleCipher",
        "import Test.Hspec\n",
        "spec :: Spec",
        'spec = describe "generated classical-cipher Scytale fixtures" $ do',
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    -- {case['id']}",
            f"    it {_quote(case['id'])} $ {fn} {_quote(case['input']['text'])} ({case['input']['key']}) `shouldBe` Right {_quote(case['expected']['text'])}",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    -- {case['id']}",
            f"    it {_quote(case['id'])} $ {fn} {_quote(case['input']['text'])} ({case['input']['key']}) `shouldSatisfy` either (const True) (const False)",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"BruteForceResult {c['key']} {_quote(c['text'])}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    -- {brute['id']}",
        f"    it {_quote(brute['id'])} $ bruteForce {_quote(brute['input']['text'])} `shouldBe` Right [{expected}]",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    -- {short['id']}",
        f"    it {_quote(short['id'])} $ bruteForce {_quote(short['input']['text'])} `shouldBe` Right []",
        f"    -- {limit['id']}",
        f"    it {_quote(limit['id'])} $ bruteForce (replicate 4097 'A') `shouldBe` Left \"scytale-brute-force-limit\"",
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
            f"        assertEquals({_quote(case['expected']['text'])}, ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}));",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}));",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    keys = ", ".join(str(c["key"]) for c in brute["expected"]["candidates"])
    texts = ", ".join(_quote(c["text"]) for c in brute["expected"]["candidates"])
    lines += [
        f"        // {brute['id']}",
        f"        var brute = ScytaleCipher.bruteForce({_quote(brute['input']['text'])});",
        f"        assertEquals(List.of({keys}), brute.stream().map(result -> result.key).toList());",
        f"        assertEquals(List.of({texts}), brute.stream().map(result -> result.text).toList());",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"        // {short['id']}",
        f"        assertTrue(ScytaleCipher.bruteForce({_quote(short['input']['text'])}).isEmpty());",
        f"        // {limit['id']}",
        '        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.bruteForce("A".repeat(4097)));',
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
        "    @Test",
        "    fun matchesAllNormativeScytaleCases() {",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        assertEquals({_quote(case['expected']['text'])}, ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}))",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        assertThrows<IllegalArgumentException> {{ ScytaleCipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}) }}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"ScytaleCipher.BruteForceResult({c['key']}, {_quote(c['text'])})"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"        // {brute['id']}",
        f"        assertEquals(listOf({expected}), ScytaleCipher.bruteForce({_quote(brute['input']['text'])}))",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"        // {short['id']}",
        f"        assertEquals(emptyList(), ScytaleCipher.bruteForce({_quote(short['input']['text'])}))",
        f"        // {limit['id']}",
        '        assertThrows<IllegalArgumentException> { ScytaleCipher.bruteForce("A".repeat(4097)) }',
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
            f"        assert.equals({_quote(case['expected']['text'])}, scytale.{fn}({_quote(case['input']['text'])}, {case['input']['key']}))",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        -- {case['id']}",
            f"        assert.has_error(function() scytale.{fn}({_quote(case['input']['text'])}, {case['input']['key']}) end)",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    lines += [
        f"        -- {brute['id']}",
        f"        local brute = scytale.brute_force({_quote(brute['input']['text'])})",
        f"        assert.equals({len(brute['expected']['candidates'])}, #brute)",
    ]
    for index, candidate in enumerate(brute["expected"]["candidates"], 1):
        lines += [
            f"        assert.same({{ key = {candidate['key']}, text = {_quote(candidate['text'])} }}, brute[{index}])"
        ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"        -- {short['id']}",
        f"        assert.equals(0, #scytale.brute_force({_quote(short['input']['text'])}))",
        f"        -- {limit['id']}",
        '        assert.has_error(function() scytale.brute_force(string.rep("A", 4097)) end, "scytale-brute-force-limit")',
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
            f"is({fn}({_quote(case['input']['text'])}, {case['input']['key']}), {_quote(case['expected']['text'])}, {_quote(case['id'])});",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"# {case['id']}",
            f"like(dies {{ {fn}({_quote(case['input']['text'])}, {case['input']['key']}) }}, qr/Key must be/, {_quote(case['id'])});",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{ key => {c['key']}, text => {_quote(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"# {brute['id']}",
        f"is([brute_force({_quote(brute['input']['text'])})], [{expected}], {_quote(brute['id'])});",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"# {short['id']}",
        f"is([brute_force({_quote(short['input']['text'])})], [], {_quote(short['id'])});",
        f"# {limit['id']}",
        f'like(dies {{ brute_force("A" x 4097) }}, qr/scytale-brute-force-limit/, {_quote(limit["id"])});',
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
            f"    assert {fn}({_quote(case['input']['text'])}, {case['input']['key']}) == {_quote(case['expected']['text'])}",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            "    with pytest.raises(ValueError):",
            f"        {fn}({_quote(case['input']['text'])}, {case['input']['key']})",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f'{{"key": {c["key"]}, "text": {_quote(c["text"])}}}'
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    # {brute['id']}",
        f"    assert brute_force({_quote(brute['input']['text'])}) == [{expected}]",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    # {short['id']}",
        f"    assert brute_force({_quote(short['input']['text'])}) == []",
        f"    # {limit['id']}",
        '    with pytest.raises(ValueError, match="scytale-brute-force-limit"):',
        '        brute_force("A" * 4097)',
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
            f"    assert_equal {_quote(case['expected']['text'])}, cipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']})",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    # {case['id']}",
            f"    assert_raises(ArgumentError) {{ cipher.{fn}({_quote(case['input']['text'])}, {case['input']['key']}) }}",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{ key: {c['key']}, text: {_quote(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    # {brute['id']}",
        f"    assert_equal [{expected}], cipher.brute_force({_quote(brute['input']['text'])})",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    # {short['id']}",
        f"    assert_equal [], cipher.brute_force({_quote(short['input']['text'])})",
        f"    # {limit['id']}",
        '    error = assert_raises(ArgumentError) { cipher.brute_force("A" * 4097) }',
        '    assert_equal "scytale-brute-force-limit", error.message',
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
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    assert_eq!({fn}({_quote(case['input']['text'])}, {case['input']['key']}).unwrap(), {_quote(case['expected']['text'])});",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    assert!({fn}({_quote(case['input']['text'])}, {case['input']['key']}).is_err());",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected: list[str] = []
    for candidate in brute["expected"]["candidates"]:
        expected += [
            "            BruteForceResult {",
            f"                key: {candidate['key']},",
            f"                text: {_quote(candidate['text'])}.to_string()",
            "            },",
        ]
    lines += [
        f"    // {brute['id']}",
        "    assert_eq!(",
        f"        brute_force({_quote(brute['input']['text'])}).unwrap(),",
        "        vec![",
        *expected,
        "        ]",
        "    );",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    // {short['id']}",
        f"    assert!(brute_force({_quote(short['input']['text'])}).unwrap().is_empty());",
        f"    // {limit['id']}",
        "    assert_eq!(",
        '        brute_force(&"A".repeat(4097)),',
        '        Err("scytale-brute-force-limit".to_string())',
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
        "    func testAllNormativeScytaleCases() throws {",
    ]
    for case in _text_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        XCTAssertEqual(try ScytaleCipher.{fn}({_quote(case['input']['text'])}, key: {case['input']['key']}), {_quote(case['expected']['text'])})",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"        // {case['id']}",
            f"        XCTAssertThrowsError(try ScytaleCipher.{fn}({_quote(case['input']['text'])}, key: {case['input']['key']}))",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    keys = ", ".join(str(c["key"]) for c in brute["expected"]["candidates"])
    texts = ", ".join(_quote(c["text"]) for c in brute["expected"]["candidates"])
    lines += [
        f"        // {brute['id']}",
        f"        let brute = try ScytaleCipher.bruteForce({_quote(brute['input']['text'])})",
        f"        XCTAssertEqual(brute.map(\\.key), [{keys}])",
        f"        XCTAssertEqual(brute.map(\\.text), [{texts}])",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"        // {short['id']}",
        f"        XCTAssertTrue(try ScytaleCipher.bruteForce({_quote(short['input']['text'])}).isEmpty)",
        f"        // {limit['id']}",
        '        XCTAssertThrowsError(try ScytaleCipher.bruteForce(String(repeating: "A", count: 4097)))',
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
            f"    expect({fn}({_quote(case['input']['text'])}, {case['input']['key']})).toBe({_quote(case['expected']['text'])});",
        ]
    for case in _invalid_cases(cases):
        fn = "encrypt" if case["operation"].endswith("encrypt") else "decrypt"
        lines += [
            f"    // {case['id']}",
            f"    expect(() => {fn}({_quote(case['input']['text'])}, {case['input']['key']})).toThrow();",
        ]
    brute = _brute_case(cases, "brute-force-ascending")
    expected = ", ".join(
        f"{{ key: {c['key']}, text: {_quote(c['text'])} }}"
        for c in brute["expected"]["candidates"]
    )
    lines += [
        f"    // {brute['id']}",
        f"    expect(bruteForce({_quote(brute['input']['text'])})).toEqual([{expected}]);",
    ]
    short = _brute_case(cases, "brute-force-short")
    limit = _brute_case(cases, "brute-force-preflight-limit")
    lines += [
        f"    // {short['id']}",
        f"    expect(bruteForce({_quote(short['input']['text'])})).toEqual([]);",
        f"    // {limit['id']}",
        '    expect(() => bruteForce("A".repeat(4097))).toThrow("scytale-brute-force-limit");',
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
        try:
            actual = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            actual = None
        if actual != expected:
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
