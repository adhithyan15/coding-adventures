## Added — French's B1 core realizes all five B1 spine nodes

Ten chapters, 258-267 (sequences 15100-15450), 36 lessons: two chapters per B1
spine node, each closing on a review that is its payoff. Before them the track
had no *imparfait* and no *conditionnel* at all, and all five B1 nodes were
declared omissions in `curriculum.d/spine/`. Those omission ledgers are now
empty, and the level gate no longer lists `spine-nodes` among French's B1
blockers; vocabulary remains the open B1 criterion. The track writes standard
metropolitan French, with a short note where Québec usage differs (*il fait
frette*, *manquer le train*, *si j'aurais* in relaxed speech).

- **`SPINE-NARRATE-EVENTS`, chapters 258-259.** The imparfait on the *nous*
  stem (`FR-C258-parlais`); *c'était* and *il y avait*, with *j'avais* for age;
  the weather with *il faisait froid* and *il pleuvait*; the scene against the
  events, and the imparfait of habit (`FR-C258-dormais-dormi`,
  `ASPECT-ONGOING-PAST`); *quand* for the event that cuts in, *pendant que*, and
  *être en train de*. Then *d'abord* with *ensuite, puis, enfin*
  (`CONNECTIVE-THEN`), the backbone of a told story under *raconter*, with
  *mais* for the turn (`NARRATIVE-SEQUENCE`), and the track's first B1 reading,
  a hundred-word journey in both pasts made only of words met earlier
  (`FR-C259-lecture`, with one objective check), retold aloud in the payoff.
- **`SPINE-GIVE-REASONS`, chapters 260-261.** *je pense que*, *je crois que*,
  *je trouve que* and *à mon avis* (`OPINION-I-THINK`); *parce que* pointing
  back against *donc* and *alors* pointing forward (`CONNECTIVE-SO`); *je
  devrais, tu devrais, on devrait* (`MODAL-SHOULD`); and the four moves of an
  explanation under *expliquer*, a learned *explicāre* built on the *plicāre*
  that French inherited as *plier* (`EXPLANATION-GIVE`).
- **`SPINE-HANDLE-TRAVEL`, chapters 262-263.** *Excusez-moi, pour aller à… ?*
  and *où est… ?* (`DIRECTION-ASK`); *tout droit* against *à droite*, and the
  *vous* commands of directions; *un aller-retour*, *un aller simple* and *je
  voudrais un aller-retour pour…* (`TRANSPORT-TICKET`); *j'ai raté le train*,
  beside *manquer* and *perdre*, and a three-part problem report
  (`PROBLEM-REPORT`).
- **`SPINE-DESCRIBE-EXPERIENCE`, chapters 264-265.** Feelings in the imparfait
  as scene, with agreement, the passé composé as a reaction, and *content de*
  (`ADJECTIVE-FEELING`); *j'espère* + bare infinitive, *j'espère que* and *mon
  rêve, c'est de* (`AMBITION-EXPRESS`); *plus … que*, *moins … que*, *aussi …
  que*, *le plus*, *meilleur* and *mieux* (`COMPARISON-BASIC`); *depuis* with the
  present against *pendant* with the passé composé, *ça fait … que*, and *il y
  a* for "ago" (`TIME-DURATION`).
- **`SPINE-EXPRESS-CONDITION`, chapters 266-267.** *si*, *s'il*, and *si* for
  "whether", kept apart from the *si* that contradicts a negative
  (`CONNECTIVE-IF`); the real condition with the plain present after *si*, even
  for tomorrow (`CONDITIONAL-REAL`); *ça dépend* and *ça dépend de*
  (`CONDITION-DEPENDS`); the conditionnel on the infinitive with the imparfait's
  endings, *je pourrais, j'aimerais, je devrais* (`MODAL-WOULD`), and *ce
  serait* with *si* + imparfait for the condition that is not true now. The
  chapter ends on `FR-W267-six-lignes`, six connected lines to one reader about
  a journey, a second connected-composition piece of evidence.

New headwords (word and phrase lessons): *c'était, il y avait*; *il faisait
froid, il faisait chaud*; *d'abord*; *raconter*; *je devrais*; *tout droit*;
*un aller-retour, un aller simple*; *j'ai raté le train*; *ça dépend*; *ce
serait*. Where an item was already taught (*ensuite, puis, enfin, penser,
croire, l'avis, donc, alors, devoir, je voudrais, expliquer, la raison, à
droite, tourner, le billet, le retour, manquer, content, espérer, le rêve,
depuis, dépendre*) the concept lesson builds on it, and *excusez-moi*, which
chapter 24 mentions in passing, is a grammar lesson so that the hyphen does not
register *moi* as a new matcher. Etymologies cite EtymDB 2.1 and Lewis & Short,
and say so where neither covers a step: the *-oit*/*-ais* history of the
imparfait ending, *chaud* beyond Old French, *abord*, *rater*, *droit*, *donc*,
the conditionnel's endings from *habēbat*, and the *ser-* stem of *serait*.
Every new atom is practised at least twice later; no forward reference, idiom,
sense or culture claim is added. `tests/corpus/french.test.ts` pins the lesson
count at 1412, the second connected composition and the reading's objective
activity; `tests/levels.test.ts` moves French from the A2-reach list to the
list of tracks that reach beyond A2.
