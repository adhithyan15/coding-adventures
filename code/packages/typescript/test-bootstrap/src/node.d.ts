declare module "node:test" {
  export function describe(name: string, fn: () => void | Promise<void>): void;
  export function it(name: string, fn: () => void | Promise<void>): void;
  export function test(name: string, fn: () => void | Promise<void>): void;
  export function beforeEach(fn: () => void | Promise<void>): void;
  export function afterEach(fn: () => void | Promise<void>): void;
  export function before(fn: () => void | Promise<void>): void;
  export function after(fn: () => void | Promise<void>): void;

  export namespace describe {
    export function only(name: string, fn: () => void | Promise<void>): void;
    export function skip(name: string, fn: () => void | Promise<void>): void;
  }

  export namespace it {
    export function only(name: string, fn: () => void | Promise<void>): void;
    export function skip(name: string, fn: () => void | Promise<void>): void;
  }

  const nodeTest: {
    describe: typeof describe;
    it: typeof it;
    test: typeof test;
    beforeEach: typeof beforeEach;
    afterEach: typeof afterEach;
    before: typeof before;
    after: typeof after;
  };

  export default nodeTest;
}

declare module "node:fs" {
  export function readFileSync(path: string, options?: { encoding?: string; flag?: string } | string): string;
  export function existsSync(path: string): boolean;
}

declare module "node:path" {
  export function dirname(p: string): string;
  export function resolve(...paths: string[]): string;
  export function join(...paths: string[]): string;
}

declare module "node:url" {
  export function fileURLToPath(url: string | URL): string;
}

declare module "node:process" {
  export const argv: string[];
  export const env: Record<string, string | undefined>;
  export function cwd(): string;
  export function exit(code?: number): never;
}
