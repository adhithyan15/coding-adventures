- `nutrition/food-group-alternative-form.adj` (new) — a sibling to the already-shipped
  `food-groups.adj` (`food_group(food, food_group)`, which sorts whole, solid foods into one of
  the five MyPlate groups). Three of that table's own per-group definitional sentences, already
  quoted verbatim in its header, also name a non-solid-food alternative that counts toward the SAME
  group — that the solid-food-only schema had no room for: fruits →
  hundred_percent_fruit_juice ("The Fruit Group includes all fruits and 100% fruit juice."),
  vegetables → hundred_percent_vegetable_juice ("Any vegetable or 100% vegetable juice counts as
  part of the Vegetable Group."), dairy → lactose_free_milk, fortified_soy_milk,
  fortified_soy_yogurt ("The Dairy Group includes milk, yogurt, cheese, lactose-free milk and
  fortified soy milk and yogurt."). New `food_group_alternative_form(food_group, alternative_form)`
  table. Honest abstention on grains and protein, whose own cited definitional sentences name no
  comparable alternative form. New e2e test file `facts_foodgroupalternativeform_e2e.rs` (3 tests:
  forward recall with citation, backward recall from a bound form, honest abstention on grains). No
  manifest objective, matching `food-groups.adj`'s own precedent of not having one. Second and
  final strong slice from the nutrition/ sweep tranche — nutrition/ (1 table) is now EFFECTIVELY
  CLOSED (2/2 strong candidates shipped).
