## HL-C09R — Malayalam anusvara returns the queue to Tamil

After Tamil light ra landed, the measured queue put Malayalam anusvara **ം**
first at **44 affected realizations**. Unicode 17 §12.9.3 identifies U+0D02 as
MALAYALAM SIGN ANUSVARA, shows it after independent vowels and dependent vowel
signs, and requires renderers to handle it on Malayalam letters and other
supported bases.

The Malayalam inventory now models that base-first encoded composition without
inventing a universal handwriting direction or pen-lift count. This removes all
**44 affected realizations** for **ം**. The reranked queue returns to Tamil with
**ய** at 38.

