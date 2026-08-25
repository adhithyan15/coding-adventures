### Changed - HL-C89's scope, corrected by measuring before building

HL10 section 7.4 also asks for a banned-word lint -- no *simply*, *just*,
*obviously*. Measured first, and it is nearly a no-op: a naive denylist flags
**535 of 1,694** lessons (`just` 359, `simply` 184), narrowing to genuinely
dismissive senses drops it to **23**, and reading those, most are still innocent
-- "*Desde luego* means 'of course'" is teaching the phrase, not talking down to
the reader. The corpus's prose is already kind. Building that half first would
have produced a gate nobody needed.


