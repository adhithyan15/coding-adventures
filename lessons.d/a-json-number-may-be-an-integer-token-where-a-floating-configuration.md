---
category: Testing & coverage
---

# A JSON number may be an integer token where a floating configuration value is expected

A lossless JSON reader may preserve every integer spelling as an arbitrary-
precision integer, including fields whose API type is binary64. Rejecting an
integer token solely because the decoded runtime type is `BigInt` makes valid
numeric JSON such as `"minimum_factor": 2` fail replay even though the
contract allows any finite number greater than one.

Do differently: normalize both integer and decimal JSON numeric values to the
field's declared floating type, then apply its finite/range checks. Keep
integer-token decoding exact for signed-64-bit fields, and test cross-language
reports that serialize integral floating values without a decimal suffix.
