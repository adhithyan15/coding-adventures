# AUTO-GENERATED input program: the decomposed typed-IR facts, LINKED to the compiled rule library.
import sys, json
sys.path.insert(0, '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj101-defensibility-100crossdomain/run100/cas/lib')
from rulelib_ff22eace640aba13 import decide          # <-- link the CAS-compiled rule library
FACTS = {"gross_income": 14600}                            # <-- the typed-IR input, translated to data
print(json.dumps(decide(FACTS)))           # CPU only; ZERO answer-time model calls
