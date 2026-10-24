---
category: Repo policy / workflow reminders
---

# A letter lesson placed last in a core needs later words that revisit it, or the gate's twice-revisited rule fails

The Malayalam B1 core's last lesson was ML-W348-gha, the letter lesson for ഘ
(anchored by ആഘോഷം). The level gate's reinforcement criterion requires every
atom at or below the level to be revisited at least twice. The core's own
review came before the letter lesson, and the four vocabulary tranches were
drafted with the core's new letters switched off (`ML_B1_CORE_LETTERS=0`,
because the core was not in yet). So nothing after ML-W348-gha used ഘ, its
atom was revisited once, and with 2,569 headwords Malayalam still attained
only A2.

The fix: two notes in the first tranche chapter now use ആഘോഷം and list the
ഘ script atom in `extra_atoms`. That needed a builder change as well, because
the Malayalam builder rebuilt each word dict from a fixed field list and
silently dropped `extra_atoms`.

What to do differently:
- When drafters work with a core's letters switched off, plan explicit
  revisits for every letter the core teaches late.
- Run the continuity measure for `revisits < 2` on the core alone, before
  drafting starts.
- Builders should reject unknown W(...) keys instead of dropping them.
