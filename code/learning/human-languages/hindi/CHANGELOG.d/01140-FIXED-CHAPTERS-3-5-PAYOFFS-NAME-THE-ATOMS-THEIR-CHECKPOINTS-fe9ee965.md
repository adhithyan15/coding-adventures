## Fixed — chapters 3-5 payoffs name the atoms their checkpoints assess

Chapters 3, 4 and 5 were migrated to typed lessons, but their ledger entries
(`chapters.d/0003.json`-`0005.json`) still carried the schema-v1 placeholder
`payoff.assesses: []` and a note waiting "until migration", so the
representativeness gate scored each closing checkpoint 0/N (#12088).

- **HI-C03-practice** assesses 4 of 7: आप कैसे हैं, आपका स्वागत है, ठीक, and the
  exchange the checkpoint introduces.
- **HI-C04-practice** assesses 4 of 6: फिर मिलेंगे, कल मिलते हैं, चलता हूँ, and the
  register choice the checkpoint introduces.
- **HI-C05-practice** assesses 5 of 6: बोलना, रहना, करना, मैं हिंदी बोलता हूँ, and
  the sentence engine the checkpoint introduces.
- Each atom is taken from the checkpoint's own block-level `hl-knowledge`
  directives; the building blocks left out are rehearsed inside the set
  phrases and scored in their own lessons, which each new note says.
- No lesson, book or narration changed: `payoff.assesses` and `payoff.note`
  are not printed.
