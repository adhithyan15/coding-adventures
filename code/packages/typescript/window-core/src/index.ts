/**
 * Shared TypeScript mirror of the repository-owned window-core contract.
 *
 * The goal is not to hide every platform difference. The goal is to give each
 * language the same nouns and the same event meanings:
 *
 * - a window has identity, size, visibility, and a render target
 * - a backend creates windows and pumps normalized events
 * - renderers inspect an explicit render-target tag instead of guessing
 */

export const VERSION = "0.1.0";

export class WindowError extends Error {
  constructor(
    readonly code:
      | "invalid-attributes"
      | "unsupported-configuration"
      | "unsupported-platform"
      | "backend",
    message: string,
  ) {
    super(message);
    this.name = "WindowError";
  }

  static invalidAttributes(message: string): WindowError {
    return new WindowError("invalid-attributes", message);
  }

  static unsupportedConfiguration(message: string): WindowError {
    return new WindowError("unsupported-configuration", message);
  }

  static unsupportedPlatform(message: string): WindowError {
    return new WindowError("unsupported-platform", message);
  }

  static backend(message: string): WindowError {
    return new WindowError("backend", message);
  }
}

export class WindowId {
  constructor(readonly value: number) {
    if (!Number.isInteger(value) || value < 0) {
      throw WindowError.invalidAttributes(
        "window ids must be non-negative integers",
      );
    }
  }
}

export class LogicalSize {
  constructor(
    readonly width: number,
    readonly height: number,
  ) {}

  validate(): LogicalSize {
    if (!Number.isFinite(this.width) || !Number.isFinite(this.height)) {
      throw WindowError.invalidAttributes(
        "window sizes must be finite numbers",
      );
    }
    if (this.width < 0 || this.height < 0) {
      throw WindowError.invalidAttributes(
        "window sizes must be non-negative",
      );
    }
    return this;
  }

  toPhysical(scaleFactor: number): PhysicalSize {
    this.validate();
    validateScaleFactor(scaleFactor);
    return new PhysicalSize(
      roundDimension(this.width * scaleFactor),
      roundDimension(this.height * scaleFactor),
    );
  }
}

export class PhysicalSize {
  constructor(
    readonly width: number,
    readonly height: number,
  ) {}

  toLogical(scaleFactor: number): LogicalSize {
    validateScaleFactor(scaleFactor);
    return new LogicalSize(this.width / scaleFactor, this.height / scaleFactor);
  }
}

export enum SurfacePreference {
  Default = "default",
  Metal = "metal",
  Direct2D = "direct2d",
  Cairo = "cairo",
  Canvas2D = "canvas2d",
}

export type MountTarget =
  | { kind: "native" }
  | { kind: "browser-body" }
  | { kind: "element-id"; value: string }
  | { kind: "query-selector"; value: string };

export const MountTargets = {
  native(): MountTarget {
    return { kind: "native" };
  },
  browserBody(): MountTarget {
    return { kind: "browser-body" };
  },
  elementId(value: string): MountTarget {
    return { kind: "element-id", value };
  },
  querySelector(value: string): MountTarget {
    return { kind: "query-selector", value };
  },
};

export interface WindowAttributes {
  title: string;
  initialSize: LogicalSize;
  minSize: LogicalSize | null;
  maxSize: LogicalSize | null;
  visible: boolean;
  resizable: boolean;
  decorations: boolean;
  transparent: boolean;
  preferredSurface: SurfacePreference;
  mountTarget: MountTarget;
}

export function defaultWindowAttributes(): WindowAttributes {
  return {
    title: "Coding Adventures Window",
    initialSize: new LogicalSize(800, 600),
    minSize: null,
    maxSize: null,
    visible: true,
    resizable: true,
    decorations: true,
    transparent: false,
    preferredSurface: SurfacePreference.Default,
    mountTarget: MountTargets.native(),
  };
}

export function validateMountTarget(target: MountTarget): void {
  if ((target.kind === "element-id" || target.kind === "query-selector") && !target.value.trim()) {
    throw WindowError.invalidAttributes(
      target.kind === "element-id"
        ? "element ids must not be blank"
        : "query selectors must not be blank",
    );
  }
}

export function validateWindowAttributes(attributes: WindowAttributes): WindowAttributes {
  attributes.initialSize.validate();
  validateMountTarget(attributes.mountTarget);

  if (attributes.minSize) {
    attributes.minSize.validate();
    if (
      attributes.minSize.width > attributes.initialSize.width ||
      attributes.minSize.height > attributes.initialSize.height
    ) {
      throw WindowError.invalidAttributes(
        "minimum size must not exceed the initial size",
      );
    }
  }

  if (attributes.maxSize) {
    attributes.maxSize.validate();
    if (
      attributes.maxSize.width < attributes.initialSize.width ||
      attributes.maxSize.height < attributes.initialSize.height
    ) {
      throw WindowError.invalidAttributes(
        "maximum size must not be smaller than the initial size",
      );
    }
  }

  if (attributes.minSize && attributes.maxSize) {
    if (
      attributes.minSize.width > attributes.maxSize.width ||
      attributes.minSize.height > attributes.maxSize.height
    ) {
      throw WindowError.invalidAttributes(
        "minimum size must not exceed maximum size",
      );
    }
  }

  return attributes;
}

export class WindowBuilder {
  private readonly attributes: WindowAttributes;

  constructor(attributes: WindowAttributes = defaultWindowAttributes()) {
    this.attributes = { ...attributes };
  }

  title(title: string): WindowBuilder {
    return this.with({ title });
  }

  initialSize(initialSize: LogicalSize): WindowBuilder {
    return this.with({ initialSize });
  }

  minSize(minSize: LogicalSize): WindowBuilder {
    return this.with({ minSize });
  }

  maxSize(maxSize: LogicalSize): WindowBuilder {
    return this.with({ maxSize });
  }

  visible(visible: boolean): WindowBuilder {
    return this.with({ visible });
  }

  resizable(resizable: boolean): WindowBuilder {
    return this.with({ resizable });
  }

  decorations(decorations: boolean): WindowBuilder {
    return this.with({ decorations });
  }

  transparent(transparent: boolean): WindowBuilder {
    return this.with({ transparent });
  }

  preferredSurface(preferredSurface: SurfacePreference): WindowBuilder {
    return this.with({ preferredSurface });
  }

  mountTarget(mountTarget: MountTarget): WindowBuilder {
    return this.with({ mountTarget });
  }

  build(): WindowAttributes {
    return validateWindowAttributes({ ...this.attributes });
  }

  buildWith<W extends WindowHandle>(
    backend: WindowBackend<W>,
  ): W {
    return backend.createWindow(this.build());
  }

  private with(patch: Partial<WindowAttributes>): WindowBuilder {
    return new WindowBuilder({ ...this.attributes, ...patch });
  }
}

export enum ElementState {
  Pressed = "pressed",
  Released = "released",
}

export type PointerButton =
  | { kind: "primary" }
  | { kind: "secondary" }
  | { kind: "middle" }
  | { kind: "other"; value: number };

export const PointerButtons = {
  primary(): PointerButton {
    return { kind: "primary" };
  },
  secondary(): PointerButton {
    return { kind: "secondary" };
  },
  middle(): PointerButton {
    return { kind: "middle" };
  },
  other(value: number): PointerButton {
    return { kind: "other", value };
  },
};

export type NamedKey =
  | "Escape"
  | "Enter"
  | "Tab"
  | "Backspace"
  | "Space"
  | "ArrowLeft"
  | "ArrowRight"
  | "ArrowUp"
  | "ArrowDown"
  | "Home"
  | "End"
  | "PageUp"
  | "PageDown";

export type Key =
  | { kind: "named"; value: NamedKey }
  | { kind: "character"; value: string };

export interface ModifiersState {
  shift: boolean;
  control: boolean;
  alt: boolean;
  meta: boolean;
}

export function defaultModifiersState(): ModifiersState {
  return { shift: false, control: false, alt: false, meta: false };
}

export type WindowEvent =
  | { type: "created"; windowId: WindowId }
  | {
      type: "resized";
      windowId: WindowId;
      logicalSize: LogicalSize;
      physicalSize: PhysicalSize;
      scaleFactor: number;
    }
  | { type: "redraw-requested"; windowId: WindowId }
  | { type: "close-requested"; windowId: WindowId }
  | { type: "destroyed"; windowId: WindowId }
  | { type: "focus-changed"; windowId: WindowId; focused: boolean }
  | { type: "visibility-changed"; windowId: WindowId; visible: boolean }
  | { type: "pointer-moved"; windowId: WindowId; x: number; y: number }
  | {
      type: "pointer-button";
      windowId: WindowId;
      button: PointerButton;
      state: ElementState;
    }
  | { type: "scroll"; windowId: WindowId; deltaX: number; deltaY: number }
  | {
      type: "key";
      windowId: WindowId;
      key: Key;
      state: ElementState;
      modifiers: ModifiersState;
      text: string | null;
    }
  | { type: "text-input"; windowId: WindowId; text: string };

export interface AppKitRenderTarget {
  kind: "appkit";
  nsWindow: number;
  nsView: number;
  metalLayer: number | null;
}

export interface Win32RenderTarget {
  kind: "win32";
  hwnd: number;
}

export interface BrowserCanvasRenderTarget {
  kind: "browser-canvas";
  mountTarget: MountTarget;
  logicalSize: LogicalSize;
  physicalSize: PhysicalSize;
  devicePixelRatio: number;
}

export interface WaylandRenderTarget {
  kind: "wayland";
  display: number;
  surface: number;
}

export interface X11RenderTarget {
  kind: "x11";
  display: number;
  window: number;
}

export type RenderTarget =
  | AppKitRenderTarget
  | Win32RenderTarget
  | BrowserCanvasRenderTarget
  | WaylandRenderTarget
  | X11RenderTarget;

export interface WindowHandle {
  id(): WindowId;
  logicalSize(): LogicalSize;
  physicalSize(): PhysicalSize;
  scaleFactor(): number;
  requestRedraw(): void;
  setTitle(title: string): void;
  setVisible(visible: boolean): void;
  renderTarget(): RenderTarget;
}

export interface WindowBackend<W extends WindowHandle> {
  backendName(): string;
  createWindow(attributes: WindowAttributes): W;
  pumpEvents(): WindowEvent[];
}

function validateScaleFactor(scaleFactor: number): void {
  if (!Number.isFinite(scaleFactor) || scaleFactor <= 0) {
    throw WindowError.invalidAttributes(
      "scale factors must be finite positive numbers",
    );
  }
}

function roundDimension(value: number): number {
  if (value < 0 || value > 0xffff_ffff) {
    throw WindowError.invalidAttributes(
      "scaled dimensions must fit into u32",
    );
  }
  return Math.round(value);
}
