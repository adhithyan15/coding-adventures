## HL-C416-f6d53572 — Spanish A1 and pre-A1 mock papers use live-ccTLD domains that can resolve

**Status: OPEN.** Found by the security review of the A2 mocks (PR #15850) and
independently confirmed by census.

The project rule is that any host, domain, email, URL, IP, named school,
business, person, street or phone number in learner-facing material must be one
that **cannot resolve to a real entity**. The A2 papers added in #15850 follow
it — all six addresses use reserved RFC 2606 `.invalid` names. The A1 and
pre-A1 papers, written earlier, do not.

| address | file set | domain |
|---|---|---|
| `carmen.ruiz@correo.es` | a1 | `.es`, live ccTLD |
| `lucia.martin@correo.es` | a1 | `.es`, live ccTLD |
| `trabajo@hotelmar.es` | a1 / pre-a1 | `.es`, live ccTLD |
| `reservas@hotelsol.es` | a1 / pre-a1 | `.es`, live ccTLD |
| `biblioteca@sanmartin.es` | a1 / pre-a1 | `.es`, live ccTLD |

`.es` is a real ccTLD and every one of these is a registrable second-level
name. Two of them — `hotelmar.es`, `hotelsol.es` — are exactly the shape a
real Spanish hotel would register, and `sanmartin.es` the shape of a real
library or town. Whether any is registered today is not the test; the rule is
that it **cannot** resolve, and these can.

**The fix is mechanical and small**: rewrite the five to `.invalid`
equivalents. RFC 2606 reserves `.invalid` precisely so that documentation and
test material cannot collide with real names.

**Why this was not fixed in #15850.** That PR adds A2 material and touches no
file under `mocks/a1/` or `mocks/pre-a1/`. Editing exam papers for two other
levels inside a vocabulary change would widen it well past its subject, and the
A1 papers are pinned by `spanish-a1-mock-audit` and `spanish-prea1-mock-audit`
— both audits parse the answer keys, so any edit to the papers must be checked
against both gates rather than made casually.

**Worth checking at the same time**, since the same rule applies corpus-wide and
no gate enforces it: whether other tracks' learner-facing material carries
resolvable hosts, emails or phone numbers. A census across all 23 tracks would
say whether this is two files or a pattern. If it is a pattern, the durable fix
is a gate rather than a sweep — the rule is currently enforced only by review
attention, which is why material written earlier drifted from it.
