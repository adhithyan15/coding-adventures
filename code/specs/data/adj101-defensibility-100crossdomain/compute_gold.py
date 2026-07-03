#!/usr/bin/env python3
"""ADJ101 — derive the program-required items' gold answers FROM THE TOOLS.

Byte-provenance applied to the benchmark's own answers: every computational gold value is
computed here by SymPy / RDKit (the same tools the framework arm must emit), not hand-typed.
Run to verify the gold in items_pilot_compute.json is reproducible.

Run:  python3 compute_gold.py
"""
import json
import os

import sympy as sp
from rdkit import Chem
from rdkit.Chem import Descriptors, rdMolDescriptors

HERE = os.path.dirname(os.path.abspath(__file__))


def gold():
    x = sp.symbols("x")
    asp = Chem.MolFromSmiles("CC(=O)Oc1ccccc1C(=O)O")
    caf = Chem.MolFromSmiles("CN1C=NC2=C1C(=O)N(C(=O)N2C)C")
    return {
        # projectile max height: (v0 * sin(theta))^2 / (2 g)
        "PHYS1": round(float((20 * sp.sin(sp.rad(30))) ** 2 / (2 * sp.Rational(98, 10))), 3),
        # real root of the depressed cubic
        "MATH1": round(float([s for s in sp.solve(x**3 - 2 * x - 5, x) if s.is_real][0]), 3),
        # 150 km / 1.5 h -> m/s
        "UNIT1": round(150000 / (1.5 * 3600), 3),
        # molecular weight of aspirin
        "CHEM1": round(Descriptors.MolWt(asp), 2),
        # H-bond donors of caffeine
        "CHEM2": rdMolDescriptors.CalcNumHBD(caf),
        # compound interest 5000 * 1.04^3
        "FIN1": round(5000 * (1.04) ** 3, 2),
    }


def main():
    g = gold()
    items = json.load(open(os.path.join(HERE, "items_pilot_compute.json")))["items"]
    ok = True
    for it in items:
        want = it["gold_answer"]
        got = g[it["id"]]
        tol = it.get("tolerance", 0) or 0
        match = abs(got - want) <= max(tol, 1e-9)
        print(f"{it['id']:7} corpus={want!s:>9}  tool={got!s:>9}  {'OK' if match else 'MISMATCH'}")
        ok = ok and match
    print("\nall gold reproduced from tools:" , ok)
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
