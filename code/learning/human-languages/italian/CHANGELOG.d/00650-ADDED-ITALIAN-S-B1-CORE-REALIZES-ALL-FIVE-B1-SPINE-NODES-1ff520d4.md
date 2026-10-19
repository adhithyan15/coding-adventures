## Added — Italian's B1 core realizes all five B1 spine nodes

Ten chapters, 263-272 (sequences 15700-16040), 35 lessons: two chapters per B1
spine node, each closing on a review that is its payoff. Before them the track
had no *imperfetto* and no *condizionale* at all, and all five B1 nodes were
declared omissions in `curriculum.d/spine/`. Those omission ledgers are now
empty, and the level gate no longer lists `spine-nodes` among Italian's B1
blockers; vocabulary remains the open B1 criterion.

- **`SPINE-NARRATE-EVENTS`, chapters 263-264.** The imperfetto on all three
  verb families (`IT-C263-parlavo`), essere's irregular *ero/era* and *c'era*,
  weather with *faceva freddo*, and the scene-against-events division of labour
  with *mentre* (`IT-C263-dormivo`, `ASPECT-ONGOING-PAST`). Then *prima, poi,
  alla fine* (`CONNECTIVE-THEN`), the backbone of a told story under
  *raccontare* (`NARRATIVE-SEQUENCE`), and the track's first B1 reading, a
  95-word journey in both pasts whose every word is taught earlier
  (`IT-C264-lettura`), retold aloud in the payoff.
- **`SPINE-GIVE-REASONS`, chapters 265-266.** *secondo me* and *credo di sì*
  (`OPINION-I-THINK`); *perché* pointing back against *quindi*/*così* pointing
  forward (`CONNECTIVE-SO`); *dovrei/dovresti* (`MODAL-SHOULD`), the first
  condizionale forms after *vorrei*; and the four moves of an explanation under
  *spiegare*, which is *piegare* unfolded (`EXPLANATION-GIVE`).
- **`SPINE-HANDLE-TRAVEL`, chapters 267-268.** *Scusi, per andare a…?* with
  *alla/al/all'* (`DIRECTION-ASK`); *a destra, a sinistra* and understanding
  *vada dritto, giri a destra*; *andata e ritorno*, *solo andata* and *vorrei un
  biglietto per…* (`TRANSPORT-TICKET`); *ho perso il treno* and a three-part
  problem report (`PROBLEM-REPORT`).
- **`SPINE-DESCRIBE-EXPERIENCE`, chapters 269-270.** Feelings in the past as
  scene, with agreement and *contento di* (`ADJECTIVE-FEELING`); *spero di* and
  *il mio sogno è* (`AMBITION-EXPRESS`); *più … di*, *meno … di* and *il più*
  (`COMPARISON-BASIC`); *da tre anni* with the present against *per* with the
  passato prossimo (`TIME-DURATION`).
- **`SPINE-EXPRESS-CONDITION`, chapters 271-272.** *se* (`CONNECTIVE-IF`), the
  real condition with both halves in the present (`CONDITIONAL-REAL`),
  *dipende* and *dipende da* (`CONDITION-DEPENDS`), the condizionale with
  *potrei, vorrei, dovrei* and *parlerei* (`MODAL-WOULD`) and *sarebbe*. The
  chapter ends on `IT-W272-sei-righe`, six connected lines to one reader about a
  journey, a second connected-composition piece of evidence.

New headwords: *era, c'era*; *faceva freddo, faceva caldo*; *secondo me*;
*dovrei*; *scusi, per andare a…?*; *a destra, a sinistra*; *andata e ritorno*;
*ho perso il treno*; *spero di*; *più … di*; *da tre anni*; *dipende*; *sarebbe*.
Where an item was already taught (*credere, quindi, così, dovere, spiegare,
raccontare, biglietto, ritorno, perdere, contento, sperare, dipendere, vorrei*)
the concept lesson builds on it. Etymologies cite EtymDB 2.1 and Lewis & Short,
and say so where neither covers a step: the Old Italian *-a* of *io parlava*,
the condizionale from infinitive + *habuī*, the stem *sar-*, *spiegare*,
*raccontare*, and *andare*, whose origin is given as not settled. Every new atom
is practised at least twice later; no forward reference, idiom, sense or culture
claim is added. `tests/corpus/italian.test.ts` pins the lesson count at 1350 and
the second connected composition.
