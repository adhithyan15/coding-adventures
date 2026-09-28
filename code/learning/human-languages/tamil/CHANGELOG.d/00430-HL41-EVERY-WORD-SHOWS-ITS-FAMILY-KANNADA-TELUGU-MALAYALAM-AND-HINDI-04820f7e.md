## HL41: every word shows its family (Kannada, Telugu, Malayalam) and Hindi

521 Tamil vocabulary lessons now print an **In the family, and next door**
panel in the book. For **போ** *pō* it reads:

| | word | said | same root |
|---|---|---|---|
| Kannada | ಹೋಗು | hōgu | yes |
| Telugu | వెళ్ళు | veḷḷu | |
| Malayalam | പോകുക | pōkuka | yes |
| Hindi (neighbour) | जाना | jānā | |
| English | to go | | |

**Where the words come from.** They come from this repository's own Kannada,
Telugu, Malayalam and Hindi tracks. A lesson counts as an equivalent only if it
has the same concept tag, or a parallel one whose first gloss shares a word, or
an unambiguous first-gloss match. Tags alone are not enough, because they can
share a spelling while meaning different things: *kattu* "shout" and *kaṭṭu*
"tie" do not match. Some matches are dropped:

- matches with no form in their own script, such as the oral-only number
  lessons;
- lists longer than four words, since a panel row is a word. Hindi's Gregorian
  months are not the Tamil solar months anyway.

**Same root.** 420 pairs are marked "yes", after the author judged each
candidate. The candidates were pairs whose romanizations are close, allowing
for two regular sound patterns:

- Kannada's *p* → *h*, as in *pō*/*hōgu* and *pal*/*hallu*;
- Malayalam's verb ending *-uka*, as in *vā*/*varuka*.

Sixty pairs were judged unrelated, including:

- *paḻam* vs Hindi *phal*;
- *iravu* vs *rātri*;
- *maṇi* vs *gaṇṭe*.

Thirty-five uncertain pairs and every pair not judged carry no mark. The data
awaits a native speaker's review. Each owner file says where its words came from.

**Book.** The panels take the book past 999 pages, so the preamble widens
the contents page-number box, as Hindi and Malayalam do. The preamble also
inputs `_shared/nasal-macron.tex`, because the Hindi romanizations use long
nasal vowels such as *ā̃*, and without it Latin Modern drops their macron.

      tamil lessons with a family panel    0  ->  521
