# Script filmstrips: Gujarati

Gujarati has a table too, in which every sign is written AFTER its
consonant, even િ, which sits to its left: `કેમકે` is ક, ે, મ, ક, ે, each
place cited to KanoAI's barakhadi templates in the sign's mark record. The
virama ્ and ૃ have no row, two signs on one consonant (ાં) are refused, and
so are the pairs the bundled font reshapes (ણુ, રુ, રૂ; ુ and ૂ under the
22 consonants that take a stem form; જ and ૹ with ા, ી, ો, ૌ). Each group
of fused pairs carries its citation in `FUSED_SIGN_PAIR_SOURCES`, and a
test fails if a pair is added or dropped without its source.
