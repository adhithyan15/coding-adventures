---
category: Perl
---

# JSON::PP needs 17 significant digits for binary64 replay

JSON::PP's ordinary scalar encoding can print a floating value with 15
significant digits, which may change the binary64 value on decode. If replay
requires exact recomputation, encode contract float fields with a 17-digit
round-trip spelling and test a non-terminating ratio such as one third.
