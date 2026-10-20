## Added — German's B1 core realizes all five B1 spine nodes

Ten chapters, 252-261 (sequences 14100-14450), 36 lessons: two chapters per B1
spine node, each closing on a review that is its payoff, and the last on a
writing task. Before them the track had *war*, *hatte*, *konnte* and the weak
*-te* Präteritum, the Perfekt, *weil*, *deshalb* and *trotzdem*, but no strong
Präteritum, no *dass*, no *wenn* and no Konjunktiv II at all, and all five B1
nodes were declared omissions in `curriculum.d/spine/`. Those omission ledgers
are now empty, and the level gate no longer lists `spine-nodes` among German's
B1 blockers; vocabulary and the one missing verb remain the open B1 criteria.
The track teaches standard German, with short notes where Swiss usage differs
(*Exgüsi*, *das Billett*, *retour*, no *ß*).

- **`SPINE-NARRATE-EVENTS`, chapters 252-253.** The strong Präteritum by vowel
  change, *ging, kam, sah, fuhr, nahm, stand*, with separable *kam … an*
  (`GE-C252-ich-ging`); one past for "I read" and "I was reading", the scene
  held open with *während*, and the colloquial *am Lesen sein* noted
  (`GE-C252-ich-las`, `ASPECT-ONGOING-PAST`); *als* for one past occasion
  against *wenn* for every time, both verb-final (`GE-C252-als`). Then *dann*
  with *zuerst* and *danach* and the verb straight after each
  (`CONNECTIVE-THEN`), *schließlich* against the taught *endlich* and the frame
  of a told story (`NARRATIVE-SEQUENCE`), and the track's first B1 reading, a
  ninety-five-word journey in the Präteritum made only of words met earlier
  (`GE-C253-lesung`, with one objective check), retold aloud in the payoff.
- **`SPINE-GIVE-REASONS`, chapters 254-255.** *ich finde, ich glaube, ich meine,
  dass …* with the verb at the end, and *dass* dropped with the verb back in
  second place (`OPINION-I-THINK`); *meiner Meinung nach* in the first place;
  *darum* beside *deshalb*, pointing forward with the verb second, against
  *weil* pointing back with the verb last (`CONNECTIVE-SO`); *du solltest* and
  *man sollte*, against the stronger *du sollst* (`MODAL-SHOULD`); and
  *erklären, warum …* with an embedded question and an explanation in three
  steps (`EXPLANATION-GIVE`).
- **`SPINE-HANDLE-TRAVEL`, chapters 256-257.** *Entschuldigung, wie komme ich
  zum Bahnhof?* with *zum* and *zur* by gender (`DIRECTION-ASK`); *links* beside
  the taught *rechts* and *geradeaus*, and the *Sie* command with the verb
  first; *hin und zurück*, *einfach*, and *nach* for a city
  (`TRANSPORT-TICKET`); *ich habe den Zug verpasst*, in the Perfekt of
  conversation, and a two-move problem report (`PROBLEM-REPORT`).
- **`SPINE-DESCRIBE-EXPERIENCE`, chapters 258-259.** *ich war froh, dass …* and
  feelings with *weil* (`ADJECTIVE-FEELING`); *ich hoffe, dass …*, *ich hoffe,
  … zu …* and *mein Traum ist es, … zu …* (`AMBITION-EXPRESS`); *-er als* with
  the umlaut of *älter* and *größer*, *besser*, *mehr*, *lieber*, and *so … wie*
  (`COMPARISON-BASIC`); *seit* with the present and the dative, *vor* for "ago",
  and the bare accusative for a finished stretch (`TIME-DURATION`).
- **`SPINE-EXPRESS-CONDITION`, chapters 260-261.** *wenn* for "if" and *falls*
  for "in case" (`CONNECTIVE-IF`); the real condition in the present even for
  tomorrow, with verb, comma, verb (`CONDITIONAL-REAL`); *es kommt darauf an*,
  *das hängt davon ab* and *ob* (`CONDITION-DEPENDS`); *ich hätte gern* and
  *das wäre*; *würde* + infinitive (`MODAL-WOULD`); and *wenn ich Zeit hätte,
  würde ich …* with *ich könnte* for the condition that is not true now. The
  chapter ends on `GE-W261-sechs-zeilen`, six connected lines to one reader
  about a trip.

New headwords (word and phrase lessons): *schließlich*; *meiner Meinung nach*;
*darum*; *du solltest / man sollte*; *wie komme ich zum Bahnhof*; *hin und
zurück*; *ich habe den Zug verpasst*; *es kommt darauf an / das hängt davon ab*;
*ich hätte gern / das wäre*. Items whose bare form appears in earlier lessons
(*dann*, *froh*, *links*, *falls*) are taught inside grammar lessons so that
they register no forward-reference matcher. Where an item was already taught
(*zuerst, danach, endlich, glauben, finden, die Meinung, deshalb, erklären,
warum, Entschuldigung, geradeaus, rechts, die Fahrkarte, einfach, verpassen,
hoffen, der Traum, lieber, werden, konnte*) the lesson builds on it.
Etymologies cite EtymDB 2.1 and say so where it is silent: the Old High German
past forms *quam* and *sah*, the split of *wenn* from *wann*, *schließlich*,
*darum*, *zurück*, *seit*, *vor*, *falls* from *im Falle*, and the umlaut of
*älter*, *hätte*, *wäre* and *würde*. Every new non-etymology atom is practised
at least twice later; no forward reference, idiom, sense or culture claim is
added. The German lesson-content pin moves from 1384 to 1420, and
`tests/levels.test.ts` moves German from the A2-reach list to the list of
tracks that reach beyond A2.
