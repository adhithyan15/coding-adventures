---
category: QR / format-marker / file-format specifics
---

# IBM 704 index-register family

(LXA/LXD/SXA/SXD/PAX/PDX/PXA): the tag selects the source/destination register only; the address field is used directly with NO `(Y - C(T))` subtraction. "Store IRA at Y" must not shift Y by IRA. Always test register-family ops with a non-zero index value to catch this; tag=1 with IRA=0 is silently correct either way.
