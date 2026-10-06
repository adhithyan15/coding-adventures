---
category: Repo policy / workflow reminders
---

# A generated sentence that splices one string into another must check the case rule of the spliced text

**What went wrong.** The vocabulary-tranche generators built each chapter's
payoff summary as `"Complete the last lesson of chapter N: " + canDo` and
lowercased the first letter of `canDo` so it would read as the middle of a
sentence. Every generated `canDo` starts with "I can", so 2012 chapters across
21 tracks printed "…: i can say …" in the books (2122 by the time the sweep landed). No check caught it: the text
was valid JSON, the books compiled, and the banned-word and closure scans do not
look at letter case.

**The fix.** One sweep capitalised the existing summaries and regenerated the
books. The tranche generators (scratch scripts, not in the repo) now keep the
first letter when the spliced text starts with the pronoun "I". Because those
scripts can't be checked from here, `tests/payoff-summary-case.test.ts` fails on
any payoff summary that lowercases the pronoun.

**Do differently.** When code lowercases or capitalises text it splices into a
sentence, check the proper-noun and pronoun cases ("I", names, languages)
before you apply the rule. Then read one generated sentence aloud before you
generate thousands of them.
