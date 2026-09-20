# der-tlv (Go)

`dertlv` provides bounded, canonical DER identifier and definite-length
framing. Elements expose slices of the caller's input, and the iterative cursor
preserves its position and sibling count after any failure.

The package does not interpret ASN.1 values, recurse, parse X.509, perform
cryptography, or access the host. Run `BUILD` for race, coverage, vet, and
trimpath build validation.
