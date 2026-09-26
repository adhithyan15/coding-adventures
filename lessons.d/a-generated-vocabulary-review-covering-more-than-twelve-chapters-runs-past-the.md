---
category: Repo policy / workflow reminders
---

# A generated vocabulary review covering more than twelve chapters runs past the 300-second lesson ceiling

Marathi's first vocabulary tranche had 26 chapters. `add_reviews` splits a
tranche in half and closes each half with a review that retrieves every word,
so the first review covered 13 chapters plus two tail words: 67 words. Its
computed duration came to 310s, against the validator's 300s ceiling.

The fix was to cut the 51 Marathi chapters into three runs of 17, so each half
is 8-9 chapters. A 25-chapter tranche, as in Urdu and Punjabi, has halves of 12
and 13 without the tail and passes.

Rule: keep each review half to about 12 chapters (60 words) or fewer. Past 50
chapters, use three tranches rather than two.
