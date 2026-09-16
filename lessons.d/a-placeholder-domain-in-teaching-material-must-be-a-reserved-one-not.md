---
category: Repo policy / workflow reminders
---

# A placeholder domain in teaching material must be a reserved one, not merely an obvious-looking one

Spanish chapter 428 teaches how to dictate an e-mail address aloud -- the point
of the chapter is the Spanish names for the at sign, the dot and the underscore.
It used the example address `ana_lopez@ejemplo.es`, and the synthesis lesson told
the reader `Confirma en ana_lopez arroba ejemplo punto es` -- an instruction to
write to it.

`ejemplo` means "example" in Spanish, so the address LOOKS like a placeholder and
reads like one to anybody following the lesson. It is not one. RFC 2606 reserves
`example.com`, `example.net` and `example.org`, and RFC 6761 reserves the
`.example` TLD. **`ejemplo.es` is none of those** -- it is an ordinary
registrable name under a real ccTLD, so it can belong to somebody, and a reader
doing what the lesson said would send them mail.

The security review caught it as a LOW finding. It is worth recording anyway,
because the failure mode is not "we used an unsafe string" but "we inferred a
string's safety from its MEANING rather than from a registry." A placeholder is
safe because a standards body reserved it, not because it translates to the word
"example".

The fix was one substitution in three lessons plus regeneration: the addresses
now end in `.example`. The teaching was not lost -- both lessons gained a
sentence saying a real Spanish address ends in `punto es`, and that the printed
ones end in the reserved word so nobody is written to by accident. That is
arguably better than the original, because it now names the convention instead of
silently relying on it.

What to do differently. Any host, domain, address or URL in learner-facing
material must come from a reserved range: `example.com/.net/.org`, the `.example`
TLD, `192.0.2.0/24` and its siblings (RFC 5737), or `2001:db8::/32`. Translating
"example" into the target language does not make a name reserved, and a corpus
that teaches people to send mail somewhere should be held to that strictly. This
was the only e-mail address in the whole corpus, so there was no house convention
to follow -- there is one now.
