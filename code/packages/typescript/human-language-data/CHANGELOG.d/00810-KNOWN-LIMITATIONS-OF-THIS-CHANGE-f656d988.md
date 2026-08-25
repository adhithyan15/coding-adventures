### Known limitations of this change

Two things this does **not** fix, stated plainly rather than implied away:

- **Ten early lessons still teach script inside a speaking lesson.** Nine chapter 2-3
  word lessons carry a `## The letters in this word` section. They are not moved here,
  because the letters they teach (உ, ய, எ, and the ெ sign) are taught nowhere in the
  writing strand — deleting the sections would remove the only place they exist. Moving
  them needs new writing lessons authored first.
- **Chapter 1's payoff is now unmeasured rather than passing.** `chapters.ts` guards the
  representativeness check with `introduced.size > 0`, and chapter 1 introduces no atoms,
  so the gate that used to report `tamil:1` at 5/24 now says nothing about it. The corpus
  total `payoffsNotRepresentative: 25` holds only because `tamil:13` took its place —
  `TA-W04-i-sign-write-nandri` moved to chapter 13, whose payoff now covers 1/4.

