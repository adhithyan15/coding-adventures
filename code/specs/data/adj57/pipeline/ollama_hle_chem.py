#!/usr/bin/env python3
"""ADJ67 — Gemma on the HLE endiandric-acid cascade, atomic + grounded.

The framework's IR decomposition supplies, per step, the pi-electron count + the
Woodward-Hoffmann rule (a stand-in for spider/CAS grounding from the Nicolaou literature);
the weak local model does ONLY the local rule-application; the harness assembles the three
answers. A ~Gemma-class local model reaches [8pi]-con, [6pi]-dis, [4+2] — the answer
frontier-model holistic reasoning got tangled on (the step ordering), which is exactly the
structural fact the grounded decomposition hands over.

Usage: python ollama_hle_chem.py <ollama-model>
"""
import json
import re
import sys
import urllib.request

MODEL = sys.argv[1] if len(sys.argv) > 1 else "gemma4:latest"

# The framework's IR decomposition of the HLE cascade question, each atom GROUNDED with the
# governing rule (a stand-in for spider/CAS grounding from the Nicolaou/Woodward-Hoffmann
# literature). The framework supplies the pi-electron count + rule per step; the weak model
# does ONLY the local rule-application. Each atom is fed in isolation.
ATOMS = [
 ("step1", "A thermal electrocyclization involves 8 pi electrons. "
   "RULE (thermal): a system with 4n pi electrons (a multiple of 4: 4,8,12) closes CONROTATORY; "
   "a system with 4n+2 pi electrons (6,10,14) closes DISROTATORY. "
   "For 8 pi electrons, decide conrotatory or disrotatory, then answer in the form [8pi]-con or [8pi]-dis. "
   "Reply with ONLY that final form."),
 ("step2", "A thermal electrocyclization involves 6 pi electrons. "
   "RULE (thermal): 4n pi electrons (4,8,12) -> CONROTATORY; 4n+2 pi electrons (6,10,14) -> DISROTATORY. "
   "For 6 pi electrons, decide conrotatory or disrotatory, then answer in the form [6pi]-con or [6pi]-dis. "
   "Reply with ONLY that final form."),
 ("step3", "A Diels-Alder reaction is a cycloaddition between a diene and a dienophile. "
   "The diene contributes 4 atoms; the dienophile contributes 2 atoms. "
   "Express this cycloaddition as [m+n] where m and n are the number of atoms on each component. "
   "Reply with ONLY that form, e.g. [4+2]."),
]

def ask(prompt):
    body=json.dumps({"model":MODEL,"messages":[{"role":"user","content":prompt}],
                     "stream":False,"options":{"temperature":0,"num_predict":800}}).encode()
    req=urllib.request.Request("http://localhost:11434/api/chat",data=body,headers={"Content-Type":"application/json"})
    return json.loads(urllib.request.urlopen(req,timeout=180).read())["message"]["content"].strip()

out={}
for key,prompt in ATOMS:
    txt=ask(prompt)
    if key=="step3":
        m=re.search(r"\[\s*(\d)\s*\+\s*(\d)\s*\]", txt)
        ans=f"[{m.group(1)}+{m.group(2)}]" if m else "?"
    else:
        m=re.search(r"\[?\s*(\d)\s*pi\s*\]?\s*[-\s]*\s*(con|dis)", txt, re.I)
        ans=f"[{m.group(1)}pi]-{m.group(2).lower()}" if m else "?"
    out[key]=ans
    print(f"  {key}: gemma -> {ans:12s}  (raw: {txt[:55]!r})")

assembled=f"{out.get('step1','?')}, {out.get('step2','?')}, {out.get('step3','?')}"
truth="[8pi]-con, [6pi]-dis, [4+2]"
print(f"\n  ASSEMBLED ({MODEL}, atomic+grounded): {assembled}")
print(f"  GROUND TRUTH:                        {truth}")
print(f"  >>> {'CORRECT' if assembled.replace(' ','')==truth.replace(' ','') else 'MISMATCH'}")
