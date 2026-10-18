## Fixed — twenty-eight frontmatter values are valid YAML

- ES-C02-tarde, ES-C282-abrazo, ES-C282-saludo, ES-C282-sonrisa, ES-C283-amable, ES-C283-disculpe, ES-C284-cierto, ES-C284-de-acuerdo, ES-C284-por-supuesto, ES-C284-seguro, ES-C285-enfermera, ES-C285-vecino, ES-C287-plato, ES-C287-vaso, ES-C288-despedida, ES-C288-llegada, ES-C288-salida, ES-C288-vuelta, ES-C295-orilla, ES-C295-sendero, ES-C297-tenedor, ES-C298-sin-duda, ES-C299-ganado, ES-C299-granja, ES-C38-luego and ES-C469-llamada had an unquoted `gloss:` with a colon and a space inside it (`gloss: … — feminine; also: late) …`), which a strict YAML parser reads as the start of a second key and rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- ES-C19-no's `headword: no` and ES-C19-si's `gloss: yes` are now quoted (`'no'` and `'yes'`), so a YAML 1.1 parser no longer reads them as booleans. The repository's reader always kept them as text.
- The repository's reader returns exactly the text it returned before for every value, and PyYAML now returns that same text.

