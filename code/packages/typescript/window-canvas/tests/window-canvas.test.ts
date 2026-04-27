import { describe, expect, it } from "vitest";
import {
  LogicalSize,
  MountTargets,
  SurfacePreference,
  WindowBuilder,
} from "@coding-adventures/window-core";
import {
  CanvasBackend,
  CanvasDocumentLike,
  CanvasElementLike,
  CanvasEnvironment,
  CanvasMountHostLike,
  CanvasWindowLike,
} from "../src/index";

class FakeEventTarget {
  private readonly listeners = new Map<string, ((event: any) => void)[]>();

  addEventListener(type: string, listener: (event: any) => void): void {
    const current = this.listeners.get(type) ?? [];
    current.push(listener);
    this.listeners.set(type, current);
  }

  dispatch(type: string, event: any = {}): void {
    for (const listener of this.listeners.get(type) ?? []) {
      listener(event);
    }
  }
}

class FakeHost implements CanvasMountHostLike {
  readonly children: CanvasElementLike[] = [];

  appendChild(child: CanvasElementLike): void {
    this.children.push(child);
  }
}

class FakeCanvas extends FakeEventTarget implements CanvasElementLike {
  width = 0;
  height = 0;
  tabIndex = -1;
  style: Record<string, string> = {};
  dataset: Record<string, string> = {};
  hidden = false;
}

class FakeDocument extends FakeEventTarget implements CanvasDocumentLike {
  body = new FakeHost();
  title = "";
  visibilityState: "visible" | "hidden" = "visible";
  readonly ids = new Map<string, FakeHost>();
  readonly selectors = new Map<string, FakeHost>();

  createElement(): CanvasElementLike {
    return new FakeCanvas();
  }

  getElementById(id: string): CanvasMountHostLike | null {
    return this.ids.get(id) ?? null;
  }

  querySelector(selector: string): CanvasMountHostLike | null {
    return this.selectors.get(selector) ?? null;
  }
}

class FakeWindow extends FakeEventTarget implements CanvasWindowLike {
  devicePixelRatio = 2;
  private nextHandle = 1;
  private readonly rafCallbacks = new Map<number, (time: number) => void>();

  requestAnimationFrame(callback: (time: number) => void): number {
    const handle = this.nextHandle++;
    this.rafCallbacks.set(handle, callback);
    return handle;
  }

  cancelAnimationFrame(handle: number): void {
    this.rafCallbacks.delete(handle);
  }

  flushAnimationFrame(handle?: number): void {
    const chosen =
      handle ?? Array.from(this.rafCallbacks.keys())[0];
    if (!chosen) {
      return;
    }
    const callback = this.rafCallbacks.get(chosen);
    this.rafCallbacks.delete(chosen);
    callback?.(16.7);
  }

  pendingAnimationFrames(): number {
    return this.rafCallbacks.size;
  }
}

function makeEnvironment(): {
  environment: CanvasEnvironment;
  documentLike: FakeDocument;
  windowLike: FakeWindow;
} {
  const documentLike = new FakeDocument();
  const windowLike = new FakeWindow();
  return {
    environment: {
      document: documentLike,
      window: windowLike,
    },
    documentLike,
    windowLike,
  };
}

describe("window-canvas", () => {
  it("mounts a canvas into the selected host and emits created", () => {
    const { environment, documentLike } = makeEnvironment();
    const backend = new CanvasBackend(environment);

    const handle = new WindowBuilder()
      .title("Mounted Canvas")
      .initialSize(new LogicalSize(400, 300))
      .mountTarget(MountTargets.browserBody())
      .preferredSurface(SurfacePreference.Canvas2D)
      .buildWith(backend);

    const events = backend.pumpEvents();
    expect(documentLike.body.children).toHaveLength(1);
    expect(handle.physicalSize().width).toBe(800);
    expect(handle.renderTarget().kind).toBe("browser-canvas");
    expect(events).toEqual([
      { type: "created", windowId: handle.id() },
    ]);
  });

  it("queues redraw requests through requestAnimationFrame", () => {
    const { environment, windowLike } = makeEnvironment();
    const backend = new CanvasBackend(environment);
    const handle = new WindowBuilder()
      .initialSize(new LogicalSize(200, 100))
      .mountTarget(MountTargets.browserBody())
      .buildWith(backend);
    backend.pumpEvents();

    handle.requestRedraw();
    handle.requestRedraw();
    expect(windowLike.pendingAnimationFrames()).toBe(1);

    windowLike.flushAnimationFrame();
    expect(backend.pumpEvents()).toEqual([
      { type: "redraw-requested", windowId: handle.id() },
    ]);
  });

  it("normalizes pointer, key, resize, and visibility events", () => {
    const { environment, documentLike, windowLike } = makeEnvironment();
    const mount = new FakeHost();
    documentLike.ids.set("app", mount);
    const backend = new CanvasBackend(environment);
    const handle = new WindowBuilder()
      .initialSize(new LogicalSize(320, 240))
      .mountTarget(MountTargets.elementId("app"))
      .buildWith(backend);
    backend.pumpEvents();

    const canvas = mount.children[0] as FakeCanvas;
    canvas.dispatch("pointermove", { clientX: 10, clientY: 20 });
    canvas.dispatch("pointerdown", { button: 0 });
    canvas.dispatch("keydown", { key: "a" });
    canvas.dispatch("keyup", { key: "Enter" });
    canvas.dispatch("focus");
    documentLike.visibilityState = "hidden";
    documentLike.dispatch("visibilitychange");
    windowLike.devicePixelRatio = 3;
    windowLike.dispatch("resize");

    expect(backend.pumpEvents()).toEqual([
      { type: "pointer-moved", windowId: handle.id(), x: 10, y: 20 },
      {
        type: "pointer-button",
        windowId: handle.id(),
        button: { kind: "primary" },
        state: "pressed",
      },
      {
        type: "key",
        windowId: handle.id(),
        key: { kind: "character", value: "a" },
        state: "pressed",
        modifiers: { shift: false, control: false, alt: false, meta: false },
        text: "a",
      },
      { type: "text-input", windowId: handle.id(), text: "a" },
      {
        type: "key",
        windowId: handle.id(),
        key: { kind: "named", value: "Enter" },
        state: "released",
        modifiers: { shift: false, control: false, alt: false, meta: false },
        text: null,
      },
      { type: "focus-changed", windowId: handle.id(), focused: true },
      { type: "visibility-changed", windowId: handle.id(), visible: false },
      {
        type: "resized",
        windowId: handle.id(),
        logicalSize: new LogicalSize(320, 240),
        physicalSize: handle.logicalSize().toPhysical(3),
        scaleFactor: 3,
      },
    ]);
  });

  it("falls back invalid dpr values and suppresses modified or non-printable text", () => {
    const { environment, documentLike, windowLike } = makeEnvironment();
    windowLike.devicePixelRatio = Number.NaN;
    const mount = new FakeHost();
    documentLike.selectors.set("#app", mount);
    const backend = new CanvasBackend(environment);
    const handle = new WindowBuilder()
      .initialSize(new LogicalSize(160, 90))
      .mountTarget(MountTargets.querySelector("#app"))
      .buildWith(backend);
    backend.pumpEvents();

    const canvas = mount.children[0] as FakeCanvas;
    expect(handle.scaleFactor()).toBe(1);
    expect(handle.physicalSize()).toEqual(new LogicalSize(160, 90).toPhysical(1));

    canvas.dispatch("pointerdown", { button: 2 });
    canvas.dispatch("pointerup", { button: 5 });
    canvas.dispatch("keydown", { key: "c", ctrlKey: true });
    canvas.dispatch("keydown", { key: "Meta", metaKey: true });
    canvas.dispatch("keydown", { key: "Tab" });

    expect(backend.pumpEvents()).toEqual([
      {
        type: "pointer-button",
        windowId: handle.id(),
        button: { kind: "secondary" },
        state: "pressed",
      },
      {
        type: "pointer-button",
        windowId: handle.id(),
        button: { kind: "other", value: 5 },
        state: "released",
      },
      {
        type: "key",
        windowId: handle.id(),
        key: { kind: "character", value: "c" },
        state: "pressed",
        modifiers: { shift: false, control: true, alt: false, meta: false },
        text: null,
      },
      {
        type: "key",
        windowId: handle.id(),
        key: { kind: "character", value: "Meta" },
        state: "pressed",
        modifiers: { shift: false, control: false, alt: false, meta: true },
        text: null,
      },
      {
        type: "key",
        windowId: handle.id(),
        key: { kind: "named", value: "Tab" },
        state: "pressed",
        modifiers: { shift: false, control: false, alt: false, meta: false },
        text: null,
      },
    ]);
  });

  it("rejects native mount targets for the browser backend", () => {
    const { environment } = makeEnvironment();
    const backend = new CanvasBackend(environment);

    expect(() => new WindowBuilder().buildWith(backend)).toThrow(
      "window-canvas requires a browser mount target",
    );
  });
});
