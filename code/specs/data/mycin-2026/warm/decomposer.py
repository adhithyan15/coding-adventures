#!/usr/bin/env python3
"""decomposer.py - local-first backend selection for the ONE warm-path model call.

MYCIN-2026 C2. The warm path needs exactly one model call: messy clinical prose ->
typed findings in the closed dictionary (decompose.py defines the prompt + the IR
shape). This module chooses WHICH local model serves that call, preferring the
most on-device option so patient data never has to leave the machine (privacy /
HIPAA by architecture):

    1. the trained MLX SPECIALIST (a small Gemma/Qwen LoRA, the privacy target -
       runs fully on the doctor's Apple-Silicon machine; see ../train/)
    2. local OLLAMA (a small instruct model on 127.0.0.1; still on-device)
    3. (none available) -> raise, with a clear message - never silently degrade.

Every backend returns the SAME IR via the shared prompt + coerce_ir, so the rest
of the warm path (ir_to_adj -> decide -> voi -> set-cover) is backend-agnostic and
still runs at 0 ANSWER-TIME model calls. This file only swaps the decompose call.

`decompose_text(prose)` is the live one-shot entry point - messy human input to
typed IR in one call - which the ER spine (C3) and a live consultation use.

Config (env, all optional):
    MYCIN_MLX_MODEL    HF id or path of the base (e.g. mlx-community/gemma-3-1b-it-4bit)
    MYCIN_MLX_ADAPTER  path to the trained LoRA adapter dir (e.g. ../train/adapters)
    MYCIN_OLLAMA_MODEL ollama model tag (default: qwen2.5:1.5b - the ~1 GB floor)

Usage:
    python3 decomposer.py "72M, neck stiffness, fever, neutrophilic CSF pleocytosis"
    python3 decomposer.py --which        # just report which backend is available
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
sys.path.insert(0, str(ROOT / "bench"))
import decompose as decompose_mod  # noqa: E402  (prompt_for, coerce_ir, ollama, OLLAMA)

DICT = ROOT / "warm" / "dictionary.json"
OLLAMA_TAGS = "http://127.0.0.1:11434/api/tags"
DEFAULT_OLLAMA_MODEL = "qwen2.5:1.5b"  # the ~1 GB tolerant-framework floor (BENCH_FINDINGS)


# --------------------------------------------------------------------------
# Backends. Each `*_backend()` returns a `gen(prompt) -> str` callable, or None
# if that backend is not available right now (no import, no server, no model).
# --------------------------------------------------------------------------

def mlx_backend() -> tuple[str, "callable"] | None:
    """The trained on-device specialist, if mlx-lm is importable and a model is
    configured. Lazy: importing this module never requires mlx."""
    model = os.environ.get("MYCIN_MLX_MODEL")
    if not model:
        return None
    adapter = os.environ.get("MYCIN_MLX_ADAPTER") or None
    if adapter and not Path(adapter).exists():
        return None
    try:
        from mlx_lm import generate, load
        from mlx_lm.sample_utils import make_sampler
    except ImportError:
        return None
    try:
        m, tok = load(model, adapter_path=adapter)
    except Exception:  # noqa: BLE001 - model not present / load failed -> unavailable
        return None
    sampler = make_sampler(temp=0.0)  # greedy, deterministic

    def gen(prompt: str) -> str:
        text = tok.apply_chat_template([{"role": "user", "content": prompt}],
                                       add_generation_prompt=True)
        out = generate(m, tok, prompt=text, max_tokens=512, sampler=sampler, verbose=False)
        for stop in ("<end_of_turn>", "<eos>", "<pad>", "<start_of_turn>"):
            out = out.split(stop)[0]
        return out.strip()

    label = f"mlx-specialist:{model}" + (f"+{adapter}" if adapter else "")
    return label, gen


def ollama_available() -> bool:
    """A short-timeout probe of the local Ollama server (no hang if it is down)."""
    try:
        with urllib.request.urlopen(OLLAMA_TAGS, timeout=2):
            return True
    except (urllib.error.URLError, OSError):
        return False


def ollama_backend() -> tuple[str, "callable"] | None:
    if not ollama_available():
        return None
    model = os.environ.get("MYCIN_OLLAMA_MODEL", DEFAULT_OLLAMA_MODEL)
    return f"ollama:{model}", (lambda prompt: decompose_mod.ollama(model, prompt))


def select_backend() -> tuple[str, "callable"]:
    """Pick the most on-device backend available, in priority order. Raises with a
    clear message if none is - we never silently fall back to nothing."""
    for factory in (mlx_backend, ollama_backend):
        picked = factory()
        if picked is not None:
            return picked
    raise RuntimeError(
        "no local decomposer backend available. Set MYCIN_MLX_MODEL (+ "
        "MYCIN_MLX_ADAPTER) for the on-device specialist, or start Ollama "
        "(`ollama serve`) with a small model pulled.")


# --------------------------------------------------------------------------
# The live entry point: messy human input -> typed IR, one model call.
# --------------------------------------------------------------------------

def decompose_text(prose: str, gen: "callable" = None, dictionary: dict = None,
                   case_id: str = "live") -> dict:
    """Decompose one messy clinical string into typed IR (the warm path's single
    model call). `gen` defaults to the selected local backend; pass one to inject
    a backend (e.g. for tests). Returns the coerced + tolerant-normalized IR."""
    if dictionary is None:
        dictionary = json.loads(DICT.read_text())
    if gen is None:
        _, gen = select_backend()
    raw = gen(decompose_mod.prompt_for(prose, dictionary))
    ir = decompose_mod.coerce_ir(case_id, raw)
    # Absorb small-model JSON variance the same way the bench does (so a sub-2B
    # specialist's output maps), if the tolerant normalizer is importable.
    try:
        import bench_models as bench
        if hasattr(bench, "tolerant_findings"):
            ir["findings"] = bench.tolerant_findings(ir, load_domains()).get("findings", ir["findings"])
    except Exception:  # noqa: BLE001 - normalization is best-effort, never fatal
        pass
    # Small models often emit `inference_justifications` / `discard` as bare
    # strings (a prose aside) instead of the typed objects the rest of the path
    # expects. Drop the non-conforming entries here - the decomposer is the layer
    # that absorbs model-output variance, so ir_to_adj/decide stay strict. (These
    # prose asides are non-findings and would be discarded anyway.)
    ir["inference_justifications"] = [j for j in ir["inference_justifications"] if isinstance(j, dict)]
    ir["discard"] = [d for d in ir["discard"] if isinstance(d, dict)]
    # Drop findings the deterministic step cannot parse - e.g. a bare "fever" with
    # no value (a present/absent finding the small model stated without its value).
    # We do NOT guess the value (that would fabricate data); we drop the malformed
    # shape so the pipeline never crashes. Unknown-but-well-formed functors/values
    # are still handled downstream (ir_to_adj records them as `dropped`); this only
    # removes shapes ir_to_adj would raise on. Reuses ir_to_adj's exact acceptance.
    try:
        import ir_to_adj as _ir
        ir["findings"] = [f for f in ir["findings"] if _well_formed(f, _ir)]
    except Exception:  # noqa: BLE001 - best-effort; if ir_to_adj is unavailable, leave as-is
        pass
    return ir


def _well_formed(finding: object, ir_to_adj_mod) -> bool:
    """True iff `finding` is a dict the deterministic normalizer can parse into a
    functor(value)."""
    if not isinstance(finding, dict):
        return False
    try:
        ir_to_adj_mod.normalize_finding(finding)
        return True
    except Exception:  # noqa: BLE001 - malformed shape -> drop it
        return False


def load_domains() -> dict:
    """The dictionary's functor -> value-domain map (for tolerant normalization)."""
    d = json.loads(DICT.read_text())
    return {f["functor"]: f["value_domain"] for f in d["findings"]}


def main(argv: list[str]) -> int:
    if "--which" in argv:
        try:
            name, _ = select_backend()
            print(f"decomposer backend: {name}")
            return 0
        except RuntimeError as e:
            print(f"decomposer: {e}", file=sys.stderr)
            return 1
    prose = " ".join(a for a in argv if not a.startswith("--"))
    if not prose:
        print('usage: decomposer.py "<clinical prose>"  |  --which', file=sys.stderr)
        return 2
    try:
        name, gen = select_backend()
    except RuntimeError as e:
        print(f"decomposer: {e}", file=sys.stderr)
        return 1
    print(f"[backend: {name}]  (1 on-device model call; data stays local)")
    ir = decompose_text(prose, gen=gen)
    print(json.dumps(ir, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
