- `nutrition/vegetable-subgroup.adj` (new) — a sibling to the already-shipped `food-groups.adj`
  (`food_group(food, food_group)`, which sorts every vegetable into the SAME coarse `vegetables`
  bucket). That table's own header already quotes, verbatim, USDA MyPlate's finer vegetable
  subgroup classification for the same five vegetables — that the one-bucket-per-food schema had
  no room for: broccoli → dark_green, spinach → dark_green, carrots → red_and_orange, tomatoes →
  red_and_orange, corn → starchy. New `vegetable_subgroup(vegetable, subgroup)` table covers the
  full five-vegetable domain with no abstention, since every shipped vegetable has a subgroup in
  the cited span. New e2e test file `facts_vegetablesubgroup_e2e.rs` (3 tests: forward recall with
  citation, backward recall from a bound subgroup, full-domain coverage with no abstention). No
  manifest objective, matching `food-groups.adj`'s own precedent of not having one. First slice
  from the nutrition/ sweep tranche (a second strong candidate, `food-group-alternative-form.adj`,
  is queued next).
