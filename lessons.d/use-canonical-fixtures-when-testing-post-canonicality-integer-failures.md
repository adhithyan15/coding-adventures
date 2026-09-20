# Use canonical fixtures when testing post-canonicality integer failures

An integer-overflow regression used a redundant leading zero, so canonicality
correctly failed before the checked unsigned-width conversion was reached.
Tests for later-stage INTEGER errors must first be canonical DER values; use a
nine-octet positive magnitude whose first octet is nonzero and below `0x80`
when exercising the unsigned 64-bit width ceiling.
