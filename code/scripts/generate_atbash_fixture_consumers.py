#!/usr/bin/env python3
"""Generate dependency-free native consumers for the normative Atbash cases."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_PATH = REPO_ROOT / "code/specs/fixtures/classical-ciphers-v1/cases.json"
MAX_FIXTURE_BYTES = 131_072
MAX_FIXTURE_DEPTH = 8
MAX_TEXT_SCALARS = 8_193
MAX_GENERATED_BYTES = 131_072

ATBASH_CASE_IDS = (
    "classical-ciphers-v1-atbash-empty",
    "classical-ciphers-v1-atbash-uppercase-alphabet",
    "classical-ciphers-v1-atbash-lowercase-alphabet",
    "classical-ciphers-v1-atbash-passthrough",
    "classical-ciphers-v1-atbash-involution-forward",
    "classical-ciphers-v1-atbash-involution-reverse",
)

TARGETS = {
    "csharp": Path(
        "code/packages/csharp/atbash-cipher/tests/"
        "CodingAdventures.AtbashCipher.Tests/"
        "GeneratedClassicalCipherFixtureTests.cs"
    ),
    "dart": Path(
        "code/packages/dart/atbash-cipher/test/"
        "generated_classical_cipher_fixture_test.dart"
    ),
    "elixir": Path(
        "code/packages/elixir/atbash_cipher/test/"
        "generated_classical_cipher_fixture_test.exs"
    ),
    "fsharp": Path(
        "code/packages/fsharp/atbash-cipher/tests/"
        "CodingAdventures.AtbashCipher.Tests/"
        "GeneratedClassicalCipherFixtureTests.fs"
    ),
    "go": Path(
        "code/packages/go/atbash-cipher/generated_classical_cipher_fixture_test.go"
    ),
    "haskell": Path(
        "code/packages/haskell/atbash-cipher/test/"
        "GeneratedClassicalCipherFixtureSpec.hs"
    ),
    "java": Path(
        "code/packages/java/atbash-cipher/src/test/java/com/codingadventures/"
        "atbashcipher/GeneratedClassicalCipherFixtureTest.java"
    ),
    "kotlin": Path(
        "code/packages/kotlin/atbash-cipher/src/test/kotlin/com/codingadventures/"
        "atbashcipher/GeneratedClassicalCipherFixtureTest.kt"
    ),
    "lua": Path(
        "code/packages/lua/atbash_cipher/tests/"
        "test_generated_classical_cipher_fixture.lua"
    ),
    "perl": Path(
        "code/packages/perl/atbash-cipher/t/02-generated-classical-cipher-fixture.t"
    ),
    "python": Path(
        "code/packages/python/atbash-cipher/tests/"
        "test_generated_classical_cipher_fixture.py"
    ),
    "ruby": Path(
        "code/packages/ruby/atbash_cipher/test/"
        "test_generated_classical_cipher_fixture.rb"
    ),
    "rust": Path(
        "code/packages/rust/atbash-cipher/tests/generated_classical_cipher_fixture.rs"
    ),
    "swift": Path(
        "code/packages/swift/atbash-cipher/Tests/AtbashCipherTests/"
        "GeneratedClassicalCipherFixtureTests.swift"
    ),
    "typescript": Path(
        "code/packages/typescript/atbash-cipher/tests/"
        "generated-classical-cipher-fixture.test.ts"
    ),
}

REGISTRATION_REQUIREMENTS: dict[Path, tuple[str, ...]] = {
    Path(
        "code/packages/fsharp/atbash-cipher/tests/"
        "CodingAdventures.AtbashCipher.Tests/"
        "CodingAdventures.AtbashCipher.Tests.fsproj"
    ): ('<Compile Include="GeneratedClassicalCipherFixtureTests.fs" />',),
    Path("code/packages/haskell/atbash-cipher/atbash-cipher.cabal"): (
        "GeneratedClassicalCipherFixtureSpec",
    ),
    Path("code/packages/haskell/atbash-cipher/test/Spec.hs"): (
        "import qualified GeneratedClassicalCipherFixtureSpec",
        "GeneratedClassicalCipherFixtureSpec.spec",
    ),
}

_SAFE_CASE_ID = re.compile(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$")


def _reject_constant(_value: str) -> None:
    raise ValueError("fixture-invalid-json")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("fixture-invalid-json")
        result[key] = value
    return result


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
            continue
        if byte == 0x22:
            in_string = True
        elif byte in (0x7B, 0x5B):
            depth += 1
            if depth > MAX_FIXTURE_DEPTH:
                raise ValueError("fixture-depth-limit")
        elif byte in (0x7D, 0x5D):
            depth -= 1
            if depth < 0:
                raise ValueError("fixture-invalid-json")
    if in_string or depth != 0:
        raise ValueError("fixture-invalid-json")


def _validate_scalars(value: Any) -> None:
    if isinstance(value, str):
        if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
            raise ValueError("fixture-invalid-scalar")
    elif isinstance(value, list):
        for item in value:
            _validate_scalars(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            _validate_scalars(key)
            _validate_scalars(item)
    elif isinstance(value, float) and not math.isfinite(value):
        raise ValueError("fixture-invalid-json")


def _parse_document(raw: bytes) -> dict[str, Any]:
    if len(raw) > MAX_FIXTURE_BYTES:
        raise ValueError("fixture-size-limit")
    _check_raw_depth(raw)
    try:
        text = raw.decode("utf-8", errors="strict")
        document = json.loads(
            text,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError, TypeError) as error:
        raise ValueError("fixture-invalid-json") from error
    if not isinstance(document, dict):
        raise ValueError("fixture-invalid-json")  # noqa: TRY004
    _validate_scalars(document)
    return document


def _validate_atbash_case(case: Any) -> dict[str, Any]:
    if not isinstance(case, dict) or set(case) != {
        "id",
        "operation",
        "input",
        "expected",
    }:
        raise ValueError("fixture-invalid-case")
    case_id = case.get("id")
    operation = case.get("operation")
    input_value = case.get("input")
    expected = case.get("expected")
    if (
        not isinstance(case_id, str)
        or _SAFE_CASE_ID.fullmatch(case_id) is None
        or not isinstance(operation, str)
        or operation != "atbash-transform"
        or not isinstance(input_value, dict)
        or set(input_value) != {"text"}
        or not isinstance(expected, dict)
        or set(expected) != {"text"}
    ):
        raise ValueError("fixture-invalid-case")
    input_text = input_value["text"]
    expected_text = expected["text"]
    if (
        not isinstance(input_text, str)
        or not isinstance(expected_text, str)
        or len(input_text) > MAX_TEXT_SCALARS
        or len(expected_text) > MAX_TEXT_SCALARS
    ):
        raise ValueError("fixture-invalid-case")
    return case


def load_cases_bytes(raw: bytes) -> tuple[list[dict[str, Any]], str]:
    """Strictly load the exact closed Atbash subset from fixture bytes."""

    document = _parse_document(raw)
    if document.get("schema_version") != 1:
        raise ValueError("fixture-invalid-case")
    cases = document.get("cases")
    if not isinstance(cases, list) or len(cases) > 64:
        raise ValueError("fixture-invalid-case")

    all_ids: set[str] = set()
    selected: list[dict[str, Any]] = []
    for case in cases:
        if not isinstance(case, dict):
            raise ValueError("fixture-invalid-case")  # noqa: TRY004
        case_id = case.get("id")
        operation = case.get("operation")
        if not isinstance(case_id, str) or case_id in all_ids:
            raise ValueError("fixture-invalid-case")
        all_ids.add(case_id)
        if not isinstance(operation, str):
            raise ValueError("fixture-invalid-case")  # noqa: TRY004
        if operation == "atbash-transform":
            selected.append(_validate_atbash_case(case))

    if tuple(case["id"] for case in selected) != ATBASH_CASE_IDS:
        raise ValueError("fixture-invalid-case")
    return selected, hashlib.sha256(raw).hexdigest()


def load_cases(path: Path) -> tuple[list[dict[str, Any]], str]:
    """Read at most the configured fixture ceiling and load its Atbash cases."""

    if path.stat().st_size > MAX_FIXTURE_BYTES:
        raise ValueError("fixture-size-limit")
    with path.open("rb") as stream:
        raw = stream.read(MAX_FIXTURE_BYTES + 1)
    return load_cases_bytes(raw)


def _header(prefix: str, digest: str, cases: list[dict[str, Any]]) -> str:
    identifiers = ", ".join(case["id"] for case in cases)
    return (
        f"{prefix} GENERATED by code/scripts/generate_atbash_fixture_consumers.py.\n"
        f"{prefix} Source SHA-256: {digest}\n"
        f"{prefix} Cases: {identifiers}\n"
    )


def _numbers(text: str) -> str:
    return ", ".join(str(ord(character)) for character in text)


def _fsharp_numbers(text: str) -> str:
    return "; ".join(str(ord(character)) for character in text)


def _dart_string(value: str) -> str:
    output = ["'"]
    for character in value:
        code_point = ord(character)
        if character == "'":
            output.append("\\'")
        elif character == "\\":
            output.append("\\\\")
        elif character == "$":
            output.append("\\$")
        elif code_point <= 0x1F or code_point == 0x7F:
            output.append(f"\\u{code_point:04x}")
        elif code_point > 0xFFFF:
            output.append(f"\\u{{{code_point:x}}}")
        else:
            output.append(character)
    output.append("'")
    return "".join(output)


def _rust_string(value: str) -> str:
    return '"' + "".join(f"\\u{{{ord(character):x}}}" for character in value) + '"'


def _case_rows(
    cases: list[dict[str, Any]],
    row: Callable[[str, str, str], str],
) -> str:
    return "\n".join(
        row(case["id"], case["input"]["text"], case["expected"]["text"])
        for case in cases
    )


def _render_csharp(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'            (Id: "{case_id}", Input: S({_numbers(text)}), '
            f"Expected: S({_numbers(expected)})),"
        ),
    )
    return (
        _header("//", digest, cases)
        + """
namespace CodingAdventures.AtbashCipher.Tests;

public sealed class GeneratedClassicalCipherFixtureTests
{
    private static string S(params int[] codePoints)
    {
        var builder = new System.Text.StringBuilder();
        foreach (var codePoint in codePoints)
        {
            builder.Append(char.ConvertFromUtf32(codePoint));
        }
        return builder.ToString();
    }

    [Fact]
    public void ExecutesCompleteNormativeAtbashObjects()
    {
        var cases = new[]
        {
"""
        + rows
        + """
        };
        foreach (var fixtureCase in cases)
        {
            Assert.Equal(fixtureCase.Expected, AtbashCipher.Encrypt(fixtureCase.Input));
        }
    }
}
"""
    )


def _render_dart(cases: list[dict[str, Any]], digest: str) -> str:
    rendered_rows: list[str] = []
    for case in cases:
        case_id = _dart_string(case["id"])
        input_text = _dart_string(case["input"]["text"])
        expected = _dart_string(case["expected"]["text"])
        one_line = f"      (id: {case_id}, input: {input_text}, expected: {expected}),"
        if len(one_line) <= 80:
            rendered_rows.append(one_line)
        else:
            rendered_rows.append(
                "\n".join(
                    (
                        "      (",
                        f"        id: {case_id},",
                        f"        input: {input_text},",
                        f"        expected: {expected},",
                        "      ),",
                    )
                )
            )
    rows = "\n".join(rendered_rows)
    return (
        _header("//", digest, cases)
        + """
import 'package:coding_adventures_atbash_cipher/atbash_cipher.dart';
import 'package:test/test.dart';

void main() {
  test('executes complete normative Atbash objects', () {
    final cases = <({String id, String input, String expected})>[
"""
        + rows
        + """
    ];
    for (final fixtureCase in cases) {
      expect(
        encrypt(fixtureCase.input),
        fixtureCase.expected,
        reason: fixtureCase.id,
      );
    }
  });
}
"""
    )


def _render_elixir(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'      {{"{case_id}", s.([{_numbers(text)}]), '
            f"s.([{_numbers(expected)}])}},"
        ),
    )
    return (
        _header("#", digest, cases)
        + """
defmodule CodingAdventures.GeneratedAtbashFixtureTest do
  use ExUnit.Case
  alias CodingAdventures.AtbashCipher

  test "executes complete normative Atbash objects" do
    s = fn scalars -> List.to_string(scalars) end
    cases = [
"""
        + rows
        + """
    ]

    for {id, input, expected} <- cases do
      assert AtbashCipher.encrypt(input) == expected, id
    end
  end
end
"""
    )


def _render_fsharp(cases: list[dict[str, Any]], digest: str) -> str:
    rows = ";\n".join(
        f'                ("{case["id"]}", '
        f"s [| {_fsharp_numbers(case['input']['text'])} |], "
        f"s [| {_fsharp_numbers(case['expected']['text'])} |])"
        for case in cases
    )
    return (
        _header("//", digest, cases)
        + """
namespace CodingAdventures.AtbashCipher.Tests

open Xunit
open CodingAdventures.AtbashCipher

module GeneratedClassicalCipherFixtureTests =
    let private s (values: int array) =
        values
        |> Array.map (fun value -> System.Char.ConvertFromUtf32 value)
        |> System.String.Concat

    [<Fact>]
    let ``executes complete normative Atbash objects`` () =
        let cases =
            [|
"""
        + rows
        + """
            |]
        for (id, input, expected) in cases do
            Assert.True(AtbashCipher.encrypt input = expected, id)
"""
    )


def _render_go(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'\t\t{{id: "{case_id}", input: s({_numbers(text)}), '
            f"expected: s({_numbers(expected)})}},"
        ),
    )
    return (
        _header("//", digest, cases)
        + """
package atbashcipher

import "testing"

func generatedAtbashFixtureString(values ...rune) string { return string(values) }

func TestGeneratedClassicalCipherFixture(t *testing.T) {
\ts := generatedAtbashFixtureString
\tcases := []struct{ id, input, expected string }{
"""
        + rows
        + """
\t}
\tfor _, fixtureCase := range cases {
\t\tt.Run(fixtureCase.id, func(t *testing.T) {
\t\t\tif actual := Encrypt(fixtureCase.input); actual != fixtureCase.expected {
\t\t\t\tt.Fatalf("got %q, want %q", actual, fixtureCase.expected)
\t\t\t}
\t\t})
\t}
}
"""
    )


def _render_haskell(cases: list[dict[str, Any]], digest: str) -> str:
    rows = "\n".join(
        ("            [ " if index == 0 else "            , ")
        + f'("{case["id"]}", s [{_numbers(case["input"]["text"])}], '
        + f"s [{_numbers(case['expected']['text'])}])"
        for index, case in enumerate(cases)
    )
    return (
        _header("--", digest, cases)
        + """
module GeneratedClassicalCipherFixtureSpec (spec) where

import AtbashCipher (encrypt)
import Test.Hspec

s :: [Int] -> String
s = map toEnum

spec :: Spec
spec = describe "classical-ciphers-v1 Atbash" $ do
    let cases =
"""
        + rows
        + r"""
            ]
    mapM_ (\(caseId, input, expected) ->
        it caseId $ encrypt input `shouldBe` expected) cases
"""
    )


def _render_java(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'            new FixtureCase("{case_id}", s({_numbers(text)}), '
            f"s({_numbers(expected)})),"
        ),
    )
    return (
        _header("//", digest, cases)
        + """
package com.codingadventures.atbashcipher;

import static org.junit.jupiter.api.Assertions.assertEquals;
import org.junit.jupiter.api.Test;

class GeneratedClassicalCipherFixtureTest {
    private record FixtureCase(String id, String input, String expected) {}

    private static String s(int... codePoints) {
        StringBuilder builder = new StringBuilder();
        for (int codePoint : codePoints) builder.appendCodePoint(codePoint);
        return builder.toString();
    }

    @Test
    void executesCompleteNormativeAtbashObjects() {
        FixtureCase[] cases = {
"""
        + rows
        + """
        };
        for (FixtureCase fixtureCase : cases) {
            assertEquals(fixtureCase.expected(), AtbashCipher.encrypt(fixtureCase.input()),
                fixtureCase.id());
        }
    }
}
"""
    )


def _render_kotlin(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'            FixtureCase("{case_id}", s({_numbers(text)}), '
            f"s({_numbers(expected)})),"
        ),
    )
    return (
        _header("//", digest, cases)
        + """
package com.codingadventures.atbashcipher

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class GeneratedClassicalCipherFixtureTest {
    private data class FixtureCase(val id: String, val input: String, val expected: String)

    private fun s(vararg codePoints: Int): String {
        val builder = StringBuilder()
        codePoints.forEach { builder.appendCodePoint(it) }
        return builder.toString()
    }

    @Test
    fun executesCompleteNormativeAtbashObjects() {
        val cases = listOf(
"""
        + rows
        + """
        )
        cases.forEach { fixtureCase ->
            assertEquals(fixtureCase.expected, AtbashCipher.encrypt(fixtureCase.input),
                fixtureCase.id)
        }
    }
}
"""
    )


def _render_lua(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'        {{ id = "{case_id}", input = s({_numbers(text)}), '
            f"expected = s({_numbers(expected)}) }},"
        ),
    )
    return (
        _header("--", digest, cases)
        + """
local atbash = require("coding_adventures.atbash_cipher")

local function s(...)
    if select("#", ...) == 0 then return "" end
    return utf8.char(...)
end

describe("classical-ciphers-v1 Atbash", function()
    it("executes complete normative objects", function()
        local cases = {
"""
        + rows
        + """
        }
        for _, fixture_case in ipairs(cases) do
            assert.are.equal(fixture_case.expected, atbash.encrypt(fixture_case.input),
                fixture_case.id)
        end
    end)
end)
"""
    )


def _render_perl(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'    ["{case_id}", scalar_text({_numbers(text)}), '
            f"scalar_text({_numbers(expected)})],"
        ),
    )
    return (
        _header("#", digest, cases)
        + """
use strict;
use warnings;
use Test2::V0;
use CodingAdventures::AtbashCipher qw(encrypt);

sub scalar_text { return join "", map { chr($_) } @_; }

my @cases = (
"""
        + rows
        + """
);
for my $fixture_case (@cases) {
    my ($id, $input, $expected) = @{$fixture_case};
    is(encrypt($input), $expected, $id);
}
done_testing;
"""
    )


def _render_python(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'        ("{case_id}", s([{_numbers(text)}]), s([{_numbers(expected)}])),'
        ),
    )
    return (
        _header("#", digest, cases)
        + "# ruff: noqa: E501\n"
        + '"""Generated complete-object Atbash conformance."""\n\n'
        + "from atbash_cipher import encrypt\n\n\n"
        + "def _s(values: list[int]) -> str:\n"
        + '    return "".join(map(chr, values))\n\n\n'
        + "def test_generated_classical_cipher_fixture() -> None:\n"
        + "    cases = [\n"
        + rows.replace("s([", "_s([")
        + "\n    ]\n"
        + "    for case_id, input_text, expected in cases:\n"
        + "        assert encrypt(input_text) == expected, case_id\n"
    )


def _render_ruby(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'      ["{case_id}", s([{_numbers(text)}]), s([{_numbers(expected)}])],'
        ),
    )
    return (
        _header("#", digest, cases)
        + """
# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_atbash_cipher"

class TestGeneratedClassicalCipherFixture < Minitest::Test
  def s(values)
    values.pack("U*")
  end

  def test_complete_normative_atbash_objects
    cases = [
"""
        + rows
        + """
    ]
    cases.each do |case_id, input, expected|
      assert_equal expected, CodingAdventures::AtbashCipher.encrypt(input), case_id
    end
  end
end
"""
    )


def _render_rust(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'        ("{case_id}", {_rust_string(text)}, {_rust_string(expected)}),'
        ),
    )
    return (
        _header("//", digest, cases)
        + """
use atbash_cipher::encrypt;

#[test]
fn executes_complete_normative_atbash_objects() {
    let cases = [
"""
        + rows
        + """
    ];
    for (case_id, input, expected) in cases {
        assert_eq!(encrypt(input), expected, "{case_id}");
    }
}
"""
    )


def _render_swift(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'            ("{case_id}", s([{_numbers(text)}]), '
            f"s([{_numbers(expected)}])),"
        ),
    )
    return (
        _header("//", digest, cases)
        + """
import XCTest
@testable import AtbashCipher

final class GeneratedClassicalCipherFixtureTests: XCTestCase {
    private func s(_ codePoints: [UInt32]) -> String {
        codePoints.map { String(UnicodeScalar($0)!) }.joined()
    }

    func testCompleteNormativeAtbashObjects() {
        let cases = [
"""
        + rows
        + """
        ]
        for (caseID, input, expected) in cases {
            XCTAssertEqual(AtbashCipher.encrypt(input), expected, caseID)
        }
    }
}
"""
    )


def _render_typescript(cases: list[dict[str, Any]], digest: str) -> str:
    rows = _case_rows(
        cases,
        lambda case_id, text, expected: (
            f'    {{ id: "{case_id}", input: s({_numbers(text)}), '
            f"expected: s({_numbers(expected)}) }},"
        ),
    )
    return (
        _header("//", digest, cases)
        + """
import { describe, expect, it } from "vitest";
import { encrypt } from "../src/index.js";

const s = (...codePoints: number[]): string => String.fromCodePoint(...codePoints);

describe("classical-ciphers-v1 Atbash", () => {
  it("executes complete normative objects", () => {
    const cases = [
"""
        + rows
        + """
    ];
    for (const fixtureCase of cases) {
      expect(encrypt(fixtureCase.input), fixtureCase.id).toBe(fixtureCase.expected);
    }
  });
});
"""
    )


RENDERERS: dict[str, Callable[[list[dict[str, Any]], str], str]] = {
    "csharp": _render_csharp,
    "dart": _render_dart,
    "elixir": _render_elixir,
    "fsharp": _render_fsharp,
    "go": _render_go,
    "haskell": _render_haskell,
    "java": _render_java,
    "kotlin": _render_kotlin,
    "lua": _render_lua,
    "perl": _render_perl,
    "python": _render_python,
    "ruby": _render_ruby,
    "rust": _render_rust,
    "swift": _render_swift,
    "typescript": _render_typescript,
}


def render_all(cases: list[dict[str, Any]], digest: str) -> dict[Path, str]:
    """Render the complete Atbash subset for every established lane."""

    return {
        TARGETS[language]: RENDERERS[language](cases, digest) for language in TARGETS
    }


def _bounded_read(path: Path, expected_size: int) -> bytes | None:
    try:
        if path.stat().st_size != expected_size or expected_size > MAX_GENERATED_BYTES:
            return None
        with path.open("rb") as stream:
            actual = stream.read(expected_size + 1)
    except OSError:
        return None
    return actual if len(actual) == expected_size else None


def _bounded_text(path: Path) -> str | None:
    try:
        size = path.stat().st_size
        if size > MAX_GENERATED_BYTES:
            return None
        with path.open("rb") as stream:
            raw = stream.read(MAX_GENERATED_BYTES + 1)
        if len(raw) != size:
            return None
        return raw.decode("utf-8", errors="strict")
    except (OSError, UnicodeDecodeError):
        return None


def check_outputs(outputs: dict[Path, str], root: Path) -> list[str]:
    """Return stale generated outputs or explicit test registrations."""

    failures: list[str] = []
    for relative_path, source in outputs.items():
        expected = source.encode("utf-8")
        actual = _bounded_read(root / relative_path, len(expected))
        if actual != expected:
            failures.append(relative_path.as_posix())
    for relative_path, requirements in REGISTRATION_REQUIREMENTS.items():
        actual = _bounded_text(root / relative_path)
        if actual is None or any(required not in actual for required in requirements):
            failures.append(relative_path.as_posix())
    return failures


def write_outputs(outputs: dict[Path, str], root: Path) -> None:
    for relative_path, source in outputs.items():
        target = root / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(source, encoding="utf-8", newline="\n")


def _print_failures(failures: Iterable[str]) -> None:
    for failure in failures:
        print(f"stale generated Atbash fixture consumer: {failure}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail on drift")
    args = parser.parse_args()

    cases, digest = load_cases(FIXTURE_PATH)
    outputs = render_all(cases, digest)
    if args.check:
        failures = check_outputs(outputs, REPO_ROOT)
        _print_failures(failures)
        return 1 if failures else 0
    write_outputs(outputs, REPO_ROOT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
