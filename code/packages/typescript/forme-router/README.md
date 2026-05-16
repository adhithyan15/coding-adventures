# @coding-adventures/forme-router

Forme route-derivation stage. Consumes a `Stream<ContentNode>`,
derives a URL route per node (from `frontmatter.slug` if set, else
slugified `sourcePath` basename), and emits a `Stream<ContentNode>`
with the `route` field populated.

## Why this exists

In v0 of `forme-render-static` and `forme-collect-chronological`,
each stage derived routes independently from `ContentNode.sourcePath`
because `ContentNode.route` arrived as `null` from the parser. The
two stages duplicated ~30 lines of `slugify` + `formatRoute` logic
across packages and had a TODO calling for a "v0.2 router stage" to
hoist the work upstream.

This is that stage.

```
source-fs → parse-markdown → router → collect-chronological
                                    └→ render-static
```

Both downstream stages can now read `ContentNode.route` directly
instead of re-deriving it. When `forme-render-static` is updated to
prefer `node.route` over its own derivation, the `slug.ts`
duplications in the two downstream packages will be deleted.

## API

```typescript
import router from "@coding-adventures/forme-router";

const stage = router; // default export

// In a PipelineConfig:
{
  stage: router,
  config: {
    routeTemplate: "/blog/{slug}.html",  // default; supports {slug} only
    slugField:     "slug",               // default frontmatter key
  },
}
```

## Slug derivation rules

In priority order:

1. `node.frontmatter[config.slugField]` if it is a non-empty string.
2. `slugify(node.sourcePath)`:
   - take the basename (POSIX or Windows separator)
   - strip `.md`, `.mdx`, `.markdown` extensions
   - lowercase
   - replace whitespace and `_` runs with `-`
   - drop any non-`[a-z0-9-]` characters
   - collapse consecutive `-`
   - trim leading/trailing `-`
   - fall back to `"untitled"` if everything is stripped

These rules match `forme-collect-chronological/src/slug.ts` and
`forme-render-static/src/slug.ts` byte-for-byte — running this
stage produces the same routes those stages used to derive
inline.

## Route template

Currently `{slug}` is the only substitution. Future versions may
add `{year}`, `{month}`, `{section}`, etc. when downstream stages
need them.

## Status

v0.1.0 — solo stage, no downstream consumers updated yet. Wiring
this into the hello-world demo and updating `forme-render-static`
to read `node.route` are follow-up PRs.

## See also

- `@coding-adventures/forme-types` — `ContentNode` shape
- `@coding-adventures/forme-stage` — `Stage<>` contract
- `code/specs/FM00-forme-vision.md` §5.4 — the collector + routing
  story
