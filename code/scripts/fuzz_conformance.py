#!/usr/bin/env python3
"""Validate the fuzz engine conformance corpus and generate cases.tfv.

`code/specs/fixtures/fuzz-v1/cases.json` pins the behaviour every language's
coverage-guided fuzz engine (VER06) must reproduce. This script is the independent
reference: standard-library Python, written apart from any lane. It re-derives
every expected value, so a lane that passes the corpus agrees with an independent
oracle, not just with itself.

    python code/scripts/fuzz_conformance.py           # check (CI)
    python code/scripts/fuzz_conformance.py --write   # regenerate cases.tfv

What `--check` enforces (VER06):
* closed schema for every case, and unique case IDs;
* bit-flip mutators invert exact bit positions modulo bit-length;
* byte-flip mutators match XOR 0xFF at target byte offsets;
* arithmetic mutators perform endian-aware wrapping additions;
* boundary mutators embed exact interest integers;
* block mutators (delete, insert, duplicate) maintain slice boundaries;
* splice operations combine target prefixes and donor suffixes;
* compound mutation sequences are 100% deterministic under SplitMix64;
* corpus admission correctly detects novel coverage edges;
* crash deduplication groups matching signatures and separates distinct crashes;
* failure ID validation rejects unregistered IDs outside the closed vocabulary;
* private failure formatting produces opaque tokens without raw payload bytes;
* cases.tfv matches cases.json compilation byte-for-byte.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tfv

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "code/specs/fixtures/fuzz-v1"
MASK = (1 << 64) - 1

SHAPES = {
    "mutate_bit_flip": {"input", "bit_offset", "count", "output"},
    "mutate_byte_flip": {"input", "byte_offset", "count", "output"},
    "mutate_arithmetic": {"input", "byte_offset", "delta", "width", "endian", "output"},
    "mutate_boundary": {"input", "byte_offset", "width", "val", "endian", "output"},
    "mutate_block_delete": {"input", "offset", "length", "output"},
    "mutate_block_insert": {"input", "offset", "insert_bytes", "output"},
    "mutate_block_duplicate": {"input", "src_offset", "length", "dst_offset", "output"},
    "mutate_splice": {"target", "donor", "target_split", "donor_split", "output"},
    "mutate_sequence": {"input", "seed", "steps", "output"},
    "corpus_admit": {"existing_edges", "trace_edges", "admitted", "new_edges"},
    "crash_dedup": {
        "crash_a_kind",
        "crash_a_frames",
        "crash_b_kind",
        "crash_b_frames",
        "is_duplicate",
    },
    "failure_id_valid": {"input", "valid"},
    "private_reporting": {
        "target",
        "failure_id",
        "raw_payload",
        "sig_hash",
        "tap_line",
        "contains_raw_payload",
    },
    "corpus_reject": {"text"},
}

REGISTERED_FAILURE_IDS = {
    "fuzz-crash",
    "fuzz-timeout",
    "fuzz-oom",
    "fuzz-leak",
    "fuzz-panic",
}


# --- PRNG Reference (TF01 §5.1 / VER05 §2 / VER06 §2) -----------------------
class SplitMix64:
    def __init__(self, seed: int) -> None:
        self.state = seed & MASK

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        return z ^ (z >> 31)

    def bounded(self, n: int) -> int:
        assert 1 <= n <= (1 << 32), f"bounded(n) requires 1 <= n <= 2^32, got {n}"
        threshold = ((1 << 64) - n) % n
        while True:
            x = self.next()
            if x >= threshold:
                return x % n


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for byte in data:
        h = ((h ^ byte) * 0x100000001B3) & MASK
    return h


def sub_seed(run_seed: int, target_id: str) -> int:
    return SplitMix64(run_seed ^ fnv1a64(target_id.encode("utf-8"))).next()


# --- Mutator Algebra Reference (VER06 §3) -----------------------------------
def bit_flip(data: bytes, bit_offset: int, count: int = 1) -> bytes:
    if not data:
        return b""
    buf = bytearray(data)
    total_bits = len(buf) * 8
    for i in range(count):
        pos = (bit_offset + i) % total_bits
        byte_idx = pos // 8
        bit_idx = pos % 8
        buf[byte_idx] ^= 1 << bit_idx
    return bytes(buf)


def byte_flip(data: bytes, byte_offset: int, count: int = 1) -> bytes:
    if not data:
        return b""
    buf = bytearray(data)
    total_bytes = len(buf)
    for i in range(count):
        pos = (byte_offset + i) % total_bytes
        buf[pos] ^= 0xFF
    return bytes(buf)


def byte_arithmetic(
    data: bytes, byte_offset: int, delta: int, width: int = 1, endian: str = "little"
) -> bytes:
    if len(data) < width:
        return data
    buf = bytearray(data)
    offset = byte_offset % (len(buf) - width + 1)
    val = int.from_bytes(buf[offset : offset + width], byteorder=endian, signed=False)
    modulus = 1 << (8 * width)
    new_val = (val + delta) % modulus
    buf[offset : offset + width] = new_val.to_bytes(
        length=width, byteorder=endian, signed=False
    )
    return bytes(buf)


def boundary_insert(
    data: bytes, byte_offset: int, width: int, val: int, endian: str = "little"
) -> bytes:
    if len(data) < width:
        return data
    buf = bytearray(data)
    offset = byte_offset % (len(buf) - width + 1)
    buf[offset : offset + width] = val.to_bytes(
        length=width, byteorder=endian, signed=False
    )
    return bytes(buf)


def block_delete(data: bytes, offset: int, length: int) -> bytes:
    if not data or length <= 0:
        return data
    actual_offset = offset % len(data)
    actual_length = min(length, len(data) - actual_offset)
    return data[:actual_offset] + data[actual_offset + actual_length :]


def block_insert(data: bytes, offset: int, insert_bytes: bytes) -> bytes:
    if not data:
        return insert_bytes
    actual_offset = offset % (len(data) + 1)
    return data[:actual_offset] + insert_bytes + data[actual_offset:]


def block_duplicate(
    data: bytes, src_offset: int, length: int, dst_offset: int
) -> bytes:
    if not data or length <= 0:
        return data
    actual_src = src_offset % len(data)
    actual_len = min(length, len(data) - actual_src)
    chunk = data[actual_src : actual_src + actual_len]
    actual_dst = dst_offset % (len(data) + 1)
    return data[:actual_dst] + chunk + data[actual_dst:]


def splice(target: bytes, donor: bytes, target_split: int, donor_split: int) -> bytes:
    t_split = max(0, min(target_split, len(target)))
    d_split = max(0, min(donor_split, len(donor)))
    return target[:t_split] + donor[d_split:]


def mutate_sequence(data: bytes, seed: int, steps: int) -> bytes:
    rng = SplitMix64(seed)
    current = data
    for _ in range(steps):
        if not current:
            current = b"\x00"
        op_choice = rng.bounded(7)
        if op_choice == 0:
            current = bit_flip(current, rng.bounded(len(current) * 8), 1)
        elif op_choice == 1:
            current = byte_flip(current, rng.bounded(len(current)), 1)
        elif op_choice == 2:
            deltas = [-35, -8, -1, 1, 8, 35]
            delta = deltas[rng.bounded(len(deltas))]
            current = byte_arithmetic(current, rng.bounded(len(current)), delta, 1)
        elif op_choice == 3:
            interest = [0, 1, 127, 128, 255]
            val = interest[rng.bounded(len(interest))]
            current = boundary_insert(current, rng.bounded(len(current)), 1, val)
        elif op_choice == 4:
            if len(current) > 1:
                del_len = 1 + rng.bounded(max(1, len(current) // 2))
                current = block_delete(current, rng.bounded(len(current)), del_len)
        elif op_choice == 5:
            ins_byte = bytes([rng.bounded(256)])
            current = block_insert(current, rng.bounded(len(current) + 1), ins_byte)
        else:
            if len(current) > 1:
                dup_len = 1 + rng.bounded(max(1, len(current) // 2))
                current = block_duplicate(
                    current,
                    rng.bounded(len(current)),
                    dup_len,
                    rng.bounded(len(current) + 1),
                )
    return current


# --- Corpus Admission & Deduplication Reference (VER06 §4, §5) --------------
def corpus_admit(
    existing_edges: list[int], trace_edges: list[int]
) -> tuple[bool, int]:
    existing_set = set(existing_edges)
    trace_set = set(trace_edges)
    new_edges = trace_set - existing_set
    if new_edges:
        return True, len(new_edges)
    return False, 0


def crash_signature(kind: str, frames: list[str]) -> str:
    raw = f"{kind}:" + ":".join(frames)
    h = fnv1a64(raw.encode("utf-8"))
    return f"{h & 0xFFFFFFFF:08x}"


def is_duplicate_crash(crash_a: dict, crash_b: dict) -> bool:
    sig_a = crash_signature(crash_a["kind"], crash_a["frames"])
    sig_b = crash_signature(crash_b["kind"], crash_b["frames"])
    return sig_a == sig_b


def format_tap_failure(
    test_num: int, target_id: str, failure_id: str, sig_hash: str
) -> str:
    return f"not ok {test_num} - {target_id} [{failure_id}:{sig_hash}]"


# --- Corpus Compilation & Validation ----------------------------------------
def load_cases(fixture_dir: Path | None = None) -> list[dict]:
    p = (fixture_dir or FIXTURE) / "cases.json"
    doc = json.loads(p.read_text(encoding="utf-8"))
    if doc.get("schema_version") != 1:
        raise SystemExit(f"{p}: unsupported schema_version")
    return doc["cases"]


def to_records(cases: list[dict]) -> list[tfv.Record]:
    records = []
    for case in cases:
        fields = [("op", ("text", case["op"]))]
        fields += [
            (key, tfv.from_json(value))
            for key, value in case.items()
            if key not in ("id", "op")
        ]
        records.append((case["id"], fields))
    return records


def check(cases: list[dict]) -> list[str]:
    problems: list[str] = []
    seen_ids: set[str] = set()

    for idx, c in enumerate(cases):
        cid = c.get("id")
        if not cid or not isinstance(cid, str):
            problems.append(f"case[{idx}]: missing or non-string id")
            continue
        if cid in seen_ids:
            problems.append(f"{cid}: duplicate case id")
        seen_ids.add(cid)

        op = c.get("op")
        if op not in SHAPES:
            problems.append(f"{cid}: unknown op {op!r}")
            continue

        required = SHAPES[op]
        actual = {k for k in c if k not in ("id", "op")}
        if actual != required:
            problems.append(
                f"{cid}: fields {sorted(actual)} != required {sorted(required)}"
            )
            continue

        try:
            if op == "mutate_bit_flip":
                inp = bytes.fromhex(c["input"]["hex"])
                bit_off = int(c["bit_offset"]["value"])
                count = int(c["count"]["value"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = bit_flip(inp, bit_off, count)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: bit_flip expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_byte_flip":
                inp = bytes.fromhex(c["input"]["hex"])
                byte_off = int(c["byte_offset"]["value"])
                count = int(c["count"]["value"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = byte_flip(inp, byte_off, count)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: byte_flip expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_arithmetic":
                inp = bytes.fromhex(c["input"]["hex"])
                byte_off = int(c["byte_offset"]["value"])
                delta = int(c["delta"]["value"])
                width = int(c["width"]["value"])
                endian = c["endian"]
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = byte_arithmetic(inp, byte_off, delta, width, endian)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: byte_arithmetic expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_boundary":
                inp = bytes.fromhex(c["input"]["hex"])
                byte_off = int(c["byte_offset"]["value"])
                width = int(c["width"]["value"])
                val = int(c["val"]["value"])
                endian = c["endian"]
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = boundary_insert(inp, byte_off, width, val, endian)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: boundary_insert expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_block_delete":
                inp = bytes.fromhex(c["input"]["hex"])
                offset = int(c["offset"]["value"])
                length = int(c["length"]["value"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = block_delete(inp, offset, length)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: block_delete expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_block_insert":
                inp = bytes.fromhex(c["input"]["hex"])
                offset = int(c["offset"]["value"])
                ins = bytes.fromhex(c["insert_bytes"]["hex"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = block_insert(inp, offset, ins)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: block_insert expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_block_duplicate":
                inp = bytes.fromhex(c["input"]["hex"])
                src = int(c["src_offset"]["value"])
                length = int(c["length"]["value"])
                dst = int(c["dst_offset"]["value"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = block_duplicate(inp, src, length, dst)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: block_duplicate expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_splice":
                target = bytes.fromhex(c["target"]["hex"])
                donor = bytes.fromhex(c["donor"]["hex"])
                t_split = int(c["target_split"]["value"])
                d_split = int(c["donor_split"]["value"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = splice(target, donor, t_split, d_split)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: splice expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "mutate_sequence":
                inp = bytes.fromhex(c["input"]["hex"])
                seed = int(c["seed"]["value"])
                steps = int(c["steps"]["value"])
                exp = bytes.fromhex(c["output"]["hex"])
                actual_out = mutate_sequence(inp, seed, steps)
                if actual_out != exp:
                    problems.append(
                        f"{cid}: mutate_sequence expected {exp.hex()}, got {actual_out.hex()}"
                    )

            elif op == "corpus_admit":
                existing = [int(x["value"]) for x in c["existing_edges"]]
                trace = [int(x["value"]) for x in c["trace_edges"]]
                exp_admitted = c["admitted"]
                exp_new_edges = int(c["new_edges"]["value"])
                act_admitted, act_new_edges = corpus_admit(existing, trace)
                if act_admitted != exp_admitted or act_new_edges != exp_new_edges:
                    problems.append(
                        f"{cid}: corpus_admit expected ({exp_admitted}, {exp_new_edges}), "
                        f"got ({act_admitted}, {act_new_edges})"
                    )

            elif op == "crash_dedup":
                crash_a = {
                    "kind": c["crash_a_kind"],
                    "frames": c["crash_a_frames"],
                }
                crash_b = {
                    "kind": c["crash_b_kind"],
                    "frames": c["crash_b_frames"],
                }
                exp_dup = c["is_duplicate"]
                act_dup = is_duplicate_crash(crash_a, crash_b)
                if act_dup != exp_dup:
                    problems.append(
                        f"{cid}: crash_dedup expected {exp_dup}, got {act_dup}"
                    )

            elif op == "failure_id_valid":
                tok = c["input"]
                exp_valid = c["valid"]
                act_valid = tok in REGISTERED_FAILURE_IDS
                if act_valid != exp_valid:
                    problems.append(
                        f"{cid}: failure_id_valid expected {exp_valid} for {tok!r}"
                    )

            elif op == "private_reporting":
                target = c["target"]
                fid = c["failure_id"]
                raw_hex = c["raw_payload"]["hex"]
                sig = c["sig_hash"]
                exp_tap = c["tap_line"]
                exp_contains = c["contains_raw_payload"]
                act_tap = format_tap_failure(1, target, fid, sig)
                if act_tap != exp_tap:
                    problems.append(
                        f"{cid}: tap_line mismatch. expected {exp_tap!r}, got {act_tap!r}"
                    )
                act_contains = raw_hex in act_tap
                if act_contains != exp_contains:
                    problems.append(
                        f"{cid}: privacy leak: raw payload {raw_hex} found in tap line"
                    )

            elif op == "corpus_reject":
                # Ensure parsing the reject case fails
                text = c["text"]
                try:
                    tfv.parse_file(text)
                except tfv.TfvError:
                    pass
                else:
                    problems.append(
                        f"{cid}: expected corpus_reject text to fail decode"
                    )

        except (ValueError, KeyError, TypeError, IndexError) as e:
            problems.append(f"{cid}: exception during validation: {e}")

    missing_ops = set(SHAPES) - {c.get("op") for c in cases}
    if missing_ops:
        problems.append(f"corpus missing operations: {sorted(missing_ops)}")

    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--write", action="store_true", help="regenerate cases.tfv")
    args = parser.parse_args(argv)

    cases = load_cases()
    problems = check(cases)
    generated = tfv.encode_file(to_records(cases))
    target = FIXTURE / "cases.tfv"
    if args.write:
        target.write_text(generated, encoding="utf-8", newline="\n")
    elif not target.exists() or target.read_text(encoding="utf-8") != generated:
        problems.append("cases.tfv is stale: run with --write")
    elif tfv.parse_file(generated) != to_records(cases):
        problems.append("cases.tfv does not round-trip")

    for problem in problems:
        print(f"fuzz-v1: {problem}", file=sys.stderr)
    print(f"fuzz-v1: {len(cases)} cases, {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
