## Fixed — chapters 6 and 20 payoffs cover more of their chapters

Two chapter payoffs sat below the 0.5 `chapter-payoff-not-representative`
floor. Each payoff lesson now recalls the chapter atoms taught before it,
using only what the introducing lessons said, and `chapters.d` lists them.

- **Chapter 6** (KA-C06-dative-subject): 4/9 (0.44) → 7/9 (0.78). A new
  "Guided Practice — the ending underneath" asks for *hesarige* and
  *kelasakke* and why Kannada's dative has *g* where Tamil has *k*
  (KA-LEX-C06-DATIVE-GE-01, KA-ETYMON-C06-DATIVE-GE-02). A new "Script — ದ"
  points to ದ in ಧನ್ಯವಾದ · ಹೌದು · ನಮಸ್ಕಾರ (KA-SCRIPT-RECOG-06);
  KA-S06-letter-da joins the prerequisites. ೊ and ಇ (KA-SCRIPT-RECOG-114,
  -115) are taught by script lessons sequenced after the payoff and stay out.
  Prose was tightened (the "nothing is doing anything" paragraph, the
  why-the-dative paragraph, the wrap-up) to keep the lesson under the cap:
  `max_seconds` 285 → 290, computed 287.
- **Chapter 20** (KA-C20-havamana): 2/6 (0.33) → 4/6 (0.67). A new "Guided
  Practice — eleven to twenty" asks for 11 and 12 as ten-plus-a-digit and
  what is buried in ಇಪ್ಪತ್ತು (KA-ETYMON-C20-HANNONDU-IPPATTU-01, -02);
  KA-C20-hannondu-ippattu joins the prerequisites. The anchor ಝರಿ and the
  letter ಝ are taught after the payoff and stay out. Computed 211 s, within
  the declared 240.

Chapters 10, 13 and 16 stay below the floor (1/3, 1/3, 2/5) and now say why
in their `payoff.note`: every atom their payoff does not assess is taught by a
script-strand lesson sequenced after the chapter's only spoken lesson, and
none of those script lessons is a fitting payoff for the chapter's can-do.
