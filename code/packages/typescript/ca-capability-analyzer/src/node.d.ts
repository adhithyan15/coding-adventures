/**
 * Ambient type declarations for Node.js built-ins.
 * Allows zero-external-dependency compilation without @types/node.
 */

declare module "node:fs" {
  export interface Dirent {
    name: string;
    isFile(): boolean;
    isDirectory(): boolean;
    isSymbolicLink(): boolean;
  }

  export interface Stats {
    size: number;
    isFile(): boolean;
    isDirectory(): boolean;
    isSymbolicLink(): boolean;
  }

  export function readFileSync(path: string, options?: { encoding?: string; flag?: string } | string): string;
  export function readdirSync(path: string, options?: { withFileTypes?: boolean }): any[];
  export function statSync(path: string): Stats;
  export function lstatSync(path: string): Stats;
  export function existsSync(path: string): boolean;
}

declare module "node:path" {
  export const sep: string;
  export function dirname(p: string): string;
  export function resolve(...paths: string[]): string;
  export function join(...paths: string[]): string;
  export function relative(from: string, to: string): string;
  export function extname(p: string): string;
}

declare module "node:process" {
  export const argv: string[];
  export const env: Record<string, string | undefined>;
  export function exit(code?: number): never;
}

declare module "node:url" {
  export function fileURLToPath(url: string | URL): string;
}
