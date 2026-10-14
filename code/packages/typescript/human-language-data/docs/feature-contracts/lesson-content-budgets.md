# Lesson-content budgets and gap reports

Build the JSON and readable gap reports locally with:

```bash
npm run build
npm run --silent report -- --format json > curriculum-gaps.json
npm run --silent report -- --format text > curriculum-gaps.txt
```

The report's `lessonBudgets` section reads all three lesson-content ceilings
from `core/chapter-policy.json`. Because prose cannot reliably distinguish an
idiom from a useful phrase, a lexical item from a newly taught sense, or one
culture paragraph from several claims, authors record stable ids explicitly:

```yaml
introduces_idioms: [ES-IDIOM-TENER-QUE]
introduces_senses: [ES-SENSE-QUEDAR-REMAIN]
introduces_culture_claims: [ES-CULTURE-TU-USTED-01]
```

Use an explicit empty list for a reviewed zero. A missing list remains
unmeasured migration debt; it is never interpreted as proof that the lesson is
within budget. The measurement is report-only while that annotation backlog is
burned down.
