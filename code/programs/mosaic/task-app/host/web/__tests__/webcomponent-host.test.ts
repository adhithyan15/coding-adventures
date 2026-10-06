import { readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { afterAll, describe, expect, it } from "vitest";

import { makeController } from "../src/controller";
import { bootWebComponentHost, createTaskAppWebComponentHost } from "../src/webcomponent";
// The dependency-free accessor is generated source and intentionally untyped.
// @ts-expect-error JavaScript ABI accessor has no source declaration file.
import { createTaskEngine } from "../../../../../../packages/rust/task-wasm/js/task-engine.mjs";

interface PersistedState {
  snapshot: string;
  order: string[];
  counter: number;
  activeProject?: string;
}

const settle = async () => {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
};

const buttonByText = (scope: ShadowRoot, text: string): HTMLButtonElement => {
  const button = [...scope.querySelectorAll("button")].find(
    (candidate) => candidate.textContent?.trim() === text,
  );
  expect(button, `${text} button`).toBeDefined();
  return button!;
};

afterAll(() => {
  document.body.replaceChildren();
  delete window.mosaicHost;
});

describe("TaskApp Web Components parity host", () => {
  it("keeps a startup failure visible instead of revealing an inert shell", async () => {
    document.body.innerHTML = `
      <p id="startup-status" role="status"></p>
      <mos-task-app></mos-task-app>
    `;

    await bootWebComponentHost({
      fetchWasm: async () => {
        throw new Error("acceptance engine failure");
      },
      theme: "light",
    });

    expect(document.body.dataset.webcomponentState).toBe("failed");
    expect(document.getElementById("startup-status")?.textContent).toContain(
      "acceptance engine failure",
    );
  });

  it("drives create, complete, restore, and delete through the emitted controls", async () => {
    const wasmPath = path.resolve(
      process.cwd(),
      "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm",
    );
    const wasm = await readFile(wasmPath);
    let persisted: PersistedState | undefined;

    const makeHost = (seed?: PersistedState) => {
      const engine = createTaskEngine(wasm);
      if (seed) {
        engine.load(seed.snapshot);
        if (seed.activeProject) engine.setActiveProject({ id: seed.activeProject });
      }
      const controller = makeController(engine, {
        initialOrder: seed?.order,
        initialCounter: seed?.counter,
        today: 20_000,
        onMutate: (snapshot, order, counter, activeProject) => {
          persisted = { snapshot, order: [...order], counter, activeProject };
        },
      });
      return createTaskAppWebComponentHost({
        controller,
        storageSession: {
          status: "Saved locally on this device",
          location: "Web Components acceptance storage",
          warning: "",
        },
        theme: "light",
      });
    };

    document.body.innerHTML = "<mos-task-app></mos-task-app>";
    window.mosaicHost = makeHost();

    const generated = path.resolve("webcomponent/generated/light");
    const componentUrl = pathToFileURL(path.join(generated, "TaskApp.js"));
    componentUrl.searchParams.set("acceptance", String(Date.now()));
    const runtimeUrl = pathToFileURL(path.join(generated, "main.js"));
    runtimeUrl.searchParams.set("acceptance", String(Date.now()));
    await import(/* @vite-ignore */ componentUrl.href);
    await import(/* @vite-ignore */ runtimeUrl.href);
    await settle();

    const root = document.querySelector("mos-task-app") as HTMLElement & {
      shadowRoot: ShadowRoot;
    };
    expect(root.shadowRoot).toBeTruthy();

    const name = root.shadowRoot.querySelector<HTMLInputElement>(
      'input[placeholder="What needs doing?"]',
    );
    expect(name).toBeTruthy();
    name!.value = "Ship the Web Component gate";
    name!.dispatchEvent(new Event("change", { bubbles: true }));
    buttonByText(root.shadowRoot, "Add task").click();
    await settle();

    expect(root.shadowRoot.textContent).toContain("Ship the Web Component gate");
    const complete = root.shadowRoot.querySelector<HTMLButtonElement>(
      'button[aria-label="Complete task: Ship the Web Component gate"]',
    );
    expect(complete).toBeTruthy();
    complete!.click();
    await settle();
    expect(
      root.shadowRoot.querySelector(
        'button[aria-label="Reopen task: Ship the Web Component gate"]',
      ),
    ).toBeTruthy();
    expect(persisted).toBeDefined();

    window.mosaicHost = makeHost(persisted);
    window.dispatchEvent(new CustomEvent("mosaic-host-ready"));
    await settle();
    expect(root.shadowRoot.textContent).toContain("Ship the Web Component gate");
    expect(
      root.shadowRoot.querySelector(
        'button[aria-label="Reopen task: Ship the Web Component gate"]',
      ),
    ).toBeTruthy();

    buttonByText(root.shadowRoot, "Delete").click();
    await settle();
    expect(root.shadowRoot.textContent).not.toContain("Ship the Web Component gate");
    // The whole lifecycle -- instantiating the WASM engine twice, three
    // round-trips through the emitted controls (add, complete, delete) and a
    // restore -- takes 3.4 to 4.9 seconds on a CI runner, right at vitest's
    // 5 s default, which timed it out once at 5047 ms. Its own budget leaves
    // real headroom; a hang still fails, just later.
  }, 30_000);
});
