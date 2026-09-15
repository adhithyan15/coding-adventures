- `language/idiom-meaning.adj` (extended) -- extended the already-shipped
  `idiom_meaning(idiom, meaning)` table from 3 to 23 rows. The source page's
  OWN title states it covers "30 Useful English Idiomatic Expressions" (the
  live page in fact lists 50), so the original 3-row slice was a narrow
  first cut, not the page's own limit. Added cut_corners,
  hit_the_nail_on_the_head, cost_an_arm_and_a_leg,
  bite_off_more_than_you_can_chew, beat_around_the_bush,
  cry_over_spilled_milk, get_your_act_together,
  kill_two_birds_with_one_stone, let_the_cat_out_of_the_bag,
  pull_someones_leg, burn_the_midnight_oil, bite_the_bullet, break_a_leg,
  call_it_a_day, steal_someones_thunder, the_ball_is_in_your_court,
  throw_in_the_towel, speak_of_the_devil, once_in_a_blue_moon, and
  catch_someone_red_handed -- all 20 from the SAME already-cited Oxford
  International English page, zero new source needed, each with its own
  clean one-sentence "Meaning: ..." definition, WebFetch-verified twice
  (a targeted second pass re-fetched five of the new rows' raw text
  directly, confirming the extraction is accurate). Idioms whose page entry
  uses a slash or bracketed variant (e.g. "Hit the sack/hay", "Cut
  [somebody] some slack") were deliberately left out of this batch, since a
  single unquoted atom cannot honestly represent an "either/or" variant
  phrase. Extended e2e test `facts_idiommeaning_e2e.rs` to 4 tests (original
  3 + a new extension test). No new manifest objective (same library, same
  objective, 167 total unchanged).
