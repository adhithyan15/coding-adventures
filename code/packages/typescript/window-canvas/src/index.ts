import {
  BrowserCanvasRenderTarget,
  defaultModifiersState,
  ElementState,
  Key,
  LogicalSize,
  ModifiersState,
  MountTarget,
  NamedKey,
  PhysicalSize,
  PointerButton,
  PointerButtons,
  RenderTarget,
  SurfacePreference,
  WindowAttributes,
  WindowBackend,
  WindowError,
  WindowEvent,
  WindowHandle,
  WindowId,
  validateWindowAttributes,
} from "@coding-adventures/window-core";

export const VERSION = "0.1.0";

type Listener = (event: any) => void;

export interface CanvasEventTargetLike {
  addEventListener(type: string, listener: Listener): void;
}

export interface CanvasMountHostLike {
  appendChild(child: CanvasElementLike): void;
}

export interface CanvasElementLike extends CanvasEventTargetLike {
  width: number;
  height: number;
  tabIndex: number;
  style: Record<string, string>;
  dataset?: Record<string, string>;
  hidden?: boolean;
}

export interface CanvasDocumentLike extends CanvasEventTargetLike {
  body: CanvasMountHostLike;
  title: string;
  visibilityState: "visible" | "hidden";
  createElement(tag: "canvas"): CanvasElementLike;
  getElementById(id: string): CanvasMountHostLike | null;
  querySelector(selector: string): CanvasMountHostLike | null;
}

export interface CanvasWindowLike extends CanvasEventTargetLike {
  devicePixelRatio: number;
  requestAnimationFrame(callback: (time: number) => void): number;
  cancelAnimationFrame(handle: number): void;
}

export interface CanvasEnvironment {
  document: CanvasDocumentLike;
  window: CanvasWindowLike;
}

export class CanvasWindow implements WindowHandle {
  private logical: LogicalSize;
  private physical: PhysicalSize;
  private scale: number;
  private redrawHandle: number | null = null;

  constructor(
    private readonly identity: WindowId,
    private readonly attributes: WindowAttributes,
    private readonly canvas: CanvasElementLike,
    private readonly environment: CanvasEnvironment,
    private readonly pushEvent: (event: WindowEvent) => void,
  ) {
    this.scale = sanitizeDevicePixelRatio(environment.window.devicePixelRatio);
    this.logical = attributes.initialSize;
    this.physical = this.logical.toPhysical(this.scale);
    this.syncCanvasGeometry();
    this.installListeners();
  }

  id(): WindowId {
    return this.identity;
  }

  logicalSize(): LogicalSize {
    return this.logical;
  }

  physicalSize(): PhysicalSize {
    return this.physical;
  }

  scaleFactor(): number {
    return this.scale;
  }

  requestRedraw(): void {
    if (this.redrawHandle !== null) {
      return;
    }

    this.redrawHandle = this.environment.window.requestAnimationFrame(() => {
      this.redrawHandle = null;
      this.pushEvent({
        type: "redraw-requested",
        windowId: this.identity,
      });
    });
  }

  setTitle(title: string): void {
    if (this.canvas.dataset) {
      this.canvas.dataset.windowTitle = title;
    }
  }

  setVisible(visible: boolean): void {
    this.canvas.hidden = !visible;
    this.canvas.style.display = visible ? "block" : "none";
    this.pushEvent({
      type: "visibility-changed",
      windowId: this.identity,
      visible,
    });
  }

  renderTarget(): RenderTarget {
    return this.browserCanvasTarget();
  }

  canvasElement(): CanvasElementLike {
    return this.canvas;
  }

  private browserCanvasTarget(): BrowserCanvasRenderTarget {
    return {
      kind: "browser-canvas",
      mountTarget: this.attributes.mountTarget,
      logicalSize: this.logical,
      physicalSize: this.physical,
      devicePixelRatio: this.scale,
    };
  }

  private installListeners(): void {
    this.canvas.addEventListener("pointermove", (event) => {
      this.pushEvent({
        type: "pointer-moved",
        windowId: this.identity,
        x: Number(event.clientX ?? 0),
        y: Number(event.clientY ?? 0),
      });
    });

    this.canvas.addEventListener("pointerdown", (event) => {
      this.pushPointerButton(ElementState.Pressed, event.button);
    });

    this.canvas.addEventListener("pointerup", (event) => {
      this.pushPointerButton(ElementState.Released, event.button);
    });

    this.canvas.addEventListener("wheel", (event) => {
      this.pushEvent({
        type: "scroll",
        windowId: this.identity,
        deltaX: Number(event.deltaX ?? 0),
        deltaY: Number(event.deltaY ?? 0),
      });
    });

    this.canvas.addEventListener("focus", () => {
      this.pushEvent({
        type: "focus-changed",
        windowId: this.identity,
        focused: true,
      });
    });

    this.canvas.addEventListener("blur", () => {
      this.pushEvent({
        type: "focus-changed",
        windowId: this.identity,
        focused: false,
      });
    });

    this.canvas.addEventListener("keydown", (event) => {
      const key = mapKey(String(event.key ?? ""));
      const modifiers = mapModifiers(event);
      const text = printableText(String(event.key ?? ""), modifiers);
      this.pushEvent({
        type: "key",
        windowId: this.identity,
        key,
        state: ElementState.Pressed,
        modifiers,
        text,
      });
      if (text) {
        this.pushEvent({
          type: "text-input",
          windowId: this.identity,
          text,
        });
      }
    });

    this.canvas.addEventListener("keyup", (event) => {
      this.pushEvent({
        type: "key",
        windowId: this.identity,
        key: mapKey(String(event.key ?? "")),
        state: ElementState.Released,
        modifiers: mapModifiers(event),
        text: null,
      });
    });

    this.environment.document.addEventListener("visibilitychange", () => {
      this.pushEvent({
        type: "visibility-changed",
        windowId: this.identity,
        visible: this.environment.document.visibilityState === "visible",
      });
    });

    this.environment.window.addEventListener("resize", () => {
      const nextScale = sanitizeDevicePixelRatio(
        this.environment.window.devicePixelRatio,
      );
      const nextPhysical = this.logical.toPhysical(nextScale);
      if (
        nextScale === this.scale &&
        nextPhysical.width === this.physical.width &&
        nextPhysical.height === this.physical.height
      ) {
        return;
      }

      this.scale = nextScale;
      this.physical = nextPhysical;
      this.syncCanvasGeometry();
      this.pushEvent({
        type: "resized",
        windowId: this.identity,
        logicalSize: this.logical,
        physicalSize: this.physical,
        scaleFactor: this.scale,
      });
    });
  }

  private pushPointerButton(state: ElementState, rawButton: unknown): void {
    this.pushEvent({
      type: "pointer-button",
      windowId: this.identity,
      button: mapPointerButton(rawButton),
      state,
    });
  }

  private syncCanvasGeometry(): void {
    this.canvas.style.width = `${this.logical.width}px`;
    this.canvas.style.height = `${this.logical.height}px`;
    this.canvas.style.display = this.attributes.visible ? "block" : "none";
    this.canvas.width = this.physical.width;
    this.canvas.height = this.physical.height;
    if (this.canvas.dataset) {
      this.canvas.dataset.windowTitle = this.attributes.title;
    }
  }
}

export class CanvasBackend implements WindowBackend<CanvasWindow> {
  private readonly environment: CanvasEnvironment;
  private readonly events: WindowEvent[] = [];
  private nextId = 0;

  constructor(environment?: CanvasEnvironment) {
    this.environment = environment ?? defaultEnvironment();
  }

  backendName(): string {
    return "canvas";
  }

  createWindow(attributes: WindowAttributes): CanvasWindow {
    validateWindowAttributes(attributes);
    this.validateBrowserAttributes(attributes);

    const host = resolveMountHost(this.environment.document, attributes.mountTarget);
    const canvas = this.environment.document.createElement("canvas");
    canvas.tabIndex = 0;
    host.appendChild(canvas);

    const windowHandle = new CanvasWindow(
      new WindowId(++this.nextId),
      attributes,
      canvas,
      this.environment,
      (event) => this.events.push(event),
    );
    this.events.push({ type: "created", windowId: windowHandle.id() });
    return windowHandle;
  }

  pumpEvents(): WindowEvent[] {
    return this.events.splice(0, this.events.length);
  }

  private validateBrowserAttributes(attributes: WindowAttributes): void {
    if (attributes.mountTarget.kind === "native") {
      throw WindowError.unsupportedConfiguration(
        "window-canvas requires a browser mount target",
      );
    }

    if (
      attributes.preferredSurface !== SurfacePreference.Default &&
      attributes.preferredSurface !== SurfacePreference.Canvas2D
    ) {
      throw WindowError.unsupportedConfiguration(
        "window-canvas only supports Default and Canvas2D surfaces",
      );
    }
  }
}

function resolveMountHost(
  documentLike: CanvasDocumentLike,
  mountTarget: MountTarget,
): CanvasMountHostLike {
  switch (mountTarget.kind) {
    case "browser-body":
      return documentLike.body;
    case "element-id": {
      const found = documentLike.getElementById(mountTarget.value);
      if (!found) {
        throw WindowError.backend(
          `window-canvas could not find element id ${mountTarget.value}`,
        );
      }
      return found;
    }
    case "query-selector": {
      const found = documentLike.querySelector(mountTarget.value);
      if (!found) {
        throw WindowError.backend(
          `window-canvas could not resolve selector ${mountTarget.value}`,
        );
      }
      return found;
    }
    case "native":
      throw WindowError.unsupportedConfiguration(
        "window-canvas requires a browser mount target",
      );
  }
}

function defaultEnvironment(): CanvasEnvironment {
  const globalWindow = globalThis.window as unknown as CanvasWindowLike | undefined;
  const globalDocument = globalThis.document as unknown as
    | CanvasDocumentLike
    | undefined;

  if (!globalWindow || !globalDocument) {
    throw WindowError.unsupportedPlatform(
      "window-canvas requires a browser-like environment or an injected environment",
    );
  }

  return {
    window: globalWindow,
    document: globalDocument,
  };
}

function sanitizeDevicePixelRatio(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 1;
}

function mapPointerButton(rawButton: unknown): PointerButton {
  switch (Number(rawButton ?? 0)) {
    case 0:
      return PointerButtons.primary();
    case 1:
      return PointerButtons.middle();
    case 2:
      return PointerButtons.secondary();
    default:
      return PointerButtons.other(Number(rawButton ?? 0));
  }
}

function mapModifiers(event: any): ModifiersState {
  return {
    ...defaultModifiersState(),
    shift: Boolean(event.shiftKey),
    control: Boolean(event.ctrlKey),
    alt: Boolean(event.altKey),
    meta: Boolean(event.metaKey),
  };
}

function printableText(key: string, modifiers: ModifiersState): string | null {
  if (key.length !== 1) {
    return null;
  }
  if (modifiers.control || modifiers.meta) {
    return null;
  }
  return key;
}

function mapKey(key: string): Key {
  const named = NAMED_KEYS[key];
  if (named) {
    return { kind: "named", value: named };
  }
  return { kind: "character", value: key };
}

const NAMED_KEYS: Record<string, NamedKey> = {
  Escape: "Escape",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  " ": "Space",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  ArrowUp: "ArrowUp",
  ArrowDown: "ArrowDown",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
};
