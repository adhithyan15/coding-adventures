#!/usr/bin/env python3
"""Independent stdlib oracle for the WD-003 ct_compare corpus profile.

This module intentionally has no imports from warrant-grammar, generated Rust,
or lane bindings. It enumerates the published finite domains and interprets
only the closed, pure Warrant v0 model expression subset.
"""
from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import pathlib
import re
import stat
import sys
from typing import Any, Iterable

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "code/specs/fixtures/ct-compare-warrant-v1/corpus.json"
DEFAULT_SPEC = ROOT / "code/specs/ct-compare.warrant"
DEFAULT_TFV = ROOT / "code/specs/fixtures/ct-compare-warrant-v1/cases.tfv"
HEX64 = re.compile(r"[0-9a-f]{64}\Z")

class CorpusError(ValueError):
    """A stable, safe corpus validation failure."""
    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code
        self.message = message


def _pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CorpusError("malformed-json", "duplicate object key")
        result[key] = value
    return result


def _read_limited(path: pathlib.Path, limit: int) -> str:
    try:
        metadata = path.lstat()
        _need(stat.S_ISREG(metadata.st_mode), "io", "input must be a regular file")
        _need(metadata.st_size <= limit, "input-limit", "input exceeds size limit")
        with path.open("rb") as source:
            payload = source.read(limit + 1)
        _need(len(payload) <= limit, "input-limit", "input exceeds size limit")
        return payload.decode("utf-8", errors="strict")
    except CorpusError:
        raise
    except (OSError, UnicodeDecodeError) as exc:
        raise CorpusError("io", "could not read validator input") from exc


def parse_json(text: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=_pairs, parse_constant=lambda _: (_ for _ in ()).throw(ValueError("non-finite number")))
    except CorpusError:
        raise
    except (json.JSONDecodeError, ValueError, RecursionError) as exc:
        raise CorpusError("malformed-json", "invalid or truncated JSON") from exc


def _fail(code: str, message: str) -> None:
    raise CorpusError(code, message)


def _need(condition: bool, code: str, message: str) -> None:
    if not condition:
        _fail(code, message)


def _exact(obj: Any, keys: set[str], where: str) -> dict[str, Any]:
    _need(type(obj) is dict and set(obj) == keys, "schema", f"{where} has unexpected fields")
    return obj


def domain(name: str, parameter: int | None = None) -> list[Any]:
    if name == "bool": return [False, True]
    if name == "u8_all": return list(range(256))
    if name == "u64_edge":
        maximum = (1 << 64) - 1
        return [*range(256), *(1 << i for i in range(8, 64)), maximum, *(maximum ^ (1 << i) for i in range(64))]
    if name == "nat_0_64": return list(range(65))
    if name == "bytes_d1":
        alphabet = (0, 1, 128, 255)
        return [b""] + [bytes((i,)) for i in range(256)] + [bytes(p) for n in (2, 3) for p in itertools.product(alphabet, repeat=n)] + [bytes([v]) * n for n in range(4, 9) for v in (0, 0xA5)]
    if name == "bytes_n":
        _need(type(parameter) is int and parameter in (0, 1, 2, 4, 16), "descriptor", "Bytes width is outside the closed ct_compare profile")
        if parameter <= 3: return [v for v in domain("bytes_d1") if len(v) == parameter]
        alphabet = (0, 1, 128, 255)
        values = [bytes([v]) * parameter for v in alphabet]
        for pos in range(parameter):
            for symbol in (1, 128, 255):
                value = bytearray(parameter); value[pos] = symbol; values.append(bytes(value))
        return values
    _fail("unsupported-domain", "unknown domain descriptor")


def _typed(value: Any, ty: str) -> Any:
    # The profile uses JSON primitives for Bool/Nat/U8 and canonical tagged
    # objects for byte strings and U64 to avoid JSON number precision loss.
    if ty == "Bool":
        _need(type(value) is bool, "value", "Bool value must be boolean"); return value
    if ty.startswith("Bytes"):
        if type(value) is dict:
            _exact(value, {"type", "value"}, "typed byte value")
            _need(value["type"] == ty and type(value["value"]) is str and re.fullmatch(r"(?:[0-9a-f]{2})*", value["value"]) is not None, "value", "invalid canonical Bytes value")
            result = bytes.fromhex(value["value"])
        elif type(value) is str and re.fullmatch(r"(?:[0-9a-f]{2})*", value): result = bytes.fromhex(value)
        else: _fail("value", "invalid Bytes value")
        match = re.fullmatch(r"Bytes<(\d+)>", ty)
        if match: _need(len(result) == int(match.group(1)), "value", "Bytes width mismatch")
        return result
    if ty.startswith("Result<"):
        if type(value) is dict and set(value) == {"type", "value"}:
            _need(value["type"] == ty, "value", "Result type tag mismatch")
            value = value["value"]
        _need(type(value) is dict and len(value) == 1, "value", "invalid Result value")
        if "ok" in value:
            inner = ty[len("Result<"):-1].split(",", 1)[0]
            return ("ok", _typed(value["ok"], inner))
        if "err" in value and type(value["err"]) is str:
            return ("err", value["err"])
        _fail("value", "invalid Result value")
    if ty in ("U8", "Nat") and type(value) is dict:
        _exact(value, {"type", "value"}, f"typed {ty} value")
        _need(value["type"] == ty and type(value["value"]) is str and re.fullmatch(r"0|[1-9][0-9]*", value["value"]) is not None, "value", f"invalid canonical {ty} value")
        value = int(value["value"])
    if ty == "U64":
        if type(value) is dict:
            _exact(value, {"type", "value"}, "typed U64 value")
            _need(value["type"] == "U64", "value", "U64 type tag mismatch")
            value = value["value"]
        _need(type(value) is str and re.fullmatch(r"0|[1-9][0-9]*", value) is not None, "value", "U64 must be canonical decimal text")
        n = int(value); _need(n < 1 << 64, "value", "U64 out of range"); return n
    if ty in ("U8", "Nat"):
        _need(type(value) is int and value >= 0 and (ty != "U8" or value <= 255) and (ty != "Nat" or value <= 64), "value", f"invalid {ty} value"); return value
    _fail("unsupported-type", "unknown Warrant type")


def _encode(value: Any, ty: str | None = None) -> bytes:
    if ty == "Bool": return b"\x01" if value else b"\x00"
    if ty in ("U8", "U64", "Nat"): return b"\x10" + int(value).to_bytes(8, "little")
    if ty and ty.startswith("Bytes"): return b"\x20" + len(value).to_bytes(4, "little") + value
    if type(value) is bool: return b"\x01" if value else b"\x00"
    if type(value) is int: return b"\x10" + value.to_bytes(8, "little")
    if type(value) is bytes: return b"\x20" + len(value).to_bytes(4, "little") + value
    if type(value) is tuple and value:
        if value[0] == "ok": return b"\x50" + _encode(value[1])
        if value[0] == "err":
            text = value[1].encode("utf-8"); return b"\x51" + len(text).to_bytes(4, "little") + text
        if value[0] == "name": return value[1].encode("utf-8")
    _fail("unsupported-value", "value cannot be canonically encoded")


def _eval(node: Any, env: dict[str, Any], models: dict[str, Any], depth: int = 0) -> Any:
    _need(depth <= 64, "ir-limit", "model IR nesting exceeds limit")
    _need(type(node) is list and node and type(node[0]) is str, "unsupported-ir", "malformed or unknown model IR node")
    tag = node[0]
    if tag in ("subject_op", "op", "call", "forall", "exists", "match", "let", "lambda"):
        _fail("unsupported-ir", "subject calls and unsupported Warrant forms are forbidden in pure models")
    if tag == "lit":
        _need(len(node) == 2, "unsupported-ir", "malformed literal node"); return _literal(node[1])
    if tag == "var":
        _need(len(node) == 2 and type(node[1]) is str and node[1] in env, "unsupported-ir", "unknown model variable"); return env[node[1]]
    if tag == "model_call":
        _need(len(node) == 3 and type(node[1]) is str and node[1] in models and type(node[2]) is list, "unsupported-ir", "unknown model call")
        model = models[node[1]]
        args = [_eval(arg, env, models, depth + 1) for arg in node[2]]
        params = model.get("params")
        _need(type(params) is list and len(params) == len(args), "unsupported-ir", "model arity mismatch")
        return _eval(model.get("body"), dict(zip(params, args)), models, depth + 1)
    if tag in ("ok", "err", "name"):
        _need(len(node) == 2, "unsupported-ir", "malformed result/name node")
        val = _eval(node[1], env, models, depth + 1) if tag == "ok" else node[1]
        return (tag, val)
    if tag == "len":
        _need(len(node) == 2, "unsupported-ir", "malformed len node"); return len(_eval(node[1], env, models, depth + 1))
    if tag == "if":
        _need(len(node) == 4, "unsupported-ir", "malformed if node")
        return _eval(node[2] if _eval(node[1], env, models, depth + 1) else node[3], env, models, depth + 1)
    arities = {"eq": 2, "ne": 2, "lt": 2, "le": 2, "gt": 2, "ge": 2, "add": 2, "sub": 2, "mul": 2, "and": 2, "or": 2, "not": 1, "implies": 2}
    _need(tag in arities and len(node) == arities.get(tag) + 1, "unsupported-ir", "unknown or malformed pure model node")
    args = [_eval(arg, env, models, depth + 1) for arg in node[1:]]
    ops = {"eq": lambda a,b:a==b, "ne": lambda a,b:a!=b, "lt": lambda a,b:a<b, "le": lambda a,b:a<=b, "gt": lambda a,b:a>b, "ge": lambda a,b:a>=b, "add": lambda a,b:a+b, "sub": lambda a,b:a-b, "mul": lambda a,b:a*b, "and": lambda a,b:a and b, "or": lambda a,b:a or b, "not": lambda a:not a, "implies": lambda a,b:(not a) or b}
    return ops[tag](*args)


def _literal(value: Any) -> Any:
    if type(value) in (bool, int, str): return value
    if type(value) is dict:
        if set(value) == {"type", "value"}: return _typed(value, value["type"])
        if set(value) == {"bytes"}: return _typed(value["bytes"], "Bytes")
    _fail("unsupported-ir", "unsupported literal value")


def _domain_spec(item: Any) -> tuple[list[Any], str]:
    _need(type(item) is dict and set(item) <= {"domain", "width", "type"} and "domain" in item and type(item["domain"]) is str, "descriptor", "malformed binder domain")
    name = item["domain"]
    width = item.get("width")
    match = re.fullmatch(r"bytes_n\((\d+)\)", name) if type(name) is str else None
    if match:
        _need(width is None, "descriptor", "duplicate bytes_n width")
        width = int(match.group(1))
        _need(width in (0, 1, 2, 4, 16), "descriptor", "Bytes width is outside the closed ct_compare profile")
        name = "bytes_n"
    if name == "bytes_n":
        _need(type(width) is int and width in (0, 1, 2, 4, 16), "descriptor", "Bytes width is outside the closed ct_compare profile")
        derived = f"Bytes<{width}>"
        _need(item.get("type", derived) == derived, "descriptor", "binder type disagrees with domain")
        return domain(name, width), derived
    types = {"bool":"Bool", "u8_all":"U8", "u64_edge":"U64", "nat_0_64":"Nat", "bytes_d1":"Bytes"}
    _need(name in types, "unsupported-domain", "unknown domain descriptor")
    _need(width is None, "descriptor", "unexpected domain width")
    _need(item.get("type", types[name]) == types[name], "descriptor", "binder type disagrees with domain")
    return domain(name), types[name]


def _stream_prefix(suite_id: str, binder_types: list[str], result_type: str) -> bytes:
    def field(text: str) -> bytes:
        raw = text.encode("utf-8")
        return len(raw).to_bytes(4, "little") + raw
    return (b"warrant-stream-v1\0" + field(suite_id) +
            len(binder_types).to_bytes(4, "little") +
            b"".join(field(ty) for ty in binder_types) + field(result_type))

def _tfv_value(value: Any, ty: str) -> str:
    if ty == "Bool": return "T" if value else "F"
    if ty in ("Nat", "U8", "U64"): return f"I{value}"
    if ty.startswith("Bytes"): return "X" + value.hex()
    if type(value) is tuple and value[0] == "ok": return "M1 S6f6b " + _tfv_value(value[1], "Bytes")
    if type(value) is tuple and value[0] == "err": return "M1 S657272 S" + str(value[1]).encode("utf-8").hex()
    return "S " + str(value)


def _check_source_surface(spec_text: str) -> None:
    _need(spec_text.splitlines() and spec_text.splitlines()[0].strip() == "warrant ct_compare", "spec", "only the Warrant v0 ct_compare header is supported")
    allowed = {"error", "model", "op", "assume", "labels", "input", "bound", "example", "claim"}
    for raw in spec_text.splitlines()[1:]:
        if raw.startswith("  ") and not raw.startswith("    ") and raw.strip():
            word = raw.strip().split(None, 1)[0].rstrip(":")
            _need(word in allowed, "spec", "unsupported Warrant declaration in ct_compare v0 source")


def _types_from_spec(spec_text: str) -> dict[str, list[str]]:
    return {name: variants.split(",") for name, variants in re.findall(r"^\s*error\s+([A-Z][A-Za-z0-9_]*)\s*=\s*([A-Z][A-Za-z0-9_]*(?:\s*,\s*[A-Z][A-Za-z0-9_]*)*)\s*$", spec_text, re.MULTILINE)}

def _models_from_spec(spec_text: str) -> dict[str, Any]:
    """Read the deliberately tiny ct_compare v0 model declaration grammar."""
    models: dict[str, Any] = {}
    pattern = re.compile(r"^\s*model\s+([a-z][a-z0-9_]*)\s*\(([^)]*)\)\s*->\s*([A-Za-z0-9_<> ,]+)\s*=\s*(.*?)\s*$", re.MULTILINE)
    for match in pattern.finditer(spec_text):
        name, raw_params, _ret, expr = match.groups()
        params: list[str] = []
        for param in filter(None, (part.strip() for part in raw_params.split(","))):
            pieces = param.split(":")
            _need(len(pieces) == 2 and re.fullmatch(r"[a-z][a-z0-9_]*", pieces[0].strip()) is not None, "spec-model", "unsupported model parameter declaration")
            params.append(pieces[0].strip())
        atom = r"[a-z][a-z0-9_]*"
        expression = re.fullmatch(rf"\s*({atom})\s*(==|!=|<=|>=|<|>)\s*({atom})\s*", expr)
        _need(expression is not None, "spec-model", "unsupported ct_compare v0 model expression")
        left, operator, right = expression.groups()
        _need(left in params and right in params and len(set(params)) == len(params), "spec-model", "model body references an undeclared parameter")
        tag = {"==": "eq", "!=": "ne", "<": "lt", "<=": "le", ">": "gt", ">=": "ge"}[operator]
        _need(name not in models, "spec-model", "duplicate model declaration")
        models[name] = {"params": params, "body": [tag, ["var", left], ["var", right]]}
    _need(models, "spec-model", "Warrant source contains no supported model declarations")
    return models

def _verify_example(example: dict[str, Any], ops: Any) -> None:
    name = example["op"]
    _need(type(ops) is dict and name in ops, "example", "unknown example operation")
    signature = ops[name]
    _need(type(signature) is dict and type(signature.get("params")) is list and type(signature.get("ret")) is str, "schema", "malformed operation signature")
    params = signature["params"]
    _need(type(example["args"]) is list and len(example["args"]) == len(params), "example", "example arguments do not match operation")
    values = {}
    for param, raw_value in zip(params, example["args"]):
        _need(type(param) is list and len(param) == 2 and type(param[0]) is str and type(param[1]) is str, "schema", "malformed operation parameter")
        values[param[0]] = _typed(raw_value, param[1])
    _need(example["result_type"] == signature["ret"], "example", "example result type disagrees with operation")
    if name == "eq_fixed":
        _need(type(example["width"]) is int and example["width"] >= 0, "example", "fixed-width example lacks a width")
    else:
        _need(example["width"] is None, "example", "unexpected example width")
    if name in ("eq", "eq_fixed", "eq_u64"):
        if name == "eq_fixed":
            _need(len(values["a"]) == example["width"] == len(values["b"]), "example", "fixed-width example argument length mismatch")
        expected = values["a"] == values["b"]
    elif name == "select_bytes":
        if len(values["a"]) != len(values["b"]): expected = ("err", "CT_SELECT_LENGTH_MISMATCH")
        else: expected = ("ok", values["a"] if values["choice"] else values["b"])
    else: _fail("unsupported-operation", "operation is outside ct_compare v0 profile")
    observed = _typed(example["expected"], signature["ret"])
    _need(observed == expected, "example-expected", "example expected value disagrees with pure oracle")

def _check_tfv(text: str, examples: list[dict[str, Any]], ops: dict[str, Any] | None = None) -> None:
    lines = text.splitlines()
    _need(lines and lines[0] == "tfv 1", "tfv", "invalid TFV header")
    records = [line for line in lines[1:] if line]
    _need(len(records) == len(examples), "tfv", "TFV record count mismatch")
    for line, example in zip(records, examples):
        pieces = line.split()
        _need(len(pieces) >= 4 and pieces[0] == example["id"] and pieces[1].startswith("R"), "tfv", "TFV example identity mismatch")
        _need("Kop" in pieces and "Kexpect" in pieces, "tfv", "TFV example lacks op or expect")
        if ops is not None:
            fields: dict[str, list[str]] = {}
            index = 2
            while index < len(pieces):
                _need(pieces[index].startswith("K") and len(pieces[index]) > 1, "tfv", "malformed TFV field key")
                key = pieces[index][1:]
                index += 1
                begin = index
                while index < len(pieces) and not pieces[index].startswith("K"):
                    index += 1
                _need(key not in fields and begin < index, "tfv", "duplicate or empty TFV field")
                fields[key] = pieces[begin:index]
            operation = ops[example["op"]]
            expected_keys = {"op", "expect", *(param[0] for param in operation["params"])}
            if example["width"] is not None: expected_keys.add("width")
            _need(set(fields) == expected_keys, "tfv", "TFV fields disagree with example schema")
            _need(fields["op"] == ["S" + example["op"].encode("utf-8").hex()], "tfv", "TFV operation value mismatch")
            _need(pieces[1] == "R" + str(len(expected_keys)), "tfv", "TFV record field count mismatch")
            for param, raw_value in zip(operation["params"], example["args"]):
                value = _typed(raw_value, param[1])
                _need(fields[param[0]] == _tfv_value(value, param[1]).split(), "tfv", "TFV argument value mismatch")
            if example["width"] is not None:
                _need(fields["width"] == [f"I{example['width']}"], "tfv", "TFV width mismatch")
        at = pieces.index("Kexpect")
        _need(at + 1 < len(pieces), "tfv", "TFV expected field is empty")
        expected = _typed(example["expected"], example["result_type"])
        _need(pieces[at + 1:] == _tfv_value(expected, example["result_type"]).split(), "tfv", "TFV expected value mismatch")


_SOURCE_TOKEN = re.compile(r'\s+|b"(?:[0-9a-fA-F]{2})*"|(?:0|[1-9][0-9]*)(?:u64|u8)?|[A-Za-z_][A-Za-z0-9_]*|==|!=|<=|>=|[()<>.,:|+*\-]')


class _SourceExpression:
    """Small fail-closed expression reader for the committed ct_compare v0 source."""
    def __init__(self, text: str, ops: set[str], models: set[str]):
        self.ops = ops
        self.models = models
        self.tokens = [m.group(0) for m in _SOURCE_TOKEN.finditer(text) if not m.group(0).isspace()]
        consumed = "".join(m.group(0) for m in _SOURCE_TOKEN.finditer(text))
        _need(consumed == text, "spec-expression", "unsupported token in ct_compare v0 expression")
        self.at = 0

    def peek(self) -> str | None:
        return self.tokens[self.at] if self.at < len(self.tokens) else None

    def take(self, expected: str | None = None) -> str:
        token = self.peek()
        _need(token is not None and (expected is None or token == expected), "spec-expression", "malformed ct_compare v0 expression")
        self.at += 1
        return token

    def expression(self, minimum: int = 0) -> Any:
        token = self.take()
        if token == "not":
            left = ["not", self.expression(7)]
        elif token == "(":
            left = self.expression()
            self.take(")")
        elif token == "if":
            condition = self.expression()
            self.take("then")
            yes = self.expression()
            self.take("else")
            left = ["if", condition, yes, self.expression()]
        elif token.startswith('b"'):
            value = token[2:-1].lower()
            left = ["lit", {"type": "Bytes", "value": value}]
        elif re.fullmatch(r"(?:0|[1-9][0-9]*)(?:u64|u8)?", token):
            suffix = "U64" if token.endswith("u64") else "U8" if token.endswith("u8") else None
            digits = token[:-3] if suffix else token
            left = ["lit", {"type": suffix, "value": digits}] if suffix else ["lit", int(digits)]
        elif token in ("true", "false"):
            left = ["lit", token == "true"]
        elif re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", token):
            name = token
            if self.peek() == ".":
                self.take(".")
                variant = self.take()
                _need(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", variant) is not None, "spec-expression", "malformed qualified name")
                name += "." + variant
            width = None
            if self.peek() == "<":
                self.take("<")
                width_token = self.take()
                _need(width_token.isdigit(), "spec-expression", "only literal operation widths are supported")
                width = int(width_token)
                self.take(">")
            if self.peek() == "(":
                self.take("(")
                args = []
                if self.peek() != ")":
                    while True:
                        args.append(self.expression())
                        if self.peek() != ",": break
                        self.take(",")
                self.take(")")
                if name == "len": left = ["len", *args]
                elif name == "Ok": left = ["ok", *args]
                elif name == "Err":
                    _need(len(args) == 1 and name + "." not in name, "spec-expression", "malformed Err constructor")
                    err_name = args[0]
                    _need(type(err_name) is list and err_name[0] == "var" and "." in err_name[1], "spec-expression", "Err requires an error variant")
                    left = ["err", err_name[1].rsplit(".", 1)[1]]
                elif name in self.ops:
                    left = ["subject_op", name, args] + ([[width]] if width is not None else [])
                elif name in self.models:
                    _need(width is None, "spec-expression", "models cannot have static widths")
                    left = ["model_call", name, args]
                else:
                    _fail("spec-expression", "unsupported call in ct_compare v0 source")
            else:
                _need(width is None, "spec-expression", "width on non-call expression")
                left = ["var", name]
        else:
            _fail("spec-expression", "unsupported ct_compare v0 expression form")

        precedence = {"implies": 1, "or": 2, "and": 3, "==": 4, "!=": 4, "<": 4, "<=": 4, ">": 4, ">=": 4, "+": 5, "-": 5, "*": 6}
        tags = {"implies": "implies", "or": "or", "and": "and", "==": "eq", "!=": "ne", "<": "lt", "<=": "le", ">": "gt", ">=": "ge", "+": "add", "-": "sub", "*": "mul"}
        while self.peek() in precedence and precedence[self.peek()] >= minimum:
            operator = self.take()
            right = self.expression(precedence[operator] + 1)
            left = [tags[operator], left, right]
        return left

    def parse(self) -> Any:
        result = self.expression()
        _need(self.peek() is None, "spec-expression", "trailing source expression tokens")
        return result


def _split_top(text: str, delimiter: str = ",") -> list[str]:
    result, start, parens, angles = [], 0, 0, 0
    for index, char in enumerate(text):
        if char == "(": parens += 1
        elif char == ")": parens -= 1
        elif char == "<": angles += 1
        elif char == ">": angles -= 1
        elif char == delimiter and parens == 0 and angles == 0:
            result.append(text[start:index].strip()); start = index + 1
    result.append(text[start:].strip())
    return [item for item in result if item]


def _source_ops(spec_text: str) -> dict[str, Any]:
    ops: dict[str, Any] = {}
    line_re = re.compile(r"op\s+([a-z][a-z0-9_]*)(?:<([^>]*)>)?\(([^)]*)\)\s*->\s*([A-Za-z0-9_<>, ]+?)(?:\s+(fresh))?")
    declarations = [
        line.strip()
        for line in spec_text.splitlines()
        if line.startswith("  ") and not line.startswith("    ") and line[2:].startswith("op ")
    ]
    for declaration in declarations:
        match = line_re.fullmatch(declaration)
        _need(match is not None, "spec-op", "unsupported or malformed op declaration in ct_compare v0 source")
        name, raw_widths, raw_params, ret, fresh = match.groups()
        params = []
        for raw in _split_top(raw_params):
            parts = raw.split(":", 1)
            _need(len(parts) == 2, "spec-op", "malformed operation parameter")
            params.append([parts[0].strip(), parts[1].strip()])
        row: dict[str, Any] = {"params": params, "ret": ret.replace(" ", "")}
        if raw_widths:
            width_params = []
            for width in _split_top(raw_widths):
                width_name = width.split(":", 1)[0].strip()
                _need(re.fullmatch(r"[A-Z][A-Za-z0-9_]*", width_name) is not None, "spec-op", "malformed static width parameter")
                width_params.append(width_name)
            row["width_params"] = width_params
        if fresh: row["fresh"] = True
        _need(name not in ops, "spec-op", "duplicate operation declaration")
        ops[name] = row
    _need(ops, "spec-op", "Warrant source contains no supported operations")
    return ops


def _source_binders(raw: str, ops: set[str], models: set[str]) -> tuple[list[list[Any]], list[Any]]:
    binders: list[list[Any]] = []
    refinements: list[Any] = []
    for part in _split_top(raw):
        pieces = _split_top(part, "|")
        decl = pieces[0]
        split = decl.split(":", 1)
        _need(len(split) == 2, "spec-claim", "malformed quantified binder")
        name, ty = split[0].strip(), split[1].strip()
        _need(re.fullmatch(r"[a-z][a-z0-9_]*", name) is not None, "spec-claim", "unsupported binder name")
        domain_name = _domain_name_for_type(ty)
        binders.append([name, {"domain": domain_name, "type": ty}])
        if len(pieces) > 1:
            refinements.append(_SourceExpression("|".join(pieces[1:]), ops, models).parse())
    admission = ["lit", True]
    if len(refinements) == 1: admission = refinements[0]
    elif refinements: admission = ["and", *refinements]
    return binders, admission


def _domain_name_for_type(ty: str) -> str:
    if ty in ("Bool", "U8", "U64", "Nat", "Bytes"):
        return {"Bool": "bool", "U8": "u8_all", "U64": "u64_edge", "Nat": "nat_0_64", "Bytes": "bytes_d1"}[ty]
    match = re.fullmatch(r"Bytes<(\d+)>", ty)
    _need(match is not None, "spec-claim", "unsupported quantified type")
    width = int(match.group(1))
    _need(width in (0, 1, 2, 4, 16), "spec-claim", "Bytes width is outside the closed ct_compare profile")
    return f"bytes_n({width})"


def _claim_definitions(spec_text: str, ops: dict[str, Any], models: dict[str, Any]) -> dict[str, dict[str, Any]]:
    definitions = {}
    lines = spec_text.splitlines()
    for index, raw_line in enumerate(lines):
        line = raw_line.strip()
        match = re.match(r"^claim\s+([a-z][a-z0-9_]*):\s*$", line)
        if not match: continue
        _need(index + 1 < len(lines), "spec-claim", "claim body is missing")
        body_line = lines[index + 1].strip()
        quant = re.fullmatch(r"forall\s+(.*?)\.\s*(.*)", body_line)
        _need(quant is not None, "spec-claim", "only one-line forall claim bodies are supported")
        binder_text, claim_expr = quant.groups()
        binders, admission = _source_binders(binder_text, set(ops), set(models))
        expression = _SourceExpression(claim_expr, set(ops), set(models)).parse()
        _need(type(expression) is list and expression[0] == "eq" and len(expression) == 3, "spec-claim", "only equality claims are supported")
        left, right = expression[1:]
        subject_nodes = [node for node in _walk_ir(expression) if node[0] == "subject_op"]
        _need(len(subject_nodes) == 1, "spec-claim", "claim must contain one subject operation")
        subject = subject_nodes[0]
        # WD-003 accepts the subject operation as one complete equality side.
        _need(left == subject or right == subject, "spec-claim", "subject operation must be one equality side")
        reference = right if left == subject else left
        _need(not any(node[0] == "subject_op" for node in _walk_ir(reference)), "spec-claim", "pure reference contains a subject call")
        op_name = subject[1]
        _need(op_name in ops, "spec-claim", "claim calls an undeclared operation")
        binder_names = [row[0] for row in binders]
        _need(len(binder_names) == len(set(binder_names)), "spec-claim", "duplicate quantified binder")
        _need(len(subject[2]) == len(ops[op_name]["params"]), "spec-claim", "subject call argument count disagrees with signature")
        for binder_row, arg, (param_name, param_ty) in zip(binders, subject[2], ops[op_name]["params"]):
            _need(arg == ["var", param_name] and param_name in binder_names, "spec-claim", "subject args do not bind operation parameters in order")
            actual_ty = binder_row[1]["type"]
            if "width_params" in ops[op_name]:
                static_widths = subject[3] if len(subject) == 4 else []
                _need(len(static_widths) == len(ops[op_name]["width_params"]) == 1, "spec-claim", "static width count disagrees with operation signature")
                param_ty = param_ty.replace("<N>", f"<{static_widths[0]}>")
            _need(actual_ty == param_ty, "spec-claim", "binder type disagrees with operation parameter")
        _need(subject[0] == "subject_op", "spec-claim", "invalid subject call")
        expected = {
            "id": f"warrant/ct_compare/exhaustive/{match.group(1)}",
            "claim": match.group(1),
            "ops": [op_name],
            "binders": binders,
            "admit": admission,
            "lhs": subject,
            "rhs": reference,
            "result_type": ops[op_name]["ret"],
            "domain_text": " x ".join(f"{row[1]['type']}[{len(_domain_spec(row[1])[0])}]" for row in binders),
        }
        if len(subject) == 4:
            _need("width_params" in ops[op_name] and len(subject[3]) == 1, "spec-claim", "unexpected subject static widths")
            width = subject[3][0]
            _need(all(row[1]["type"] == f"Bytes<{width}>" for row in binders), "spec-claim", "fixed width disagrees with binder types")
        definitions[match.group(1)] = expected
    _need(definitions, "spec-claim", "Warrant source contains no supported claims")
    return definitions


def _walk_ir(node: Any) -> Iterable[list[Any]]:
    if type(node) is list and node and type(node[0]) is str:
        yield node
        for child in node[1:]:
            if type(child) is list:
                if child and type(child[0]) is str: yield from _walk_ir(child)
                else:
                    for item in child: yield from _walk_ir(item)


def _typed_json(value: Any, ty: str) -> Any:
    if ty == "Bool": return value
    if ty.startswith("Bytes"): return {"type": ty, "value": value.hex()}
    if ty in ("U8", "U64", "Nat"): return {"type": ty, "value": str(value)}
    if ty.startswith("Result<"):
        inner = ty[len("Result<"):-1].split(",", 1)[0]
        if value[0] == "ok": return {"type": ty, "value": {"ok": _typed_json(value[1], inner)}}
        if value[0] == "err": return {"type": ty, "value": {"err": value[1]}}
    _fail("spec-example", "unsupported example result type")


def _source_examples(spec_text: str, ops: dict[str, Any], models: dict[str, Any]) -> list[dict[str, Any]]:
    examples = []
    source_lines = spec_text.splitlines()
    for line_index, raw_line in enumerate(source_lines):
        line = raw_line.strip()
        match = re.match(r"^example\s+([a-z][a-z0-9_]*):\s*(.*?)\s*$", line)
        if not match: continue
        name, body = match.groups()
        if not body:
            _need(line_index + 1 < len(source_lines), "spec-example", "example body is missing")
            body = source_lines[line_index + 1].strip()
        expr = _SourceExpression(body, set(ops), set(models)).parse()
        _need(type(expr) is list and expr[0] == "eq" and len(expr) == 3, "spec-example", "example must compare one operation result")
        subject = expr[1] if type(expr[1]) is list and expr[1][0] == "subject_op" else expr[2] if type(expr[2]) is list and expr[2][0] == "subject_op" else None
        expected_ir = expr[2] if subject is expr[1] else expr[1]
        _need(subject is not None and subject[1] in ops, "spec-example", "example has no supported operation")
        op_name = subject[1]
        signature = ops[op_name]
        _need(len(subject[2]) == len(signature["params"]), "spec-example", "example argument count disagrees with operation signature")
        args = []
        for raw_arg, param in zip(subject[2], signature["params"]):
            value = _eval(raw_arg, {}, models)
            args.append(_typed_json(value, param[1]))
        width = subject[3][0] if len(subject) == 4 and len(subject[3]) == 1 else None
        expected_value = _eval(expected_ir, {}, models)
        examples.append({
            "id": f"warrant/ct_compare/example/{name}",
            "op": op_name,
            "args": args,
            "expected": _typed_json(expected_value, signature["ret"]),
            "result_type": signature["ret"],
            "width": width,
        })
    return examples


def _fault_rows(ops: dict[str, Any]) -> list[dict[str, str]]:
    rows = []
    for name, signature in ops.items():
        param_types = [param[1] for param in signature["params"]]
        if signature["ret"] == "Bool" and param_types == ["Bytes", "Bytes"]:
            faults = [("ignore-last-byte", "input"), ("invert", "output")]
        elif signature["ret"] == "Bool" and param_types == ["Bytes<N>", "Bytes<N>"]:
            faults = [("ignore-last-byte", "input"), ("invert", "output")]
        elif signature["ret"] == "Bool" and param_types == ["U64", "U64"]:
            faults = [("ignore-high-bit", "input"), ("invert", "output")]
        elif signature["ret"].startswith("Result<") and param_types == ["Bytes", "Bytes", "Bool"]:
            faults = [("missing-error", "output"), ("swap-choice", "input")]
        else:
            _fail("spec-op", "operation signature has no supported WD-003 fault profile")
        rows.extend({"applies": applies, "fault": fault, "op": name} for fault, applies in faults)
    return rows


def _same_rows(actual: Any, expected: list[dict[str, Any]]) -> bool:
    if type(actual) is not list or len(actual) != len(expected): return False
    try:
        actual_rows = [json.dumps(row, sort_keys=True, separators=(",", ":")) for row in actual]
        expected_rows = [json.dumps(row, sort_keys=True, separators=(",", ":")) for row in expected]
        return len(set(actual_rows)) == len(actual_rows) and sorted(actual_rows) == sorted(expected_rows)
    except (TypeError, ValueError):
        return False

def _control_rows(faults: list[dict[str, str]], claims: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for fault_row in faults:
        op = fault_row["op"]
        required = [claim["id"] for claim in claims.values() if claim["ops"] == [op]]
        _need(bool(required), "faults", f"faulted operation {op} has no claim target")
        rows.append({
            "fault": fault_row["fault"],
            "id": f"warrant/ct_compare/negctl/exhaustive/{op}/{fault_row['fault']}",
            "op": op,
            "required_claims": required,
            "target_kind": "exhaustive",
        })
    return rows


def validate_document(document: Any, spec_text: str, tfv_text: str | None = None) -> dict[str, int]:
    expected_top = {"schema_version", "profile", "module", "spec", "spec_sha256", "generator", "types", "ops", "models", "examples", "suites", "faults", "controls"}
    root = _exact(document, expected_top, "corpus")
    _need(type(root["schema_version"]) is int and root["schema_version"] == 1, "schema", "unsupported schema version")
    _need(root["profile"] == "warrant-corpus-v1" and root["module"] == "ct_compare" and root["spec"] == "code/specs/ct-compare.warrant", "profile", "only the committed ct_compare v0 profile is admitted")
    _need(type(root["spec_sha256"]) is str and HEX64.fullmatch(root["spec_sha256"]) and hashlib.sha256(spec_text.encode("utf-8")).hexdigest() == root["spec_sha256"], "spec-digest", "Warrant source digest mismatch")
    _check_source_surface(spec_text)
    source_ops = _source_ops(spec_text)
    _need(root["ops"] == source_ops, "spec-op", "corpus operation signatures disagree with checked Warrant source")
    _need(type(root["types"]) is dict and root["types"] == _types_from_spec(spec_text), "profile", "type table disagrees with checked Warrant source")
    expected_models = _models_from_spec(spec_text)
    _need(root["models"] == expected_models, "model", "corpus models disagree with the checked Warrant source")
    models = root["models"]
    _need(type(models) is dict, "schema", "models must be an object")
    source_examples = _source_examples(spec_text, source_ops, models)
    _need(root["examples"] == source_examples, "example-source", "corpus examples disagree with checked Warrant source")
    claim_definitions = _claim_definitions(spec_text, source_ops, models)
    for name, model in models.items():
        _exact(model, {"params", "body"}, f"model {name}")
        _need(type(model["params"]) is list and len(set(model["params"])) == len(model["params"]), "model", "invalid model parameters")
        # Validate all branches and reject forbidden nodes even if unreachable.
        _validate_ir(model["body"], set(model["params"]), models, 0)
    suites = root["suites"]
    _need(type(suites) is list and suites, "schema", "suites must be non-empty")
    summary: dict[str, int] = {}
    seen = set()
    for suite in suites:
        _need(type(suite) is dict, "schema", "suite must be an object")
        required = {"id", "kind", "claim", "ops", "binders", "admit", "lhs", "rhs", "cases", "discarded", "expected_calls", "domain_text", "inputs_sha256", "expected_sha256", "result_type"}
        _exact(suite, required, "suite")
        _need(suite["kind"] == "exhaustive" and suite["id"] == f"warrant/ct_compare/exhaustive/{suite['claim']}", "suite", "suite identity or kind mismatch")
        _need(suite["claim"] in claim_definitions, "suite-source", "suite claim does not occur in checked Warrant source")
        source_claim = claim_definitions[suite["claim"]]
        for field in ("id", "claim", "ops", "binders", "admit", "lhs", "rhs", "result_type", "domain_text"):
            _need(suite[field] == source_claim[field], "suite-source", f"suite {field} disagrees with checked Warrant source")
        _need(type(suite["ops"]) is list and len(suite["ops"]) == 1 and suite["ops"][0] in root["ops"], "suite", "suite operation is outside the closed profile")
        _need(suite["result_type"] == root["ops"][suite["ops"][0]]["ret"], "suite", "suite result type disagrees with operation")
        _need(suite["id"] not in seen, "suite", "duplicate suite id"); seen.add(suite["id"])
        binders = suite["binders"]
        _need(type(binders) is list and binders, "descriptor", "suite binders must be non-empty")
        names, domains, tys = [], [], []
        for binder in binders:
            _need(type(binder) is list and len(binder) == 2 and type(binder[0]) is str, "descriptor", "malformed binder")
            values, ty = _domain_spec(binder[1]); names.append(binder[0]); domains.append(values); tys.append(ty)
        _need(len(set(names)) == len(names), "descriptor", "duplicate binder")
        _validate_ir(suite["admit"], set(names), models, 0)
        _validate_ir(suite["rhs"], set(names), models, 0)
        _validate_subject_ir(suite["lhs"], set(names), root["ops"])
        total = 1
        for values in domains: total *= len(values)
        _need(total <= 400_000, "budget", "suite exceeds exhaustive budget")
        result_type = suite["result_type"]
        prefix = _stream_prefix(suite["id"], tys, result_type)
        input_hash = hashlib.sha256(prefix)
        expected_hash = hashlib.sha256(prefix)
        admitted = discarded = 0
        for vals in itertools.product(*domains):
            env = dict(zip(names, vals))
            if not _eval(suite["admit"], env, models): discarded += 1; continue
            _need(admitted < 400_000, "budget", "admitted suite exceeds exhaustive budget")
            input_hash.update(b"".join(_encode(v, ty) for v, ty in zip(vals, tys)))
            result = _eval(suite["rhs"], env, models)
            expected_hash.update(_encode(result, result_type))
            admitted += 1
        _need(admitted > 0, "suite", "suite admits no cases")
        _need(type(suite["cases"]) is int and suite["cases"] == admitted and type(suite["discarded"]) is int and suite["discarded"] == discarded, "suite-count", "suite admitted/discarded count mismatch")
        _need(type(suite["expected_calls"]) is int and suite["expected_calls"] == admitted, "suite-calls", "expected_calls must equal admitted subject-call count")
        _need(type(suite["inputs_sha256"]) is str and suite["inputs_sha256"] == input_hash.hexdigest(), "input-digest", "suite input digest mismatch")
        _need(type(suite["expected_sha256"]) is str and suite["expected_sha256"] == expected_hash.hexdigest(), "expected-digest", "suite expected digest mismatch")
        summary[suite["claim"]] = admitted
    _need(set(summary) == set(claim_definitions), "suite", "suite set disagrees with checked Warrant claims")
    expected_faults = _fault_rows(source_ops)
    _need(_same_rows(root["faults"], expected_faults), "faults", "fault registry disagrees with source operation profile")
    expected_controls = _control_rows(expected_faults, claim_definitions)
    _need(_same_rows(root["controls"], expected_controls), "controls", "controls disagree with source operation and claim coverage")
    examples = root["examples"]
    _need(type(examples) is list, "schema", "examples must be a list")
    # Examples are oracle checked from typed arguments and the closed operation
    # semantics below. The subject operation is never invoked.
    seen_examples: set[str] = set()
    for example in examples:
        _need(type(example) is dict and set(example) == {"id", "op", "args", "expected", "result_type", "width"}, "example", "malformed example")
        _need(type(example["id"]) is str and example["id"] not in seen_examples, "example", "duplicate or invalid example id")
        seen_examples.add(example["id"])
        _verify_example(example, root["ops"])
    if tfv_text is not None: _check_tfv(tfv_text, examples, root["ops"])
    return summary


def _validate_ir(node: Any, allowed: set[str], models: dict[str, Any], depth: int) -> None:
    _need(depth <= 64 and type(node) is list and node and type(node[0]) is str, "unsupported-ir", "malformed model IR")
    tag = node[0]
    if tag == "var":
        _need(len(node) == 2 and node[1] in allowed, "unsupported-ir", "unbound model variable"); return
    if tag == "model_call":
        _need(len(node) == 3 and node[1] in models and type(node[2]) is list, "unsupported-ir", "unknown model call")
        model = models[node[1]]; _need(len(node[2]) == len(model.get("params", [])), "unsupported-ir", "model arity mismatch")
        for child in node[2]: _validate_ir(child, allowed, models, depth + 1)
        return
    if tag == "lit": _need(len(node) == 2, "unsupported-ir", "malformed literal"); _literal(node[1]); return
    if tag in ("subject_op", "op", "call") or tag not in {"ok", "err", "name", "len", "if", "eq", "ne", "lt", "le", "gt", "ge", "add", "sub", "mul", "and", "or", "not", "implies"}:
        _fail("unsupported-ir", "unknown or forbidden node in pure model")
    if tag in ("err", "name"):
        _need(len(node) == 2 and type(node[1]) is str, "unsupported-ir", "malformed error/name node"); return
    for child in node[1:]:
        _validate_ir(child, allowed, models, depth + 1)


def _validate_subject_ir(node: Any, allowed: set[str], ops: dict[str, Any]) -> None:
    _need(type(node) is list and node and type(node[0]) is str, "unsupported-ir", "malformed subject expression")
    tag = node[0]
    if tag == "subject_op":
        _need(len(node) in (3, 4) and type(node[1]) is str and type(node[2]) is list, "unsupported-ir", "malformed subject operation")
        _need(node[1] in ops, "unsupported-ir", "unknown subject operation")
        _need(len(node[2]) == len(ops[node[1]]["params"]), "unsupported-ir", "subject operation arity mismatch")
        if len(node) == 4:
            _need(type(node[3]) is list and all(type(width) is int and width >= 0 for width in node[3]), "unsupported-ir", "malformed static widths")
        for arg in node[2]: _validate_ir(arg, allowed, {}, 0)
        return
    if tag in ("op", "call"): _fail("unsupported-ir", "overloaded operation call is forbidden")
    if tag == "var":
        _need(len(node) == 2 and node[1] in allowed, "unsupported-ir", "unbound subject variable")
        return
    if tag == "lit":
        _need(len(node) == 2, "unsupported-ir", "malformed literal")
        _literal(node[1]); return
    if tag not in {"ok", "err", "name", "len", "if", "eq", "ne", "lt", "le", "gt", "ge", "add", "sub", "mul", "and", "or", "not", "implies", "model_call"}:
        _fail("unsupported-ir", "unknown subject expression node")
    if tag == "model_call":
        # Subject claims may mention the same pure reference model as the
        # right-hand side; this never enters the subject binding table.
        models = {"same": {"params": ["a", "b"]}, "same_u64": {"params": ["a", "b"]}}
        _need(len(node) == 3 and node[1] in models and type(node[2]) is list and len(node[2]) == 2, "unsupported-ir", "unknown or malformed pure model call")
        for arg in node[2]: _validate_ir(arg, allowed, models, 0)
        return
    arity = {"ok": 1, "err": 1, "name": 1, "len": 1, "not": 1, "if": 3,
             "eq": 2, "ne": 2, "lt": 2, "le": 2, "gt": 2, "ge": 2,
             "add": 2, "sub": 2, "mul": 2, "and": 2, "or": 2, "implies": 2}
    _need(len(node) == arity[tag] + 1, "unsupported-ir", "malformed subject expression arity")
    for child in node[1:]: _validate_subject_ir(child, allowed, ops)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=pathlib.Path, default=DEFAULT_CORPUS)
    parser.add_argument("--spec", type=pathlib.Path, default=DEFAULT_SPEC)
    parser.add_argument("--tfv", type=pathlib.Path, default=DEFAULT_TFV)
    args = parser.parse_args(argv)
    try:
        corpus_text = _read_limited(args.corpus, 32 * 1024 * 1024)
        spec_text = _read_limited(args.spec, 4 * 1024 * 1024)
        tfv_text = _read_limited(args.tfv, 32 * 1024 * 1024)
        result = validate_document(parse_json(corpus_text), spec_text, tfv_text)
        print(f"warrant-corpus-v1: {sum(result.values())} cases across {len(result)} suites valid")
        return 0
    except (OSError, CorpusError) as exc:
        code = exc.code if isinstance(exc, CorpusError) else "io"
        message = exc.message if isinstance(exc, CorpusError) else "could not read validator input"
        print(f"warrant-corpus-v1: {code}: {message}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    raise SystemExit(main())
