# logic-gates

An OCaml implementation of the portable logic-gates contract. It provides
validated primitive and NAND-derived gates, n-ary gates, multiplexers,
encoders, decoders, tri-state output, latches, edge-triggered flip-flops,
registers, shift registers, and wrapping counters.

Selectors and register values are least-significant-bit first. Stateful
operations return the next explicit state instead of retaining hidden global
state, which keeps simulations deterministic and easy to test. Decoder width
is capped at 16 bits so oversized output allocation fails deterministically
before memory is reserved.

## Development

```bash
# Run tests
bash BUILD
```

The build runs ocamlformat checks, Alcotest, and nonempty bisect_ppx coverage
with a 95% production-line minimum.
