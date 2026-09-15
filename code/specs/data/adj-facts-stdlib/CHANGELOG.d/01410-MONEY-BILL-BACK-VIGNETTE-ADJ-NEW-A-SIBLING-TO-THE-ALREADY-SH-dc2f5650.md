- `money/bill-back-vignette.adj` (new) — a sibling to the already-shipped `us-bills.adj`
  (`bill_portrait(dollars, portrait)`, ONE front-of-note portrait per bill). That table's own
  header already quotes, verbatim, the SAME U.S. Currency Education Program feature-sheet sentence
  used for the front portrait, and for six of the seven bills that same sentence also names the
  back-of-note vignette — that the portrait-only schema had no room for: 1 → great_seal,
  2 → declaration_signing, 10 → treasury_building, 20 → white_house, 50 → us_capitol,
  100 → independence_hall. New `bill_back_vignette(dollars, vignette)` table. Honest abstention on
  the $5 note, whose own cited feature-sheet sentence stops after the front portrait and names no
  back vignette. New e2e test file `facts_billbackvignette_e2e.rs` (3 tests: forward recall with
  citation, backward recall from a bound vignette, honest abstention on the $5 note). No manifest
  objective, matching `us-bills.adj`'s own precedent of not having one. First slice from the
  money/ sweep tranche.
