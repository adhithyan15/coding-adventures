## 0.9.6 — the Learn session, review quiz (HL03 phase 6, slice 6b-2)

- **The review pass, wired into Learn mode** — the second of the app's two
  mechanisms. Below the teaching sweep, a randomised cumulative quiz draws over
  everything covered so far (`plan.reviewGrid`, the concept×language grid up to
  the cursor), SRS-weighted by the engine's `pickNext` so missed/overdue items
  resurface and mastered ones fade.
- A cell is asked as **"‹meaning› — in ‹language›?"** and the options are the
  **same concept in other languages** (plus the answer) — the cross-language
  look-alikes the interleaving exists to expose (Telugu ధన్యవాద vs Hindi
  धन्यवाद, both from `dhanya`). If a concept lives in only one language, the
  remaining option slots are filled from elsewhere in the grid so there is
  always a real choice.
- Answering threads through the tested engine: `applyAnswer` **promotes** a hit
  (comes back later) or **demotes** a miss (resurfaces soon) and logs which wrong
  word was picked; the SRS clock advances. A **"what you keep confusing"** panel
  rolls those up from `confusions(log)`, showing the actual words (e.g. "Picked
  ధన్యవాద (telugu) for धन्यवाद (hindi)").
- Moving the concept cursor redraws the review from the new covered set. Progress
  lives in a module-level `let` for now (persistence is a later slice). DOM-only
  shell over the tested engine; 185 tests still pass. Verified in a real browser:
  the quiz renders below the sweep with four real-script options, no tofu.

