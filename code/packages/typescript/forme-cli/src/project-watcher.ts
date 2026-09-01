import { watch as nodeWatch, type WatchEventType } from "node:fs";
import { isAbsolute, relative, resolve } from "node:path";

export interface ProjectChange {
  readonly eventType: WatchEventType;
  readonly path: string | null;
}

export interface FsWatcherLike {
  close(): void;
  on(event: "error", listener: (error: Error) => void): FsWatcherLike;
}

export type FsWatchFactory = (
  root: string,
  options: { readonly recursive: true },
  listener: (eventType: WatchEventType, filename: string | Buffer | null) => void,
) => FsWatcherLike;

/**
 * Adapt the host filesystem watcher to the AsyncIterable consumed by FM03.
 * Output/cache directories are absolute, containment-checked ignore roots.
 */
export function watchProject(
  projectRoot: string,
  ignoredPaths: readonly string[],
  factory: FsWatchFactory = nodeWatch,
): AsyncIterable<ProjectChange> {
  const ignored = ignoredPaths.map(path => {
    const absolute = isAbsolute(path) ? path : resolve(projectRoot, path);
    if (!contained(projectRoot, absolute)) {
      throw new Error(`watch ignore path ${JSON.stringify(path)} is outside the project`);
    }
    return absolute;
  });
  return new ProjectChangeStream(projectRoot, ignored, factory);
}

class ProjectChangeStream implements AsyncIterable<ProjectChange>, AsyncIterator<ProjectChange> {
  private readonly queued: ProjectChange[] = [];
  private readonly readers: Array<{
    resolve(value: IteratorResult<ProjectChange>): void;
    reject(error: Error): void;
  }> = [];
  private readonly watcher: FsWatcherLike;
  private closed = false;
  private error: Error | null = null;

  constructor(
    private readonly root: string,
    private readonly ignored: readonly string[],
    factory: FsWatchFactory,
  ) {
    this.watcher = factory(root, { recursive: true }, (eventType, filename) => {
      if (this.closed) return;
      const path = filename === null ? null : resolve(root, filename.toString());
      if (path !== null && this.ignored.some(ignore => path === ignore || contained(ignore, path))) return;
      this.push({ eventType, path });
    });
    this.watcher.on("error", error => this.fail(error));
  }

  [Symbol.asyncIterator](): AsyncIterator<ProjectChange> { return this; }

  next(): Promise<IteratorResult<ProjectChange>> {
    const value = this.queued.shift();
    if (value !== undefined) return Promise.resolve({ done: false, value });
    if (this.error !== null) return Promise.reject(this.error);
    if (this.closed) return Promise.resolve({ done: true, value: undefined });
    return new Promise((resolve, reject) => this.readers.push({ resolve, reject }));
  }

  return(): Promise<IteratorResult<ProjectChange>> {
    if (!this.closed) this.watcher.close();
    this.closed = true;
    for (const reader of this.readers.splice(0)) reader.resolve({ done: true, value: undefined });
    return Promise.resolve({ done: true, value: undefined });
  }

  private push(value: ProjectChange): void {
    const reader = this.readers.shift();
    if (reader !== undefined) reader.resolve({ done: false, value });
    else this.queued.push(value);
  }

  private fail(error: Error): void {
    this.error = error;
    this.closed = true;
    this.watcher.close();
    for (const reader of this.readers.splice(0)) reader.reject(error);
  }
}

function contained(root: string, path: string): boolean {
  const rel = relative(root, path);
  return rel === "" || rel === "." || (!rel.startsWith("../") && !rel.startsWith("..\\") && !isAbsolute(rel));
}
