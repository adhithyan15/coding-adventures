## Fixed — chapters 210 and 266 take their last lesson as payoff

The last two Spanish chapters below the 0.5 `chapter-payoff-not-representative`
floor had the same shape. The payoff was a synthesis or review that introduces
nothing, and the chapter's own atoms come from a lesson placed after it. Both
payoffs scored 0. A checkpoint after the last lesson would only repeat it, and
moving the late lesson earlier would break its prerequisites. So the payoff
moves to the chapter's last lesson. That lesson gains a short recall of what
the old payoff proved, and the can-do now also names what the chapter ends on.
No lesson moves, and the curriculum graph is unchanged.

- **Chapter 210** (0/2 → **2/2**). The payoff is now **ES-C56-depende** (2287),
  which introduces *depende* and the unstated condition. A new "Guided
  Practice — an open condition and a closed one" has the learner say
  *depende*, then *Si tuviera tiempo, comería en casa* (the unreal condition
  from chapter 208, reviewed in 209). It then asks which of the two already gives the
  answer. So the payoff still asks for speech about a world that does not
  exist. The can-do and summary become "I can speak about a world that does
  not exist, and say it depends when the condition is still open." The lesson
  now requires and practises ES-GRAMMAR-SI-UNREAL-CONDITION. Computed 267 s;
  max_seconds 240 → 270. ES-C56-sintesis-condiciones stays as the chapter's
  synthesis.
- **Chapter 266** (0/1 → **1/1**). The payoff is now **ES-C67-tercera-lectura**
  (2569), the third connected reading. A new "Guided Practice — the front of
  the sentence" points out that four of the paragraph's sentences open on a
  *when*, then asks what that front position carries (emphasis) and which loss
  it helps replace (Latin's endings). That is the thread ES-C67-repaso-a1
  names. The lesson now requires and practises ES-GRAMMAR-CIRCUMSTANTIAL-MOVES
  and ES-GRAMMAR-WORD-ORDER-EMPHASIS. The can-do and summary add ", and read a
  paragraph of Spanish straight through." Computed 283 s against the declared
  290. The review stays as the chapter's review.
- Each `payoff.note` says why the earlier lesson could not be the payoff.
  Regenerated book chapters 210 and 266, narration, the two lessons' modality
  owners (modality unchanged) and hash shards. Spanish now has no chapter-gate
  debt.

