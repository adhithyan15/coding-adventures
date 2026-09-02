## HL-C303 — Kannada A1: no conjunction exists, and the script is not closed

`core/exam-inventory-kannada-a1.json` enumerates **258** A1 points and the corpus
covers **167** of them, 65%. That is the second-highest coverage of the nine
inventories written (Telugu 65% on 326 points, Tamil 59%, Hindi 55%, Marathi
29%), and the number is not the interesting part.

### The joining column is empty, for the fourth Indic track running

**2 of 11 covered**, and neither covered point is a conjunction.

    mattu      (and)        0 occurrences in 268 lessons, 0 in the generated book
    athava     (or)         0
    aadare     (but)        0
    eekendare  (because)    0
    anta/endu  (quotative)  0

This was checked in both scripts and in romanization, in the lesson markdown and
in the compiled `.tex`. It is not that the words are taught late; they are not in
the track at all. Hindi, Telugu and Tamil all reported the same hole, and Tamil
measured 0 of 7. Kannada's two covered points are worth naming because they show
what the language actually does instead:

* **the -i participle** (`hoogi baruttene`, "having gone, I come back") is
  Kannada's real clause-chaining machinery, and chapter 4 teaches the joint
  itself. Dravidian chains participles where European languages coordinate, so
  this is not a consolation prize -- it is the door the rest of the column opens
  through, and it is already open.
* **chapter 64**, which is *named* `JOIN` and teaches `aameele`, `kuuda`,
  `haagaadare`, `beega` and `innuu`. Every one of them connects a TURN. None
  connects a clause. A chapter named for the thing, not doing the thing, is the
  clearest single illustration of this gap in the corpus.

The downstream cost is larger than eleven points. Missing `athava` blocks
"express a preference between alternatives"; missing `aadare` blocks "offer a
counter-argument"; missing the quotative blocks "give an opinion" -- the most
used A1 function there is -- and leaves `yoocisu` ("to think", taught in chapter
33) unable to take an object. Three function points and a taught verb are idle
for want of words that are each one lesson long.

### The script column is the OPPOSITE of Tamil's, and only a census showed it

Tamil's inventory reported its script column nearly closed: 52 of 52 characters
taught. Carrying that expectation into Kannada would have produced false notes.
Measured instead:

    characters taught by a script lesson      42
    distinct characters used in headwords     69
    used but NEVER taught                     27   (61% closure)

The untaught 27 are not exotic. `ma` appears in 40 headwords, `la` in 37, `va`
in 30, `ta` in 20. A learner who has completed every one of the 42 script
lessons still cannot read `maatanaaDu`, the verb chapter 5 is built on.

**The general lesson: an inventory must measure the track it is for.** Tamil's
shape and Hindi's shape disagree with each other, and Kannada matches neither.

### Two more findings worth a tranche

* **The third person does not exist.** `avanu`, `avalu`, `avaru` and their
  possessives are absent, so a learner can talk about themselves and to one
  other person and cannot say one word about anybody who is not in the room.
  This also idles the four family chapters and every third-person verb ending.
* **The accusative `-annu` is untaught**, so no taught transitive verb can take
  a definite object, and object pronouns cannot be formed at all.

### What the exam-levels entry does not say

`core/exam-levels.json` gives `tracks.tamil` an explicit diglossia caveat and
`tracks.kannada` none. Kannada is as diglossic as Tamil. The corpus in fact
handles it well -- all 249 schema-v2 lessons declare `variety:
standard-colloquial`, a real and uniform editorial choice, and three pragmatics
atoms tell the learner when a politeness word is genuinely used. What no lesson
does is tell the learner that a literary register EXISTS. Recorded at
KA-A1-REG-04; a caveat on `tracks.kannada` would be a one-line fix.
