# Bounded SAT evidence

`sat_http09.py` is an isolated, dependency-free proof harness for the narrow
HTTP/0.9 request contract.  For address lengths 0, 1, and 2 it encodes a
one-hot Boolean byte-class model and asks whether an accepted request can
contain a forbidden byte (CR, LF, space, or another non-printable byte).  The
queries are UNSAT.  A deliberately weakened control is SAT and prints a
forbidden-byte witness.  The harness then exhaustively replays the bounded
classes through `encode_http09_request` and `parse_http09_request`.

Run from this package directory:

```sh
PYTHONPATH=src .venv/bin/python verification/sat_http09.py
```

The output is bounded SAT/model evidence plus finite public-export
conformance.  It is not a DRAT certificate, a proof of the production Python
implementation, or a claim about HTTP versions beyond the HWEB00 profile.
The runner has no runtime dependency and is intentionally separate from the
normal package API.

When a Go 1.26 toolchain is available, `verification/go_sat` is an opt-in
bridge to the repository's CDCL solver. It accepts DIMACS on stdin and emits
`SAT` or `UNSAT`; it is kept out of the Python build and package gate:

```sh
printf 'p cnf 1 1\n1 0\n' | (cd verification/go_sat && go run .)
```

The default Python command remains the portable bounded replay. The bridge
does not turn that replay into a DRAT-checked production proof.
