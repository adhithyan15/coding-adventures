#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["torch", "transformers>=4.44", "accelerate", "sentencepiece"]
# ///
"""ADJ85 — does ADJ82's attention-routing finding hold ACROSS MODEL FAMILIES?

ADJ82 showed, on Qwen2.5-0.5B-Instruct, that a *targeted COPY* extraction routes ~2x more
attention onto the load-bearing override span than a free ANSWER — the framework helps a
small model by routing attention through the load-bearing byte via task design, NOT by
magically refocusing the model. This script re-runs the IDENTICAL probe (same 5 items, same
metric: override-attn-share during generation, normalized by attention-to-the-passage) on
ANY list of HF causal-LM instruct models, so we can see whether the ~2x routing replicates
across families (Llama, Gemma, Phi, SmolLM, ...) or is a Qwen idiosyncrasy.

Run (self-contained via uv inline deps):
  uv run attn_cross.py Qwen/Qwen2.5-0.5B-Instruct meta-llama/Llama-3.2-1B-Instruct \
                       google/gemma-2-2b-it microsoft/Phi-3.5-mini-instruct \
                       HuggingFaceTB/SmolLM2-1.7B-Instruct
(Gated models — Llama, Gemma — need `huggingface-cli login` / a token first.)

The metric is byte-for-byte ADJ82's so results are directly comparable to attn2_results.json.
"""
import json
import os
import statistics as st
import sys
import traceback

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

# IDENTICAL items to ADJ82 (do not change — keeps the comparison apples-to-apples).
ITEMS = [
    {"passage": "Acme Corp offers a generous paid-leave policy. The standard annual allotment is 20 days. "
                "Part-time staff hired after January 2020 accrue at a reduced rate of 12 days per year. "
                "Jordan joined Acme as a part-time employee in March 2022.",
     "override": "reduced rate of 12 days",
     "copy_q": "Copy the exact words from the passage that describe how much leave part-time staff accrue.",
     "answer_q": "How many days of annual paid leave does Jordan accrue?"},
    {"passage": "The posted speed limit on this highway is 65 mph for general traffic. "
                "Commercial trucks are restricted to a lower limit of 55 mph on the same stretch. "
                "The vehicle in question is a commercial truck.",
     "override": "lower limit of 55 mph",
     "copy_q": "Copy the exact words from the passage that describe the limit for commercial trucks.",
     "answer_q": "What speed limit applies to this vehicle?"},
    {"passage": "Overdue library books are fined $0.25 per day. Borrowers younger than eighteen are "
                "exempt from all overdue fines. A twelve-year-old borrower returns a book ten days late.",
     "override": "younger than eighteen are exempt from all overdue fines",
     "copy_q": "Copy the exact words from the passage that describe the rule for borrowers under eighteen.",
     "answer_q": "What overdue fine is charged to this borrower?"},
    {"passage": "Products carry a one-year warranty covering defects. Units sold as refurbished instead "
                "carry a reduced warranty term of 90 days. The customer purchased a refurbished unit.",
     "override": "refurbished instead carry a reduced warranty term of 90 days",
     "copy_q": "Copy the exact words from the passage that describe the warranty for refurbished units.",
     "answer_q": "What is the warranty period for this customer's purchase?"},
    {"passage": "Loyalty members receive a 10% discount on most purchases. Items marked clearance are "
                "excluded from all discounts. A loyalty member is buying an item marked clearance.",
     "override": "clearance are excluded from all discounts",
     "copy_q": "Copy the exact words from the passage that describe the rule for clearance items.",
     "answer_q": "What discount applies to this member's clearance item?"},
]


def span_tokens(offsets, cs, ce):
    return [i for i, (s, e) in enumerate(offsets) if e > s and s < ce and e > cs]


def device():
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


_DT = {"float32": torch.float32, "bfloat16": torch.bfloat16, "float16": torch.float16}


def run_model(model_id, dev):
    # default float32 (faithful to ADJ82's numerical precision); ATTN_DTYPE=bfloat16 for big
    # models that won't fit in fp32 — the COPY-vs-ANSWER ratio is within-model, so dtype is fine.
    dtype = _DT[os.environ.get("ATTN_DTYPE", "float32")]
    tok = AutoTokenizer.from_pretrained(model_id)
    model = AutoModelForCausalLM.from_pretrained(
        model_id, dtype=dtype, attn_implementation="eager", low_cpu_mem_usage=True).to(dev).eval()
    rows = []
    for it in ITEMS:
        rec = {"override": it["override"]}
        for cond, q in [("COPY", it["copy_q"]), ("ANSWER", it["answer_q"])]:
            user = f"Passage: {it['passage']}\n\n{q}"
            prompt = tok.apply_chat_template([{"role": "user", "content": user}],
                                             tokenize=False, add_generation_prompt=True)
            p_cs = prompt.find(it["passage"]); p_ce = p_cs + len(it["passage"])
            o_cs = prompt.find(it["override"]); o_ce = o_cs + len(it["override"])
            enc = tok(prompt, return_offsets_mapping=True, return_tensors="pt")
            offs = enc.pop("offset_mapping")[0].tolist()
            passage_idx = list(set(span_tokens(offs, p_cs, p_ce)))
            override_idx = span_tokens(offs, o_cs, o_ce)
            enc = {k: v.to(dev) for k, v in enc.items()}
            plen = enc["input_ids"].shape[1]
            with torch.no_grad():
                gen = model.generate(**enc, max_new_tokens=40, do_sample=False,
                                     pad_token_id=tok.eos_token_id)
            full = gen[0].unsqueeze(0)
            resp = tok.decode(gen[0][plen:], skip_special_tokens=True)
            with torch.no_grad():
                atts = model(full, output_attentions=True).attentions  # tuple[L] [1,H,S,S]
            shares = []
            for p in range(plen, full.shape[1]):
                per_layer = []
                for a in atts:
                    row = a[0, :, p, :]
                    pa = row[:, passage_idx].sum().item() if passage_idx else 0.0
                    oa = row[:, override_idx].sum().item() if override_idx else 0.0
                    per_layer.append(oa / pa if pa > 0 else 0.0)
                shares.append(sum(per_layer) / len(per_layer))
            rec[cond] = {"override_attn_share": round(sum(shares) / len(shares), 4) if shares else None,
                         "response": resp[:60]}
        rows.append(rec)
        print(f"  override={it['override']!r}")
        print(f"    COPY  ={rec['COPY']['override_attn_share']}  resp={rec['COPY']['response']!r}")
        print(f"    ANSWER={rec['ANSWER']['override_attn_share']}  resp={rec['ANSWER']['response']!r}")
    c = st.mean(r["COPY"]["override_attn_share"] for r in rows)
    a = st.mean(r["ANSWER"]["override_attn_share"] for r in rows)
    return {"model": model_id, "rows": rows, "mean_COPY": round(c, 4), "mean_ANSWER": round(a, 4),
            "ratio": round(c / a, 2) if a else None}


def main():
    models = sys.argv[1:] or ["Qwen/Qwen2.5-0.5B-Instruct"]
    dev = device()
    print(f"device={dev}  models={models}\n")
    summary = []
    for m in models:
        print(f"==== {m} ====")
        try:
            summary.append(run_model(m, dev))
        except Exception as e:  # noqa: BLE001 — keep going across families
            print(f"  !! FAILED: {e}")
            traceback.print_exc()
            summary.append({"model": m, "error": str(e)})
        print()
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "attn_cross_results.json")
    json.dump(summary, open(out, "w"), indent=2)
    print("==== CROSS-FAMILY SUMMARY (override-attn-share during generation) ====")
    print(f"  {'model':42s} {'COPY':>7} {'ANSWER':>7} {'ratio':>6}")
    for s in summary:
        if "error" in s:
            print(f"  {s['model'][:42]:42s}  ERROR: {s['error'][:40]}")
        else:
            print(f"  {s['model'][:42]:42s} {s['mean_COPY']:7.3f} {s['mean_ANSWER']:7.3f} {str(s['ratio'])+'x':>6}")
    print("\nADJ82 reference (Qwen2.5-0.5B): COPY=0.232  ANSWER=0.157  ratio=1.5x  (3/5 strong ~2x)")
    print("Finding holds across a family iff COPY > ANSWER (ratio > ~1.3) on the targeted-copy items.")


if __name__ == "__main__":
    main()
