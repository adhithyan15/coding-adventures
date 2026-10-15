# CV04a: bounded adjacent lane-ordering work

## Source and validation boundary

Base a20c12ac29538863807209b4ccb935950064b2be retains the exact reviewed
quadruple-selector/typography package contents from f64d2188, integrated main
fdf1a247. Its unchanged LANG production11/native48 validation remains running
in the original isolated checkout and separate target; no changes there.
This second isolated checkout and its separate ordering target preserve those
inputs. Latest audited main1cf6e05e has48 additional paths, including Mosaic
source changes; reconciliation and final current-head gates remain pending.
Keep one active implementation PR17288; this is local preparation for that PR.

## Existing behavior and resource problem

Automatic ordering starts from declared lane order and eight seeded shuffles.
Each start tries adjacent swaps for at most L passes, accepts only strictly
smaller weighted-distance cost and stops an unchanged pass. Across starts,
minimum cost wins, then minimum distance from source order; ties retain the
earlier result. Seeds use the existing sorted weighted signature and UTF16 hash.
Invisible, unknown and within-lane edges keep their existing exclusion rules;
reversed and repeated visible edges contribute to the same undirected weight.

Every present trial rebuilds positions and visits every weighted pair. A dense
16-lane equal-weight graph has no improving swaps but still repeats those scans.
Measure actual weighted-edge visits, not elapsed time. First instrument the
existing cost path under cfg(test), and observe the new counted control fail.
Do not increase its bound after seeing the failure.

## Compatible delta evaluation

For a unique-lane permutation, initialize each lane's balance as weight to lanes
on its left minus weight to lanes on its right. For adjacent u,v in that order,
delta = balance(u) - balance(v) + 2*weight(u,v). A retained swap changes only
balance(u) += 2*weight(u,v), balance(v) -= 2*weight(u,v). Rejecting a swap leaves
order, balances and cost untouched. Build pair weights once per start with
borrowed IDs; never rescan all weighted pairs for a trial.

Preserve nine starts, seeds, pass count, exact costs, chosen order and tie behavior.
Use checked signed arithmetic for balances/deltas and checked cost arithmetic
before comparisons/conversions. An unrepresentable cost must fail closed instead
of wrapping into a smaller score. No new production cap, unsafe code or dependency
is required. Canonical authored subgraphs reject duplicate lane IDs already;
preserve existing behavior for malformed direct IR with a non-bijective order
through the retained full-cost path, and explicitly exclude it from the optimized
unique-lane work claim. Keep that parser refusal and test fallback equivalence.

Three bounded weighted-pair initialization visits per start (initial cost,
balances and pair index) give at most27W visits for nine starts; trial lookups
take logarithmic pair-index work with no weighted-pair traversal. The counted
dense control must retain complete public layout output. This is a bounded
ordering claim, not a bound on unrelated rank, route or arbitrary oblique work.

## Exact controls and validation

Retain an independent cfg(test) copy of the original full-cost algorithm, including
seed and shuffle code. Compare complete chosen order, cost and source distance
for every four-lane graph with pair weights0/1/2 and every initial permutation;
include Unicode IDs, zero-edge cases, ties, duplicate/reversed/invisible edges,
both flow axes and all four directions. Count only the actual optimized path.
Compare full public layouts against the reference-chosen order with automatic
ordering disabled and its metadata flag restored; verify authored parser/paint
fixtures with line hops on/off and complete PaintScene output/metadata.

Run affected hierarchy/parser/paint suites and strict all-target lint; determine
the metadata reverse cone including dev dependencies for final acceptance.
Independent exact-source security review is required before publication.
Full453-cone/current-main/hostedLinux/whole final security gates remain pending.
Base ESTree, own types and no new crates.io dependencies remain binding.

Full final-source Miri **DEFERRED/PENDING**, never passed. The original frozen
run/497inputs/checkout/receipts/target remain untouched and unpolled. Implementation
delivery precedes genuine final-source Miri acceptance. Full Closure parity,
output source joins and all transformation lineage remain OPEN; heartbeat ACTIVE.

## Initial receipts

Original current LANG validator54386 has now returned actual EXIT0 at a20:
all eleven unchanged production BUILD commands and48 exact native controls,
322 tests/docs across108 targets, zero failures/one existing archive ignore.
This is the exact roster, not full unfiltered LANG or full453-cone acceptance.

Ordering candidate b5e0d090 returned actual validator81534 EXIT1/Cargo101 with
E0382 in its test fixture, before any tests. That compile receipt is preserved
and is not the counted red. Clone the non-Copy direction and retry the same bound.
The fresh ignored lock is archived; the original tested checkout's exact lock
is copied into the new checkout without changing the original inputs.

Compiling candidate8dae97b1 observes the intended17280 visits against3240, but
validator42897 returns actualEXIT1/Cargo101 with1PASS/2FAIL: the other failure is
the fixture's PartialEq comparison of matching pre-existing self-loop NaNs.
Prove all route endpoint coordinate bits, then compare complete layouts with
only already-proven NaNs normalized in test-owned copies. Unexpected NaNs and
all other fields remain checked. This preserves source behavior and does not
claim finite self-loop geometry acceptance; that pre-existing route issue is
outside this ordering resource repair. Retain the original failed receipt.

Candidate9a668a14 supplies the clean behavioral red: validator42897's successor
returns actual wrapperEXIT0 for expected Cargo101, exactly2PASS/1FAIL, and
17280 weighted visits against3240. It is expected-red evidence, not native PASS.
Delta candidatee363 passes all five focused controls; final authored scene and
global729 graph comparisons are added at32abf007 and still need full validation.
The test-source whitespace helper's truncation was confined to this new checkout;
original bytes and failure receipt are archived, committed e363 paint bytes were
restored exactly and only the owned reference/test was reapplied. Native a20
checkout remained unchanged. No security/current-main/hosted acceptance inferred.


## Scoped acceptance

Actual source `47747b20f72d72211ffedff92cb3b10b8a89f4a9` completes validator83452 EXIT0,612/10
nonduplicated, full metadata2 cone plus parser boundary suite, strict3/docs;
focus6 excluded. Actual3240 greedy cost/balance/index visits retain the3240 bound,
all729 global graphs/17496 permutations/48 complete authored scenes match the
original oracle. The counter excludes unchanged seed/signature construction and
outer source/edge/identifier costs. Cost, balances and delta are checked; original
source-distance tie summation is unchanged. Scoped independent review PASS,
SHA256 115ae65b65c04398c6b7dfc9bdb64e45c71b32b8114414a8088516b6c063f991; whole-PR acceptance PENDING.
All original failures, recovery snapshots and prior source frames remain intact.
Fresh main `cbd9a37c8f02d7269c212a944f79588f559b50e2` still needs bounded closed
edge-label/Mosaic/content reconciliation; the separately owned quintuple PR
is still open in that audit. Full453-cone, arbitrary unique
oblique work, final whole security and hosted gates remain PENDING.
Full final-source Miri DEFERRED/PENDING; full objective OPEN, heartbeat ACTIVE.
