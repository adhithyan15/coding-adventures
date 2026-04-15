import { execFileSync } from "node:child_process";

export const CI_WORKFLOW_PATH = ".github/workflows/ci.yml";

export type CIWorkflowChange = {
  toolchains: Set<string>;
  requiresFullRebuild: boolean;
};

const TOOLCHAIN_MARKERS: Record<string, string[]> = {
  python: [
    "needs_python", "setup-python", "python-version", "setup-uv",
    "python --version", "uv --version", "pytest",
    "set up python", "install uv",
  ],
  ruby: [
    "needs_ruby", "setup-ruby", "ruby-version", "bundler",
    "gem install bundler", "ruby --version", "bundle --version",
    "set up ruby", "install bundler",
  ],
  go: ["needs_go", "setup-go", "go-version", "go version", "set up go"],
  typescript: [
    "needs_typescript", "setup-node", "node-version", "npm install -g jest",
    "node --version", "npm --version", "set up node",
  ],
  rust: [
    "needs_rust", "rust-toolchain", "cargo", "rustc", "tarpaulin",
    "wasm32-unknown-unknown", "set up rust", "install cargo-tarpaulin",
  ],
  elixir: [
    "needs_elixir", "setup-beam", "elixir-version", "otp-version",
    "elixir --version", "mix --version", "set up elixir",
  ],
  lua: [
    "needs_lua", "gh-actions-lua", "gh-actions-luarocks", "luarocks",
    "lua -v", "msvc", "set up lua", "set up luarocks",
  ],
  perl: ["needs_perl", "cpanm", "perl --version", "install cpanm"],
  haskell: [
    "needs_haskell", "haskell-actions/setup", "ghc-version", "cabal-version",
    "ghc --version", "cabal --version", "set up haskell",
  ],
  java: [
    "needs_java", "setup-java", "java-version", "java --version",
    "temurin", "set up jdk", "set up gradle", "setup-gradle",
    "disable long-lived gradle services",
    "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
  ],
  kotlin: [
    "needs_kotlin", "setup-java", "java-version",
    "temurin", "set up jdk", "set up gradle", "setup-gradle",
    "disable long-lived gradle services",
    "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
  ],
  dotnet: [
    "needs_dotnet", "setup-dotnet", "dotnet-version", "dotnet --version",
    "set up .net",
  ],
};

const UNSAFE_MARKERS = [
  "./build-tool",
  "build-tool.exe",
  "-detect-languages",
  "-emit-plan",
  "-force",
  "-plan-file",
  "-validate-build-files",
  "actions/checkout",
  "build-plan",
  "cancel-in-progress:",
  "concurrency:",
  "diff-base",
  "download-artifact",
  "event_name",
  "fetch-depth",
  "git fetch origin main",
  "git_ref",
  "is_main",
  "matrix:",
  "permissions:",
  "pr_base_ref",
  "pull_request:",
  "push:",
  "runs-on:",
  "strategy:",
  "upload-artifact",
];

export function analyzeCIWorkflowChanges(root: string, diffBase: string): CIWorkflowChange {
  return analyzeCIWorkflowPatch(getFileDiff(root, diffBase, CI_WORKFLOW_PATH));
}

export function analyzeCIWorkflowPatch(patch: string): CIWorkflowChange {
  const toolchains = new Set<string>();
  let hunk: string[] = [];

  const flush = (): CIWorkflowChange | null => {
    const { toolchains: hunkToolchains, unsafe } = classifyHunk(hunk);
    hunk = [];
    if (unsafe) {
      return { toolchains: new Set<string>(), requiresFullRebuild: true };
    }
    for (const toolchain of hunkToolchains) {
      toolchains.add(toolchain);
    }
    return null;
  };

  for (const line of patch.split("\n")) {
    if (line.startsWith("@@")) {
      const result = flush();
      if (result) {
        return result;
      }
      continue;
    }

    if (
      line.startsWith("diff --git ") ||
      line.startsWith("index ") ||
      line.startsWith("--- ") ||
      line.startsWith("+++ ")
    ) {
      continue;
    }

    hunk.push(line);
  }

  const result = flush();
  if (result) {
    return result;
  }
  return { toolchains, requiresFullRebuild: false };
}

export function sortedToolchains(toolchains: Set<string>): string[] {
  return Array.from(toolchains).sort();
}

function classifyHunk(lines: string[]): { toolchains: Set<string>; unsafe: boolean } {
  const hunkToolchains = new Set<string>();
  const changedToolchains = new Set<string>();
  const changedLines: string[] = [];

  for (const line of lines) {
    if (line.length === 0 || !isDiffLine(line)) {
      continue;
    }

    const content = line.slice(1).trim();
    for (const toolchain of detectToolchains(content)) {
      hunkToolchains.add(toolchain);
    }

    if (!isChangedLine(line)) {
      continue;
    }
    if (content.length === 0 || content.startsWith("#")) {
      continue;
    }

    changedLines.push(content);
    for (const toolchain of detectToolchains(content)) {
      changedToolchains.add(toolchain);
    }
  }

  if (changedLines.length === 0) {
    return { toolchains: new Set<string>(), unsafe: false };
  }

  let resolvedToolchains = changedToolchains;
  if (resolvedToolchains.size === 0) {
    if (hunkToolchains.size !== 1) {
      return { toolchains: new Set<string>(), unsafe: true };
    }
    resolvedToolchains = hunkToolchains;
  }

  for (const content of changedLines) {
    if (touchesSharedCIBehavior(content)) {
      return { toolchains: new Set<string>(), unsafe: true };
    }
    if (detectToolchains(content).size > 0) {
      continue;
    }
    if (isToolchainScopedStructuralLine(content)) {
      continue;
    }
    return { toolchains: new Set<string>(), unsafe: true };
  }

  return { toolchains: resolvedToolchains, unsafe: false };
}

function detectToolchains(content: string): Set<string> {
  const found = new Set<string>();
  const normalized = content.toLowerCase();

  for (const [toolchain, markers] of Object.entries(TOOLCHAIN_MARKERS)) {
    if (markers.some((marker) => normalized.includes(marker))) {
      found.add(toolchain);
    }
  }

  return found;
}

function touchesSharedCIBehavior(content: string): boolean {
  const normalized = content.toLowerCase();
  return UNSAFE_MARKERS.some((marker) => normalized.includes(marker));
}

function isToolchainScopedStructuralLine(content: string): boolean {
  return [
    "if:",
    "run:",
    "shell:",
    "with:",
    "env:",
    "{",
    "}",
    "else",
    "fi",
    "then",
    "printf ",
    "echo ",
    "curl ",
    "powershell ",
    "call ",
    "cd ",
  ].some((prefix) => content.startsWith(prefix));
}

function isDiffLine(line: string): boolean {
  return line.startsWith(" ") || isChangedLine(line);
}

function isChangedLine(line: string): boolean {
  return line.startsWith("+") || line.startsWith("-");
}

function getFileDiff(root: string, diffBase: string, relativePath: string): string {
  for (const args of [
    ["diff", "--unified=0", `${diffBase}...HEAD`, "--", relativePath],
    ["diff", "--unified=0", diffBase, "HEAD", "--", relativePath],
  ]) {
    try {
      return execFileSync("git", args, { cwd: root, encoding: "utf-8" });
    } catch {
      // Try the next diff form.
    }
  }
  return "";
}
