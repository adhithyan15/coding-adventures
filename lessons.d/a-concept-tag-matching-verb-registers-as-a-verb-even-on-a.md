# A concept_tag matching /(^|-)VERB-/ registers as a verb even on a review

`ES-VERB-ETYMOLOGY-CERTAINTY-REVIEW` on a review lesson that introduces no verb
pushed Spanish's namespaced-verb `extras` count 43 -> 44 and failed
`verbs.test.ts`. The tag is read by `verbCoverage`, not just by humans. Name a
review's concept tag for what it reviews, without the `VERB-` infix.
