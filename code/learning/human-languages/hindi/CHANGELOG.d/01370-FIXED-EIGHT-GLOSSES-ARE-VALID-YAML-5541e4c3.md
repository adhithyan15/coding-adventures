## Fixed — eight glosses are valid YAML

- HI-C25-din, HI-C26-raat, HI-C27-shubh-raatri, HI-C28-subah, HI-C29-shaam, HI-C30-dopahar-widened, HI-C32-shubh-sandhya and HI-C33-shubh-dopahar had a `gloss:` that opened with a quoted English word and went on past the closing quote (`gloss: "day" — …`), which a strict YAML parser rejects. Each is now one single-quoted scalar, with an apostrophe written `''`. The wording is unchanged.
- HI-C27-shubh-raatri's gloss also ended in a quote, so the repository's reader stripped the first and last quote marks and shipped `good night" — … meaning "shining`. It now ships the whole gloss, `"good night" — … meaning "shining"`. The other seven read exactly as before. Narration and book chapters pick this up when they are regenerated.
