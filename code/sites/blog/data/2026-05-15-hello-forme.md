---
title: Hello, Forme
date: 2026-05-15
excerpt: The first post written through the Forme pipeline end-to-end.
---

# Hello, Forme

This is the inaugural post of the **Coding Adventures blog**, and the
first piece of content shipped end-to-end through the
[Forme](https://github.com/adhithyan15/coding-adventures) universal
authoring pipeline.

The path it took from `data/2026-05-15-hello-forme.md` to
`/coding-adventures/blog/2026-05-15-hello-forme.html` is exactly the
shape laid out in the FM00 spec:

1. **`forme-source-fs`** walked `data/`, found this file, and emitted a
   `ContentSource`.
2. **`forme-parse-markdown`** split off the frontmatter you see above,
   parsed the body as GFM, and emitted a `ContentNode`.
3. **`forme-resolve-asset-refs-fs`** discovered the local diagram below,
   assigned its stable logical identity, and preserved its fragment target.
4. **`forme-router`** assigned one canonical route, then fanned the
   routed node out to the page and collection branches.
5. **`forme-collect-chronological`** sorted this and the other posts
   by date while preserving that canonical route.
6. **`forme-render-static`** matched the reusable classless Style IR theme,
   recorded `usedStyle`, compiled the page slice through the AOT path, and
   emitted a `RenderedPage` with an asset placeholder.
7. **`forme-load-assets-fs`** loaded and hashed the SVG bytes while enforcing
   canonical storage-root containment.
8. **`forme-emit-site-fs`** joined the rendered page and asset streams,
   replaced the placeholder with a fingerprinted public URL, copied the SVG,
   and recorded it in the `DeployArtifact` manifest.

![Forme turns source content into reusable IR and many output surfaces.](assets/forme-pipeline.svg#pipeline)

The point isn't the post — it's that everything between the parser
and the deployer is *plug-compatible*. Want a different theme?
Pass a different resolved `StyleDocument`; the renderer does not change.
Want to ship to S3 instead of disk? Replace `forme-emit-site-fs`. The rest of
the DAG stays put.

> One pipeline, many surfaces. That's the Forme bet.

The next few posts will work through what the kernel + orchestrator
gives you, and how collection outputs grow into indexes and feeds.
