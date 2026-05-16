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
3. **`forme-collect-chronological`** sorted this and the other posts
   by date and assigned a route.
4. **`forme-render-static`** wrapped the rendered body in a minimal
   classless HTML5 theme and emitted a `RenderedPage`.
5. **`forme-emit-fs`** wrote the result to `dist/` and emitted a
   `DeployArtifact`.

The point isn't the post — it's that everything between the parser
and the deployer is *plug-compatible*. Want a different theme?
Replace `forme-render-static`. Want to ship to S3 instead of disk?
Replace `forme-emit-fs`. The other four stages don't change.

> One pipeline, many surfaces. That's the Forme bet.

The next few posts will work through what the kernel + orchestrator
gives you, and where the v0.2 router stage will fold the collector
back into the renderer cleanly.
