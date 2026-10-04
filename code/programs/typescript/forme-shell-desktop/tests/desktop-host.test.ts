import { describe, expect, it } from "vitest";
import { createDesktopHost, type DesktopBridge } from "../src/desktop-host.js";

function bridge(): DesktopBridge & { readonly calls: Array<{ command: string; arguments_: unknown }> } {
  let bytesBase64: string | null = null;
  let revision: string | null = null;
  let identity = 0;
  let target: { targetId: string; label: string; destination: string } | null = null;
  const calls: Array<{ command: string; arguments_: unknown }> = [];
  return {
    previewUrl: "http://127.0.0.1:43123/",
    calls,
    async invoke<T>(command: string, arguments_?: unknown): Promise<T> {
      calls.push({ command, arguments_ });
      if (command === "project_load") {
        return (bytesBase64 === null ? null : { bytesBase64, revision }) as T;
      }
      if (command === "project_compare_and_swap") {
        const request = (arguments_ as { request: { expectedRevision: string | null; bytesBase64: string } }).request;
        expect(request.expectedRevision).toBe(revision);
        bytesBase64 = request.bytesBase64;
        revision = `revision-${calls.filter((call) => call.command === command).length}`;
        return { revision } as T;
      }
      if (command === "identity_create") {
        identity += 1;
        return `01952c0d-7e63-7000-8000-${String(identity).padStart(12, "0")}` as T;
      }
      if (command === "preview_build") {
        const request = (arguments_ as { request: { revision: string } }).request;
        return {
          outcome: "ready",
          revision: request.revision,
          buildId: "build-1",
          diagnostics: [],
        } as T;
      }
      if (command === "target_list") return (target === null ? [] : [target]) as T;
      if (command === "target_configure") {
        target = { targetId: "local-target-1", label: "Local folder", destination: "Local folder “site”" };
        return target as T;
      }
      if (command === "target_publish") {
        const request = (arguments_ as {
          request: { targetId: string; revision: string };
        }).request;
        return {
          outcome: "published",
          revision: request.revision,
          manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
          targetId: request.targetId,
          diagnostics: [],
        } as T;
      }
      if (command === "workspace_dispose") return undefined as T;
      throw new Error(`unexpected command ${command}`);
    },
  };
}

describe("Tauri authoring host adapter", () => {
  it("opens an empty native profile as first-run without creating state", async () => {
    const native = bridge();
    const host = createDesktopHost(native);
    expect(await host.open(new AbortController().signal)).toBeNull();
    expect(native.calls.map((call) => call.command)).toEqual(["project_load"]);
  });

  it("creates a site and first draft through native identity and CAS commands", async () => {
    const native = bridge();
    const host = createDesktopHost(native);
    const workspace = await host.create(
      { title: "My first site", themeId: "forme-classless" },
      new AbortController().signal,
    );
    expect(workspace.previewUrl).toBe(native.previewUrl);
    expect(workspace.session.project).toMatchObject({
      title: "My first site",
      activeDocumentId: "01952c0d-7e63-7000-8000-000000000002",
      documents: [{
        id: "01952c0d-7e63-7000-8000-000000000002",
        slug: "welcome",
        title: "Welcome",
        status: "draft",
      }],
    });
    expect(native.calls.filter((call) => call.command === "identity_create")).toHaveLength(2);
    expect(native.calls.filter((call) => call.command === "project_compare_and_swap")).toHaveLength(2);
    expect(workspace.publishers.map((publisher) => publisher.target)).toEqual([{
      targetId: "local-target-1",
      label: "Local folder",
      destination: "Local folder “site”",
    }]);

    const attempt = await workspace.preview.request(workspace.session);
    expect(attempt).toEqual({
      outcome: "ready",
      revision: workspace.session.storageRevision,
      buildId: "build-1",
      diagnostics: [],
    });
    const previewCall = native.calls.find((call) => call.command === "preview_build");
    expect(previewCall?.arguments_).toEqual({
      request: {
        revision: workspace.session.storageRevision,
      },
    });

    const published = await workspace.publishers[0]!.publish(workspace.session);
    expect(published).toEqual({
      outcome: "published",
      revision: "revision-2",
      manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      targetId: "local-target-1",
      diagnostics: [],
    });
    expect(workspace.session.project.documents[0]?.status).toBe("published");
    expect(workspace.session.project.workflow.lastPublication).toEqual({
      authoringRevision: "revision-2",
      manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      targetId: "local-target-1",
    });

    await workspace.dispose();
    expect(native.calls.at(-1)?.command).toBe("workspace_dispose");
    await workspace.dispose();
    expect(native.calls.filter((call) => call.command === "workspace_dispose")).toHaveLength(1);
    await expect(workspace.preview.request(workspace.session)).rejects.toThrow(/unavailable/);
    await expect(workspace.createDocumentIdentity()).rejects.toThrow(/unavailable/);
  });

  it("reopens the durable first draft from native bytes", async () => {
    const native = bridge();
    const host = createDesktopHost(native);
    const created = await host.create(
      { title: "Durable site", themeId: "forme-classless" },
      new AbortController().signal,
    );
    await created.dispose();
    const reopened = await host.open(new AbortController().signal);
    expect(reopened?.session.project.title).toBe("Durable site");
    expect(reopened?.session.project.documents).toHaveLength(1);
  });

  it("keeps a known pre-commit publication failure retryable", async () => {
    const native = bridge();
    const invoke = native.invoke.bind(native);
    native.invoke = async <T>(command: string, arguments_?: unknown): Promise<T> => {
      if (command !== "target_publish") return await invoke<T>(command, arguments_);
      native.calls.push({ command, arguments_ });
      const request = (arguments_ as { request: { targetId: string; revision: string } }).request;
      return {
        outcome: "failed",
        revision: request.revision,
        manifestSha256: null,
        targetId: request.targetId,
        diagnostics: [{
          severity: "error",
          code: "E_BUILD_FAILED",
          message: "Publication did not commit and may be retried.",
        }],
      } as T;
    };
    const workspace = await createDesktopHost(native).create(
      { title: "Retry site", themeId: "forme-classless" },
      new AbortController().signal,
    );

    for (let attempt = 0; attempt < 2; attempt += 1) {
      await expect(workspace.publishers[0]!.publish(workspace.session)).resolves.toMatchObject({
        outcome: "failed",
        manifestSha256: null,
        diagnostics: [{ code: "E_BUILD_FAILED" }],
      });
    }
    expect(native.calls.filter((call) => call.command === "target_publish")).toHaveLength(2);
    await workspace.dispose();
  });

  it("makes an indeterminate native settlement sticky without retrying authority", async () => {
    const native = bridge();
    const invoke = native.invoke.bind(native);
    native.invoke = async <T>(command: string, arguments_?: unknown): Promise<T> => {
      if (command !== "target_publish") return await invoke<T>(command, arguments_);
      native.calls.push({ command, arguments_ });
      const request = (arguments_ as { request: { targetId: string; revision: string } }).request;
      return {
        outcome: "indeterminate",
        revision: request.revision,
        manifestSha256: null,
        targetId: request.targetId,
        diagnostics: [{
          severity: "error",
          code: "E_PUBLISH_RECONCILE",
          message: "Publication state must be reconciled before retrying.",
        }],
      } as T;
    };
    const workspace = await createDesktopHost(native).create(
      { title: "Poison site", themeId: "forme-classless" },
      new AbortController().signal,
    );

    const first = await workspace.publishers[0]!.publish(workspace.session);
    const second = await workspace.publishers[0]!.publish(workspace.session);
    expect(first.outcome).toBe("indeterminate");
    expect(second).toMatchObject({
      outcome: "indeterminate",
      diagnostics: [{ code: "E_PUBLISH_RECONCILE" }],
    });
    expect(native.calls.filter((call) => call.command === "target_publish")).toHaveLength(1);
    await workspace.dispose();
  });

  it("waits for active publication retirement and treats a late response as indeterminate", async () => {
    const native = bridge();
    const invoke = native.invoke.bind(native);
    let resolvePublish!: (value: unknown) => void;
    const deferred = new Promise<unknown>((resolve) => { resolvePublish = resolve; });
    native.invoke = async <T>(command: string, arguments_?: unknown): Promise<T> => {
      if (command !== "target_publish") return await invoke<T>(command, arguments_);
      native.calls.push({ command, arguments_ });
      return await deferred as T;
    };
    const workspace = await createDesktopHost(native).create(
      { title: "Retirement site", themeId: "forme-classless" },
      new AbortController().signal,
    );
    const publication = workspace.publishers[0]!.publish(workspace.session);
    await Promise.resolve();
    const busy = await workspace.publishers[0]!.publish(workspace.session);
    expect(busy).toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PUBLISH_BUSY" }] });
    const disposal = workspace.dispose();
    await Promise.resolve();
    expect(native.calls.some((call) => call.command === "workspace_dispose")).toBe(false);
    resolvePublish({});

    await expect(publication).resolves.toMatchObject({
      outcome: "indeterminate",
      diagnostics: [{ code: "E_PUBLISH_RECONCILE" }],
    });
    await disposal;
    expect(native.calls.at(-1)?.command).toBe("workspace_dispose");
  });
});
