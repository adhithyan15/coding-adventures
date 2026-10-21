## Added — the B1 core, chapters 339-348, realizes all five B1 spine nodes

- **Ten chapters, 49 lessons, two chapters per B1 node** (sequences 16100-16580, path
  ranks 3400-3490, extension ranks 3360-3450). Each spine concept has one lesson
  with that `concept_tag`, and `curriculum.d/spine/0250-0290` now omit nothing.
  - **339-340, narrate events.** ML-C339-ayirunnu (**ആയിരുന്നു** "was",
    **ഉണ്ടായിരുന്നു** "there was, had"), ML-C339-vayikkukayayirunnu
    (ASPECT-ONGOING-PAST: **-ഉകയായിരുന്നു** as the scene, the plain past as the
    event), ML-C339-pokumayirunnu (**-ഉമായിരുന്നു** "used to"), ML-C340-ennittu
    (CONNECTIVE-THEN: **എന്നിട്ട്** beside the **-ഇട്ട്** of chapter 111),
    ML-C340-atinusesham (NARRATIVE-SEQUENCE: **അതിനുശേഷം**, and **-അപ്പോൾ** "when
    it happened"), and ML-C340-gramathil, a reading of about a hundred words told
    in all three pasts.
  - **341-342, give reasons.** ML-C341-vicarikkuka (**വിചാരിക്കുക**),
    ML-C341-ennu-karutunnu (OPINION-I-THINK: **… എന്ന് ഞാൻ വിചാരിക്കുന്നു /
    കരുതുന്നു**, **എനിക്ക് … എന്ന് തോന്നുന്നു**, **എന്റെ അഭിപ്രായത്തിൽ**),
    ML-C341-atukondu (CONNECTIVE-SO: reason first with **അതുകൊണ്ട്**, against
    **കാരണം**; **അതുകൊണ്ടാണ്** "that is why"), ML-C342-pokanam (MODAL-SHOULD: the
    obligative **-അണം**, explained as **വേണം** worn onto the verb, with
    **-ഏണ്ടതാണ്**, **-ഏണ്ട** and **-അരുത്**), ML-C342-vannatukondu
    (EXPLANATION-GIVE: **-അതുകൊണ്ട്** on a verb, and a plan with its reason).
  - **343-344, handle travel.** ML-C343-evideyanu (DIRECTION-ASK: **… എവിടെയാണ്?**,
    **… -ലേക്ക് എങ്ങനെ പോകണം?**, **ഇടത്തോട്ട് / വലത്തോട്ട്**),
    ML-C344-ticket-venam (TRANSPORT-TICKET), ML-C344-bus-kittiyilla
    (PROBLEM-REPORT: **എനിക്ക് ബസ് കിട്ടിയില്ല**, **-പ്പോയി**, **-ആമോ?** "could
    you …?").
  - **345-346, describe experience.** ML-C345-santosham-tonni (ADJECTIVE-FEELING:
    the dative with **തോന്നി**; a feeling as scene or as event),
    ML-C345-kanan-agrahamundu (AMBITION-EXPRESS), ML-C346-atinekkal
    (COMPARISON-BASIC: **-ക്കാൾ**, **ഏറ്റവും**), ML-C346-varsamayi
    (TIME-DURATION: **-ആയി** after a span, **മുതൽ**).
  - **347-348, express a condition.** ML-C347-varunnundenkil (CONNECTIVE-IF:
    **-എങ്കിൽ** on a full verb, **ഇല്ലെങ്കിൽ**), ML-C347-maza-peytal
    (CONDITIONAL-REAL), ML-C348-asrayiccirikkunnu (CONDITION-DEPENDS),
    ML-C348-undayirunnenkil (MODAL-WOULD: **-ഇരുന്നെങ്കിൽ … -ഉമായിരുന്നു**), and
    ML-W348-six-lines, six connected lines about a trip (connected composition).
  - A review closes every chapter and is its payoff, assessing every atom the
    chapter introduces.
- **Eight letters, each anchored by the word lesson just before it:** ML-W340-ii
  **ഈ** (ഈയിടെ), ML-W341-ai **ഐ** (ഐക്യം), ML-W342-au **ഔ** (ഔദ്യോഗികമായ),
  ML-W343-da **ഡ** (റോഡ്), ML-W344-au-sign **ൗ** (സൗകര്യം), ML-W345-visarga
  **ഃ** (ദുഃഖം), ML-W346-vocalic-r **ഋ** (ഋതു) and ML-W348-gha **ഘ** (ആഘോഷം).
  None has filmstrip geometry, so no lesson promises a strip; the six with a
  cited stroke order describe it in words, and ഡ and ഘ ask the learner to copy the
  printed shape.
- **Script inventory.** `data/scripts/malayalam.json` gains mark rows for the au
  length mark **ൗ** (U+0D57, the modern spelling of the au sign) and the visarga
  **ഃ** (U+0D03), and handwriting rows for the independent vowels **ഈ ഐ ഔ ഋ**,
  which enter glyph closure only once source-backed. All six cite Jayasree's
  recorded strokes drawn alone (stroke count, start, order and direction; no
  coordinate copied; CC BY 4.0).
- **Sources.** Etymologies cite DEDR (ഈയിടെ: 410, 448), CDIAL (ദുഃഖം 6375, ഐക്യം
  2533, വിചാരിക്കുക 11683) or Monier-Williams (സൗകര്യം, ഋതു, ഔദ്യോഗികമായ via
  *udyoga*, ആഘോഷം via *āghoṣa*); റോഡ് is borrowed from English. Where the source
  is silent, the lesson says so: the move of *āghoṣa* "a proclaiming" to
  "celebration" is not traced.
- **Pins.** tests/exam-inventories/malayalam.test.ts (lessons 1686 → 1735; `shown`
  and the directly-owned overlap 69 → 77), tests/levels.test.ts (Malayalam leaves
  the A2 reach list for the over-A2 list), tests/letter-anchoring-ceilings
  (`unreadInventory` 10 → 4), tests/corpus/malayalam/track.test.ts (the writing
  runway now ends on ML-W348-six-lines), and the inventory evidence for the six
  new rows.
