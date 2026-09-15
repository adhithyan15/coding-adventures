- `language/contraction.adj` (extended) — extended the already-shipped `contraction(word,
  expansion)` table from 3 to 16 rows. The original dont/cant/wont rows turn out to be exactly
  THREE members of a larger 16-row "Negative Contractions" table on the same already-cited
  Grammarly page -- every negative contraction the source lists, each with exactly ONE
  unambiguous expansion. Added arent, couldnt, didnt, doesnt, hadnt, hasnt, havent, isnt,
  mustnt, shouldnt, wasnt, werent, wouldnt -> are_not/could_not/did_not/does_not/had_not/
  has_not/have_not/is_not/must_not/should_not/was_not/were_not/would_not. Deliberately did
  NOT pull from the page's separate "Common Contractions" table, whose entries (e.g. `he's`
  -> "he has, he is") are genuinely ambiguous -- WebFetch re-verified the full Negative
  Contractions table across two separate passes, both confirming byte-identical rows and that
  every row has a single expansion with no comma-separated alternatives. `shouldnt` was
  previously this table's own honest-abstention example; the query file and e2e tests were
  updated to use `hes` (the genuinely ambiguous case) as the new abstention example instead.
  New e2e test `contraction_extension_recalls_newly_added_negative_contractions`. No new
  manifest objective (same library, same objective, 168 total unchanged).
