#!/usr/bin/env python3
"""Generate native Vigenere tests from the language-neutral cipher fixtures.

The strict JSON boundary lives in this generator.  Generated tests contain
ordinary, dependency-free source and exercise every complete normative
Vigenere expected object with the package's existing native test runner.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

import generate_scytale_fixture_consumers as shared
from package_parity_report import IMPLEMENTATION_LANGUAGES

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_PATH = REPO_ROOT / "code/specs/fixtures/classical-ciphers-v1/cases.json"
MAX_FIXTURE_BYTES = shared.MAX_FIXTURE_BYTES
MAX_FIXTURE_DEPTH = shared.MAX_FIXTURE_DEPTH
EXPECTED_LIMITS = shared.EXPECTED_LIMITS
IDENTIFIER_PATTERN = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
VIGENERE_OPERATIONS = {
    "vigenere-encrypt",
    "vigenere-decrypt",
    "vigenere-find-key-length",
    "vigenere-find-key",
    "vigenere-break",
}
EXPECTED_ANALYSIS = {
    "english_frequencies": [
        0.08167,
        0.01492,
        0.02782,
        0.04253,
        0.12702,
        0.02228,
        0.02015,
        0.06094,
        0.06966,
        0.00153,
        0.00772,
        0.04025,
        0.02406,
        0.06749,
        0.07507,
        0.01929,
        0.00095,
        0.05987,
        0.06327,
        0.09056,
        0.02758,
        0.00978,
        0.02360,
        0.00150,
        0.01974,
        0.00074,
    ],
    "ic_near_max_ratio": 0.9,
    "key_length_tie_break": "smallest-near-maximum",
    "chi_squared_tie_break": "smallest-shift",
    "insufficient_signal_key_length": 1,
    "empty_group_key_letter": "A",
    "shorten_repeating_key": False,
}
EXPECTED_OPERATION_ERROR_IDS = [
    "scytale-invalid-key",
    "scytale-brute-force-limit",
    "vigenere-invalid-key",
    "vigenere-analysis-limit",
    "vigenere-key-length-limit",
]
EXPECTED_VALIDATION_ERROR_IDS = [
    "fixture-depth-limit",
    "fixture-duplicate-id",
    "fixture-invalid-json",
    "fixture-invalid-scalar",
    "fixture-schema-invalid",
    "fixture-size-limit",
]
ALL_OPERATIONS = {
    "atbash-transform",
    "scytale-encrypt",
    "scytale-decrypt",
    "scytale-brute-force",
    *VIGENERE_OPERATIONS,
}
EXPECTED_VIGENERE_CASE_IDS = {
    "classical-ciphers-v1-vigenere-standard-encrypt",
    "classical-ciphers-v1-vigenere-standard-decrypt",
    "classical-ciphers-v1-vigenere-case-punctuation-encrypt",
    "classical-ciphers-v1-vigenere-case-punctuation-decrypt",
    "classical-ciphers-v1-vigenere-unicode-does-not-advance",
    "classical-ciphers-v1-vigenere-unicode-round-trip",
    "classical-ciphers-v1-vigenere-empty-key-before-empty-text",
    "classical-ciphers-v1-vigenere-nonascii-key",
    "classical-ciphers-v1-vigenere-insufficient-empty",
    "classical-ciphers-v1-vigenere-insufficient-two-letters",
    "classical-ciphers-v1-vigenere-insufficient-zero-score",
    "classical-ciphers-v1-vigenere-max-length-one",
    "classical-ciphers-v1-vigenere-smallest-ic-tie",
    "classical-ciphers-v1-vigenere-parameter-before-input-limit",
    "classical-ciphers-v1-vigenere-analysis-preflight-limit",
    "classical-ciphers-v1-vigenere-empty-groups-are-a",
    "classical-ciphers-v1-vigenere-smallest-chi-squared-tie",
    "classical-ciphers-v1-vigenere-nonpositive-key-length-before-limit",
    "classical-ciphers-v1-vigenere-find-key-analysis-limit",
    "classical-ciphers-v1-vigenere-find-key-length-limit",
    "classical-ciphers-v1-vigenere-long-find-length-twenty",
    "classical-ciphers-v1-vigenere-long-find-length-forty",
    "classical-ciphers-v1-vigenere-long-find-key",
    "classical-ciphers-v1-vigenere-no-repeating-key-shortening",
    "classical-ciphers-v1-vigenere-long-break",
    "classical-ciphers-v1-vigenere-empty-break",
}
MAX_TEXT_SCALARS = EXPECTED_LIMITS["max_fixture_text_scalars"]
MAX_VIGENERE_KEY_LENGTH = EXPECTED_LIMITS["max_vigenere_key_length"]

TARGETS = {
    "csharp": Path(
        "code/packages/csharp/vigenere-cipher/tests/CodingAdventures.VigenereCipher.Tests/GeneratedClassicalCipherFixtureTests.cs"
    ),
    "dart": Path(
        "code/packages/dart/vigenere-cipher/test/generated_classical_cipher_fixture_test.dart"
    ),
    "elixir": Path(
        "code/packages/elixir/vigenere_cipher/test/generated_classical_cipher_fixture_test.exs"
    ),
    "fsharp": Path(
        "code/packages/fsharp/vigenere-cipher/tests/CodingAdventures.VigenereCipher.Tests/GeneratedClassicalCipherFixtureTests.fs"
    ),
    "go": Path(
        "code/packages/go/vigenere-cipher/generated_classical_cipher_fixture_test.go"
    ),
    "haskell": Path(
        "code/packages/haskell/vigenere-cipher/test/GeneratedClassicalCipherFixtureSpec.hs"
    ),
    "java": Path(
        "code/packages/java/vigenere-cipher/src/test/java/com/codingadventures/vigenerecipher/GeneratedClassicalCipherFixtureTest.java"
    ),
    "kotlin": Path(
        "code/packages/kotlin/vigenere-cipher/src/test/kotlin/com/codingadventures/vigenerecipher/GeneratedClassicalCipherFixtureTest.kt"
    ),
    "lua": Path(
        "code/packages/lua/vigenere_cipher/tests/test_generated_classical_cipher_fixture.lua"
    ),
    "perl": Path(
        "code/packages/perl/vigenere-cipher/t/02-generated-classical-cipher-fixture.t"
    ),
    "python": Path(
        "code/packages/python/vigenere-cipher/tests/test_generated_classical_cipher_fixture.py"
    ),
    "ruby": Path(
        "code/packages/ruby/vigenere_cipher/test/test_generated_classical_cipher_fixture.rb"
    ),
    "rust": Path(
        "code/packages/rust/vigenere-cipher/tests/generated_classical_cipher_fixture.rs"
    ),
    "swift": Path(
        "code/packages/swift/vigenere-cipher/Tests/VigenereCipherTests/GeneratedClassicalCipherFixtureTests.swift"
    ),
    "typescript": Path(
        "code/packages/typescript/vigenere-cipher/tests/generated-classical-cipher-fixture.test.ts"
    ),
}


def load_cases_bytes(raw: bytes) -> tuple[list[dict[str, Any]], str]:
    """Strictly load, validate, and select all normative Vigenere cases."""
    if len(raw) > MAX_FIXTURE_BYTES:
        raise ValueError("fixture-size-limit")
    shared._check_raw_depth(raw)
    try:
        document = json.loads(
            raw.decode("utf-8", errors="strict"),
            object_pairs_hook=shared._reject_duplicate_names,
            parse_constant=shared._reject_nonfinite,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("fixture-invalid-json") from error
    shared._check_scalars(document)
    if not isinstance(document, dict):
        raise ValueError("fixture-invalid-profile")  # noqa: TRY004
    if (
        document.get("schema_version") != 1
        or document.get("profile") != "cr01-cr03-portable-v1"
    ):
        raise ValueError("fixture-invalid-profile")
    if document.get("limits") != EXPECTED_LIMITS:
        raise ValueError("fixture-invalid-profile")
    if document.get("analysis") != EXPECTED_ANALYSIS:
        raise ValueError("fixture-invalid-profile")
    if document.get("operation_error_ids") != EXPECTED_OPERATION_ERROR_IDS:
        raise ValueError("fixture-invalid-profile")
    if document.get("validation_error_ids") != EXPECTED_VALIDATION_ERROR_IDS:
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
        if not isinstance(operation, str) or operation not in ALL_OPERATIONS:
            raise ValueError("fixture-invalid-case")
        if operation not in VIGENERE_OPERATIONS:
            continue
        if case_id not in EXPECTED_VIGENERE_CASE_IDS:
            raise ValueError("fixture-invalid-case")
        _validate_case(case)
        selected.append(case)
    if {case["id"] for case in selected} != EXPECTED_VIGENERE_CASE_IDS or {
        case["operation"] for case in selected
    } != VIGENERE_OPERATIONS:
        raise ValueError("fixture-invalid-vigenere-roster")
    return selected, hashlib.sha256(raw).hexdigest()


def _validate_case(case: dict[str, Any]) -> None:
    operation = case["operation"]
    input_value = case["input"]
    expected = case["expected"]
    if not isinstance(input_value, dict) or not isinstance(expected, dict):
        raise ValueError("fixture-invalid-case")  # noqa: TRY004
    if operation in {"vigenere-encrypt", "vigenere-decrypt"}:
        if set(input_value) != {"text", "key"} or not all(
            isinstance(input_value[name], str) for name in ("text", "key")
        ):
            raise ValueError("fixture-invalid-case")
        if (
            len(input_value["text"]) > MAX_TEXT_SCALARS
            or len(input_value["key"]) > MAX_VIGENERE_KEY_LENGTH + 1
        ):
            raise ValueError("fixture-invalid-case")
        allowed_expected = ({"text"}, {"error_id"})
    elif operation == "vigenere-find-key-length":
        _validate_analysis_input(input_value, "max_length")
        allowed_expected = ({"key_length"}, {"error_id"})
    elif operation == "vigenere-find-key":
        _validate_analysis_input(input_value, "key_length")
        allowed_expected = ({"key"}, {"error_id"})
    else:
        if set(input_value) != {"ciphertext"} or not isinstance(
            input_value["ciphertext"], str
        ):
            raise ValueError("fixture-invalid-case")
        if len(input_value["ciphertext"]) > MAX_TEXT_SCALARS:
            raise ValueError("fixture-invalid-case")
        allowed_expected = ({"key", "plaintext"}, {"error_id"})
    if set(expected) not in allowed_expected:
        raise ValueError("fixture-invalid-case")
    for name in ("text", "key", "plaintext", "error_id"):
        if name in expected and not isinstance(expected[name], str):
            raise ValueError("fixture-invalid-case")
    if any(
        len(expected[name]) > MAX_TEXT_SCALARS
        for name in ("text", "plaintext")
        if name in expected
    ):
        raise ValueError("fixture-invalid-case")
    if "key_length" in expected and (
        type(expected["key_length"]) is not int
        or not 1 <= expected["key_length"] <= MAX_VIGENERE_KEY_LENGTH
    ):
        raise ValueError("fixture-invalid-case")
    if (
        "error_id" in expected
        and expected["error_id"] not in EXPECTED_OPERATION_ERROR_IDS
    ):
        raise ValueError("fixture-invalid-case")
    if "key" in expected:
        key = expected["key"]
        minimum = 1 if operation == "vigenere-break" else 0
        if (
            not minimum <= len(key) <= MAX_VIGENERE_KEY_LENGTH
            or re.fullmatch(r"[A-Z]*", key) is None
        ):
            raise ValueError("fixture-invalid-case")


def _validate_analysis_input(value: dict[str, Any], parameter: str) -> None:
    direct = {"ciphertext", parameter}
    repeated = {"repeat_scalar", "repeat_count", parameter}
    if set(value) not in (direct, repeated):
        raise ValueError("fixture-invalid-case")
    if type(value[parameter]) is not int or not -1 <= value[parameter] <= 41:
        raise ValueError("fixture-invalid-case")
    if "ciphertext" in value and (
        not isinstance(value["ciphertext"], str)
        or len(value["ciphertext"]) > MAX_TEXT_SCALARS
    ):
        raise ValueError("fixture-invalid-case")
    if "repeat_scalar" in value and (
        not isinstance(value["repeat_scalar"], str)
        or len(value["repeat_scalar"]) != 1
        or type(value["repeat_count"]) is not int
        or value["repeat_count"] < 0
        or value["repeat_count"] > MAX_TEXT_SCALARS
    ):
        raise ValueError("fixture-invalid-case")


def load_cases(path: Path) -> tuple[list[dict[str, Any]], str]:
    with path.open("rb") as stream:
        raw = stream.read(MAX_FIXTURE_BYTES + 1)
    return load_cases_bytes(raw)


def _header(prefix: str, digest: str) -> str:
    return (
        f"{prefix} GENERATED by code/scripts/generate_vigenere_fixture_consumers.py.\n"
        f"{prefix} Source SHA-256: {digest}\n"
        f"{prefix} Do not edit by hand.\n"
    )


def _method_id(case: dict[str, Any]) -> str:
    return case["id"].removeprefix("classical-ciphers-v1-vigenere-").replace("-", "_")


def _pascal_id(case: dict[str, Any]) -> str:
    return "".join(part.capitalize() for part in _method_id(case).split("_"))


def _quote_lua(value: str) -> str:
    values = [ord(character) for character in value]
    if not values:
        return '""'
    chunks = [values[index : index + 100] for index in range(0, len(values), 100)]
    return " .. ".join(f"utf8.char({', '.join(map(str, chunk))})" for chunk in chunks)


def _text_expr(
    case: dict[str, Any], quote: Callable[[str], str], repeat: Callable[[str, int], str]
) -> str:
    input_value = case["input"]
    if "ciphertext" in input_value:
        return quote(input_value["ciphertext"])
    if "text" in input_value:
        return quote(input_value["text"])
    return repeat(quote(input_value["repeat_scalar"]), input_value["repeat_count"])


def _ends(lines: list[str]) -> str:
    return "\n".join(lines) + "\n"


def render_csharp(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_csharp
    lines = [
        _header("//", digest),
        "using System.Text;",
        "",
        "namespace CodingAdventures.VigenereCipher.Tests;",
        "",
        "public sealed class GeneratedClassicalCipherFixtureTests",
        "{",
        "    private static string Scalars(params int[] values)",
        "    {",
        "        var result = new StringBuilder();",
        "        foreach (var value in values) result.Append(char.ConvertFromUtf32(value));",
        "        return result.ToString();",
        "    }",
        "",
        "    private static string Repeat(string scalar, int count)",
        "    {",
        "        var result = new StringBuilder(scalar.Length * count);",
        "        for (var index = 0; index < count; index++) result.Append(scalar);",
        "        return result.ToString();",
        "    }",
        "",
        "    private static string ErrorId(Exception error)",
        "    {",
        '        if (error is ArgumentOutOfRangeException range && (range.ParamName == "keyLength" || range.ParamName == "maxLength"))',
        '            return "vigenere-key-length-limit";',
        "        var message = error.Message.ToLowerInvariant();",
        '        if (message.Contains("key length")) return "vigenere-key-length-limit";',
        '        if (message.Contains("analysis limit")) return "vigenere-analysis-limit";',
        '        return "vigenere-invalid-key";',
        "    }",
    ]
    for case in cases:
        operation = case["operation"]
        value = _text_expr(case, q, lambda scalar, count: f"Repeat({scalar}, {count})")
        expected = case["expected"]
        lines += ["", "    [Fact]", f"    public void {_pascal_id(case)}()", "    {"]
        if operation in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"VigenereCipher.{operation.removeprefix('vigenere-').capitalize()}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"        Assert.Equal({q(expected['text'])}, {call});")
            else:
                lines.append(
                    f"        var error = Assert.ThrowsAny<ArgumentException>(() => {call});"
                )
                lines.append(
                    f"        Assert.Equal({q(expected['error_id'])}, ErrorId(error));"
                )
        elif operation == "vigenere-find-key-length":
            call = (
                f"VigenereCipher.FindKeyLength({value}, {case['input']['max_length']})"
            )
            if "key_length" in expected:
                lines.append(f"        Assert.Equal({expected['key_length']}, {call});")
            else:
                lines.append(
                    f"        var error = Assert.ThrowsAny<ArgumentException>(() => {call});"
                )
                lines.append(
                    f"        Assert.Equal({q(expected['error_id'])}, ErrorId(error));"
                )
        elif operation == "vigenere-find-key":
            call = f"VigenereCipher.FindKey({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"        Assert.Equal({q(expected['key'])}, {call});")
            else:
                lines.append(
                    f"        var error = Assert.ThrowsAny<ArgumentException>(() => {call});"
                )
                lines.append(
                    f"        Assert.Equal({q(expected['error_id'])}, ErrorId(error));"
                )
        else:
            lines += [
                f"        var result = VigenereCipher.BreakCipher({value});",
                f"        Assert.Equal({q(expected['key'])}, result.Key);",
                f"        Assert.Equal({q(expected['plaintext'])}, result.Plaintext);",
            ]
        lines.append("    }")
    lines.append("}")
    return _ends(lines)


def render_dart(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_dart
    lines = [
        _header("//", digest),
        "import 'package:coding_adventures_vigenere_cipher/vigenere_cipher.dart';",
        "import 'package:test/test.dart';",
        "",
        "String errorId(Object error) {",
        "  final message = error.toString().toLowerCase();",
        "  if (message.contains('key length') ||",
        "      message.contains('keylength') ||",
        "      message.contains('maxlength')) return 'vigenere-key-length-limit';",
        "  if (message.contains('analysis limit')) return 'vigenere-analysis-limit';",
        "  return 'vigenere-invalid-key';",
        "}",
        "",
        "void main() {",
    ]
    for case in cases:
        value = _text_expr(
            case, q, lambda scalar, count: f"List.filled({count}, {scalar}).join()"
        )
        expected = case["expected"]
        operation = case["operation"]
        lines += [f"  test({q(case['id'])}, () {{"]
        if operation in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"{operation.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"    expect({call}, {q(expected['text'])});")
            else:
                lines += [
                    "    Object? caught;",
                    "    try {",
                    f"      {call};",
                    "    } catch (error) {",
                    "      caught = error;",
                    "    }",
                    "    expect(caught, isNotNull);",
                    f"    expect(errorId(caught!), {q(expected['error_id'])});",
                ]
        elif operation == "vigenere-find-key-length":
            call = f"findKeyLength({value}, maxLength: {case['input']['max_length']})"
            if "key_length" in expected:
                if len(value) > 200:
                    lines += [
                        "    expect(",
                        "        findKeyLength(",
                        f"            {value},",
                        f"            maxLength: {case['input']['max_length']}),",
                        f"        {expected['key_length']});",
                    ]
                else:
                    lines.append(f"    expect({call}, {expected['key_length']});")
            else:
                lines += [
                    "    Object? caught;",
                    "    try {",
                    f"      {call};",
                    "    } catch (error) {",
                    "      caught = error;",
                    "    }",
                    "    expect(caught, isNotNull);",
                    f"    expect(errorId(caught!), {q(expected['error_id'])});",
                ]
        elif operation == "vigenere-find-key":
            call = f"findKey({value}, {case['input']['key_length']})"
            if "key" in expected:
                if len(value) > 200:
                    lines += [
                        "    expect(",
                        "        findKey(",
                        f"            {value},",
                        f"            {case['input']['key_length']}),",
                        f"        {q(expected['key'])});",
                    ]
                else:
                    lines.append(f"    expect({call}, {q(expected['key'])});")
            else:
                lines += [
                    "    Object? caught;",
                    "    try {",
                    f"      {call};",
                    "    } catch (error) {",
                    "      caught = error;",
                    "    }",
                    "    expect(caught, isNotNull);",
                    f"    expect(errorId(caught!), {q(expected['error_id'])});",
                ]
        else:
            if len(value) > 200:
                lines += ["    final result = breakCipher(", f"        {value});"]
            else:
                lines.append(f"    final result = breakCipher({value});")
            lines.append(f"    expect(result.key, {q(expected['key'])});")
            plaintext = q(expected["plaintext"])
            if len(plaintext) > 80:
                lines += ["    expect(result.plaintext,", f"        {plaintext});"]
            else:
                lines.append(f"    expect(result.plaintext, {plaintext});")
        lines.append("  });")
    lines.append("}")
    return _ends(lines)


def render_elixir(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_elixir
    lines = [
        _header("#", digest),
        "defmodule CodingAdventures.GeneratedVigenereFixtureTest do",
        "  use ExUnit.Case",
        "  alias CodingAdventures.VigenereCipher",
        "",
        "  defp error_id(error) do",
        "    message = error |> Exception.message() |> String.downcase()",
        "    cond do",
        '      String.contains?(message, "key length") -> "vigenere-key-length-limit"',
        '      String.contains?(message, "analysis limit") -> "vigenere-analysis-limit"',
        '      true -> "vigenere-invalid-key"',
        "    end",
        "  end",
    ]
    for case in cases:
        value = _text_expr(
            case, q, lambda scalar, count: f"String.duplicate({scalar}, {count})"
        )
        expected = case["expected"]
        op = case["operation"]
        lines += ["", f"  test {q(case['id'])} do"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            name = op.removeprefix("vigenere-")
            call = f"VigenereCipher.{name}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"    assert {call} == {q(expected['text'])}")
            else:
                lines += [
                    f"    error = assert_raise ArgumentError, fn -> {call} end",
                    f"    assert error_id(error) == {q(expected['error_id'])}",
                ]
        elif op == "vigenere-find-key-length":
            call = f"VigenereCipher.find_key_length({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"    assert {call} == {expected['key_length']}")
            else:
                lines += [
                    f"    error = assert_raise ArgumentError, fn -> {call} end",
                    f"    assert error_id(error) == {q(expected['error_id'])}",
                ]
        elif op == "vigenere-find-key":
            call = f"VigenereCipher.find_key({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"    assert {call} == {q(expected['key'])}")
            else:
                lines += [
                    f"    error = assert_raise ArgumentError, fn -> {call} end",
                    f"    assert error_id(error) == {q(expected['error_id'])}",
                ]
        else:
            lines += [
                f"    result = VigenereCipher.break_cipher({value})",
                f"    assert result.key == {q(expected['key'])}",
                f"    assert result.plaintext == {q(expected['plaintext'])}",
            ]
        lines.append("  end")
    lines.append("end")
    return _ends(lines)


def render_fsharp(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_fsharp
    lines = [
        _header("//", digest),
        "namespace CodingAdventures.VigenereCipher.Tests",
        "",
        "open System",
        "open Xunit",
        "open CodingAdventures.VigenereCipher",
        "",
        "type GeneratedClassicalCipherFixtureTests() =",
        '    let scalars values = values |> List.map Char.ConvertFromUtf32 |> String.concat ""',
        "    let errorId (error: exn) =",
        "        let message = error.Message.ToLowerInvariant()",
        '        if message.Contains("key length") then "vigenere-key-length-limit"',
        '        elif message.Contains("analysis limit") then "vigenere-analysis-limit"',
        '        else "vigenere-invalid-key"',
    ]
    for case in cases:
        value = _text_expr(
            case, q, lambda scalar, count: f"(String.replicate {count} {scalar})"
        )
        expected = case["expected"]
        op = case["operation"]
        lines += ["", "    [<Fact>]", f"    member _.``{case['id']}``() ="]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"VigenereCipher.{op.removeprefix('vigenere-')} {value} {q(case['input']['key'])}"
            if "text" in expected:
                lines.append(f"        Assert.Equal({q(expected['text'])}, {call})")
            else:
                lines += [
                    f"        let error = Assert.ThrowsAny<ArgumentException>(fun () -> {call} |> ignore)",
                    f"        Assert.Equal({q(expected['error_id'])}, errorId error)",
                ]
        elif op == "vigenere-find-key-length":
            call = f"VigenereCipher.findKeyLength {value} {case['input']['max_length']}"
            if "key_length" in expected:
                lines.append(f"        Assert.Equal({expected['key_length']}, {call})")
            else:
                lines += [
                    f"        let error = Assert.ThrowsAny<ArgumentException>(fun () -> {call} |> ignore)",
                    f"        Assert.Equal({q(expected['error_id'])}, errorId error)",
                ]
        elif op == "vigenere-find-key":
            call = f"VigenereCipher.findKey {value} {case['input']['key_length']}"
            if "key" in expected:
                lines.append(f"        Assert.Equal({q(expected['key'])}, {call})")
            else:
                lines += [
                    f"        let error = Assert.ThrowsAny<ArgumentException>(fun () -> {call} |> ignore)",
                    f"        Assert.Equal({q(expected['error_id'])}, errorId error)",
                ]
        else:
            lines += [
                f"        let result = VigenereCipher.breakCipher {value}",
                f"        Assert.Equal({q(expected['key'])}, result.Key)",
                f"        Assert.Equal({q(expected['plaintext'])}, result.Plaintext)",
            ]
    return _ends(lines)


def render_go(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_go
    lines = [
        _header("//", digest),
        "package vigenerecipher",
        "",
        "import (",
        '\t"fmt"',
        '\t"strings"',
        '\t"testing"',
        ")",
        "",
        "func generatedErrorID(message string) string {",
        "\tmessage = strings.ToLower(message)",
        '\tif strings.Contains(message, "key length") {',
        '\t\treturn "vigenere-key-length-limit"',
        "\t}",
        '\tif strings.Contains(message, "analysis limit") {',
        '\t\treturn "vigenere-analysis-limit"',
        "\t}",
        '\treturn "vigenere-invalid-key"',
        "}",
        "",
        "func generatedPanicID(action func()) (result string) {",
        "\tdefer func() {",
        "\t\tif value := recover(); value != nil {",
        "\t\t\tresult = generatedErrorID(fmt.Sprint(value))",
        "\t\t}",
        "\t}()",
        "\taction()",
        '\treturn ""',
        "}",
    ]
    for case in cases:
        value = _text_expr(
            case, q, lambda scalar, count: f"strings.Repeat({scalar}, {count})"
        )
        expected = case["expected"]
        op = case["operation"]
        lines += ["", f"func TestGenerated{_pascal_id(case)}(t *testing.T) {{"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            name = op.removeprefix("vigenere-").capitalize()
            if "text" in expected:
                lines.append(
                    f"\tgot, err := {name}({value}, {q(case['input']['key'])})"
                )
                lines += [
                    "\tif err != nil {",
                    '\t\tt.Fatalf("unexpected error: %v", err)',
                    "\t}",
                    f"\tif got != {q(expected['text'])} {{",
                    f'\t\tt.Fatalf("{case["id"]}: got %q", got)',
                    "\t}",
                ]
            else:
                lines.append(f"\t_, err := {name}({value}, {q(case['input']['key'])})")
                lines += [
                    "\tif err == nil {",
                    '\t\tt.Fatal("expected fixture error")',
                    "\t}",
                    f"\tif gotID := generatedErrorID(err.Error()); gotID != {q(expected['error_id'])} {{",
                    '\t\tt.Fatalf("error id = %q", gotID)',
                    "\t}",
                ]
        elif op == "vigenere-find-key-length":
            call = f"FindKeyLength({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines += [
                    f"\tif got := {call}; got != {expected['key_length']} {{",
                    '\t\tt.Fatalf("key length = %d", got)',
                    "\t}",
                ]
            else:
                lines += [
                    f"\tif gotID := generatedPanicID(func() {{ {call} }}); gotID != {q(expected['error_id'])} {{",
                    '\t\tt.Fatalf("error id = %q", gotID)',
                    "\t}",
                ]
        elif op == "vigenere-find-key":
            call = f"FindKey({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines += [
                    f"\tif got := {call}; got != {q(expected['key'])} {{",
                    '\t\tt.Fatalf("key = %q", got)',
                    "\t}",
                ]
            else:
                lines += [
                    f"\tif gotID := generatedPanicID(func() {{ {call} }}); gotID != {q(expected['error_id'])} {{",
                    '\t\tt.Fatalf("error id = %q", gotID)',
                    "\t}",
                ]
        else:
            lines += [
                f"\tkey, plaintext, err := BreakCipher({value})",
                "\tif err != nil {",
                '\t\tt.Fatalf("unexpected error: %v", err)',
                "\t}",
                f"\tif key != {q(expected['key'])} {{",
                '\t\tt.Fatalf("key = %q", key)',
                "\t}",
                f"\tif plaintext != {q(expected['plaintext'])} {{",
                '\t\tt.Fatalf("plaintext = %q", plaintext)',
                "\t}",
            ]
        lines.append("}")
    return _ends(lines)


def render_haskell(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_haskell
    lines = [
        "{-# LANGUAGE ScopedTypeVariables #-}",
        _header("--", digest),
        "module GeneratedClassicalCipherFixtureSpec (spec) where",
        "",
        "import Control.Exception (SomeException, evaluate, try)",
        "import Data.Char (chr, toLower)",
        "import Data.List (isInfixOf)",
        "import Test.Hspec",
        "import VigenereCipher",
        "",
        "errorId :: String -> String",
        "errorId message",
        '    | "key length" `isInfixOf` lowered = "vigenere-key-length-limit"',
        '    | "analysis limit" `isInfixOf` lowered = "vigenere-analysis-limit"',
        '    | otherwise = "vigenere-invalid-key"',
        "  where lowered = map toLower message",
        "",
        "expectError :: String -> IO a -> Expectation",
        "expectError expected action = do",
        "    result <- try action",
        "    case result of",
        "        Left (caught :: SomeException) -> errorId (show caught) `shouldBe` expected",
        '        Right _ -> expectationFailure "expected fixture error"',
        "",
        "expectLeft :: String -> Either String a -> Expectation",
        "expectLeft expected result = case result of",
        "    Left message -> errorId message `shouldBe` expected",
        '    Right _ -> expectationFailure "expected fixture error"',
        "",
        "spec :: Spec",
        'spec = describe "generated classical-ciphers-v1 Vigenere cases" $ do',
    ]
    for case in cases:
        value = _text_expr(
            case, q, lambda scalar, count: f"(concat (replicate {count} {scalar}))"
        )
        expected = case["expected"]
        op = case["operation"]
        lines.append(f"    it {q(case['id'])} $ do")
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"{op.removeprefix('vigenere-')} {value} {q(case['input']['key'])}"
            if "text" in expected:
                lines.append(f"        {call} `shouldBe` Right {q(expected['text'])}")
            else:
                lines.append(f"        expectLeft {q(expected['error_id'])} ({call})")
        elif op == "vigenere-find-key-length":
            call = f"findKeyLengthWithLimit {value} {case['input']['max_length']}"
            if "key_length" in expected:
                lines.append(f"        {call} `shouldBe` {expected['key_length']}")
            else:
                lines.append(
                    f"        expectError {q(expected['error_id'])} (evaluate ({call}))"
                )
        elif op == "vigenere-find-key":
            call = f"findKey {value} {case['input']['key_length']}"
            if "key" in expected:
                lines.append(f"        {call} `shouldBe` {q(expected['key'])}")
            else:
                lines.append(
                    f"        expectError {q(expected['error_id'])} (evaluate (length ({call})))"
                )
        else:
            lines += [
                f"        let result = breakCipher {value}",
                f"        recoveredKey result `shouldBe` {q(expected['key'])}",
                f"        recoveredPlaintext result `shouldBe` {q(expected['plaintext'])}",
            ]
    return _ends(lines)


def render_java(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_java
    lines = [
        _header("//", digest),
        "package com.codingadventures.vigenerecipher;",
        "",
        "import org.junit.jupiter.api.Test;",
        "import static org.junit.jupiter.api.Assertions.*;",
        "",
        "class GeneratedClassicalCipherFixtureTest {",
        "    private static String errorId(Exception error) {",
        "        String message = error.getMessage().toLowerCase();",
        '        if (message.contains("key length")) return "vigenere-key-length-limit";',
        '        if (message.contains("analysis limit")) return "vigenere-analysis-limit";',
        '        return "vigenere-invalid-key";',
        "    }",
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"{scalar}.repeat({count})")
        expected = case["expected"]
        op = case["operation"]
        lines += ["", "    @Test", f"    void {_method_id(case)}() {{"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"VigenereCipher.{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"        assertEquals({q(expected['text'])}, {call});")
            else:
                lines += [
                    f"        Exception error = assertThrows(IllegalArgumentException.class, () -> {call});",
                    f"        assertEquals({q(expected['error_id'])}, errorId(error));",
                ]
        elif op == "vigenere-find-key-length":
            call = (
                f"VigenereCipher.findKeyLength({value}, {case['input']['max_length']})"
            )
            if "key_length" in expected:
                lines.append(f"        assertEquals({expected['key_length']}, {call});")
            else:
                lines += [
                    f"        Exception error = assertThrows(IllegalArgumentException.class, () -> {call});",
                    f"        assertEquals({q(expected['error_id'])}, errorId(error));",
                ]
        elif op == "vigenere-find-key":
            call = f"VigenereCipher.findKey({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"        assertEquals({q(expected['key'])}, {call});")
            else:
                lines += [
                    f"        Exception error = assertThrows(IllegalArgumentException.class, () -> {call});",
                    f"        assertEquals({q(expected['error_id'])}, errorId(error));",
                ]
        else:
            lines += [
                f"        VigenereCipher.BreakResult result = VigenereCipher.breakCipher({value});",
                f"        assertEquals({q(expected['key'])}, result.key);",
                f"        assertEquals({q(expected['plaintext'])}, result.plaintext);",
            ]
        lines.append("    }")
    lines.append("}")
    return _ends(lines)


def render_kotlin(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_kotlin
    lines = [
        _header("//", digest),
        "package com.codingadventures.vigenerecipher",
        "",
        "import org.junit.jupiter.api.Test",
        "import kotlin.test.assertEquals",
        "import kotlin.test.assertFailsWith",
        "",
        "class GeneratedClassicalCipherFixtureTest {",
        "    private fun scalars(vararg values: Int): String = buildString { values.forEach { appendCodePoint(it) } }",
        "    private fun errorId(error: Exception): String {",
        "        val message = error.message.orEmpty().lowercase()",
        '        if (message.contains("key length")) return "vigenere-key-length-limit"',
        '        if (message.contains("analysis limit")) return "vigenere-analysis-limit"',
        '        return "vigenere-invalid-key"',
        "    }",
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"{scalar}.repeat({count})")
        expected = case["expected"]
        op = case["operation"]
        lines += ["", "    @Test", f"    fun {_method_id(case)}() {{"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"VigenereCipher.{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"        assertEquals({q(expected['text'])}, {call})")
            else:
                lines += [
                    f"        val error = assertFailsWith<IllegalArgumentException> {{ {call} }}",
                    f"        assertEquals({q(expected['error_id'])}, errorId(error))",
                ]
        elif op == "vigenere-find-key-length":
            call = (
                f"VigenereCipher.findKeyLength({value}, {case['input']['max_length']})"
            )
            if "key_length" in expected:
                lines.append(f"        assertEquals({expected['key_length']}, {call})")
            else:
                lines += [
                    f"        val error = assertFailsWith<IllegalArgumentException> {{ {call} }}",
                    f"        assertEquals({q(expected['error_id'])}, errorId(error))",
                ]
        elif op == "vigenere-find-key":
            call = f"VigenereCipher.findKey({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"        assertEquals({q(expected['key'])}, {call})")
            else:
                lines += [
                    f"        val error = assertFailsWith<IllegalArgumentException> {{ {call} }}",
                    f"        assertEquals({q(expected['error_id'])}, errorId(error))",
                ]
        else:
            lines += [
                f"        val result = VigenereCipher.breakCipher({value})",
                f"        assertEquals({q(expected['key'])}, result.key)",
                f"        assertEquals({q(expected['plaintext'])}, result.plaintext)",
            ]
        lines.append("    }")
    lines.append("}")
    return _ends(lines)


def render_lua(cases: list[dict[str, Any]], digest: str) -> str:
    q = _quote_lua
    lines = [
        _header("--", digest),
        'package.path = "../src/?.lua;../src/?/init.lua;" .. package.path',
        'local vigenere = require("coding_adventures.vigenere_cipher")',
        "",
        "local function error_id(message)",
        "    message = string.lower(tostring(message))",
        '    if string.find(message, "key length", 1, true) then return "vigenere-key-length-limit" end',
        '    if string.find(message, "analysis limit", 1, true) then return "vigenere-analysis-limit" end',
        '    return "vigenere-invalid-key"',
        "end",
        "",
        'describe("generated classical-ciphers-v1 Vigenere cases", function()',
    ]
    for case in cases:
        value = _text_expr(
            case, q, lambda scalar, count: f"string.rep({scalar}, {count})"
        )
        expected = case["expected"]
        op = case["operation"]
        lines.append(f"    it({q(case['id'])}, function()")
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"vigenere.{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"        assert.equals({q(expected['text'])}, {call})")
            else:
                lines += [
                    f"        local ok, caught = pcall(function() return {call} end)",
                    "        assert.is_false(ok)",
                    f"        assert.equals({q(expected['error_id'])}, error_id(caught))",
                ]
        elif op == "vigenere-find-key-length":
            call = f"vigenere.find_key_length({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"        assert.equals({expected['key_length']}, {call})")
            else:
                lines += [
                    f"        local ok, caught = pcall(function() return {call} end)",
                    "        assert.is_false(ok)",
                    f"        assert.equals({q(expected['error_id'])}, error_id(caught))",
                ]
        elif op == "vigenere-find-key":
            call = f"vigenere.find_key({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"        assert.equals({q(expected['key'])}, {call})")
            else:
                lines += [
                    f"        local ok, caught = pcall(function() return {call} end)",
                    "        assert.is_false(ok)",
                    f"        assert.equals({q(expected['error_id'])}, error_id(caught))",
                ]
        else:
            lines += [
                f"        local key, plaintext = vigenere.break_cipher({value})",
                f"        assert.equals({q(expected['key'])}, key)",
                f"        assert.equals({q(expected['plaintext'])}, plaintext)",
            ]
        lines.append("    end)")
    lines.append("end)")
    return _ends(lines)


def render_perl(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_perl
    lines = [
        _header("#", digest),
        "use Test2::V0;",
        "use utf8;",
        "use CodingAdventures::VigenereCipher qw(encrypt decrypt find_key_length find_key break_cipher);",
        "",
        "sub error_id {",
        "    my ($message) = @_;",
        "    $message = lc($message // q{});",
        '    return "vigenere-key-length-limit" if index($message, "key length") >= 0;',
        '    return "vigenere-analysis-limit" if index($message, "analysis limit") >= 0;',
        '    return "vigenere-invalid-key";',
        "}",
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"({scalar} x {count})")
        expected = case["expected"]
        op = case["operation"]
        lines += ["", f"subtest {q(case['id'])} => sub {{"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(
                    f"    is({call}, {q(expected['text'])}, 'fixture result');"
                )
            else:
                lines += [
                    f"    eval {{ {call}; 1 }};",
                    "    my $caught = $@;",
                    "    ok($caught, 'fixture error raised');",
                    f"    is(error_id($caught), {q(expected['error_id'])}, 'error id');",
                ]
        elif op == "vigenere-find-key-length":
            call = f"find_key_length({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"    is({call}, {expected['key_length']}, 'key length');")
            else:
                lines += [
                    f"    eval {{ {call}; 1 }};",
                    "    my $caught = $@;",
                    "    ok($caught, 'fixture error raised');",
                    f"    is(error_id($caught), {q(expected['error_id'])}, 'error id');",
                ]
        elif op == "vigenere-find-key":
            call = f"find_key({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"    is({call}, {q(expected['key'])}, 'key');")
            else:
                lines += [
                    f"    eval {{ {call}; 1 }};",
                    "    my $caught = $@;",
                    "    ok($caught, 'fixture error raised');",
                    f"    is(error_id($caught), {q(expected['error_id'])}, 'error id');",
                ]
        else:
            lines += [
                f"    my ($key, $plaintext) = break_cipher({value});",
                f"    is($key, {q(expected['key'])}, 'key');",
                f"    is($plaintext, {q(expected['plaintext'])}, 'plaintext');",
            ]
        lines.append("};")
    lines += ["", "done_testing;"]
    return _ends(lines)


def render_python(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_python
    lines = [
        _header("#", digest),
        "import pytest",
        "from vigenere_cipher import break_cipher, decrypt, encrypt, find_key, find_key_length",
        "",
        "def error_id(error: Exception) -> str:",
        "    message = str(error).lower()",
        '    if "key length" in message:',
        '        return "vigenere-key-length-limit"',
        '    if "analysis limit" in message:',
        '        return "vigenere-analysis-limit"',
        '    return "vigenere-invalid-key"',
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"({scalar} * {count})")
        expected = case["expected"]
        op = case["operation"]
        lines += ["", f"def test_{_method_id(case)}() -> None:"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"    assert {call} == {q(expected['text'])}")
            else:
                lines += [
                    "    with pytest.raises(ValueError) as captured:",
                    f"        {call}",
                    f"    assert error_id(captured.value) == {q(expected['error_id'])}",
                ]
        elif op == "vigenere-find-key-length":
            call = f"find_key_length({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"    assert {call} == {expected['key_length']}")
            else:
                lines += [
                    "    with pytest.raises(ValueError) as captured:",
                    f"        {call}",
                    f"    assert error_id(captured.value) == {q(expected['error_id'])}",
                ]
        elif op == "vigenere-find-key":
            call = f"find_key({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"    assert {call} == {q(expected['key'])}")
            else:
                lines += [
                    "    with pytest.raises(ValueError) as captured:",
                    f"        {call}",
                    f"    assert error_id(captured.value) == {q(expected['error_id'])}",
                ]
        else:
            lines += [
                f"    key, plaintext = break_cipher({value})",
                f"    assert key == {q(expected['key'])}",
                f"    assert plaintext == {q(expected['plaintext'])}",
            ]
    return _ends(lines)


def render_ruby(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_ruby
    lines = [
        _header("#", digest),
        "# frozen_string_literal: true",
        'require "minitest/autorun"',
        'require "coding_adventures_vigenere_cipher"',
        "",
        "class TestGeneratedClassicalCipherFixture < Minitest::Test",
        "  VC = CodingAdventures::VigenereCipher",
        "",
        "  def error_id(error)",
        "    message = error.message.downcase",
        '    return "vigenere-key-length-limit" if message.include?("key length")',
        '    return "vigenere-analysis-limit" if message.include?("analysis limit")',
        '    "vigenere-invalid-key"',
        "  end",
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"({scalar} * {count})")
        expected = case["expected"]
        op = case["operation"]
        lines += ["", f"  def test_{_method_id(case)}"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = (
                f"VC.{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            )
            if "text" in expected:
                lines.append(f"    assert_equal {q(expected['text'])}, {call}")
            else:
                lines += [
                    f"    error = assert_raises(ArgumentError) {{ {call} }}",
                    f"    assert_equal {q(expected['error_id'])}, error_id(error)",
                ]
        elif op == "vigenere-find-key-length":
            call = f"VC.find_key_length({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"    assert_equal {expected['key_length']}, {call}")
            else:
                lines += [
                    f"    error = assert_raises(ArgumentError) {{ {call} }}",
                    f"    assert_equal {q(expected['error_id'])}, error_id(error)",
                ]
        elif op == "vigenere-find-key":
            call = f"VC.find_key({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"    assert_equal {q(expected['key'])}, {call}")
            else:
                lines += [
                    f"    error = assert_raises(ArgumentError) {{ {call} }}",
                    f"    assert_equal {q(expected['error_id'])}, error_id(error)",
                ]
        else:
            lines += [
                f"    key, plaintext = VC.break_cipher({value})",
                f"    assert_equal {q(expected['key'])}, key",
                f"    assert_equal {q(expected['plaintext'])}, plaintext",
            ]
        lines.append("  end")
    lines.append("end")
    return _ends(lines)


def render_rust(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_rust
    lines = [
        _header("//", digest),
        "#[rustfmt::skip]",
        "mod generated {",
        "use std::any::Any;",
        "use std::panic::{catch_unwind, UnwindSafe};",
        "use vigenere_cipher::{break_cipher, decrypt, encrypt, find_key, find_key_length};",
        "",
        "fn error_id(message: &str) -> &'static str {",
        "    let message = message.to_lowercase();",
        '    if message.contains("key length") { return "vigenere-key-length-limit"; }',
        '    if message.contains("analysis limit") { return "vigenere-analysis-limit"; }',
        '    "vigenere-invalid-key"',
        "}",
        "",
        "fn panic_id<F: FnOnce() + UnwindSafe>(action: F) -> &'static str {",
        '    let caught = catch_unwind(action).expect_err("expected fixture panic");',
        "    let message = panic_message(caught.as_ref());",
        "    error_id(message)",
        "}",
        "",
        "fn panic_message(value: &(dyn Any + Send)) -> &str {",
        "    if let Some(message) = value.downcast_ref::<String>() { return message; }",
        "    if let Some(message) = value.downcast_ref::<&str>() { return message; }",
        '    "unknown panic"',
        "}",
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"{scalar}.repeat({count})")
        value_argument = f"&{value}" if "repeat_scalar" in case["input"] else value
        expected = case["expected"]
        op = case["operation"]
        lines += ["", "#[test]", f"fn {_method_id(case)}() {{"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"{op.removeprefix('vigenere-')}({value_argument}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"    assert_eq!({call}.unwrap(), {q(expected['text'])});")
            else:
                lines.append(
                    f"    assert_eq!(error_id(&{call}.unwrap_err()), {q(expected['error_id'])});"
                )
        elif op == "vigenere-find-key-length":
            call = f"find_key_length({value_argument}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"    assert_eq!({call}, {expected['key_length']});")
            else:
                lines.append(
                    f"    assert_eq!(panic_id(|| {{ {call}; }}), {q(expected['error_id'])});"
                )
        elif op == "vigenere-find-key":
            call = f"find_key({value_argument}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"    assert_eq!({call}, {q(expected['key'])});")
            else:
                lines.append(
                    f"    assert_eq!(panic_id(|| {{ {call}; }}), {q(expected['error_id'])});"
                )
        else:
            lines += [
                f"    let result = break_cipher({value_argument});",
                f"    assert_eq!(result.key, {q(expected['key'])});",
                f"    assert_eq!(result.plaintext, {q(expected['plaintext'])});",
            ]
        lines.append("}")
    lines.append("}")
    return _ends(lines)


def render_swift(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_swift
    lines = [
        _header("//", digest),
        "import XCTest",
        "@testable import VigenereCipher",
        "",
        "final class GeneratedClassicalCipherFixtureTests: XCTestCase {",
        "    private func scalars(_ values: [Int]) -> String {",
        "        String(String.UnicodeScalarView(values.compactMap(UnicodeScalar.init)))",
        "    }",
        "",
        "    private func errorID(_ error: Error) -> String {",
        "        switch error {",
        '        case VigenereCipherError.analysisLimit: return "vigenere-analysis-limit"',
        '        case VigenereCipherError.keyLengthLimit: return "vigenere-key-length-limit"',
        '        default: return "vigenere-invalid-key"',
        "        }",
        "    }",
    ]
    for case in cases:
        value = _text_expr(
            case,
            q,
            lambda scalar, count: f"String(repeating: {scalar}, count: {count})",
        )
        expected = case["expected"]
        op = case["operation"]
        throws = " throws" if "error_id" not in expected else ""
        lines += ["", f"    func test_{_method_id(case)}(){throws} {{"]
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"VigenereCipher.{op.removeprefix('vigenere-')}({value}, key: {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(
                    f"        XCTAssertEqual(try {call}, {q(expected['text'])})"
                )
            else:
                lines += [
                    "        do {",
                    f"            _ = try {call}",
                    '            XCTFail("expected fixture error")',
                    "        } catch {",
                    f"            XCTAssertEqual(errorID(error), {q(expected['error_id'])})",
                    "        }",
                ]
        elif op == "vigenere-find-key-length":
            call = f"VigenereCipher.findKeyLength({value}, maxLength: {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(
                    f"        XCTAssertEqual(try {call}, {expected['key_length']})"
                )
            else:
                lines += [
                    "        do {",
                    f"            _ = try {call}",
                    '            XCTFail("expected fixture error")',
                    "        } catch {",
                    f"            XCTAssertEqual(errorID(error), {q(expected['error_id'])})",
                    "        }",
                ]
        elif op == "vigenere-find-key":
            call = f"VigenereCipher.findKey({value}, keyLength: {case['input']['key_length']})"
            if "key" in expected:
                lines.append(
                    f"        XCTAssertEqual(try {call}, {q(expected['key'])})"
                )
            else:
                lines += [
                    "        do {",
                    f"            _ = try {call}",
                    '            XCTFail("expected fixture error")',
                    "        } catch {",
                    f"            XCTAssertEqual(errorID(error), {q(expected['error_id'])})",
                    "        }",
                ]
        else:
            lines += [
                f"        let result = try VigenereCipher.breakCipher({value})",
                f"        XCTAssertEqual(result.key, {q(expected['key'])})",
                f"        XCTAssertEqual(result.plaintext, {q(expected['plaintext'])})",
            ]
        lines.append("    }")
    lines.append("}")
    return _ends(lines)


def render_typescript(cases: list[dict[str, Any]], digest: str) -> str:
    q = shared._quote_typescript
    lines = [
        _header("//", digest),
        'import { describe, it, expect } from "vitest";',
        'import { breakCipher, decrypt, encrypt, findKey, findKeyLength } from "../src/index.js";',
        "",
        "function errorId(error: unknown): string {",
        "  const message = String(error).toLowerCase();",
        '  if (message.includes("key length")) return "vigenere-key-length-limit";',
        '  if (message.includes("analysis limit")) return "vigenere-analysis-limit";',
        '  return "vigenere-invalid-key";',
        "}",
        "",
        'describe("generated classical-ciphers-v1 Vigenere cases", () => {',
    ]
    for case in cases:
        value = _text_expr(case, q, lambda scalar, count: f"{scalar}.repeat({count})")
        expected = case["expected"]
        op = case["operation"]
        lines.append(f"  it({q(case['id'])}, () => {{")
        if op in {"vigenere-encrypt", "vigenere-decrypt"}:
            call = f"{op.removeprefix('vigenere-')}({value}, {q(case['input']['key'])})"
            if "text" in expected:
                lines.append(f"    expect({call}).toBe({q(expected['text'])});")
            else:
                lines += [
                    "    let caught: unknown;",
                    "    try {",
                    f"      {call};",
                    "    } catch (error) {",
                    "      caught = error;",
                    "    }",
                    "    expect(caught).toBeDefined();",
                    f"    expect(errorId(caught)).toBe({q(expected['error_id'])});",
                ]
        elif op == "vigenere-find-key-length":
            call = f"findKeyLength({value}, {case['input']['max_length']})"
            if "key_length" in expected:
                lines.append(f"    expect({call}).toBe({expected['key_length']});")
            else:
                lines += [
                    "    let caught: unknown;",
                    "    try {",
                    f"      {call};",
                    "    } catch (error) {",
                    "      caught = error;",
                    "    }",
                    "    expect(caught).toBeDefined();",
                    f"    expect(errorId(caught)).toBe({q(expected['error_id'])});",
                ]
        elif op == "vigenere-find-key":
            call = f"findKey({value}, {case['input']['key_length']})"
            if "key" in expected:
                lines.append(f"    expect({call}).toBe({q(expected['key'])});")
            else:
                lines += [
                    "    let caught: unknown;",
                    "    try {",
                    f"      {call};",
                    "    } catch (error) {",
                    "      caught = error;",
                    "    }",
                    "    expect(caught).toBeDefined();",
                    f"    expect(errorId(caught)).toBe({q(expected['error_id'])});",
                ]
        else:
            lines += [
                f"    const result = breakCipher({value});",
                f"    expect(result.key).toBe({q(expected['key'])});",
                f"    expect(result.plaintext).toBe({q(expected['plaintext'])});",
            ]
        lines.append("  });")
    lines.append("});")
    return _ends(lines)


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
        raise ValueError("fixture-invalid-target-roster")
    if not all(isinstance(case, dict) for case in cases):
        raise ValueError("fixture-invalid-case")  # noqa: TRY004
    if {case.get("id") for case in cases} != EXPECTED_VIGENERE_CASE_IDS:
        raise ValueError("fixture-invalid-vigenere-roster")
    for case in cases:
        if (
            not isinstance(case, dict)
            or case.get("operation") not in VIGENERE_OPERATIONS
        ):
            raise ValueError("fixture-invalid-case")
        _validate_case(case)
    prefixes = {
        "csharp": "//",
        "dart": "//",
        "elixir": "#",
        "fsharp": "//",
        "go": "//",
        "haskell": "--",
        "java": "//",
        "kotlin": "//",
        "lua": "--",
        "perl": "#",
        "python": "#",
        "ruby": "#",
        "rust": "//",
        "swift": "//",
        "typescript": "//",
    }
    markers: list[str] = []
    for case in cases:
        marker = case["id"]
        error_id = case["expected"].get("error_id")
        if IDENTIFIER_PATTERN.fullmatch(marker) is None or (
            error_id is not None and IDENTIFIER_PATTERN.fullmatch(error_id) is None
        ):
            raise ValueError("fixture-invalid-case")
        markers.append(
            marker if error_id is None else f"{marker} expected-error={error_id}"
        )
    return {
        TARGETS[language]: RENDERERS[language](cases, digest)
        + "\n"
        + "\n".join(
            f"{prefixes[language]} Fixture case: {marker}" for marker in markers
        )
        + "\n"
        for language in TARGETS
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
        "--check",
        action="store_true",
        help="fail if generated outputs are absent or stale",
    )
    args = parser.parse_args()
    try:
        cases, digest = load_cases(FIXTURE_PATH)
        outputs = render_all(cases, digest)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    if args.check:
        failures = check_outputs(outputs)
        if failures:
            for failure in failures:
                print(
                    f"stale generated Vigenere fixture consumer: {failure}",
                    file=sys.stderr,
                )
            return 1
        return 0
    for relative_path, source in outputs.items():
        path = REPO_ROOT / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8", newline="\n")
        print(relative_path.as_posix())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
