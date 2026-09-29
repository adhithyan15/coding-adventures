### Fixed — a `list<list<text>>` slot never read the host

A slot typed as a list of rows fell through the host-binding match to the
*sample* value — a constant — while every neighbouring prop read from the host
correctly. The generated shell therefore compiled, ran, and showed an **empty
table** no matter what the host sent.

Rows are how every table in Mosaic is modelled: Engram's deck list, TaskApp's
project nav. So this was not an exotic corner, it was the one slot shape whose
whole purpose is to carry data, silently bound to nothing.

Adds a nested-list reader (`mosaicStringListList` / `MosaicHostValue.stringListList`)
and the match arm that reaches it.

