# Reference fixtures: make every expected answer earn your trust

A fixture is a small, committed input-and-answer pair. It is useful only when
an independent program can recompute the answer. Otherwise it is just a file
that agrees with itself.

The neural-learning corpus now has 30 fixture families, from NN03's first
forward pass through NN32's precision and buffer-residency experiment. Together
they contain 33 lab documents. Each family has a Python reference validator
that performs three different jobs:

1. reject malformed or surprising data;
2. recompute the mathematical result from the inputs;
3. compare the recomputed result with the stored expectation.

NN33 adds one catalog that proves none of those validators has fallen out of
the loop. One command validates the catalog, proves its roster is complete,
and then runs every family validator.

## The smallest possible comparison

Suppose a fixture stores the expected value `0.15`, while the reference program
computes the same sum as binary floating-point arithmetic:

```text
stored       = 0.15
recomputed   = 0.1 + 0.05
             = 0.15000000000000002

absolute error = |recomputed - stored|
               = |0.15000000000000002 - 0.15|
               = 0.000000000000000027755575615628914
```

With an absolute tolerance of `0.000000000001`:

```text
0.000000000000000027755575615628914 <= 0.000000000001
```

The check passes. That does **not** mean every nearby answer is accepted. A
value whose error exceeds the declared tolerance fails, and a validator that
silently exits without evidence also fails the NN33 orchestrator.

## Why run each validator in a separate process?

The 30 validators grew over time and expose slightly different Python function
names. Their command-line contract is already uniform:

```text
exit 0     the fixture is structurally and numerically valid
exit != 0  validation failed
stdout     short human-readable evidence
```

NN33 preserves that stable boundary. It invokes the current Python executable
and an exact cataloged script path as an argument list. It never constructs a
shell command. A timeout, invalid UTF-8, oversized output, non-zero exit, or
empty success is a catalog failure.

## The four-step evidence chain

The Reference Catalog workbench exposes the same chain used by the command:

1. **Load:** reject duplicate JSON keys, non-finite numbers, unknown fields,
   and unsafe paths.
2. **Prove coverage:** require the complete, ordered NN03-NN32 spec roster,
   30 unique fixture roots, 30 unique validator scripts, and 33 lab documents.
3. **Recompute:** launch every registered reference validator without a shell.
4. **Compare:** let each family apply its own exact or tolerance-based oracle;
   any failure stops the complete run.

The browser visualizer deliberately says **registered**, not **executed**. It
can recompute the tiny tolerance example, but Python execution belongs to the
CLI and CI. This distinction keeps evidence honest.

## Run the complete corpus gate

From the repository root:

```bash
python code/scripts/validate_reference_fixture_catalog.py
```

To diagnose one family after the full catalog itself has been checked:

```bash
python code/scripts/validate_reference_fixture_catalog.py \
  --family precision-residency
```

## Cross-language direction

The JSON fixtures are the shared contract. A TypeScript, Swift, Kotlin, Ruby,
or C# consumer should load the same files, recompute the same operations, and
apply the same declared tolerances. A language port does not copy Python
output; it earns parity independently.

Performance-sensitive consumers may eventually call a Rust core through a
stable C ABI. That changes where computation runs, not what counts as correct.
The Rust result must still be checked against the fixture expectation, with
shape, precision, byte order, ownership, and tolerance kept explicit.

## What this tranche does not claim

- Registration is not proof that a browser executed Python.
- A zero exit from the NN33 orchestrator means all current reference validators
  passed; it does not benchmark their performance.
- Python remains the first reference lane, not the only supported consumer.
- The next roadmap tranche adds representative non-Python consumers rather
  than weakening this oracle into a language-specific format.
