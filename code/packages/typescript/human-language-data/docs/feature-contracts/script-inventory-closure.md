# Script-inventory closure

Script inventories (`data/scripts/<script>.json`) close a track's headwords:
`validate` warns about any script character no row covers. Letters, marks,
cited independent vowels and final consonants count, and so do a script's
own `digits` rows, which is what lets the Bengali numbers lessons (০–৯) close.
