import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  Kinds,
  streamOf,
  type AssetRef,
  type ContentNode,
  type PageMeta,
  type RenderedPage,
} from "@coding-adventures/forme-types";
import { createOutputProvenance } from "@coding-adventures/forme-identity";
import { defineStage } from "@coding-adventures/forme-stage";
import { validateStyleDocument } from "@coding-adventures/forme-style-ir";
import { slicePerPage } from "@coding-adventures/forme-aot-css-slicer";
import { landingStyle } from "./landing-style.ts";
import {
  parseLandingModel,
  type LandingLab,
  type LandingLink,
  type LandingModel,
  type LandingPath,
  type LandingStat,
  type LandingWorkshopItem,
} from "./model.ts";

interface RenderLandingConfig {
  readonly cssPath: string;
}

const renderLanding = defineStage({
  name: "@coding-adventures/site-landing-render",
  version: "0.1.0",
  apiVersion: 1,
  description: "Render the Coding Adventures landing model with Style IR and its web layout layer.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.RenderedPage),
  capabilities: ["storage:read"],
  configSchema: {
    type: "object",
    required: ["cssPath"],
    properties: { cssPath: { type: "string" } },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = rawConfig as RenderLandingConfig;
    if (typeof config?.cssPath !== "string" || config.cssPath.length === 0) {
      throw new Error("site-landing-render: config.cssPath must be a non-empty string");
    }
    const layoutCss = await readFile(resolve(config.cssPath), "utf8");
    if (layoutCss.includes("</style")) {
      throw new Error("site-landing-render: layout CSS must not contain a closing style tag");
    }
    const validated = validateStyleDocument(landingStyle);
    if (validated.warnings.length !== 0) {
      throw new Error(`site-landing-render: Style IR warnings: ${validated.warnings.map(item => item.message).join("; ")}`);
    }
    const usedStyle = validated.document.rules.map(item => item.id);

    for await (const node of rawInput as AsyncIterable<ContentNode>) {
      ctx.cancellation.throwIfCancelled();
      if (node.route === null) {
        throw new Error(`site-landing-render: ${node.sourcePath} has no canonical route`);
      }
      const model = parseLandingModel(node.frontmatter.landing);
      const ogAsset = requireOgAsset(node.assetRefs, model.site.ogImage);
      const ogImage = assetPlaceholder(ogAsset);
      const styleArtifact = slicePerPage(
        validated.document,
        [{ id: node.route, usedRuleIds: usedStyle }],
        { activeContexts: [], scopePrefix: () => "" },
      ).artefacts.get(node.route);
      if (styleArtifact === undefined || styleArtifact.warnings.length !== 0) {
        throw new Error("site-landing-render: Style IR compilation failed");
      }
      const html = renderDocument(model, ogImage, `${styleArtifact.css}\n${layoutCss}`);
      const meta: PageMeta = {
        title: model.site.title,
        description: model.site.description,
        canonicalUrl: model.site.canonicalUrl,
        openGraph: {
          type: "website",
          title: model.site.title,
          description: model.site.shortDescription,
          url: model.site.canonicalUrl,
          image: ogImage,
        },
        structured: [],
        extra: { generator: "Forme" },
      };
      const page: RenderedPage = {
        route: node.route,
        html,
        usedStyle,
        usedIslands: [],
        usedAssets: [ogAsset.id],
        meta,
        provenance: createOutputProvenance([node]),
        source: node.identity,
      };
      yield page as never;
    }
  },
});

function requireOgAsset(refs: readonly AssetRef[], sourcePath: string): AssetRef {
  const matches = refs.filter(ref => ref.role === "image" && ref.sourcePath === sourcePath);
  if (matches.length !== 1 || matches[0] === undefined) {
    throw new Error(`site-landing-render: expected one resolved OpenGraph asset for ${JSON.stringify(sourcePath)}`);
  }
  return matches[0];
}

function assetPlaceholder(ref: AssetRef): string {
  return `forme-asset:${encodeURIComponent(ref.id)}${ref.urlSuffix ?? ""}`;
}

function renderDocument(model: LandingModel, ogImage: string, css: string): string {
  const site = model.site;
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="theme-color" content="#15231f">
  <meta name="description" content="${attr(site.description)}">
  <link rel="canonical" href="${attr(site.canonicalUrl)}">
  <meta property="og:type" content="website">
  <meta property="og:title" content="${attr(site.title)}">
  <meta property="og:description" content="${attr(site.shortDescription)}">
  <meta property="og:url" content="${attr(site.canonicalUrl)}">
  <meta property="og:image" content="${attr(ogImage)}">
  <meta property="og:image:width" content="${site.ogImageWidth}">
  <meta property="og:image:height" content="${site.ogImageHeight}">
  <meta property="og:image:alt" content="Coding Adventures — Build the stack.">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:image" content="${attr(ogImage)}">
  <meta name="generator" content="Forme">
  <title>${html(site.title)}</title>
  <style>
${css}
  </style>
</head>
<body>
  <!-- Generated by Forme. Edit data/index.landing, landing-style.ts, or landing.css. -->
  <a class="skip-link" href="#main">Skip to content</a>
  ${renderHeader(model)}
  <main id="main">
    ${renderHero(model)}
    ${renderSnapshot(model)}
    ${renderPaths(model.paths)}
    ${renderLabs(model.labs)}
    ${renderForme(model)}
    ${renderWorkshop(model.workshop)}
  </main>
  <footer class="site-footer">
    <div class="shell footer-grid">
      <p class="footer-line">${html(model.footer)}</p>
      <div class="footer-meta"><div>Generated by Forme</div><div>Updated ${html(site.updated)}</div></div>
    </div>
  </footer>
</body>
</html>
`;
}

function renderHeader(model: LandingModel): string {
  return `<header class="site-header">
    <div class="shell masthead">
      <a class="brand" href="/coding-adventures/" aria-label="Coding Adventures home">
        <span class="brand-mark" aria-hidden="true">CA</span>
        <span class="brand-copy"><span class="brand-name">coding-adventures</span><span class="brand-note">An inspectable computing lab</span></span>
      </a>
      <nav class="site-nav" aria-label="Primary navigation">
        <a href="#paths">Learning paths</a><a href="#labs">Live labs</a><a href="#forme">Forme</a>
        <a class="github-link" href="${attr(model.site.repositoryUrl)}">View source <span aria-hidden="true">↗</span></a>
      </nav>
    </div>
  </header>`;
}

function renderHero(model: LandingModel): string {
  const hero = model.hero;
  return `<section class="hero"><div class="shell hero-grid"><div>
      <p class="eyebrow">${html(hero.eyebrow)}</p>
      <h1>${html(hero.title)}<em>${html(hero.accent)}</em></h1>
      <p class="hero-intro">${html(hero.intro)}</p>
      <div class="hero-actions">${button(hero.primaryAction, "button-primary", " ↓")}${button(hero.secondaryAction, "button-secondary")}</div>
    </div><aside class="stack-map" aria-label="Computing stack map">
      <div class="map-head"><span>System map / 01</span><span>All layers live</span></div>
      <ol class="stack-list">${hero.stack.map(item => `<li><span>${html(item.label)}</span><small>${html(item.detail)}</small></li>`).join("")}</ol>
    </aside></div></section>`;
}

function renderSnapshot(model: LandingModel): string {
  return `<section class="snapshot" aria-labelledby="snapshot-title"><div class="shell">
    <h2 id="snapshot-title" class="eyebrow">Repository snapshot</h2>
    <div class="snapshot-grid">${model.snapshot.items.map(stat).join("")}</div>
    <div class="snapshot-note"><span>Measured on main · ${html(model.site.updated)}</span><span>${html(model.snapshot.note)}</span></div>
  </div></section>`;
}

function renderPaths(paths: readonly LandingPath[]): string {
  return `<section class="section trails" id="paths"><div class="shell">
    <div class="section-head"><div><p class="eyebrow">Three ways in</p><h2>Pick a thread.<br>Follow it end to end.</h2></div>
      <p class="section-lede">Each route crosses specifications, teaching material, reusable packages, tests, and a runnable surface. The destination matters; the layers are the lesson.</p></div>
    <div class="trail-grid">${paths.map(path => `<article class="trail-card"><span class="trail-number">${html(path.number)}</span><h3>${html(path.title)}</h3><p>${html(path.description)}</p><div class="trail-links">${path.links.map(link).join("")}</div></article>`).join("")}</div>
  </div></section>`;
}

function renderLabs(labs: readonly LandingLab[]): string {
  return `<section class="section labs" id="labs"><div class="shell">
    <div class="section-head"><div><p class="eyebrow">Live from the repository</p><h2>Open the machinery.</h2></div>
      <p class="section-lede">These are working builds, not screenshots. Change inputs, inspect intermediate state, and watch the repository's packages collaborate in real time.</p></div>
    <div class="lab-grid">${labs.map(lab => `<a class="lab-card${lab.featured ? " featured" : ""}" href="${attr(lab.href)}"><div class="card-top"><span>${html(lab.kicker)}</span><span class="card-arrow">↗</span></div><h3>${html(lab.title)}</h3><p>${html(lab.description)}</p><div class="tag-row">${lab.tags.map(tag => `<span class="tag">${html(tag)}</span>`).join("")}</div></a>`).join("")}</div>
  </div></section>`;
}

function renderForme(model: LandingModel): string {
  const forme = model.forme;
  return `<section class="forme" id="forme"><div class="shell forme-layout"><div>
    <p class="eyebrow">${html(forme.eyebrow)}</p><h2>${html(forme.title)}</h2><p class="forme-copy">${html(forme.description)}</p>
    <div class="forme-actions">${forme.actions.map((action, index) => button(action, index === 0 ? "button-primary" : "button-secondary")).join("")}</div>
  </div><div><ol class="pipeline">${forme.pipeline.map(item => `<li><strong>${html(item.label)}</strong><span>${html(item.detail)}</span></li>`).join("")}</ol>
    <div class="forme-status">${forme.status.map(item => `<div class="status-cell"><strong>${html(item.value)}</strong><span>${html(item.label)}</span></div>`).join("")}</div>
  </div></div></section>`;
}

function renderWorkshop(items: readonly LandingWorkshopItem[]): string {
  return `<section class="section workshop"><div class="shell">
    <div class="section-head"><div><p class="eyebrow">On the workbench</p><h2>What the lab is building now.</h2></div><p class="section-lede">The repository is active infrastructure. These are the current integration fronts on main.</p></div>
    <div class="workshop-grid">${items.map(item => `<article class="workshop-item"><span>${html(item.kicker)}</span><h3><a href="${attr(item.href)}">${html(item.title)}</a></h3><p>${html(item.description)}</p></article>`).join("")}</div>
  </div></section>`;
}

function button(action: LandingLink, className: string, suffix = ""): string {
  return `<a class="button ${className}" href="${attr(action.href)}">${html(action.label)}${suffix}</a>`;
}

function link(item: LandingLink): string {
  return `<a href="${attr(item.href)}">${html(item.label)}</a>`;
}

function stat(item: LandingStat): string {
  return `<div class="stat"><strong>${html(item.value)}</strong><span>${html(item.label)}</span></div>`;
}

function html(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

function attr(value: string): string {
  return html(value).replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}

export default renderLanding;
export { renderLanding, renderDocument };
