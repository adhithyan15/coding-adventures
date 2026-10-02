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

<div id="forme-pipeline-steps">
  <p>The complete pipeline is listed below. With JavaScript enabled, optional controls can focus one step at a time.</p>
</div>

1. **`forme-source-fs`** walked `data/`, found this file, and emitted a
   `ContentSource`.
2. **`forme-parse-markdown`** split off the frontmatter you see above,
   parsed the body as GFM, and emitted a `ContentNode`.
3. **`blog-attach-interactivity`** attached the one reviewed pipeline-step
   module to this post and left every other post without a script asset.
4. **`forme-resolve-asset-refs-fs`** discovered the local diagram below,
   assigned its stable logical identity, and preserved its fragment target.
5. **`forme-router`** assigned one canonical route, then fanned the
   routed node out to the page and collection branches.
6. **`forme-collect-chronological`** sorted this and the other posts
   by date while preserving that canonical route.
7. **`forme-render-static`** matched the reusable classless Style IR theme,
   recorded `usedStyle`, compiled the page slice through the AOT path, and
   emitted a `RenderedPage` with exact island usage and asset placeholders.
8. **`forme-load-assets-fs`** loaded and hashed the SVG and JavaScript bytes
   while enforcing canonical storage-root containment.
9. **`blog-surface`** rendered the chronological collection into the blog
   index, RSS, Atom, and sitemap artifacts.
10. **`forme-emit-site-fs`** joined the rendered page and asset streams,
    replaced placeholders with fingerprinted public URLs, emitted only the
    selected script, and recorded the article `DeployArtifact` manifest.
11. **`forme-emit-fs`** wrote the index, feeds, and sitemap through the
    independent surface branch.

![Forme turns source content into reusable IR and many output surfaces.](assets/forme-pipeline.svg#pipeline)

The point isn't the post — it's that everything between the parser
and the deployer is *plug-compatible*. Want a different theme?
Pass a different resolved `StyleDocument`; the renderer does not change.
Want to ship to S3 instead of disk? Replace `forme-emit-site-fs`. The rest of
the DAG stays put.

> One pipeline, many surfaces. That's the Forme bet.

The next few posts will work through what the kernel + orchestrator
gives you, and how collection outputs grow into indexes and feeds.
