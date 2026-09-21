# der-tlv (Python)

`der-tlv` splits one canonical ASN.1 DER value into its identifier, header,
value, complete encoding, and untouched remainder. Elements expose
`memoryview` slices over the caller's buffer; hostile declared lengths are
bounded before slicing.

```python
from der_tlv import decode_exact

integer = decode_exact(bytes.fromhex("02012a"))
assert integer.tag.number == 2
assert integer.value.tobytes() == b"\x2a"
```

The package implements framing only. It does not interpret ASN.1 types,
recurse through constructed values, parse X.509, perform cryptography, choose
trust, or access the host. Callers that open constructed values must enforce
one shared depth and total-work budget.

Run the native front door with `BUILD` or `BUILD_windows`.
