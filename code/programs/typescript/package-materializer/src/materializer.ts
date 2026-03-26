import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";

const SHELL_SKIP_FILES = new Set<string>(["package-lock.json", "tsconfig.json", "vitest.config.ts"]);

export interface MaterializeOptions {
  repoRoot: string;
  outputRoot?: string;
}

export interface MaterializationResult {
  outputDir: string;
  packageName: string;
  sharedPackages: string[];
}

function ensureDir(dir: string): void {
  mkdirSync(dir, { recursive: true });
}

function normalize(file: string): string {
  return file.split(path.sep).join("/");
}

function listFiles(dir: string): string[] {
  if (!existsSync(dir)) return [];

  const files: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...listFiles(abs));
    } else if (entry.isFile()) {
      files.push(abs);
    }
  }
  return files;
}

function isInside(child: string, parent: string): boolean {
  const rel = path.relative(parent, child);
  return rel !== "" && !rel.startsWith("..") && !path.isAbsolute(rel);
}

function resolveModule(fromFile: string, specifier: string): string | null {
  if (!specifier.startsWith(".")) return null;

  const absolute = path.resolve(path.dirname(fromFile), specifier);
  const candidates = [absolute];

  if (specifier.endsWith(".js")) {
    candidates.push(absolute.replace(/\.js$/, ".ts"));
  }

  if (!path.extname(absolute)) {
    candidates.push(`${absolute}.ts`);
    candidates.push(path.join(absolute, "index.ts"));
  }

  for (const candidate of candidates) {
    if (existsSync(candidate) && statSync(candidate).isFile()) {
      return candidate;
    }
  }

  return null;
}

function parseSpecifiers(sourceText: string): string[] {
  const specifiers = new Set<string>();
  const patterns = [
    /\bimport\s+(?:[^"'`]+\s+from\s+)?["']([^"']+)["']/g,
    /\bexport\s+[^"'`]*\s+from\s+["']([^"']+)["']/g,
  ];

  for (const pattern of patterns) {
    for (const match of sourceText.matchAll(pattern)) {
      specifiers.add(match[1]);
    }
  }

  return Array.from(specifiers);
}

function sharedRoots(repoRoot: string) {
  return {
    sharedTypescriptRoot: path.join(repoRoot, "code", "src", "typescript"),
    sharedTokensRoot: path.join(repoRoot, "code", "src", "tokens"),
    shellTypescriptRoot: path.join(repoRoot, "code", "packages", "typescript"),
  };
}

function packageNameForFile(sharedTypescriptRoot: string, file: string): string {
  const rel = path.relative(sharedTypescriptRoot, file);
  return rel.split(path.sep)[0];
}

export function collectPackageClosure(packageName: string, repoRoot: string): string[] {
  const { sharedTypescriptRoot } = sharedRoots(repoRoot);
  const visited = new Set<string>();
  const queue: string[] = [packageName];

  while (queue.length > 0) {
    const current = queue.shift();
    if (!current || visited.has(current)) continue;

    const packageRoot = path.join(sharedTypescriptRoot, current);
    if (!existsSync(packageRoot)) {
      throw new Error(`Shared source package not found: ${current}`);
    }

    visited.add(current);

    for (const file of listFiles(packageRoot)) {
      if (!file.endsWith(".ts")) continue;

      const sourceText = readFileSync(file, "utf8");
      for (const specifier of parseSpecifiers(sourceText)) {
        const resolved = resolveModule(file, specifier);
        if (!resolved) continue;
        if (!isInside(resolved, sharedTypescriptRoot)) continue;

        const dependency = packageNameForFile(sharedTypescriptRoot, resolved);
        if (dependency !== current && !visited.has(dependency)) {
          queue.push(dependency);
        }
      }
    }
  }

  return Array.from(visited).sort();
}

function copyShellMetadata(shellPackageDir: string, outputDir: string): void {
  for (const entry of readdirSync(shellPackageDir, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    if (SHELL_SKIP_FILES.has(entry.name)) continue;

    cpSync(path.join(shellPackageDir, entry.name), path.join(outputDir, entry.name));
  }
}

function sanitizePackageManifest(packageFile: string): void {
  const manifest = JSON.parse(readFileSync(packageFile, "utf8")) as Record<string, unknown>;
  delete manifest.dependencies;
  delete manifest.devDependencies;
  writeFileSync(packageFile, `${JSON.stringify(manifest, null, 2)}\n`);
}

function copyDirectory(sourceDir: string, destinationDir: string): void {
  ensureDir(path.dirname(destinationDir));
  cpSync(sourceDir, destinationDir, { recursive: true });
}

function generateTargetWrappers(packageName: string, sourcePackageDir: string, outputSrcDir: string): void {
  for (const file of listFiles(sourcePackageDir)) {
    if (!file.endsWith(".ts")) continue;

    const rel = path.relative(sourcePackageDir, file);
    const wrapperPath = path.join(outputSrcDir, rel);
    ensureDir(path.dirname(wrapperPath));

    const targetSpecifier = normalize(path.join("typescript", packageName, rel)).replace(/\.ts$/, ".js");
    const relativeSpecifier = normalize(path.relative(path.dirname(wrapperPath), path.join(outputSrcDir, targetSpecifier)));
    const specifier = relativeSpecifier.startsWith(".") ? relativeSpecifier : `./${relativeSpecifier}`;
    writeFileSync(wrapperPath, `export * from ${JSON.stringify(specifier)};\n`);
  }
}

export function materializePackage(packageName: string, options: MaterializeOptions): MaterializationResult {
  const { repoRoot } = options;
  const { sharedTypescriptRoot, sharedTokensRoot, shellTypescriptRoot } = sharedRoots(repoRoot);

  const shellPackageDir = path.join(shellTypescriptRoot, packageName);
  const sourcePackageDir = path.join(sharedTypescriptRoot, packageName);
  const outputRoot = options.outputRoot ?? path.join(repoRoot, ".out", "publish", "typescript");
  const outputDir = path.join(outputRoot, packageName);

  if (!existsSync(shellPackageDir)) {
    throw new Error(`Shell package not found: ${packageName}`);
  }
  if (!existsSync(sourcePackageDir)) {
    throw new Error(`Shared source package not found: ${packageName}`);
  }

  const closure = collectPackageClosure(packageName, repoRoot);

  rmSync(outputDir, { recursive: true, force: true });
  ensureDir(outputDir);
  ensureDir(path.join(outputDir, "src"));

  copyShellMetadata(shellPackageDir, outputDir);
  generateTargetWrappers(packageName, sourcePackageDir, path.join(outputDir, "src"));

  for (const dependency of closure) {
    copyDirectory(
      path.join(sharedTypescriptRoot, dependency),
      path.join(outputDir, "src", "typescript", dependency),
    );
  }

  if (existsSync(sharedTokensRoot)) {
    copyDirectory(sharedTokensRoot, path.join(outputDir, "src", "tokens"));
  }

  const packageFile = path.join(outputDir, "package.json");
  if (existsSync(packageFile)) {
    sanitizePackageManifest(packageFile);
  }

  writeFileSync(
    path.join(outputDir, "materialization.json"),
    `${JSON.stringify(
      {
        language: "typescript",
        package: packageName,
        shared_packages: closure,
        copied_tokens_dir: existsSync(sharedTokensRoot) ? "src/tokens" : null,
      },
      null,
      2,
    )}\n`,
  );

  return {
    outputDir,
    packageName,
    sharedPackages: closure,
  };
}
