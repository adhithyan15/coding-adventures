# Human-language book generation rejects ad-hoc lesson block titles

The lesson parser accepts arbitrary `##` headings, so focused curriculum,
activity, continuity, and writing-stage tests can all pass while
`npm run generate:books` later fails with `generated books require known body
blocks`. This happened when Gujarati retrieval lessons used `Reading check` and
`Writing from sound`. Reuse the corpus's canonical taxonomy — for example
`Guided Practice` and `Writing — from sound` — and run book generation before
publishing any new lesson shape.
