# NN23 Initialization and Activation Distribution Labs

Status: Draft

NN23 isolates one deep-training question: how does the first weight scale alter
the values that reach later layers?

The V1 corpus fixes four two-value inputs and three two-by-two sign templates.
It applies four scaling rules without random sampling:

```text
tiny    = 0.1
Xavier  = sqrt(1 / fan_in)
He      = sqrt(2 / fan_in)
large   = 2
```

Each layer computes a bias-free matrix product followed by `tanh` or ReLU. The
trace records every preactivation and activation. Distribution summaries use
population variance, count values with absolute magnitude below `1e-12` as
zero, and count `tanh` values with absolute magnitude at least `0.95` as
saturated.

The fixed signs are an educational oracle, not a replacement for pseudorandom
initialization. They make scale the only changing variable and let every
language reproduce the experiment exactly.

## Cross-Language and Rust-Core Direction

Consumers should reproduce the scalar fixture before using native random-number
generators. A Rust core can later accept row-major input and weight-template
buffers, derive initializer scales, run fused matrix-plus-activation kernels,
and reduce distribution statistics with SIMD. C ABI and WASM bindings should
preserve the documented row order and population-variance convention, with an
optional trace mode that returns the unfused per-layer buffers.
