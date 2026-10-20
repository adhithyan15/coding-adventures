# A fixed trailer makes length mutations move earlier field boundaries

A PKCS #1 v1.5 hostile test expected truncation to corrupt the padding. The
validator derives the fixed-size `DigestInfo` start from the complete encoded
message length, so removing one final byte moves the expected separator one
byte earlier instead. Every padding byte was still `ff`; the new separator
position was also `ff`, and the correct failure was `InvalidSeparator`.

For formats parsed backward from a fixed trailer, recalculate every derived
boundary after an insertion or deletion before naming the expected error.
Assert a specific category only when validation order and the mutated layout
make that category part of the contract; otherwise assert closed rejection.
