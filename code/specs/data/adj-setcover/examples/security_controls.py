#!/usr/bin/env python3
"""security_controls.py — the SAME set-cover library, on a NON-medical domain.

Minimum-cost selection of security controls that cover every threat, using the
exact same generic `setcover` library that derives a drug regimen — proving the
constructs (n-ary combination coverage + defeasance) are domain-general, not
medicine-specific.

  - threats             = the requirements to cover
  - controls            = the elements (each with an operational-burden cost)
  - combination         = defense-in-depth: MFA + monitoring JOINTLY cover
                          credential-theft, which neither covers alone (n-ary)
  - defeater            = a control with a known bypass (an observed CVE) — its
                          coverage edge is voided, exactly like a culture showing
                          a drug is resistant

Run:  python3 security_controls.py
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from setcover import Combination, SetCoverSpec, find_cli, solve  # noqa: E402

# Operational-burden cost per control (lower = preferred).
COSTS = {"mfa": 1, "monitoring": 2, "edr": 1, "waf": 2, "dlp": 2}
# What each control covers ALONE (credential_theft is covered by NO single control).
COVERS = {"edr": ["malware"], "waf": ["malware"], "dlp": ["data_exfiltration"]}
# Defense-in-depth: MFA + monitoring together cover credential theft.
COMBINATIONS = [Combination(("mfa", "monitoring"), "credential_theft")]
THREATS = ["credential_theft", "malware", "data_exfiltration"]


def scenario(title: str, defeated: list[tuple[str, str]], cli, cache_dir) -> None:
    spec = SetCoverSpec(costs=COSTS, requirements=THREATS, covers=COVERS,
                        combinations=COMBINATIONS, defeated=defeated)
    res = solve(spec, cli=cli, cache_dir=cache_dir)
    print("=" * 70 + f"\n{title}\n" + "=" * 70)
    if defeated:
        print(f"  defeated edges (known bypass): {defeated}")
    if res.selected is None:
        print(f"  NO control set covers every threat ({res.outcome}) -> escalate.")
        return
    print(f"  CONTROL PLAN (cost {res.cost:.0f}): {' + '.join(res.selected)}"
          + ("   [cache hit]" if res.cached else ""))
    for c in res.used_combinations:
        print(f"    + combination {' + '.join(c.elements)} covers {c.covers}")


def main() -> int:
    cli = find_cli()
    if cli is None:
        print("security_controls: adj-lang-cli not built", file=sys.stderr)
        return 3
    cache = Path(__file__).resolve().parent / "_cache"
    # A: nominal — malware covered by the cheaper edr.
    scenario("1) Nominal threat model", [], cli, cache)
    # B: a CVE bypasses edr (observed) — its malware edge is defeated, so the cover
    #    re-derives onto the costlier waf. Same defeasance mechanic as drug resistance.
    scenario("2) edr has a known bypass (CVE) — coverage re-derives", [("edr", "malware")], cli, cache)
    # C: re-run A to demonstrate the content-addressed cache (instant, identical).
    scenario("3) Re-run the nominal model (content-addressed cache)", [], cli, cache)
    print("\nSame generic library as the drug-regimen deriver — combinations + "
          "defeasance are domain-general.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
