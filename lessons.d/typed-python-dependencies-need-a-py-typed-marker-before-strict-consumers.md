# Typed Python dependencies need a py typed marker before strict consumers

The first strict MyPy run for Python `der-asn1` rejected the fully annotated local
`der-tlv` dependency as untyped because that distribution omitted its PEP 561
`py.typed` marker. Add the marker to typed libraries when they are introduced, and
verify them from a downstream consumer rather than weakening MyPy with an ignored
import. The new consumer should ship its own marker at the same time.
