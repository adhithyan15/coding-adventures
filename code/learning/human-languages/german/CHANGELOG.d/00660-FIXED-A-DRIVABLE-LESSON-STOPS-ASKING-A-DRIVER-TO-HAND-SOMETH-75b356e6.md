## Fixed — a drivable lesson stops asking a driver to hand something over

A spoken cue is read to a driver as an ordinary turn (issue #12070).
GE-R20-danke-bitte-entschuldigung asked for "*here you go*, handing something
over"; it now says "*here you go*, offering something", the same shape as its
neighbours ("*please*, asking for something"). The drivable-lesson gesture
check in human-language-data now knows "hand over", found by a security
review. The lesson stays `drivable: true` (only its modality source hash
changes).

