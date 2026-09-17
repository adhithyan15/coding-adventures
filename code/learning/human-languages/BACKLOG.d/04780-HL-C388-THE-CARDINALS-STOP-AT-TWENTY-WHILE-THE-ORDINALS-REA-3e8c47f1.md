## HL-C388 — the cardinals stop at twenty while the ordinals reach a hundredth

`HI-A1-NUM-03` wants **21–100**. Its note says the corpus stops at **बीस**, and
counting confirms that: **चालीस, पचास, साठ, सत्तर, अस्सी** and **नब्बे** appear
in no headword and no lesson body anywhere in the track.

**But the ordinals went further than the cardinals did.** `HI-C75-fifth` teaches
**बीसवाँ** and **सौवाँ** — *twentieth* and *hundredth*. So a reader can already
say **सौवाँ** and has never been told what **सौ** is.

That is the hook this chapter should open on, and it is also a warning: a naive
grep for सौ or तीस finds `HI-C75-fifth`, `HI-C75-third` and `HI-C77-practice`
and reports the cardinals as present. **They are substrings inside तीसरा,
बीसवाँ and सौवाँ.** Check the romanization before believing a numeral match.

### The shape the language forces

| range | what it costs |
|---|---|
| the decades — तीस, चालीस, पचास, साठ, सत्तर, अस्सी, नब्बे, सौ | **eight words**, learnable as a spine |
| everything between them | **seventy-two words**, each individually eroded |

Hindi does not build 21–99 the way Spanish or English does. **इक्कीस** is not
*ek* plus *bīs* in any way a learner can reassemble, and neither is **बाईस** or
**पच्चीस**. They have to be met as words.

So the honest chapter teaches **the decades as a spine and the pattern as a
recognition skill**, and says plainly that the numbers between are their own
words. It does not pretend a generative rule exists.

### Proposed chapter 104, sequences 4420–4460

Following the shape of chapters 21–22 (a dense word lesson plus a history
lesson) and of chapters 99–103 (five lessons ending in repaso and síntesis):

1. **the decades** — तीस … नब्बे
2. **सौ**, hooked on the **सौवाँ** the reader already says
3. **the in-betweens** — इक्कीस, बाईस, तेईस as recognition, with the erosion named
4. **repaso**
5. **síntesis: reading a price** — which is what mock 2 reading item 4 needs
   (`पचास रुपये`), and what the NUM-03 note is really asking for

### Before probing NUM-03

**Decide honestly whether the decades plus a recognition pattern is "21 to 100".**
It gives every price, age and house number a reader is likely to meet, and it
does not give all eighty words. If that is judged short, leave the probe off and
say what is covered — this campaign has already withdrawn one probe for claiming
a series was complete when it was not (`HL-C386`).

Five wiring files are needed for a new chapter: `chapters.d/0104.json` (pure
ASCII), the curriculum extension and path entries, the spine node's `segments`
array, and `core/book-generation.d/targets.d/hindi-0104.json` with
`"scriptSet": "hindi-main"`.
