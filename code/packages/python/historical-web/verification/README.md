# Bounded SAT evidence

`sat_http09.py` is an isolated proof harness for the narrow HTTP/0.9 request
contract. For address lengths 0, 1, and 2 it encodes each payload octet as
eight Boolean bits, asks the repository's Go CDCL solver whether admission can
coexist with a forbidden byte, and checks a deliberately weakened SAT control.
It then replays every octet payload in that bound through the public parser.

Run from this package directory:

```sh
PATH=/path/to/go-1.26/bin:$PATH PYTHONPATH=src \
  .venv/bin/python verification/sat_http09.py
```

The observed run reports 3 UNSAT SAT-model queries, a SAT negative control, and
131,586 parser cases (all 256^n payloads for n=0..2, both LF and CRLF). A final
CR before LF is explicitly accounted for because the parser consumes it as the
optional CRLF prefix. This is bounded model evidence and finite conformance,
not a DRAT certificate, production proof, or claim about later HTTP versions.
