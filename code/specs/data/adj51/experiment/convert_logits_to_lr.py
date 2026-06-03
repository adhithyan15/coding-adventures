#!/usr/bin/env python3
"""Convert signed logit_delta values in contributes/interacts clauses
to LRs (multiplicative). adj-lang's surface syntax uses LR > 0; the
deriver wrote signed logits per the prompt's math reference. Conversion
is LR = exp(logit_delta), so the engine sees identical posterior shifts.
"""

import math
import re
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: convert_logits_to_lr.py <in.adj> <out.adj>")
        return 2
    src = open(sys.argv[1]).read()

    def replace_signed(match: re.Match) -> str:
        keyword = match.group(1)
        signed_logit = float(match.group(2))
        lr = math.exp(signed_logit)
        return f"{keyword} {lr:.4f}"

    # Match: contributes <signed_number> from ...
    pattern = re.compile(
        r"^(contributes|interacts)\s+(-?\d+(?:\.\d+)?)",
        re.MULTILINE,
    )
    out = pattern.sub(replace_signed, src)

    open(sys.argv[2], "w").write(out)
    n_changes = len(pattern.findall(src))
    print(f"converted {n_changes} clause magnitudes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
