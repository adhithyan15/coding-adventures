## 0.1.0

- Initial Dart lane of the arithmetic identity: half/full adders, N-bit
  ripple-carry addition with carry-out and signed overflow, and the
  six-operation ALU with zero/carry/negative/overflow flags — all wired
  from the in-lane logic gates. Bit rejection is inherited from the gates;
  shape rejection carries identical messages.
- Tests mirror the reference suites (65 unit tests incl. the bit-domain
  contract and the 100k-bit case), add 7 seeded property tests over the
  in-lane `pbt` package against a BigInt oracle and a 2,000-iteration fuzz
  campaign over the in-lane `fuzz` package; 100% line/branch and 94.1%
  function coverage (one implicit enum constructor).
