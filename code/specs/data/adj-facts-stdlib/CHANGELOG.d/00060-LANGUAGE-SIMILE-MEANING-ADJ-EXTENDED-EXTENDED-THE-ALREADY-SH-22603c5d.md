- `language/simile-meaning.adj` (extended) -- extended the already-shipped
  `simile_meaning(simile, meaning)` table from 3 to 15 rows. The SAME
  already-cited Grammarly "Common simile examples" table names twelve MORE
  similes beyond the original three, each paired with its own clean
  meaning in the SAME table row shape as the original three: added
  like_a_fish_out_of_water, as_fast_as_a_cheetah, like_a_deer_in_headlights,
  as_cool_as_a_cucumber, like_a_kid_in_a_candy_store, as_strong_as_an_ox,
  like_watching_paint_dry, as_cute_as_a_button, like_two_peas_in_a_pod,
  as_flat_as_a_pancake, like_birds_of_a_feather, and as_hungry_as_a_horse.
  Zero new source page -- WebFetch-verified twice (a targeted second pass
  re-fetched five of the new rows' raw table-row text directly). Checked
  vocabulary-in-context.adj first this cycle (the three remaining words on
  its source page -- incubate, predators, migrate -- lack an explicitly
  stated meaning, only illustrative example sentences, confirmed
  non-extendable). Extended e2e test `facts_simile_meaning_e2e.rs` to 4
  tests (original 3 + a new extension test). No new manifest objective
  (same library, same objective, 167 total unchanged). This is the FOURTH
  successful extend-pattern win this window.
