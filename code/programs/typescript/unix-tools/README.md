# unix-tools

A collection of classic POSIX Unix utilities reimplemented in TypeScript, built on top of the `cli-builder` package from this monorepo.

## Purpose

This package serves as both a practical set of command-line tools and a learning exercise in how real Unix utilities work under the hood. Each tool is implemented as a standalone TypeScript module that uses CLI Builder for argument parsing, so the source code focuses purely on business logic.

## Available Tools

### pwd

Print the absolute pathname of the current working directory.

```bash
npx tsx src/pwd.ts          # logical path (default)
npx tsx src/pwd.ts -P       # physical path (resolve symlinks)
npx tsx src/pwd.ts --help   # show usage
```

## How It Fits in the Stack

```
unix-tools (this package)
  └── cli-builder        (argument parsing, help generation)
       └── state-machine (tokenizer engine)
       └── directed-graph (dependency resolution)
```

Each tool has a corresponding `.json` spec file (e.g., `pwd.json`) that declaratively defines its CLI interface -- flags, arguments, mutual exclusivity groups, and help text. CLI Builder reads the spec and handles all parsing and validation.

## Adding a New Tool

1. Create a JSON spec file (e.g., `ls.json`) describing the CLI interface.
2. Create a TypeScript source file (e.g., `src/ls.ts`) with the business logic.
3. Add a script entry in `package.json`.
4. Update the BUILD file to run the new tool.

## Development

```bash
npm install
npx tsx src/pwd.ts
```
