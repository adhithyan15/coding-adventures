import { createHash } from "node:crypto";
import { readFile, readdir, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const blogRoot = resolve(here, "dist/blog");
const reportPath = resolve(here, "dist/.forme-build-report.json");
const report = JSON.parse(await readFile(reportPath, "utf8")) as BuildReport;
const articles = report.outputs.articles;
const surface = report.outputs.surface;
if (
  report.schemaVersion !== 1 ||
  report.outcome !== "success" ||
  articles?.kind !== "DeployArtifact" ||
  surface?.kind !== "DeployArtifact"
) {
  throw new Error("Blog Forme build report is missing its successful named artifacts");
}
const expectedSurface = ["atom.xml", "index.html", "rss.xml", "sitemap.xml"];
for (const file of expectedSurface) await readFile(resolve(blogRoot, file));
const surfaceRoutes = surface.manifest.routes.map(route => route.pattern).sort();
const expectedSurfaceRoutes = expectedSurface.map(file => `/blog/${file}`).sort();
if (JSON.stringify(surfaceRoutes) !== JSON.stringify(expectedSurfaceRoutes)) {
  throw new Error(`Blog surface routes differ: ${JSON.stringify(surfaceRoutes)}`);
}
if (surface.manifest.assets.length !== 0) {
  throw new Error("Blog surface artifact must not claim article assets");
}
if (surface.files.length !== expectedSurface.length) {
  throw new Error(`Blog surface file count differs: ${surface.files.length}`);
}

const articleFiles = (await readdir(blogRoot))
  .filter(path => /^\d{4}-\d{2}-\d{2}-.+\.html$/.test(path))
  .sort();
if (articleFiles.length === 0) throw new Error("Blog must emit at least one article page");
const articleRoutes = articles.manifest.routes.map(route => route.pattern).sort();
if (
  articleRoutes.length !== articleFiles.length ||
  articleRoutes.some(route => !route.startsWith("/blog/") || !route.endsWith(".html"))
) {
  throw new Error(`Blog article routes differ: ${JSON.stringify(articleRoutes)}`);
}
if (articles.files.length !== articleFiles.length + 1) {
  throw new Error(`Blog article artifact file count differs: ${articles.files.length}`);
}

const assetFiles = (await readdir(resolve(blogRoot, "assets")))
  .filter(path => /^forme-pipeline\.[0-9a-f]{64}\.svg$/.test(path));
if (assetFiles.length !== 1 || assetFiles[0] === undefined) {
  throw new Error(`Blog must emit exactly one fingerprinted pipeline asset: ${JSON.stringify(assetFiles)}`);
}
const assetPath = `blog/assets/${assetFiles[0]}`;
const assetBytes = await readFile(resolve(here, "dist", assetPath));
const sha256 = createHash("sha256").update(assetBytes).digest("hex");
const manifestAsset = articles.manifest.assets[0];
if (
  articles.manifest.assets.length !== 1 ||
  manifestAsset === undefined ||
  manifestAsset.id !== "01952c0d-7e63-7000-8000-000000000201" ||
  manifestAsset.path !== assetPath ||
  manifestAsset.mime !== "image/svg+xml" ||
  manifestAsset.sha256 !== sha256 ||
  !assetPath.includes(sha256)
) {
  throw new Error(`Blog manifest asset differs: ${JSON.stringify(manifestAsset)}`);
}

const helloHtml = await readFile(resolve(blogRoot, "2026-05-15-hello-forme.html"), "utf8");
const expectedAssetUrl = `/coding-adventures/${assetPath}#pipeline`;
if (!helloHtml.includes(expectedAssetUrl) || helloHtml.includes("forme-asset:")) {
  throw new Error(`Blog article did not contain the rewritten asset URL ${expectedAssetUrl}`);
}

await verifyReportFiles(articles.files);
await verifyReportFiles(surface.files);
await rm(reportPath);

console.log(
  `blog verification: ${articleFiles.length} articles + ${expectedSurface.length} surface files + ${assetPath}`,
);

async function verifyReportFiles(files: readonly ReportFile[]): Promise<void> {
  for (const file of files) {
    if (file.sha256 === null) throw new Error(`Blog report did not hash ${file.path}`);
    const bytes = await readFile(resolve(here, "dist", file.path));
    const actual = createHash("sha256").update(bytes).digest("hex");
    if (actual !== file.sha256) {
      throw new Error(`Blog report and on-disk bytes differ for ${file.path}`);
    }
  }
}

interface ReportFile {
  readonly path: string;
  readonly sha256: string | null;
}

interface ReportArtifact {
  readonly kind: string;
  readonly manifest: {
    readonly routes: readonly { readonly pattern: string }[];
    readonly assets: readonly {
      readonly id: string;
      readonly path: string;
      readonly mime: string;
      readonly sha256: string;
    }[];
  };
  readonly files: readonly ReportFile[];
}

interface BuildReport {
  readonly schemaVersion: number;
  readonly outcome: string;
  readonly outputs: {
    readonly articles?: ReportArtifact;
    readonly surface?: ReportArtifact;
  };
}
