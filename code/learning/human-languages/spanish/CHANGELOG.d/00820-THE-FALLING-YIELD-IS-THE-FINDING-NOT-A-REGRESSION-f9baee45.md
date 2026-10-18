## The falling yield is the finding, not a regression

Tranches 1-2 ranked lexemes by how often they appeared across failing exam
rows. That stopped discriminating when 145 of 146 remaining lexemes appeared in
exactly one row. Tranche 3 then ranked by how close each row was to being
unblocked and taught only words that were the sole survivor in their row — one
word, one item, both halves. That rule took the failing count from 79 to 40 and
left **two** solo blockers where there had been forty-one.

There is no third rule, and this was measured rather than assumed. A greedy set
cover over the forty remaining rows comes out **flat at roughly 2.7 words per
item** all the way from five words to a hundred and seven, because only one
lexeme — *explicar* — still appears in more than one row. The other hundred and
six appear exactly once, so set cover degenerates into "teach them all" and
every ordering costs the same.

**That is the ranking having finished its job.** The cheap wins were taken in
tranches 1-3, and what remains is a uniformly priced tail. So the grouping rule
is now scene coherence, with a tie-break: among equally priced scenes, prefer
the ones whose words happen to finish whole rows. Twenty-two words for nine
items is 2.4 words per item, which is the flat rate the set cover predicted.

