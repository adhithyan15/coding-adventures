#!/usr/bin/env python3
"""Generate the pinned Unicode 17 tracked-artifact policy substrate.

The generated modules are source-embedded so validation never reads Unicode
data from the filesystem and never inherits the host runtime's tables. This
generator is the only networked step: it downloads exact Unicode Consortium
files, verifies their SHA-256 digests, renders both runtimes, and exercises the
official normalization and case-folding vectors before accepting output.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import sys
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
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
    for target in PYTHON_TARGETS:
        _write_or_check(root, target, python_output, args.check)
    _write_or_check(root, CSHARP_TARGET, csharp_output, args.check)
    for target in LICENSE_TARGETS:
        _write_bytes_or_check(root, target, upstream_license, args.check)
    _self_check(_load_generated_module(root / PYTHON_TARGETS[0]), sources)
    print(
        f"Unicode {UNICODE_VERSION} generated and verified: "
        f"{len(tables[0])} combining, {len(tables[1])} decomposition, "
        f"{len(tables[2])} composition, {len(tables[3])} folding, "
        f"{len(tables[4])} uppercase rows"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
