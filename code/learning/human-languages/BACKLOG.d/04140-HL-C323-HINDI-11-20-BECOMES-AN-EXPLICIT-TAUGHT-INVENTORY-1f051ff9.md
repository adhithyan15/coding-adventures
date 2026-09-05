## HL-C323 — Hindi 11–20 becomes an explicit taught inventory

After the polite-yes repair, **उन्नीस** is the next independent modern Hindi
word in the ranked glossed-but-never-taught queue. Chapter 22 already presents
all ten number words from 11 through 20 in its teaching table, tells learners
to memorize the individually irregular forms, practises the range, and assesses
the lesson's number concept.

The lesson headword used **ग्यारह — बीस** as a human-readable range. Downstream
token reports and study artifacts cannot infer the eight interior forms from an
em dash, leaving the prominently taught **उन्नीस** and its peers undeclared.

Replace the range notation with the explicit inventory **ग्यारह बारह तेरह
चौदह पंद्रह सोलह सत्रह अठारह उन्नीस बीस** and regenerate the derived Hindi
artifacts. This records existing teaching without adding a new vocabulary
claim, while ensuring every member of the taught range is machine-visible.
