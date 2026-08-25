#!/usr/bin/env python3
"""Generate the pinned Unicode 17 tracked-artifact policy substrate.

The generated modules are source-embedded so validation never reads Unicode
data from the filesystem and never inherits the host runtime's tables. This
generator is the only networked step: it downloads exact Unicode Consortium
files, verifies their SHA-256 digests, renders every runtime, and exercises the
official normalization and case-folding vectors before accepting output.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import urllib.parse
import urllib.request
from pathlib import Path

UNICODE_VERSION = "17.0.0"
UCD_BASE = f"https://www.unicode.org/Public/{UNICODE_VERSION}/ucd/"
LICENSE_URL = "https://www.unicode.org/license.txt"
LICENSE_SHA256 = "e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96"
LICENSE_SIZE = 1995
LICENSE_PATH = Path("code/specs/fixtures/build-tool-v1/UNICODE-LICENSE.txt")
LICENSE_TARGETS = (
    Path("code/programs/python/build-tool/UNICODE-LICENSE.txt"),
    Path("code/programs/dotnet/build-tool-csharp/UNICODE-LICENSE.txt"),
    Path("code/programs/dotnet/build-tool-fsharp/UNICODE-LICENSE.txt"),
    Path("code/programs/typescript/build-tool/UNICODE-LICENSE.txt"),
    Path("code/programs/ruby/build-tool/UNICODE-LICENSE.txt"),
    Path("code/programs/elixir/build-tool/UNICODE-LICENSE.txt"),
    Path("code/programs/lua/build-tool/UNICODE-LICENSE.txt"),
)
SOURCES = {
    "UnicodeData.txt": "2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c",
    "CompositionExclusions.txt": "2f239196ef3b5b61db5cc476e9bd80f534d15aa1b74e1be1dea5d042a344c85f",
    "CaseFolding.txt": "ff8d8fefbf123574205085d6714c36149eb946d717a0c585c27f0f4ef58c4183",
    "SpecialCasing.txt": "efc25faf19de21b92c1194c111c932e03d2a5eaf18194e33f1156e96de4c9588",
    "NormalizationTest.txt": "5019ffd530751a741900c849c0e010332f142a3612234639bd200b82138a87db",
}
SOURCE_SIZES = {
    "UnicodeData.txt": 2_198_209,
    "CompositionExclusions.txt": 9_007,
    "CaseFolding.txt": 87_539,
    "SpecialCasing.txt": 17_049,
    "NormalizationTest.txt": 2_827_429,
}

PYTHON_TARGETS = (
    Path("code/scripts/tracked_artifact_unicode17.py"),
    Path(
        "code/programs/python/build-tool/src/build_tool/tracked_artifact_unicode17.py"
    ),
)
CSHARP_TARGET = Path(
    "code/programs/dotnet/build-tool-csharp/TrackedArtifactUnicode17.g.cs"
)
TYPESCRIPT_TARGET = Path(
    "code/programs/typescript/build-tool/src/tracked-artifact-unicode17.ts"
)
RUBY_TARGET = Path(
    "code/programs/ruby/build-tool/lib/build_tool/tracked_artifact_unicode17.rb"
)
ELIXIR_TARGET = Path(
    "code/programs/elixir/build-tool/lib/build_tool/tracked_artifact_unicode17.ex"
)
LUA_TARGET = Path(
    "code/programs/lua/build-tool/lib/build_tool/tracked_artifact_unicode17.lua"
)


class _RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, file_pointer, code, message, headers, new_url):
        raise RuntimeError(f"Unicode download refused redirect from {request.full_url}")


_HTTPS_OPENER = urllib.request.build_opener(_RejectRedirects())


def _download_exact(
    url: str,
    *,
    expected_size: int,
    expected_hash: str,
    label: str,
) -> bytes:
    parsed = urllib.parse.urlsplit(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname != "www.unicode.org"
        or parsed.port not in {None, 443}
        or parsed.username is not None
        or parsed.password is not None
    ):
        raise RuntimeError(f"Unicode source URL left the pinned HTTPS origin: {url}")
    with _HTTPS_OPENER.open(url, timeout=30) as response:  # nosec B310
        if response.geturl() != url:
            raise RuntimeError(f"Unicode source final URL drift for {label}")
        payload = response.read(expected_size + 1)
    if len(payload) != expected_size:
        raise RuntimeError(
            f"Unicode source size mismatch for {label}: "
            f"expected {expected_size}, received {len(payload)}"
        )
    actual_hash = hashlib.sha256(payload).hexdigest()
    if actual_hash != expected_hash:
        raise RuntimeError(
            f"Unicode source hash mismatch for {label}: "
            f"expected {expected_hash}, received {actual_hash}"
        )
    return payload


def _download_sources() -> dict[str, str]:
    sources: dict[str, str] = {}
    for name, expected_hash in SOURCES.items():
        payload = _download_exact(
            UCD_BASE + name,
            expected_size=SOURCE_SIZES[name],
            expected_hash=expected_hash,
            label=name,
        )
        sources[name] = payload.decode("utf-8")
    return sources


def _parse_sources(
    sources: dict[str, str],
) -> tuple[
    list[tuple[int, int]],
    list[tuple[int, bool, tuple[int, ...]]],
    list[tuple[int, int, int]],
    list[tuple[int, tuple[int, ...]]],
    list[tuple[int, tuple[int, ...]]],
]:
    exclusions = {
        int(line.split("#", 1)[0].strip(), 16)
        for line in sources["CompositionExclusions.txt"].splitlines()
        if line.split("#", 1)[0].strip()
    }
    combining: list[tuple[int, int]] = []
    decomposition: list[tuple[int, bool, tuple[int, ...]]] = []
    composition: list[tuple[int, int, int]] = []
    uppercase: dict[int, tuple[int, ...]] = {}

    for line in sources["UnicodeData.txt"].splitlines():
        fields = line.split(";")
        scalar = int(fields[0], 16)
        combining_class = int(fields[3])
        if combining_class:
            combining.append((scalar, combining_class))

        raw_decomposition = fields[5]
        if raw_decomposition:
            values = raw_decomposition.split()
            compatibility = values[0].startswith("<")
            if compatibility:
                values = values[1:]
            mapping = tuple(int(value, 16) for value in values)
            decomposition.append((scalar, compatibility, mapping))
            if not compatibility and len(mapping) == 2 and scalar not in exclusions:
                composition.append((mapping[0], mapping[1], scalar))

        if fields[12]:
            uppercase[scalar] = (int(fields[12], 16),)

    folding: dict[int, tuple[int, ...]] = {}
    for line in sources["CaseFolding.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        scalar = int(fields[0], 16)
        status = fields[1]
        mapping = tuple(int(value, 16) for value in fields[2].split())
        if status in {"C", "F"}:
            folding[scalar] = mapping

    for line in sources["SpecialCasing.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[4]:
            continue
        uppercase[int(fields[0], 16)] = tuple(
            int(value, 16) for value in fields[3].split()
        )

    return (
        sorted(combining),
        sorted(decomposition),
        sorted(composition),
        sorted(folding.items()),
        sorted(uppercase.items()),
    )


def _mapping_lines(rows: list[tuple[int, tuple[int, ...]]]) -> str:
    return "\n".join(
        f"{scalar:X};{','.join(f'{value:X}' for value in mapping)}"
        for scalar, mapping in rows
    )


def _render_python(
    tables: tuple[
        list[tuple[int, int]],
        list[tuple[int, bool, tuple[int, ...]]],
        list[tuple[int, int, int]],
        list[tuple[int, tuple[int, ...]]],
        list[tuple[int, tuple[int, ...]]],
    ],
) -> str:
    combining, decomposition, composition, folding, uppercase = tables
    combining_text = "\n".join(f"{cp:X};{ccc}" for cp, ccc in combining)
    decomposition_text = "\n".join(
        f"{cp:X};{'K' if compat else 'C'};{','.join(f'{value:X}' for value in mapping)}"
        for cp, compat, mapping in decomposition
    )
    composition_text = "\n".join(
        f"{left:X},{right:X};{result:X}" for left, right, result in composition
    )
    folding_text = _mapping_lines(folding)
    uppercase_text = _mapping_lines(uppercase)
    source_hashes = "\n".join(
        f"# {name}: sha256:{digest}" for name, digest in SOURCES.items()
    )
    return f'''# ruff: noqa
"""Generated Unicode {UNICODE_VERSION} data and algorithms.

DO NOT EDIT. Run ``python code/scripts/generate_tracked_artifact_unicode17.py``.
Sources: {UCD_BASE}
{source_hashes}
Unicode data is used under Unicode License v3. Every source and binary
distribution carries the full notice as UNICODE-LICENSE.txt
(sha256:{LICENSE_SHA256}).
"""

from __future__ import annotations

UNICODE_VERSION = "{UNICODE_VERSION}"

_COMBINING_DATA = """{combining_text}"""
_DECOMPOSITION_DATA = """{decomposition_text}"""
_COMPOSITION_DATA = """{composition_text}"""
_FOLDING_DATA = """{folding_text}"""
_UPPERCASE_DATA = """{uppercase_text}"""


def _parse_combining() -> dict[int, int]:
    return {{
        int(scalar, 16): int(value)
        for scalar, value in (line.split(";") for line in _COMBINING_DATA.splitlines())
    }}


def _parse_decomposition() -> dict[int, tuple[bool, tuple[int, ...]]]:
    result: dict[int, tuple[bool, tuple[int, ...]]] = {{}}
    for line in _DECOMPOSITION_DATA.splitlines():
        scalar, kind, mapping = line.split(";")
        result[int(scalar, 16)] = (
            kind == "K",
            tuple(int(value, 16) for value in mapping.split(",")),
        )
    return result


def _parse_composition() -> dict[tuple[int, int], int]:
    result: dict[tuple[int, int], int] = {{}}
    for line in _COMPOSITION_DATA.splitlines():
        pair, composite = line.split(";")
        left, right = pair.split(",")
        result[(int(left, 16), int(right, 16))] = int(composite, 16)
    return result


def _parse_mapping(data: str) -> dict[int, tuple[int, ...]]:
    result: dict[int, tuple[int, ...]] = {{}}
    for line in data.splitlines():
        scalar, mapping = line.split(";")
        result[int(scalar, 16)] = tuple(
            int(value, 16) for value in mapping.split(",") if value
        )
    return result


_COMBINING = _parse_combining()
_DECOMPOSITION = _parse_decomposition()
_COMPOSITION = _parse_composition()
_FOLDING = _parse_mapping(_FOLDING_DATA)
_UPPERCASE = _parse_mapping(_UPPERCASE_DATA)

_S_BASE = 0xAC00
_L_BASE = 0x1100
_V_BASE = 0x1161
_T_BASE = 0x11A7
_L_COUNT = 19
_V_COUNT = 21
_T_COUNT = 28
_N_COUNT = _V_COUNT * _T_COUNT
_S_COUNT = _L_COUNT * _N_COUNT


def _decompose_scalar(scalar: int, compatibility: bool, output: list[int]) -> None:
    if _S_BASE <= scalar < _S_BASE + _S_COUNT:
        index = scalar - _S_BASE
        output.append(_L_BASE + index // _N_COUNT)
        output.append(_V_BASE + (index % _N_COUNT) // _T_COUNT)
        trailing = _T_BASE + index % _T_COUNT
        if trailing != _T_BASE:
            output.append(trailing)
        return
    row = _DECOMPOSITION.get(scalar)
    if row is None or (row[0] and not compatibility):
        output.append(scalar)
        return
    for mapped in row[1]:
        _decompose_scalar(mapped, compatibility, output)


def _canonical_order(scalars: list[int]) -> None:
    index = 0
    while index < len(scalars):
        if _COMBINING.get(scalars[index], 0) == 0:
            index += 1
            continue
        end = index + 1
        while end < len(scalars) and _COMBINING.get(scalars[end], 0) != 0:
            end += 1
        scalars[index:end] = sorted(
            scalars[index:end], key=lambda scalar: _COMBINING.get(scalar, 0)
        )
        index = end


def _compose_pair(left: int, right: int) -> int | None:
    if _L_BASE <= left < _L_BASE + _L_COUNT and _V_BASE <= right < _V_BASE + _V_COUNT:
        return _S_BASE + ((left - _L_BASE) * _V_COUNT + right - _V_BASE) * _T_COUNT
    if (
        _S_BASE <= left < _S_BASE + _S_COUNT
        and (left - _S_BASE) % _T_COUNT == 0
        and _T_BASE < right < _T_BASE + _T_COUNT
    ):
        return left + right - _T_BASE
    return _COMPOSITION.get((left, right))


def _normalize(value: str, compatibility: bool) -> str:
    decomposed: list[int] = []
    for character in value:
        _decompose_scalar(ord(character), compatibility, decomposed)
    _canonical_order(decomposed)
    if not decomposed:
        return ""
    output = [decomposed[0]]
    starter_index = 0
    starter = decomposed[0]
    last_class = 255 if _COMBINING.get(starter, 0) else 0
    for scalar in decomposed[1:]:
        combining_class = _COMBINING.get(scalar, 0)
        composite = None
        if last_class == 0 or last_class < combining_class:
            composite = _compose_pair(starter, scalar)
        if composite is not None:
            output[starter_index] = composite
            starter = composite
            continue
        output.append(scalar)
        if combining_class == 0:
            starter_index = len(output) - 1
            starter = scalar
        last_class = combining_class
    return "".join(chr(scalar) for scalar in output)


def nfc(value: str) -> str:
    """Return Unicode 17 NFC without consulting host Unicode tables."""
    return _normalize(value, False)


def nfkc(value: str) -> str:
    """Return Unicode 17 NFKC without consulting host Unicode tables."""
    return _normalize(value, True)


def casefold(value: str) -> str:
    """Return Unicode 17 locale-independent full default case folding."""
    return "".join(
        chr(mapped)
        for character in value
        for mapped in _FOLDING.get(ord(character), (ord(character),))
    )


def nfkc_casefold(value: str) -> str:
    """Apply the contract's Unicode 17 NFKC-then-full-fold operation."""
    return casefold(nfkc(value))


def full_uppercase(value: str) -> str:
    """Return Unicode 17 root-locale full uppercase mapping."""
    return "".join(
        chr(mapped)
        for character in value
        for mapped in _UPPERCASE.get(ord(character), (ord(character),))
    )
'''


def _csharp_raw(value: str) -> str:
    return f'"""\n{value}\n"""'


def _render_csharp(
    tables: tuple[
        list[tuple[int, int]],
        list[tuple[int, bool, tuple[int, ...]]],
        list[tuple[int, int, int]],
        list[tuple[int, tuple[int, ...]]],
        list[tuple[int, tuple[int, ...]]],
    ],
) -> str:
    combining, decomposition, composition, folding, uppercase = tables
    combining_text = "\n".join(f"{cp:X};{ccc}" for cp, ccc in combining)
    decomposition_text = "\n".join(
        f"{cp:X};{'K' if compat else 'C'};{','.join(f'{value:X}' for value in mapping)}"
        for cp, compat, mapping in decomposition
    )
    composition_text = "\n".join(
        f"{left:X},{right:X};{result:X}" for left, right, result in composition
    )
    folding_text = _mapping_lines(folding)
    uppercase_text = _mapping_lines(uppercase)
    hashes = ", ".join(f"{name} sha256:{digest}" for name, digest in SOURCES.items())
    return f'''// <auto-generated />
// Unicode {UNICODE_VERSION}; generated by code/scripts/generate_tracked_artifact_unicode17.py.
// Sources: {UCD_BASE}
// {hashes}
// Unicode License v3: every source and binary distribution carries the full
// notice as UNICODE-LICENSE.txt (sha256:{LICENSE_SHA256}).

using System.Globalization;
using System.Text;

namespace CodingAdventures.BuildTool.CSharp;

internal static class TrackedArtifactUnicode17
{{
    internal const string Version = "{UNICODE_VERSION}";

    private static readonly string CombiningData = {_csharp_raw(combining_text)};
    private static readonly string DecompositionData = {_csharp_raw(decomposition_text)};
    private static readonly string CompositionData = {_csharp_raw(composition_text)};
    private static readonly string FoldingData = {_csharp_raw(folding_text)};
    private static readonly string UppercaseData = {_csharp_raw(uppercase_text)};

    private static readonly Dictionary<int, int> Combining = ParseCombining();
    private static readonly Dictionary<int, (bool Compatibility, int[] Mapping)> Decomposition = ParseDecomposition();
    private static readonly Dictionary<long, int> Composition = ParseComposition();
    private static readonly Dictionary<int, int[]> Folding = ParseMapping(FoldingData);
    private static readonly Dictionary<int, int[]> Uppercase = ParseMapping(UppercaseData);

    private const int SBase = 0xAC00;
    private const int LBase = 0x1100;
    private const int VBase = 0x1161;
    private const int TBase = 0x11A7;
    private const int LCount = 19;
    private const int VCount = 21;
    private const int TCount = 28;
    private const int NCount = VCount * TCount;
    private const int SCount = LCount * NCount;

    internal static string Nfc(string value) => Normalize(value, compatibility: false);

    internal static string Nfkc(string value) => Normalize(value, compatibility: true);

    internal static string NfkcCaseFold(string value) => CaseFold(Nfkc(value));

    internal static string CaseFold(string value) => Map(value, Folding);

    internal static string FullUppercase(string value) => Map(value, Uppercase);

    private static string Normalize(string value, bool compatibility)
    {{
        var decomposed = new List<int>();
        foreach (var rune in value.EnumerateRunes())
        {{
            DecomposeScalar(rune.Value, compatibility, decomposed);
        }}
        CanonicalOrder(decomposed);
        if (decomposed.Count == 0)
        {{
            return string.Empty;
        }}

        var output = new List<int> {{ decomposed[0] }};
        var starterIndex = 0;
        var starter = decomposed[0];
        var lastClass = CombiningClass(starter) == 0 ? 0 : 255;
        foreach (var scalar in decomposed.Skip(1))
        {{
            var combiningClass = CombiningClass(scalar);
            int? composite = null;
            if (lastClass == 0 || lastClass < combiningClass)
            {{
                composite = ComposePair(starter, scalar);
            }}
            if (composite is not null)
            {{
                output[starterIndex] = composite.Value;
                starter = composite.Value;
                continue;
            }}
            output.Add(scalar);
            if (combiningClass == 0)
            {{
                starterIndex = output.Count - 1;
                starter = scalar;
            }}
            lastClass = combiningClass;
        }}
        return FromScalars(output);
    }}

    private static void DecomposeScalar(int scalar, bool compatibility, List<int> output)
    {{
        if (scalar >= SBase && scalar < SBase + SCount)
        {{
            var index = scalar - SBase;
            output.Add(LBase + index / NCount);
            output.Add(VBase + index % NCount / TCount);
            var trailing = TBase + index % TCount;
            if (trailing != TBase)
            {{
                output.Add(trailing);
            }}
            return;
        }}
        if (!Decomposition.TryGetValue(scalar, out var row) || (row.Compatibility && !compatibility))
        {{
            output.Add(scalar);
            return;
        }}
        foreach (var mapped in row.Mapping)
        {{
            DecomposeScalar(mapped, compatibility, output);
        }}
    }}

    private static void CanonicalOrder(List<int> scalars)
    {{
        var index = 0;
        while (index < scalars.Count)
        {{
            if (CombiningClass(scalars[index]) == 0)
            {{
                index++;
                continue;
            }}
            var end = index + 1;
            while (end < scalars.Count && CombiningClass(scalars[end]) != 0)
            {{
                end++;
            }}
            for (var current = index + 1; current < end; current++)
            {{
                var value = scalars[current];
                var valueClass = CombiningClass(value);
                var insertion = current;
                while (insertion > index && CombiningClass(scalars[insertion - 1]) > valueClass)
                {{
                    scalars[insertion] = scalars[insertion - 1];
                    insertion--;
                }}
                scalars[insertion] = value;
            }}
            index = end;
        }}
    }}

    private static int? ComposePair(int left, int right)
    {{
        if (left >= LBase && left < LBase + LCount && right >= VBase && right < VBase + VCount)
        {{
            return SBase + ((left - LBase) * VCount + right - VBase) * TCount;
        }}
        if (left >= SBase && left < SBase + SCount && (left - SBase) % TCount == 0 && right > TBase && right < TBase + TCount)
        {{
            return left + right - TBase;
        }}
        return Composition.GetValueOrDefault(PairKey(left, right), 0) is var result && result != 0 ? result : null;
    }}

    private static int CombiningClass(int scalar) => Combining.GetValueOrDefault(scalar, 0);

    private static string Map(string value, IReadOnlyDictionary<int, int[]> table)
    {{
        var output = new StringBuilder(value.Length);
        foreach (var rune in value.EnumerateRunes())
        {{
            if (table.TryGetValue(rune.Value, out var mapping))
            {{
                foreach (var scalar in mapping)
                {{
                    output.Append(new Rune(scalar));
                }}
            }}
            else
            {{
                output.Append(rune);
            }}
        }}
        return output.ToString();
    }}

    private static string FromScalars(IEnumerable<int> scalars)
    {{
        var output = new StringBuilder();
        foreach (var scalar in scalars)
        {{
            output.Append(new Rune(scalar));
        }}
        return output.ToString();
    }}

    private static Dictionary<int, int> ParseCombining()
    {{
        var result = new Dictionary<int, int>();
        foreach (var line in Lines(CombiningData))
        {{
            var fields = line.Split(';');
            result.Add(ParseHex(fields[0]), int.Parse(fields[1], CultureInfo.InvariantCulture));
        }}
        return result;
    }}

    private static Dictionary<int, (bool Compatibility, int[] Mapping)> ParseDecomposition()
    {{
        var result = new Dictionary<int, (bool Compatibility, int[] Mapping)>();
        foreach (var line in Lines(DecompositionData))
        {{
            var fields = line.Split(';');
            result.Add(ParseHex(fields[0]), (fields[1] == "K", ParseHexList(fields[2])));
        }}
        return result;
    }}

    private static Dictionary<long, int> ParseComposition()
    {{
        var result = new Dictionary<long, int>();
        foreach (var line in Lines(CompositionData))
        {{
            var fields = line.Split(';');
            var pair = fields[0].Split(',');
            result.Add(PairKey(ParseHex(pair[0]), ParseHex(pair[1])), ParseHex(fields[1]));
        }}
        return result;
    }}

    private static Dictionary<int, int[]> ParseMapping(string data)
    {{
        var result = new Dictionary<int, int[]>();
        foreach (var line in Lines(data))
        {{
            var fields = line.Split(';');
            result.Add(ParseHex(fields[0]), ParseHexList(fields[1]));
        }}
        return result;
    }}

    private static IEnumerable<string> Lines(string data)
    {{
        using var reader = new StringReader(data);
        while (reader.ReadLine() is {{ }} line)
        {{
            if (line.Length > 0)
            {{
                yield return line;
            }}
        }}
    }}

    private static int[] ParseHexList(string value) => value.Split(',').Select(ParseHex).ToArray();

    private static int ParseHex(string value) => int.Parse(value, NumberStyles.HexNumber, CultureInfo.InvariantCulture);

    private static long PairKey(int left, int right) => ((long)left << 21) | (uint)right;
}}
'''


def _typescript_raw(value: str) -> str:
    return f"`{value}`"


def _render_typescript(
    tables: tuple[
        list[tuple[int, int]],
        list[tuple[int, bool, tuple[int, ...]]],
        list[tuple[int, int, int]],
        list[tuple[int, tuple[int, ...]]],
        list[tuple[int, tuple[int, ...]]],
    ],
) -> str:
    combining, decomposition, composition, folding, uppercase = tables
    combining_text = "\n".join(f"{cp:X};{ccc}" for cp, ccc in combining)
    decomposition_text = "\n".join(
        f"{cp:X};{'K' if compat else 'C'};{','.join(f'{value:X}' for value in mapping)}"
        for cp, compat, mapping in decomposition
    )
    composition_text = "\n".join(
        f"{left:X},{right:X};{result:X}" for left, right, result in composition
    )
    folding_text = _mapping_lines(folding)
    uppercase_text = _mapping_lines(uppercase)
    hashes = ", ".join(f"{name} sha256:{digest}" for name, digest in SOURCES.items())
    return f'''// Generated Unicode {UNICODE_VERSION} data and algorithms.
// DO NOT EDIT. Run `python code/scripts/generate_tracked_artifact_unicode17.py`.
// Sources: {UCD_BASE}
// {hashes}
// Unicode License v3: every source and binary distribution carries the full
// notice as UNICODE-LICENSE.txt (sha256:{LICENSE_SHA256}).

export const UNICODE_VERSION = "{UNICODE_VERSION}";

const combiningData = {_typescript_raw(combining_text)};
const decompositionData = {_typescript_raw(decomposition_text)};
const compositionData = {_typescript_raw(composition_text)};
const foldingData = {_typescript_raw(folding_text)};
const uppercaseData = {_typescript_raw(uppercase_text)};

interface DecompositionRow {{
  readonly compatibility: boolean;
  readonly mapping: readonly number[];
}}

function lines(data: string): string[] {{
  return data.length === 0 ? [] : data.split("\\n");
}}

function parseHex(value: string): number {{
  return Number.parseInt(value, 16);
}}

function parseHexList(value: string): number[] {{
  return value.length === 0 ? [] : value.split(",").map(parseHex);
}}

function parseCombining(): Map<number, number> {{
  const result = new Map<number, number>();
  for (const line of lines(combiningData)) {{
    const [scalar, value] = line.split(";");
    result.set(parseHex(scalar), Number.parseInt(value, 10));
  }}
  return result;
}}

function parseDecomposition(): Map<number, DecompositionRow> {{
  const result = new Map<number, DecompositionRow>();
  for (const line of lines(decompositionData)) {{
    const [scalar, kind, mapping] = line.split(";");
    result.set(parseHex(scalar), {{
      compatibility: kind === "K",
      mapping: parseHexList(mapping),
    }});
  }}
  return result;
}}

function pairKey(left: number, right: number): number {{
  return left * 0x110000 + right;
}}

function parseComposition(): Map<number, number> {{
  const result = new Map<number, number>();
  for (const line of lines(compositionData)) {{
    const [pair, composite] = line.split(";");
    const [left, right] = pair.split(",");
    result.set(pairKey(parseHex(left), parseHex(right)), parseHex(composite));
  }}
  return result;
}}

function parseMapping(data: string): Map<number, readonly number[]> {{
  const result = new Map<number, readonly number[]>();
  for (const line of lines(data)) {{
    const [scalar, mapping] = line.split(";");
    result.set(parseHex(scalar), parseHexList(mapping));
  }}
  return result;
}}

const combining = parseCombining();
const decomposition = parseDecomposition();
const composition = parseComposition();
const folding = parseMapping(foldingData);
const uppercase = parseMapping(uppercaseData);

const sBase = 0xac00;
const lBase = 0x1100;
const vBase = 0x1161;
const tBase = 0x11a7;
const lCount = 19;
const vCount = 21;
const tCount = 28;
const nCount = vCount * tCount;
const sCount = lCount * nCount;

function combiningClass(scalar: number): number {{
  return combining.get(scalar) ?? 0;
}}

function scalarValues(value: string): number[] {{
  const result: number[] = [];
  for (const character of value) {{
    result.push(character.codePointAt(0)!);
  }}
  return result;
}}

function fromScalars(scalars: readonly number[]): string {{
  return scalars.map((scalar) => String.fromCodePoint(scalar)).join("");
}}

function decomposeScalar(
  scalar: number,
  compatibility: boolean,
  output: number[],
): void {{
  if (scalar >= sBase && scalar < sBase + sCount) {{
    const index = scalar - sBase;
    output.push(lBase + Math.floor(index / nCount));
    output.push(vBase + Math.floor((index % nCount) / tCount));
    const trailing = tBase + (index % tCount);
    if (trailing !== tBase) {{
      output.push(trailing);
    }}
    return;
  }}

  const row = decomposition.get(scalar);
  if (row === undefined || (row.compatibility && !compatibility)) {{
    output.push(scalar);
    return;
  }}
  for (const mapped of row.mapping) {{
    decomposeScalar(mapped, compatibility, output);
  }}
}}

function canonicalOrder(scalars: number[]): void {{
  let index = 0;
  while (index < scalars.length) {{
    if (combiningClass(scalars[index]) === 0) {{
      index += 1;
      continue;
    }}
    let end = index + 1;
    while (end < scalars.length && combiningClass(scalars[end]) !== 0) {{
      end += 1;
    }}
    for (let current = index + 1; current < end; current += 1) {{
      const value = scalars[current];
      const valueClass = combiningClass(value);
      let insertion = current;
      while (
        insertion > index &&
        combiningClass(scalars[insertion - 1]) > valueClass
      ) {{
        scalars[insertion] = scalars[insertion - 1];
        insertion -= 1;
      }}
      scalars[insertion] = value;
    }}
    index = end;
  }}
}}

function composePair(left: number, right: number): number | undefined {{
  if (
    left >= lBase &&
    left < lBase + lCount &&
    right >= vBase &&
    right < vBase + vCount
  ) {{
    return sBase + ((left - lBase) * vCount + right - vBase) * tCount;
  }}
  if (
    left >= sBase &&
    left < sBase + sCount &&
    (left - sBase) % tCount === 0 &&
    right > tBase &&
    right < tBase + tCount
  ) {{
    return left + right - tBase;
  }}
  return composition.get(pairKey(left, right));
}}

function normalize(value: string, compatibility: boolean): string {{
  const decomposed: number[] = [];
  for (const scalar of scalarValues(value)) {{
    decomposeScalar(scalar, compatibility, decomposed);
  }}
  canonicalOrder(decomposed);
  if (decomposed.length === 0) {{
    return "";
  }}

  const output = [decomposed[0]];
  let starterIndex = 0;
  let starter = decomposed[0];
  let lastClass = combiningClass(starter) === 0 ? 0 : 255;
  for (const scalar of decomposed.slice(1)) {{
    const scalarClass = combiningClass(scalar);
    const composite =
      lastClass === 0 || lastClass < scalarClass
        ? composePair(starter, scalar)
        : undefined;
    if (composite !== undefined) {{
      output[starterIndex] = composite;
      starter = composite;
      continue;
    }}
    output.push(scalar);
    if (scalarClass === 0) {{
      starterIndex = output.length - 1;
      starter = scalar;
    }}
    lastClass = scalarClass;
  }}
  return fromScalars(output);
}}

function mapScalars(
  value: string,
  table: ReadonlyMap<number, readonly number[]>,
): string {{
  const output: number[] = [];
  for (const scalar of scalarValues(value)) {{
    output.push(...(table.get(scalar) ?? [scalar]));
  }}
  return fromScalars(output);
}}

export function nfc(value: string): string {{
  return normalize(value, false);
}}

export function nfkc(value: string): string {{
  return normalize(value, true);
}}

export function casefold(value: string): string {{
  return mapScalars(value, folding);
}}

export function nfkcCasefold(value: string): string {{
  return casefold(nfkc(value));
}}

export function fullUppercase(value: string): string {{
  return mapScalars(value, uppercase);
}}
'''


def _render_ruby(
    tables: tuple[
        list[tuple[int, int]],
        list[tuple[int, bool, tuple[int, ...]]],
        list[tuple[int, int, int]],
        list[tuple[int, tuple[int, ...]]],
        list[tuple[int, tuple[int, ...]]],
    ],
) -> str:
    """Render the process-free Ruby implementation from the pinned UCD rows."""
    combining, decomposition, composition, folding, uppercase = tables
    combining_text = "\n".join(f"      {cp:X};{ccc}" for cp, ccc in combining)
    decomposition_text = "\n".join(
        f"      {cp:X};{'K' if compat else 'C'};{','.join(f'{value:X}' for value in mapping)}"
        for cp, compat, mapping in decomposition
    )
    composition_text = "\n".join(
        f"      {left:X},{right:X};{result:X}" for left, right, result in composition
    )
    folding_text = "\n".join(
        f"      {line}" for line in _mapping_lines(folding).splitlines()
    )
    uppercase_text = "\n".join(
        f"      {line}" for line in _mapping_lines(uppercase).splitlines()
    )
    hashes = ", ".join(f"{name} sha256:{digest}" for name, digest in SOURCES.items())
    header = f'''# frozen_string_literal: true

# Generated Unicode {UNICODE_VERSION} data and algorithms.
# DO NOT EDIT. Run `python code/scripts/generate_tracked_artifact_unicode17.py`.
# Sources: {UCD_BASE}
# {hashes}
# Unicode License v3: every source and binary distribution carries the full
# notice as UNICODE-LICENSE.txt (sha256:{LICENSE_SHA256}).

module BuildTool
  # A deliberately source-embedded Unicode snapshot for validation policy.
  #
  # Ruby's host tables move with the runtime. Keeping normalization and casing
  # here makes one validator result independent of the installed Ruby version.
  module TrackedArtifactUnicode17
    module_function

    UNICODE_VERSION = "{UNICODE_VERSION}"

    COMBINING_DATA = <<~UNICODE_COMBINING
{combining_text}
    UNICODE_COMBINING
    DECOMPOSITION_DATA = <<~UNICODE_DECOMPOSITION
{decomposition_text}
    UNICODE_DECOMPOSITION
    COMPOSITION_DATA = <<~UNICODE_COMPOSITION
{composition_text}
    UNICODE_COMPOSITION
    FOLDING_DATA = <<~UNICODE_FOLDING
{folding_text}
    UNICODE_FOLDING
    UPPERCASE_DATA = <<~UNICODE_UPPERCASE
{uppercase_text}
    UNICODE_UPPERCASE
'''
    body = r"""
    COMBINING_DATA.freeze
    DECOMPOSITION_DATA.freeze
    COMPOSITION_DATA.freeze
    FOLDING_DATA.freeze
    UPPERCASE_DATA.freeze

    S_BASE = 0xAC00
    L_BASE = 0x1100
    V_BASE = 0x1161
    T_BASE = 0x11A7
    L_COUNT = 19
    V_COUNT = 21
    T_COUNT = 28
    N_COUNT = V_COUNT * T_COUNT
    S_COUNT = L_COUNT * N_COUNT

    def data_lines(data)
      data.each_line(chomp: true).reject(&:empty?)
    end

    def parse_hex_list(value)
      return [] if value.empty?

      value.split(",").map { |item| Integer(item, 16) }
    end

    def parse_combining
      data_lines(COMBINING_DATA).to_h do |line|
        scalar, value = line.split(";", 2)
        [Integer(scalar, 16), Integer(value, 10)]
      end.freeze
    end

    def parse_decomposition
      data_lines(DECOMPOSITION_DATA).to_h do |line|
        scalar, kind, mapping = line.split(";", 3)
        [Integer(scalar, 16), [kind == "K", parse_hex_list(mapping).freeze].freeze]
      end.freeze
    end

    def pair_key(left, right)
      (left * 0x110000) + right
    end

    def parse_composition
      data_lines(COMPOSITION_DATA).to_h do |line|
        pair, composite = line.split(";", 2)
        left, right = pair.split(",", 2)
        [pair_key(Integer(left, 16), Integer(right, 16)), Integer(composite, 16)]
      end.freeze
    end

    def parse_mapping(data)
      data_lines(data).to_h do |line|
        scalar, mapping = line.split(";", 2)
        [Integer(scalar, 16), parse_hex_list(mapping).freeze]
      end.freeze
    end

    COMBINING = parse_combining
    DECOMPOSITION = parse_decomposition
    COMPOSITION = parse_composition
    FOLDING = parse_mapping(FOLDING_DATA)
    UPPERCASE = parse_mapping(UPPERCASE_DATA)

    def combining_class(scalar)
      COMBINING.fetch(scalar, 0)
    end

    def decompose_scalar(scalar, compatibility, output)
      if scalar >= S_BASE && scalar < S_BASE + S_COUNT
        index = scalar - S_BASE
        output << L_BASE + (index / N_COUNT)
        output << V_BASE + ((index % N_COUNT) / T_COUNT)
        trailing = T_BASE + (index % T_COUNT)
        output << trailing unless trailing == T_BASE
        return
      end

      row = DECOMPOSITION[scalar]
      if row.nil? || (row[0] && !compatibility)
        output << scalar
        return
      end
      row[1].each { |mapped| decompose_scalar(mapped, compatibility, output) }
    end

    def canonical_order(scalars)
      index = 0
      while index < scalars.length
        if combining_class(scalars[index]).zero?
          index += 1
          next
        end

        ending = index + 1
        ending += 1 while ending < scalars.length && !combining_class(scalars[ending]).zero?
        ((index + 1)...ending).each do |current|
          value = scalars[current]
          value_class = combining_class(value)
          insertion = current
          while insertion > index && combining_class(scalars[insertion - 1]) > value_class
            scalars[insertion] = scalars[insertion - 1]
            insertion -= 1
          end
          scalars[insertion] = value
        end
        index = ending
      end
    end

    def compose_pair(left, right)
      if left >= L_BASE && left < L_BASE + L_COUNT &&
          right >= V_BASE && right < V_BASE + V_COUNT
        return S_BASE + (((left - L_BASE) * V_COUNT) + right - V_BASE) * T_COUNT
      end
      if left >= S_BASE && left < S_BASE + S_COUNT &&
          ((left - S_BASE) % T_COUNT).zero? && right > T_BASE && right < T_BASE + T_COUNT
        return left + right - T_BASE
      end

      COMPOSITION[pair_key(left, right)]
    end

    def normalize(value, compatibility)
      decomposed = []
      value.codepoints.each { |scalar| decompose_scalar(scalar, compatibility, decomposed) }
      canonical_order(decomposed)
      return "" if decomposed.empty?

      output = [decomposed[0]]
      starter_index = 0
      starter = decomposed[0]
      last_class = combining_class(starter).zero? ? 0 : 255
      decomposed.drop(1).each do |scalar|
        scalar_class = combining_class(scalar)
        composite = compose_pair(starter, scalar) if last_class.zero? || last_class < scalar_class
        unless composite.nil?
          output[starter_index] = composite
          starter = composite
          next
        end

        output << scalar
        if scalar_class.zero?
          starter_index = output.length - 1
          starter = scalar
        end
        last_class = scalar_class
      end
      output.pack("U*")
    end

    def map_scalars(value, table)
      value.codepoints.flat_map { |scalar| table.fetch(scalar, [scalar]) }.pack("U*")
    end

    def nfc(value)
      normalize(value, false)
    end

    def nfkc(value)
      normalize(value, true)
    end

    def casefold(value)
      map_scalars(value, FOLDING)
    end

    def nfkc_casefold(value)
      casefold(nfkc(value))
    end

    def full_uppercase(value)
      map_scalars(value, UPPERCASE)
    end
  end
end
"""
    return header + body


def _render_elixir(
    tables: tuple[
        list[tuple[int, int]],
        list[tuple[int, bool, tuple[int, ...]]],
        list[tuple[int, int, int]],
        list[tuple[int, tuple[int, ...]]],
        list[tuple[int, tuple[int, ...]]],
    ],
) -> str:
    """Render the process-free Elixir implementation from the pinned UCD rows."""
    combining, decomposition, composition, folding, uppercase = tables

    combining_text = ",\n".join(
        f"    0x{scalar:X} => {combining_class}"
        for scalar, combining_class in combining
    )

    decomposition_rows = []
    for scalar, compatibility, mapping in decomposition:
        rendered_mapping = ", ".join(f"0x{value:X}" for value in mapping)
        row = (
            f"    0x{scalar:X} => "
            f"{{{str(compatibility).lower()}, [{rendered_mapping}]}}"
        )
        if len(row) <= 98:
            decomposition_rows.append(row)
        else:
            mapping_lines = ",\n".join(f"         0x{value:X}" for value in mapping)
            decomposition_rows.append(
                f"    0x{scalar:X} =>\n"
                f"      {{{str(compatibility).lower()},\n"
                f"       [\n{mapping_lines}\n       ]}}"
            )
    decomposition_text = ",\n".join(decomposition_rows)

    composition_text = ",\n".join(
        f"    0x{(left * 0x110000) + right:X} => 0x{result:X}"
        for left, right, result in composition
    )
    folding_text = ",\n".join(
        f"    0x{scalar:X} => [{', '.join(f'0x{value:X}' for value in mapping)}]"
        for scalar, mapping in folding
    )
    uppercase_text = ",\n".join(
        f"    0x{scalar:X} => [{', '.join(f'0x{value:X}' for value in mapping)}]"
        for scalar, mapping in uppercase
    )
    hashes = ", ".join(f"{name} sha256:{digest}" for name, digest in SOURCES.items())
    header = f'''# Generated Unicode {UNICODE_VERSION} data and algorithms.
# DO NOT EDIT. Run `python code/scripts/generate_tracked_artifact_unicode17.py`.
# Sources: {UCD_BASE}
# {hashes}
# Unicode License v3: every source and binary distribution carries the full
# notice as UNICODE-LICENSE.txt (sha256:{LICENSE_SHA256}).

defmodule BuildTool.TrackedArtifactUnicode17 do
  @moduledoc false

  # Elixir and Erlang host tables move with the runtime. These generated maps
  # keep validation results independent of the installed BEAM version.
  @unicode_version "{UNICODE_VERSION}"
  @combining %{{
{combining_text}
  }}
  @decomposition %{{
{decomposition_text}
  }}
  @composition %{{
{composition_text}
  }}
  @folding %{{
{folding_text}
  }}
  @uppercase %{{
{uppercase_text}
  }}
'''
    body = r"""
  @s_base 0xAC00
  @l_base 0x1100
  @v_base 0x1161
  @t_base 0x11A7
  @l_count 19
  @v_count 21
  @t_count 28
  @n_count @v_count * @t_count
  @s_count @l_count * @n_count

  def unicode_version, do: @unicode_version

  def nfc(value), do: normalize(value, false)
  def nfkc(value), do: normalize(value, true)
  def casefold(value), do: map_scalars(value, @folding)
  def nfkc_casefold(value), do: value |> nfkc() |> casefold()
  def full_uppercase(value), do: map_scalars(value, @uppercase)

  defp combining_class(scalar), do: Map.get(@combining, scalar, 0)

  defp decompose_scalar(scalar, _compatibility)
       when scalar >= @s_base and scalar < @s_base + @s_count do
    index = scalar - @s_base
    leading = @l_base + div(index, @n_count)
    vowel = @v_base + div(rem(index, @n_count), @t_count)
    trailing = @t_base + rem(index, @t_count)

    if trailing == @t_base do
      [leading, vowel]
    else
      [leading, vowel, trailing]
    end
  end

  defp decompose_scalar(scalar, compatibility) do
    case Map.get(@decomposition, scalar) do
      nil ->
        [scalar]

      {true, _mapping} when not compatibility ->
        [scalar]

      {_compatibility_mapping, mapping} ->
        Enum.flat_map(mapping, &decompose_scalar(&1, compatibility))
    end
  end

  defp canonical_order(scalars) do
    Enum.reduce(scalars, [], fn scalar, ordered ->
      scalar_class = combining_class(scalar)

      if scalar_class == 0 do
        ordered ++ [scalar]
      else
        insert_combining(ordered, scalar, scalar_class)
      end
    end)
  end

  defp insert_combining(ordered, scalar, scalar_class) do
    {later, earlier_reversed} =
      ordered
      |> Enum.reverse()
      |> Enum.split_while(fn previous ->
        previous_class = combining_class(previous)
        previous_class != 0 and previous_class > scalar_class
      end)

    Enum.reverse(earlier_reversed) ++ [scalar] ++ Enum.reverse(later)
  end

  defp pair_key(left, right), do: left * 0x110000 + right

  defp compose_pair(left, right)
       when left >= @l_base and left < @l_base + @l_count and
              right >= @v_base and right < @v_base + @v_count do
    @s_base + ((left - @l_base) * @v_count + right - @v_base) * @t_count
  end

  defp compose_pair(left, right)
       when left >= @s_base and left < @s_base + @s_count and
              rem(left - @s_base, @t_count) == 0 and right > @t_base and
              right < @t_base + @t_count do
    left + right - @t_base
  end

  defp compose_pair(left, right), do: Map.get(@composition, pair_key(left, right))

  defp compose([]), do: []

  defp compose([first | rest]) do
    last_class = if combining_class(first) == 0, do: 0, else: 255

    {output, _starter_index, _starter, _last_class} =
      Enum.reduce(rest, {[first], 0, first, last_class}, fn scalar,
                                                            {output, starter_index, starter,
                                                             previous_class} ->
        scalar_class = combining_class(scalar)

        composite =
          if previous_class == 0 or previous_class < scalar_class do
            compose_pair(starter, scalar)
          end

        if is_nil(composite) do
          output = output ++ [scalar]

          if scalar_class == 0 do
            {output, length(output) - 1, scalar, scalar_class}
          else
            {output, starter_index, starter, scalar_class}
          end
        else
          {List.replace_at(output, starter_index, composite), starter_index, composite,
           previous_class}
        end
      end)

    output
  end

  defp normalize(value, compatibility) do
    value
    |> String.to_charlist()
    |> Enum.flat_map(&decompose_scalar(&1, compatibility))
    |> canonical_order()
    |> compose()
    |> List.to_string()
  end

  defp map_scalars(value, table) do
    value
    |> String.to_charlist()
    |> Enum.flat_map(fn scalar -> Map.get(table, scalar, [scalar]) end)
    |> List.to_string()
  end
end
"""
    return header + body


def _render_lua(
    tables: tuple[
        list[tuple[int, int]],
        list[tuple[int, bool, tuple[int, ...]]],
        list[tuple[int, int, int]],
        list[tuple[int, tuple[int, ...]]],
        list[tuple[int, tuple[int, ...]]],
    ],
) -> str:
    """Render the process-free Lua implementation from the pinned UCD rows."""
    combining, decomposition, composition, folding, uppercase = tables
    combining_text = "\n".join(f"{cp:X};{ccc}" for cp, ccc in combining)
    decomposition_text = "\n".join(
        f"{cp:X};{'K' if compat else 'C'};{','.join(f'{value:X}' for value in mapping)}"
        for cp, compat, mapping in decomposition
    )
    composition_text = "\n".join(
        f"{left:X},{right:X};{result:X}" for left, right, result in composition
    )
    folding_text = _mapping_lines(folding)
    uppercase_text = _mapping_lines(uppercase)
    hashes = ", ".join(f"{name} sha256:{digest}" for name, digest in SOURCES.items())
    header = f'''-- Generated Unicode {UNICODE_VERSION} data and algorithms.
-- DO NOT EDIT. Run `python code/scripts/generate_tracked_artifact_unicode17.py`.
-- Sources: {UCD_BASE}
-- {hashes}
-- Unicode License v3: every source and binary distribution carries the full
-- notice as UNICODE-LICENSE.txt (sha256:{LICENSE_SHA256}).

-- Lua's host runtime has no pinned normalization or full-casing tables. These
-- source-embedded rows keep validator results independent of installed modules.
local Unicode = {{}}

Unicode.UNICODE_VERSION = "{UNICODE_VERSION}"

local COMBINING_DATA = [[
{combining_text}
]]
local DECOMPOSITION_DATA = [[
{decomposition_text}
]]
local COMPOSITION_DATA = [[
{composition_text}
]]
local FOLDING_DATA = [[
{folding_text}
]]
local UPPERCASE_DATA = [[
{uppercase_text}
]]
'''
    body = r"""
local S_BASE = 0xAC00
local L_BASE = 0x1100
local V_BASE = 0x1161
local T_BASE = 0x11A7
local L_COUNT = 19
local V_COUNT = 21
local T_COUNT = 28
local N_COUNT = V_COUNT * T_COUNT
local S_COUNT = L_COUNT * N_COUNT

local function data_lines(data)
    return data:gmatch("[^\n]+")
end

local function parse_hex_list(value)
    local values = {}
    for item in value:gmatch("[^,]+") do
        values[#values + 1] = assert(tonumber(item, 16))
    end
    return values
end

local function parse_combining()
    local result = {}
    for line in data_lines(COMBINING_DATA) do
        local scalar, value = line:match("^([^;]+);([^;]+)$")
        result[assert(tonumber(scalar, 16))] = assert(tonumber(value, 10))
    end
    return result
end

local function parse_decomposition()
    local result = {}
    for line in data_lines(DECOMPOSITION_DATA) do
        local scalar, kind, mapping = line:match("^([^;]+);([^;]+);(.*)$")
        result[assert(tonumber(scalar, 16))] = {
            compatibility = kind == "K",
            mapping = parse_hex_list(mapping),
        }
    end
    return result
end

local function pair_key(left, right)
    return left * 0x110000 + right
end

local function parse_composition()
    local result = {}
    for line in data_lines(COMPOSITION_DATA) do
        local left, right, composite = line:match("^([^,]+),([^;]+);([^;]+)$")
        result[pair_key(assert(tonumber(left, 16)), assert(tonumber(right, 16)))] =
            assert(tonumber(composite, 16))
    end
    return result
end

local function parse_mapping(data)
    local result = {}
    for line in data_lines(data) do
        local scalar, mapping = line:match("^([^;]+);(.*)$")
        result[assert(tonumber(scalar, 16))] = parse_hex_list(mapping)
    end
    return result
end

local COMBINING = parse_combining()
local DECOMPOSITION = parse_decomposition()
local COMPOSITION = parse_composition()
local FOLDING = parse_mapping(FOLDING_DATA)
local UPPERCASE = parse_mapping(UPPERCASE_DATA)

local function combining_class(scalar)
    return COMBINING[scalar] or 0
end

local function decompose_scalar(scalar, compatibility, output)
    if scalar >= S_BASE and scalar < S_BASE + S_COUNT then
        local index = scalar - S_BASE
        output[#output + 1] = L_BASE + index // N_COUNT
        output[#output + 1] = V_BASE + (index % N_COUNT) // T_COUNT
        local trailing = T_BASE + index % T_COUNT
        if trailing ~= T_BASE then
            output[#output + 1] = trailing
        end
        return
    end

    local row = DECOMPOSITION[scalar]
    if row == nil or (row.compatibility and not compatibility) then
        output[#output + 1] = scalar
        return
    end
    for _, mapped in ipairs(row.mapping) do
        decompose_scalar(mapped, compatibility, output)
    end
end

local function canonical_order(scalars)
    local index = 1
    while index <= #scalars do
        if combining_class(scalars[index]) == 0 then
            index = index + 1
        else
            local ending = index + 1
            while ending <= #scalars and combining_class(scalars[ending]) ~= 0 do
                ending = ending + 1
            end
            for current = index + 1, ending - 1 do
                local value = scalars[current]
                local value_class = combining_class(value)
                local insertion = current
                while insertion > index and
                    combining_class(scalars[insertion - 1]) > value_class
                do
                    scalars[insertion] = scalars[insertion - 1]
                    insertion = insertion - 1
                end
                scalars[insertion] = value
            end
            index = ending
        end
    end
end

local function compose_pair(left, right)
    if left >= L_BASE and left < L_BASE + L_COUNT and
        right >= V_BASE and right < V_BASE + V_COUNT
    then
        return S_BASE + ((left - L_BASE) * V_COUNT + right - V_BASE) * T_COUNT
    end
    if left >= S_BASE and left < S_BASE + S_COUNT and
        (left - S_BASE) % T_COUNT == 0 and right > T_BASE and
        right < T_BASE + T_COUNT
    then
        return left + right - T_BASE
    end
    return COMPOSITION[pair_key(left, right)]
end

local function from_scalars(scalars)
    local chunks = {}
    for index, scalar in ipairs(scalars) do
        chunks[index] = utf8.char(scalar)
    end
    return table.concat(chunks)
end

local function normalize(value, compatibility)
    local decomposed = {}
    for _, scalar in utf8.codes(value) do
        decompose_scalar(scalar, compatibility, decomposed)
    end
    canonical_order(decomposed)
    if #decomposed == 0 then
        return ""
    end

    local output = {decomposed[1]}
    local starter_index = 1
    local starter = decomposed[1]
    local last_class = combining_class(starter) == 0 and 0 or 255
    for index = 2, #decomposed do
        local scalar = decomposed[index]
        local scalar_class = combining_class(scalar)
        local composite
        if last_class == 0 or last_class < scalar_class then
            composite = compose_pair(starter, scalar)
        end
        if composite ~= nil then
            output[starter_index] = composite
            starter = composite
        else
            output[#output + 1] = scalar
            if scalar_class == 0 then
                starter_index = #output
                starter = scalar
            end
            last_class = scalar_class
        end
    end
    return from_scalars(output)
end

local function map_scalars(value, mapping)
    local output = {}
    for _, scalar in utf8.codes(value) do
        local mapped = mapping[scalar]
        if mapped == nil then
            output[#output + 1] = scalar
        else
            for _, mapped_scalar in ipairs(mapped) do
                output[#output + 1] = mapped_scalar
            end
        end
    end
    return from_scalars(output)
end

function Unicode.nfc(value)
    return normalize(value, false)
end

function Unicode.nfkc(value)
    return normalize(value, true)
end

function Unicode.casefold(value)
    return map_scalars(value, FOLDING)
end

function Unicode.nfkc_casefold(value)
    return Unicode.casefold(Unicode.nfkc(value))
end

function Unicode.full_uppercase(value)
    return map_scalars(value, UPPERCASE)
end

return Unicode
"""
    return header + body


def _load_generated_module(path: Path):
    spec = importlib.util.spec_from_file_location("tracked_unicode17_generated", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load generated module {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _scalars(field: str) -> str:
    return "".join(chr(int(value, 16)) for value in field.split())


def _self_check(module, sources: dict[str, str]) -> None:
    if module.UNICODE_VERSION != UNICODE_VERSION:
        raise RuntimeError("generated Unicode version drift")
    for line in sources["NormalizationTest.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload or payload.startswith("@"):
            continue
        columns = [_scalars(field.strip()) for field in payload.split(";")[:5]]
        c1, c2, c3, c4, c5 = columns
        if not (
            module.nfc(c1) == c2
            and module.nfc(c2) == c2
            and module.nfc(c3) == c2
            and module.nfc(c4) == c4
            and module.nfc(c5) == c4
            and module.nfkc(c1) == c4
            and module.nfkc(c2) == c4
            and module.nfkc(c3) == c4
            and module.nfkc(c4) == c4
            and module.nfkc(c5) == c4
        ):
            raise RuntimeError(f"normalization self-check failed: {line}")
    for line in sources["CaseFolding.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[1] not in {"C", "F"}:
            continue
        source = chr(int(fields[0], 16))
        expected = _scalars(fields[2])
        if module.casefold(source) != expected:
            raise RuntimeError(f"case-fold self-check failed: {line}")
    uppercase: dict[int, tuple[int, ...]] = {}
    for line in sources["UnicodeData.txt"].splitlines():
        fields = line.split(";")
        if fields[12]:
            uppercase[int(fields[0], 16)] = (int(fields[12], 16),)
    for line in sources["SpecialCasing.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[4]:
            continue
        uppercase[int(fields[0], 16)] = tuple(
            int(value, 16) for value in fields[3].split()
        )
    for scalar, mapping in uppercase.items():
        source = chr(scalar)
        expected = "".join(chr(value) for value in mapping)
        if module.full_uppercase(source) != expected:
            raise RuntimeError(f"full-uppercase self-check failed: U+{scalar:04X}")
    outlined = "".join(
        chr(0x1CCD6 + ord(character) - ord("A")) if character != "_" else "_"
        for character in "NODE_MODULES"
    )
    if module.nfkc_casefold(outlined) != "node_modules":
        raise RuntimeError("Unicode 17 outlined-letter sentinel failed")
    if module.nfc(chr(0x105D2) + "\u0307") != chr(0x105C9):
        raise RuntimeError("Unicode 17 Todhri composition sentinel failed")


def _typescript_self_check_payload(module, sources: dict[str, str]) -> dict:
    normalization: list[list[str]] = []
    for line in sources["NormalizationTest.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload or payload.startswith("@"):
            continue
        normalization.append(
            [_scalars(field.strip()) for field in payload.split(";")[:5]]
        )

    folding: list[list[str]] = []
    for line in sources["CaseFolding.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[1] not in {"C", "F"}:
            continue
        source = chr(int(fields[0], 16))
        folding.append([source, _scalars(fields[2]), module.nfkc_casefold(source)])

    uppercase: dict[int, tuple[int, ...]] = {}
    for line in sources["UnicodeData.txt"].splitlines():
        fields = line.split(";")
        if fields[12]:
            uppercase[int(fields[0], 16)] = (int(fields[12], 16),)
    for line in sources["SpecialCasing.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[4]:
            continue
        uppercase[int(fields[0], 16)] = tuple(
            int(value, 16) for value in fields[3].split()
        )

    return {
        "unicodeVersion": UNICODE_VERSION,
        "normalization": normalization,
        "folding": folding,
        "uppercase": [
            [chr(scalar), "".join(chr(value) for value in mapping)]
            for scalar, mapping in sorted(uppercase.items())
        ],
    }


_TYPESCRIPT_SELF_CHECK = r"""import { readFileSync } from "node:fs";
import {
  UNICODE_VERSION,
  casefold,
  fullUppercase,
  nfc,
  nfkc,
  nfkcCasefold,
} from "./tracked-artifact-unicode17.ts";

type Payload = {
  unicodeVersion: string;
  normalization: string[][];
  folding: string[][];
  uppercase: string[][];
};

const payload = JSON.parse(readFileSync(0, "utf8")) as Payload;
const fail = (kind: string, index: number): never => {
  throw new Error(`${kind} TypeScript self-check failed at vector ${index}`);
};

if (UNICODE_VERSION !== payload.unicodeVersion) {
  throw new Error("generated TypeScript Unicode version drift");
}
for (const [index, row] of payload.normalization.entries()) {
  const [c1, c2, c3, c4, c5] = row;
  if (
    nfc(c1) !== c2 || nfc(c2) !== c2 || nfc(c3) !== c2 ||
    nfc(c4) !== c4 || nfc(c5) !== c4 || nfkc(c1) !== c4 ||
    nfkc(c2) !== c4 || nfkc(c3) !== c4 || nfkc(c4) !== c4 ||
    nfkc(c5) !== c4
  ) fail("normalization", index);
}
for (const [index, row] of payload.folding.entries()) {
  const [source, expectedFold, expectedNfkcFold] = row;
  if (casefold(source) !== expectedFold) fail("case-fold", index);
  if (nfkcCasefold(source) !== expectedNfkcFold) fail("NFKC-case-fold", index);
}
for (const [index, row] of payload.uppercase.entries()) {
  if (fullUppercase(row[0]) !== row[1]) fail("full-uppercase", index);
}

const outlined = "\u{1CCE3}\u{1CCE4}\u{1CCD9}\u{1CCDA}_\u{1CCE2}\u{1CCE4}\u{1CCD9}\u{1CCEA}\u{1CCE1}\u{1CCDA}\u{1CCE8}";
if (nfkcCasefold(outlined) !== "node_modules") {
  throw new Error("Unicode 17 outlined-letter TypeScript sentinel failed");
}
if (nfc("\u{105D2}\u0307") !== "\u{105C9}") {
  throw new Error("Unicode 17 Todhri TypeScript sentinel failed");
}
process.stdout.write("ok\n");
"""


def _self_check_typescript(
    root: Path,
    typescript_output: str,
    sources: dict[str, str],
    python_module,
) -> None:
    node = shutil.which("node")
    tsx_cli = root / "code/programs/typescript/build-tool/node_modules/tsx/dist/cli.mjs"
    if node is None or not tsx_cli.is_file():
        raise RuntimeError(
            "TypeScript Unicode self-check requires Node.js and the pinned build-tool "
            "dependencies; run `npm ci` in code/programs/typescript/build-tool"
        )

    with tempfile.TemporaryDirectory(prefix="unicode17-typescript-check-") as temporary:
        temporary_path = Path(temporary)
        generated_path = temporary_path / "tracked-artifact-unicode17.ts"
        runner_path = temporary_path / "self-check.ts"
        generated_path.write_text(typescript_output, encoding="utf-8", newline="\n")
        runner_path.write_text(_TYPESCRIPT_SELF_CHECK, encoding="utf-8", newline="\n")
        result = subprocess.run(
            [node, str(tsx_cli), str(runner_path)],
            cwd=temporary_path,
            input=json.dumps(
                _typescript_self_check_payload(python_module, sources),
                ensure_ascii=True,
                separators=(",", ":"),
            ),
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=180,
            check=False,
        )
    if result.returncode != 0 or result.stdout != "ok\n":
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic"
        raise RuntimeError(f"generated TypeScript Unicode self-check failed: {detail}")


_RUBY_SELF_CHECK = r"""# frozen_string_literal: true

require "json"
require_relative "tracked_artifact_unicode17"

unicode = BuildTool::TrackedArtifactUnicode17
payload = JSON.parse($stdin.read)
fail_vector = lambda do |kind, index|
  raise "#{kind} Ruby self-check failed at vector #{index}"
end

raise "generated Ruby Unicode version drift" unless unicode::UNICODE_VERSION == payload.fetch("unicodeVersion")

payload.fetch("normalization").each_with_index do |row, index|
  c1, c2, c3, c4, c5 = row
  valid = unicode.nfc(c1) == c2 && unicode.nfc(c2) == c2 &&
          unicode.nfc(c3) == c2 && unicode.nfc(c4) == c4 &&
          unicode.nfc(c5) == c4 && unicode.nfkc(c1) == c4 &&
          unicode.nfkc(c2) == c4 && unicode.nfkc(c3) == c4 &&
          unicode.nfkc(c4) == c4 && unicode.nfkc(c5) == c4
  fail_vector.call("normalization", index) unless valid
end

payload.fetch("folding").each_with_index do |row, index|
  source, expected_fold, expected_nfkc_fold = row
  fail_vector.call("case-fold", index) unless unicode.casefold(source) == expected_fold
  unless unicode.nfkc_casefold(source) == expected_nfkc_fold
    fail_vector.call("NFKC-case-fold", index)
  end
end

payload.fetch("uppercase").each_with_index do |row, index|
  fail_vector.call("full-uppercase", index) unless unicode.full_uppercase(row[0]) == row[1]
end

outlined = "\u{1CCE3}\u{1CCE4}\u{1CCD9}\u{1CCDA}_\u{1CCE2}\u{1CCE4}\u{1CCD9}\u{1CCEA}\u{1CCE1}\u{1CCDA}\u{1CCE8}"
unless unicode.nfkc_casefold(outlined) == "node_modules"
  raise "Unicode 17 outlined-letter Ruby sentinel failed"
end
unless unicode.nfc("\u{105D2}\u0307") == "\u{105C9}"
  raise "Unicode 17 Todhri Ruby sentinel failed"
end

$stdout.write("ok\n")
"""


def _self_check_ruby(
    root: Path,
    ruby_output: str,
    sources: dict[str, str],
    python_module,
) -> None:
    del root  # Kept parallel with the TypeScript self-check call signature.
    ruby = shutil.which("ruby")
    if ruby is None:
        raise RuntimeError("Ruby Unicode self-check requires Ruby on PATH")

    with tempfile.TemporaryDirectory(prefix="unicode17-ruby-check-") as temporary:
        temporary_path = Path(temporary)
        generated_path = temporary_path / "tracked_artifact_unicode17.rb"
        runner_path = temporary_path / "self_check.rb"
        generated_path.write_text(ruby_output, encoding="utf-8", newline="\n")
        runner_path.write_text(_RUBY_SELF_CHECK, encoding="utf-8", newline="\n")
        result = subprocess.run(
            [ruby, str(runner_path)],
            cwd=temporary_path,
            input=json.dumps(
                _typescript_self_check_payload(python_module, sources),
                ensure_ascii=True,
                separators=(",", ":"),
            ),
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=180,
            check=False,
        )
    if result.returncode != 0 or result.stdout != "ok\n":
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic"
        raise RuntimeError(f"generated Ruby Unicode self-check failed: {detail}")


_ELIXIR_SELF_CHECK = r"""Code.compiler_options(ignore_module_conflict: true)
Code.require_file("tracked_artifact_unicode17.ex", __DIR__)

unicode = BuildTool.TrackedArtifactUnicode17
payload = IO.read(:stdio, :eof) |> :json.decode()
fail_vector = fn kind, index -> raise "#{kind} Elixir self-check failed at vector #{index}" end

unless unicode.unicode_version() == payload["unicodeVersion"] do
  raise "generated Elixir Unicode version drift"
end

payload["normalization"]
|> Enum.with_index()
|> Enum.each(fn {[c1, c2, c3, c4, c5], index} ->
  valid =
    unicode.nfc(c1) == c2 and unicode.nfc(c2) == c2 and
      unicode.nfc(c3) == c2 and unicode.nfc(c4) == c4 and
      unicode.nfc(c5) == c4 and unicode.nfkc(c1) == c4 and
      unicode.nfkc(c2) == c4 and unicode.nfkc(c3) == c4 and
      unicode.nfkc(c4) == c4 and unicode.nfkc(c5) == c4

  unless valid, do: fail_vector.("normalization", index)
end)

payload["folding"]
|> Enum.with_index()
|> Enum.each(fn {[source, expected_fold, expected_nfkc_fold], index} ->
  unless unicode.casefold(source) == expected_fold, do: fail_vector.("case-fold", index)

  unless unicode.nfkc_casefold(source) == expected_nfkc_fold,
    do: fail_vector.("NFKC-case-fold", index)
end)

payload["uppercase"]
|> Enum.with_index()
|> Enum.each(fn {[source, expected], index} ->
  unless unicode.full_uppercase(source) == expected,
    do: fail_vector.("full-uppercase", index)
end)

outlined = "\u{1CCE3}\u{1CCE4}\u{1CCD9}\u{1CCDA}_\u{1CCE2}\u{1CCE4}\u{1CCD9}\u{1CCEA}\u{1CCE1}\u{1CCDA}\u{1CCE8}"

unless unicode.nfkc_casefold(outlined) == "node_modules" do
  raise "Unicode 17 outlined-letter Elixir sentinel failed"
end

unless unicode.nfc("\u{105D2}\u0307") == "\u{105C9}" do
  raise "Unicode 17 Todhri Elixir sentinel failed"
end

IO.write("ok\n")
"""


def _self_check_elixir(
    root: Path,
    elixir_output: str,
    sources: dict[str, str],
    python_module,
) -> None:
    elixir = shutil.which("elixir")
    if elixir is None:
        raise RuntimeError("Elixir Unicode self-check requires Elixir on PATH")

    del root  # Kept parallel with the TypeScript self-check call signature.
    with tempfile.TemporaryDirectory(prefix="unicode17-elixir-check-") as temporary:
        temporary_path = Path(temporary)
        generated_path = temporary_path / "tracked_artifact_unicode17.ex"
        runner_path = temporary_path / "self_check.exs"
        generated_path.write_text(elixir_output, encoding="utf-8", newline="\n")
        runner_path.write_text(_ELIXIR_SELF_CHECK, encoding="utf-8", newline="\n")
        result = subprocess.run(
            [elixir, str(runner_path)],
            cwd=temporary_path,
            input=json.dumps(
                _typescript_self_check_payload(python_module, sources),
                ensure_ascii=True,
                separators=(",", ":"),
            ),
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=180,
            check=False,
        )
    if result.returncode != 0 or result.stdout != "ok\n":
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic"
        raise RuntimeError(f"generated Elixir Unicode self-check failed: {detail}")


def _lua_scalar_field(value: str) -> str:
    return ",".join(f"{ord(character):X}" for character in value)


def _lua_self_check_payload(module, sources: dict[str, str]) -> str:
    lines = [f"V;{UNICODE_VERSION}"]
    for line in sources["NormalizationTest.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload or payload.startswith("@"):
            continue
        columns = [_scalars(field.strip()) for field in payload.split(";")[:5]]
        lines.append("N;" + ";".join(_lua_scalar_field(value) for value in columns))

    for line in sources["CaseFolding.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[1] not in {"C", "F"}:
            continue
        source = chr(int(fields[0], 16))
        values = (source, _scalars(fields[2]), module.nfkc_casefold(source))
        lines.append("F;" + ";".join(_lua_scalar_field(value) for value in values))

    uppercase: dict[int, tuple[int, ...]] = {}
    for line in sources["UnicodeData.txt"].splitlines():
        fields = line.split(";")
        if fields[12]:
            uppercase[int(fields[0], 16)] = (int(fields[12], 16),)
    for line in sources["SpecialCasing.txt"].splitlines():
        payload = line.split("#", 1)[0].strip()
        if not payload:
            continue
        fields = [field.strip() for field in payload.split(";")]
        if fields[4]:
            continue
        uppercase[int(fields[0], 16)] = tuple(
            int(value, 16) for value in fields[3].split()
        )
    for scalar, mapping in sorted(uppercase.items()):
        source = chr(scalar)
        expected = "".join(chr(value) for value in mapping)
        lines.append(f"U;{_lua_scalar_field(source)};{_lua_scalar_field(expected)}")
    return "\n".join(lines) + "\n"


_LUA_SELF_CHECK = r"""local unicode = dofile("tracked_artifact_unicode17.lua")

local function split(value, separator)
    local fields = {}
    local start = 1
    while true do
        local position = value:find(separator, start, true)
        if position == nil then
            fields[#fields + 1] = value:sub(start)
            return fields
        end
        fields[#fields + 1] = value:sub(start, position - 1)
        start = position + #separator
    end
end

local function from_scalar_field(value)
    local chunks = {}
    if value == "" then return "" end
    for scalar in value:gmatch("[^,]+") do
        chunks[#chunks + 1] = utf8.char(assert(tonumber(scalar, 16)))
    end
    return table.concat(chunks)
end

local counts = {N = 0, F = 0, U = 0}
local saw_version = false
for line in io.lines() do
    local fields = split(line, ";")
    local kind = fields[1]
    if kind == "V" then
        assert(fields[2] == unicode.UNICODE_VERSION, "generated Lua Unicode version drift")
        saw_version = true
    elseif kind == "N" then
        counts.N = counts.N + 1
        local c1, c2, c3, c4, c5 =
            from_scalar_field(fields[2]), from_scalar_field(fields[3]),
            from_scalar_field(fields[4]), from_scalar_field(fields[5]),
            from_scalar_field(fields[6])
        local valid = unicode.nfc(c1) == c2 and unicode.nfc(c2) == c2 and
            unicode.nfc(c3) == c2 and unicode.nfc(c4) == c4 and
            unicode.nfc(c5) == c4 and unicode.nfkc(c1) == c4 and
            unicode.nfkc(c2) == c4 and unicode.nfkc(c3) == c4 and
            unicode.nfkc(c4) == c4 and unicode.nfkc(c5) == c4
        assert(valid, "normalization Lua self-check failed at vector " .. counts.N)
    elseif kind == "F" then
        counts.F = counts.F + 1
        local source = from_scalar_field(fields[2])
        assert(
            unicode.casefold(source) == from_scalar_field(fields[3]),
            "case-fold Lua self-check failed at vector " .. counts.F
        )
        assert(
            unicode.nfkc_casefold(source) == from_scalar_field(fields[4]),
            "NFKC-case-fold Lua self-check failed at vector " .. counts.F
        )
    elseif kind == "U" then
        counts.U = counts.U + 1
        assert(
            unicode.full_uppercase(from_scalar_field(fields[2])) ==
                from_scalar_field(fields[3]),
            "full-uppercase Lua self-check failed at vector " .. counts.U
        )
    else
        error("unknown Lua self-check record")
    end
end
assert(saw_version, "missing generated Lua Unicode version")

local outlined = utf8.char(
    0x1CCE3, 0x1CCE4, 0x1CCD9, 0x1CCDA, 0x5F, 0x1CCE2,
    0x1CCE4, 0x1CCD9, 0x1CCEA, 0x1CCE1, 0x1CCDA, 0x1CCE8
)
assert(
    unicode.nfkc_casefold(outlined) == "node_modules",
    "Unicode 17 outlined-letter Lua sentinel failed"
)
assert(
    unicode.nfc(utf8.char(0x105D2, 0x0307)) == utf8.char(0x105C9),
    "Unicode 17 Todhri Lua sentinel failed"
)
io.write("ok\n")
"""


_LUA_OUTPUT_LIMIT = 8192


def _terminate_process_tree(process: subprocess.Popen) -> None:
    """Terminate an isolated emitted-runtime process and its descendants."""
    if process.poll() is not None:
        return
    try:
        if os.name == "nt":
            system_root = os.environ.get("SystemRoot") or os.environ.get("WINDIR")
            taskkill = Path(system_root or "C:/Windows") / "System32/taskkill.exe"
            if taskkill.is_file():
                subprocess.run(
                    [str(taskkill), "/PID", str(process.pid), "/T", "/F"],
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=5,
                    check=False,
                    env=_lua_self_check_environment(),
                )
        else:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    except (OSError, subprocess.SubprocessError):
        pass
    finally:
        if process.poll() is None:
            process.kill()


def _run_bounded_process(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    input_text: str,
    timeout: int,
    output_limit: int = _LUA_OUTPUT_LIMIT,
) -> subprocess.CompletedProcess:
    """Run an isolated child while draining but retaining only bounded output."""

    def drain(stream, target: list[bytes]) -> None:
        retained = bytearray()
        try:
            while chunk := stream.read(8192):
                remaining = output_limit - len(retained)
                if remaining > 0:
                    retained.extend(chunk[:remaining])
        finally:
            stream.close()
            target.append(bytes(retained))

    options: dict[str, object] = {}
    if os.name == "nt":
        options["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        options["start_new_session"] = True

    stdout: list[bytes] = []
    stderr: list[bytes] = []
    with tempfile.TemporaryFile() as input_stream:
        input_stream.write(input_text.encode("utf-8"))
        input_stream.seek(0)
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=env,
            stdin=input_stream,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            **options,
        )
        assert process.stdout is not None
        assert process.stderr is not None
        readers = (
            threading.Thread(target=drain, args=(process.stdout, stdout), daemon=True),
            threading.Thread(target=drain, args=(process.stderr, stderr), daemon=True),
        )
        for reader in readers:
            reader.start()
        try:
            returncode = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired as error:
            _terminate_process_tree(process)
            process.wait(timeout=10)
            raise RuntimeError(
                f"emitted-runtime self-check exceeded {timeout} seconds"
            ) from error
        finally:
            for reader in readers:
                reader.join(timeout=10)
        if any(reader.is_alive() for reader in readers):
            _terminate_process_tree(process)
            raise RuntimeError("emitted-runtime output readers did not terminate")

    return subprocess.CompletedProcess(
        command,
        returncode,
        stdout[0].decode("utf-8", errors="replace"),
        stderr[0].decode("utf-8", errors="replace"),
    )


def _lua_self_check_environment() -> dict[str, str]:
    """Retain only Windows loader state; Lua receives no user initialization."""
    return {
        name: value
        for name in ("SystemRoot", "WINDIR")
        if (value := os.environ.get(name)) is not None
    }


def _self_check_lua(
    root: Path,
    lua_output: str,
    sources: dict[str, str],
    python_module,
    lua_executable: Path,
) -> None:
    del root  # Kept parallel with the other emitted-runtime call signatures.
    lua = lua_executable.expanduser().resolve(strict=True)
    if not lua.is_file():
        raise RuntimeError(f"Lua Unicode self-check executable is not a file: {lua}")
    environment = _lua_self_check_environment()

    with tempfile.TemporaryDirectory(prefix="unicode17-lua-check-") as temporary:
        temporary_path = Path(temporary)
        generated_path = temporary_path / "tracked_artifact_unicode17.lua"
        runner_path = temporary_path / "self_check.lua"
        generated_path.write_text(lua_output, encoding="utf-8", newline="\n")
        runner_path.write_text(_LUA_SELF_CHECK, encoding="utf-8", newline="\n")
        version = _run_bounded_process(
            [str(lua), "-E", "-v"],
            cwd=temporary_path,
            env=environment,
            input_text="",
            timeout=10,
        )
        version_text = version.stdout + version.stderr
        if version.returncode != 0 or not any(
            line.startswith("Lua 5.4.7") for line in version_text.splitlines()[:1]
        ):
            raise RuntimeError("Lua Unicode self-check requires pinned Lua 5.4.7")
        result = _run_bounded_process(
            [str(lua), "-E", str(runner_path)],
            cwd=temporary_path,
            env=environment,
            input_text=_lua_self_check_payload(python_module, sources),
            timeout=180,
        )
    normalized_stdout = result.stdout.replace("\r\n", "\n")
    if result.returncode != 0 or normalized_stdout != "ok\n":
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic"
        raise RuntimeError(f"generated Lua Unicode self-check failed: {detail}")


def _write_or_check(root: Path, target: Path, content: str, check: bool) -> None:
    absolute = root / target
    content = content.replace("\r\n", "\n")
    if check:
        if not absolute.exists() or absolute.read_text(encoding="utf-8") != content:
            raise RuntimeError(f"generated file is stale: {target.as_posix()}")
        return
    absolute.parent.mkdir(parents=True, exist_ok=True)
    absolute.write_text(content, encoding="utf-8", newline="\n")


def _write_bytes_or_check(
    root: Path, target: Path, content: bytes, check: bool
) -> None:
    absolute = root / target
    if check:
        if not absolute.exists() or absolute.read_bytes() != content:
            raise RuntimeError(f"generated file is stale: {target.as_posix()}")
        return
    absolute.parent.mkdir(parents=True, exist_ok=True)
    absolute.write_bytes(content)


def _selected_runtime_self_checks(requested: list[str] | None) -> tuple[str, ...]:
    """Return the emitted runtimes whose official-vector checks must run."""
    if requested is None:
        return ("typescript", "ruby", "elixir", "lua")
    return tuple(dict.fromkeys(requested))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument(
        "--self-check-runtime",
        action="append",
        choices=("typescript", "ruby", "elixir", "lua"),
        help=(
            "limit emitted-runtime official-vector checks; repeat to select more "
            "than one runtime (default: every emitted runtime)"
        ),
    )
    parser.add_argument(
        "--lua-executable",
        type=Path,
        help=(
            "exact repository-pinned Lua 5.4.7 executable used when the Lua "
            "emitted-runtime check is selected"
        ),
    )
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    license_payload = (root / LICENSE_PATH).read_bytes()
    if hashlib.sha256(license_payload).hexdigest() != LICENSE_SHA256:
        raise RuntimeError("local Unicode License v3 notice is missing or modified")
    upstream_license = _download_exact(
        LICENSE_URL,
        expected_size=LICENSE_SIZE,
        expected_hash=LICENSE_SHA256,
        label="Unicode License v3 notice",
    )
    sources = _download_sources()
    tables = _parse_sources(sources)
    python_output = _render_python(tables)
    csharp_output = _render_csharp(tables)
    typescript_output = _render_typescript(tables)
    ruby_output = _render_ruby(tables)
    elixir_output = _render_elixir(tables)
    lua_output = _render_lua(tables)
    for target in PYTHON_TARGETS:
        _write_or_check(root, target, python_output, args.check)
    _write_or_check(root, CSHARP_TARGET, csharp_output, args.check)
    _write_or_check(root, TYPESCRIPT_TARGET, typescript_output, args.check)
    _write_or_check(root, RUBY_TARGET, ruby_output, args.check)
    _write_or_check(root, ELIXIR_TARGET, elixir_output, args.check)
    _write_or_check(root, LUA_TARGET, lua_output, args.check)
    for target in LICENSE_TARGETS:
        _write_bytes_or_check(root, target, upstream_license, args.check)
    python_module = _load_generated_module(root / PYTHON_TARGETS[0])
    _self_check(python_module, sources)
    selected_runtimes = _selected_runtime_self_checks(args.self_check_runtime)
    if "typescript" in selected_runtimes:
        _self_check_typescript(root, typescript_output, sources, python_module)
    if "ruby" in selected_runtimes:
        _self_check_ruby(root, ruby_output, sources, python_module)
    if "elixir" in selected_runtimes:
        _self_check_elixir(root, elixir_output, sources, python_module)
    if "lua" in selected_runtimes:
        if args.lua_executable is None:
            parser.error("--lua-executable is required for the Lua self-check")
        _self_check_lua(
            root,
            lua_output,
            sources,
            python_module,
            args.lua_executable,
        )
    print(
        f"Unicode {UNICODE_VERSION} generated and verified: "
        f"{len(tables[0])} combining, {len(tables[1])} decomposition, "
        f"{len(tables[2])} composition, {len(tables[3])} folding, "
        f"{len(tables[4])} uppercase rows; emitted runtime checks "
        f"{','.join(selected_runtimes)}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
