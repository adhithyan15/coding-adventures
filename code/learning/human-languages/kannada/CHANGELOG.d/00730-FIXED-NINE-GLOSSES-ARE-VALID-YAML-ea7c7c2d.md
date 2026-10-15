## Fixed — nine glosses are valid YAML

- KA-C23-dina, KA-C24-ratri, KA-C25-shubha-ratri, KA-C26-belagge, KA-C27-sanje, KA-C28-aparahna, KA-C29-shubhodaya, KA-C30-shubha-sanje and KA-C31-shubha-madhyahna had a `gloss:` that opened with a quoted English word and went on past the closing quote (`gloss: "day" — …`), which a strict YAML parser rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- KA-C23-dina's and KA-C25-shubha-ratri's glosses also ended in a quote, so the repository's reader stripped the first and last quote marks and shipped a gloss with one stray quote at each end. They now ship the whole gloss. The other seven read exactly as before. Narration and book chapters pick this up when they are regenerated.
