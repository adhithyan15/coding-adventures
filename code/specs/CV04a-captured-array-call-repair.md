# CV04a — bounded captured-array call repair

The actual merged main parent is 99f58d48b43, rather than the initial audited
45fae01a043. The closed post-merge proof covers all 2236 changed paths, 2224
pure imports, 12 overlaps, 205 retained owned objects, 24 transport objects and
57 backend objects. Registry tuples remain exactly the same 376. The incoming
entier standard-name admission remains alongside the owned normal-exit controls.

The eight original matrix sources have now been run through the real native
preparation/inference/specialisation pipeline and JVM class writer. Three
dimensions produce six parameters; four dimensions produce eight. MsX64
rejects every main call at four arguments and the same callee signatures at
four parameters. SysV rejects the eight-argument calls/parameters at six.
JVM real-array signatures contain an array reference and five/seven longs:
11/15 operand slots. The emitted caller Code attribute currently reserves only
8. Diagnostic PASS means inspection completed, not backend runtime acceptance.

## Native scope

Support the existing integer-word internal call convention up to eight total
arguments on both x86 ABIs. Preserve explicit refusal above eight: larger
signatures are a separate ABI slice. Allocate a fixed outgoing stack area from
the largest direct call in the function. It follows MsX64 shadow space and
is disjoint from every negative RBP local/root slot; align the whole frame to
16 bytes. Extra parameters arrive at RBP+16+shadow+8*extra_index after the
callee's push RBP. Spill them into the same deterministic local slots as
register parameters. Marshal extra arguments through a scratch register from
the caller's existing RBP slots, preserving exact order, then load GPR args.
Keep recursive and external return-address safepoints and RBP root identities.
Check frame/offset arithmetic and signed immediate representability. Do not
change float builtin ABIs, collector semantics or any old capacity test.

Tests must execute six/eight distinct arguments, return a value dependent on
the stack positions and order, cover both ABI byte layouts, recursion and
reference-root stack maps, and retain nine-argument refusal. Run the original
eight captured/forwarded arrays with Exit42 on each available standard backend.

## JVM scope

At each direct call, count the actual emitted argument load widths (long/double
two, references/int/float one) with checked arithmetic. Set max_stack to the
maximum of the existing conservative non-call bound eight, each direct-call
load peak, and its return width. Consume ignored non-void returns with pop/pop2
so every IIR call ends with an empty operand stack. Validate the static method
descriptor limit of 255 parameter slots before local allocation, counting wide
types twice. No constant-only increase and no claim of general CFG analysis.
Unsupported/mismatched typed operands still refuse; bounds do not legalize a
malformed invocation. Test mixed widths, high valid boundary255, rejection256,
multiple calls, ignored returns and actual real JVM execution of the original
array rows with verification enabled. Preserve existing Code/scene/resource
tests and run relevant affected suites and strict lint.

Miri remains DEFERRED/PENDING and its frozen checkout/inputs/target/receipts
untouched. No new crates.io dependencies. General-hop 32/32 resource failures,
full Closure parity, byte/source and transformation lineage, stable identities,
pass/sweep order, hosted CI, merge and final-source Miri acceptance remain OPEN.
