## HL-C09HS — Devanagari closure debt is measured, not waved through

Exercising the completion gate against all three Devanagari tracks finds **244
affected lesson realizations** and **18 distinct missing glyphs**: ख, ़, ँ, ठ,
फ, ज, थ, ष, ट, ढ, झ, घ, छ, ड, ण, ळ, ः, and ञ. The inventory therefore remains
fail-closed at 28 sourced rows. Integration coverage now pins both the affected
realization count and missing set so future vocabulary or inventory expansion
cannot change this debt silently.

The next implementable tranche is the five most widespread gaps: Chandrabindu
**ँ**, Visarga **ः**, and consonants **ख, ज,** and **ण**. Each needs
source-backed stroke data and font-fit paths before the completion audit can be
repeated; no flag-only shortcut is acceptable.

