#!/usr/bin/env python3
"""ADJ82 — attention revisited (fixing ADJ75's flaws).

ADJ75 measured a STATIC last-position effect and used a length-confounded
normalization (attn / (1/S)); it found no clean signal. Here we fix all three:
  - measure attention DURING GENERATION (at the generated-token positions),
  - normalize by attention-to-the-PASSAGE (length-robust; share of passage-attention
    landing on the load-bearing span), NOT by 1/S,
  - report raw shares and triangulate, with the Jain&Wallace caveat noted.

Design: same passage, two framings:
  COPY   = "copy the exact words describing the subject's status"  (the ADJ81 step
           that worked) -- copying a span should route attention THROUGH it.
  ANSWER = "how many days ...?"                                    (free answer, skims)
Hypothesis: override-span attention-share during generation is much higher under COPY
than ANSWER -> the framework helps by making the TASK route attention through the
load-bearing span, not by changing the model.
"""
import json
import os
import re
import sys

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

MODEL = "Qwen/Qwen2.5-0.5B-Instruct"
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


def main():
    dev = "mps" if torch.backends.mps.is_available() else "cpu"
    tok = AutoTokenizer.from_pretrained(MODEL)
    model = AutoModelForCausalLM.from_pretrained(MODEL, dtype=torch.float32,
                                                 attn_implementation="eager").to(dev).eval()
    rows = []
    for it in ITEMS:
        rec = {"override": it["override"]}
        for cond, q in [("COPY", it["copy_q"]), ("ANSWER", it["answer_q"])]:
            user = f"Passage: {it['passage']}\n\n{q}"
            prompt = tok.apply_chat_template([{"role": "user", "content": user}],
                                             tokenize=False, add_generation_prompt=True)
            # locate passage + override char spans within the prompt string
            p_cs = prompt.find(it["passage"]); p_ce = p_cs + len(it["passage"])
            o_cs = prompt.find(it["override"]); o_ce = o_cs + len(it["override"])
            enc = tok(prompt, return_offsets_mapping=True, return_tensors="pt")
            offs = enc.pop("offset_mapping")[0].tolist()
            passage_idx = set(span_tokens(offs, p_cs, p_ce))
            override_idx = span_tokens(offs, o_cs, o_ce)
            enc = {k: v.to(dev) for k, v in enc.items()}
            plen = enc["input_ids"].shape[1]
            with torch.no_grad():
                gen = model.generate(**enc, max_new_tokens=40, do_sample=False,
                                     pad_token_id=tok.eos_token_id)
            full = gen[0].unsqueeze(0)
            resp_text = tok.decode(gen[0][plen:], skip_special_tokens=True)
            with torch.no_grad():
                out = model(full, output_attentions=True)
            atts = out.attentions  # tuple[L] [1,H,S,S]
            gen_positions = range(plen, full.shape[1])
            shares = []
            for p in gen_positions:
                per_layer = []
                for a in atts:
                    row = a[0, :, p, :]  # [H, S]
                    pa = row[:, list(passage_idx)].sum().item() if passage_idx else 0.0
                    oa = row[:, override_idx].sum().item() if override_idx else 0.0
                    per_layer.append(oa / pa if pa > 0 else 0.0)
                shares.append(sum(per_layer) / len(per_layer))
            share = sum(shares) / len(shares) if shares else float("nan")
            rec[cond] = {"override_attn_share": round(share, 3), "response": resp_text[:60]}
        rows.append(rec)
        print(f"override={it['override']!r}")
        print(f"  COPY   override-attn-share during gen = {rec['COPY']['override_attn_share']:.3f}   resp={rec['COPY']['response']!r}")
        print(f"  ANSWER override-attn-share during gen = {rec['ANSWER']['override_attn_share']:.3f}   resp={rec['ANSWER']['response']!r}")
    json.dump(rows, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "attn2_results.json"), "w"), indent=2)
    import statistics as st
    print(f"\nMEAN override-attn-share during generation (n={len(rows)}):")
    print(f"  COPY   = {st.mean(r['COPY']['override_attn_share'] for r in rows):.3f}")
    print(f"  ANSWER = {st.mean(r['ANSWER']['override_attn_share'] for r in rows):.3f}")


if __name__ == "__main__":
    main()
