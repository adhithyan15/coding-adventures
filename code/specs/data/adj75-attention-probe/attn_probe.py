#!/usr/bin/env python3
"""ADJ75 — does asking for discards change attention?

Controlled probe on an open-weights model (Qwen2.5-0.5B-Instruct). Same passage
and question; only the INSTRUCTION FRAMING changes:
  BARE     : "Answer the question."
  DISCARD  : "...for each statement decide if it applies; discard the ones that
             do not and note why; then answer."

We append "FINAL ANSWER:" so the next-token distribution IS the answer, and read,
at that final position, holding the passage fixed:
  (1) MECHANISTIC: attention mass onto the buried-override-span tokens
      (mean over heads, mean over layers), normalized by attention onto the
      whole passage -> "override attention share".
  (2) BEHAVIORAL : P(correct first-token) vs P(trap first-token) at that position.

Triangulation: if DISCARD raises BOTH the override attention share AND the
correct/trap probability ratio, that is evidence the discard framing shifts
attention/processing toward the load-bearing span (not just a raw-attention
artifact). Caveat: attention weights are a contested proxy (Jain & Wallace 2019);
the behavioral logit measure is the causal anchor.
"""

import json
import os
import sys

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

HERE = os.path.dirname(os.path.abspath(__file__))
MODEL = "Qwen/Qwen2.5-0.5B-Instruct"

BARE = "Answer the question based on the passage."
DISCARD = ("Read the passage. For each statement, decide whether it applies to the "
           "question; discard the statements that do not apply and note why they do "
           "not; then answer based only on the statements that apply.")


def span_token_indices(offsets, char_start, char_end):
    """Token indices whose char span overlaps [char_start, char_end)."""
    idx = [i for i, (s, e) in enumerate(offsets) if e > s and s < char_end and e > char_start]
    return (idx[0], idx[-1] + 1) if idx else None


def first_token_id(tok, s):
    # prompt ends with a trailing space, so the number appears WITHOUT a leading space
    ids = tok.encode(s, add_special_tokens=False)
    return ids[0]


def build(tok, instruction, item):
    user = f"{instruction}\n\nPassage: {item['passage']}\n\nQuestion: {item['question']}"
    msgs = [{"role": "user", "content": user}]
    prompt = tok.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True)
    prompt += "FINAL ANSWER: "
    return prompt


def main():
    items = json.load(open(os.path.join(HERE, "attn_items.json")))["items"]
    dev = "mps" if torch.backends.mps.is_available() else "cpu"
    print(f"loading {MODEL} on {dev} ...", flush=True)
    tok = AutoTokenizer.from_pretrained(MODEL)
    model = AutoModelForCausalLM.from_pretrained(
        MODEL, torch_dtype=torch.float32, attn_implementation="eager"
    ).to(dev).eval()

    rows = []
    for item in items:
        cid = first_token_id(tok, item["correct"])
        tid = first_token_id(tok, item["trap"])
        rec = {"id": item["id"]}
        for cond, instr in [("bare", BARE), ("discard", DISCARD)]:
            prompt = build(tok, instr, item)
            cs = prompt.find(item["override_phrase"])
            ce = cs + len(item["override_phrase"]) if cs >= 0 else -1
            enc_off = tok(prompt, return_offsets_mapping=True)
            span = span_token_indices(enc_off["offset_mapping"], cs, ce) if cs >= 0 else None
            enc = tok(prompt, return_tensors="pt").to(dev)
            with torch.no_grad():
                out = model(**enc, output_attentions=True)
            logits = out.logits[0, -1].float()
            probs = torch.softmax(logits, dim=-1)
            p_correct = probs[cid].item()
            p_trap = probs[tid].item()
            # attention from last position to override span, mean over heads & layers
            atts = out.attentions  # tuple[L] of [1, H, S, S]
            L = len(atts)
            last = enc["input_ids"].shape[1] - 1
            # passage token range: approximate as everything; we normalize by span vs passage
            if span:
                span_share = []
                for a in atts:
                    v = a[0, :, last, span[0]:span[1]].mean().item()  # mean over heads & span
                    span_share.append(v)
                attn_span = sum(span_share) / L
                # baseline: mean attention onto a same-length window is 1/S; report ratio to uniform
                S = enc["input_ids"].shape[1]
                attn_span_vs_uniform = attn_span / (1.0 / S)
            else:
                attn_span = float("nan"); attn_span_vs_uniform = float("nan")
            rec[cond] = {
                "p_correct": p_correct, "p_trap": p_trap,
                "ratio": p_correct / p_trap if p_trap > 0 else float("inf"),
                "attn_span_per_token": attn_span,
                "attn_span_vs_uniform": attn_span_vs_uniform,
                "span_found": span is not None,
            }
        rows.append(rec)
        b, d = rec["bare"], rec["discard"]
        print(f"{item['id']}: ratio P(correct)/P(trap)  bare={b['ratio']:.2f} -> discard={d['ratio']:.2f}   "
              f"attn_span(xUniform) bare={b['attn_span_vs_uniform']:.2f} -> discard={d['attn_span_vs_uniform']:.2f}",
              flush=True)

    json.dump(rows, open(os.path.join(HERE, "attn_results.json"), "w"), indent=2)

    def agg(cond, key):
        vals = [r[cond][key] for r in rows if r[cond]["span_found"] and r[cond][key] == r[cond][key]]
        return sum(vals) / len(vals) if vals else float("nan")

    print("\n" + "=" * 70)
    print("AGGREGATE (n={})".format(len(rows)))
    print("=" * 70)
    print(f"  P(correct)/P(trap) ratio:   bare={agg('bare','ratio'):.2f}   discard={agg('discard','ratio'):.2f}")
    print(f"  P(correct):                 bare={agg('bare','p_correct'):.3f}  discard={agg('discard','p_correct'):.3f}")
    print(f"  P(trap):                    bare={agg('bare','p_trap'):.3f}  discard={agg('discard','p_trap'):.3f}")
    print(f"  override attn (x uniform):  bare={agg('bare','attn_span_vs_uniform'):.2f}   discard={agg('discard','attn_span_vs_uniform'):.2f}")
    # paired counts
    up_ratio = sum(1 for r in rows if r['discard']['ratio'] > r['bare']['ratio'])
    up_attn = sum(1 for r in rows if r['discard']['attn_span_vs_uniform'] > r['bare']['attn_span_vs_uniform'])
    print(f"\n  items where DISCARD raised correct/trap ratio: {up_ratio}/{len(rows)}")
    print(f"  items where DISCARD raised override attention: {up_attn}/{len(rows)}")


if __name__ == "__main__":
    main()
