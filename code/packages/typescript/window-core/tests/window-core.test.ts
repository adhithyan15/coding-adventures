import { describe, expect, it } from "vitest";
import {
  defaultModifiersState,
  LogicalSize,
  MountTargets,
  PhysicalSize,
  PointerButtons,
  RenderTarget,
  SurfacePreference,
  WindowBackend,
  WindowBuilder,
  WindowHandle,
  WindowId,
} from "../src/index";

class MockWindow implements WindowHandle {
  constructor(
    private readonly identity: WindowId,
    private title = "mock",
    private visible = true,
  ) {}

  id(): WindowId {
    return this.identity;
  }

  logicalSize(): LogicalSize {
    return new LogicalSize(320, 200);
  }

  physicalSize(): PhysicalSize {
    return new PhysicalSize(640, 400);
  }

  scaleFactor(): number {
    return 2;
  }

  requestRedraw(): void {}

  setTitle(title: string): void {
    this.title = title;
  }

  setVisible(visible: boolean): void {
    this.visible = visible;
  }

  renderTarget(): RenderTarget {
    return {
      kind: "browser-canvas",
      mountTarget: MountTargets.browserBody(),
      logicalSize: new LogicalSize(320, 200),
      physicalSize: new PhysicalSize(640, 400),
      devicePixelRatio: 2,
    };
  }
}

class MockBackend implements WindowBackend<MockWindow> {
  lastTitle = "";

  backendName(): string {
    return "mock";
  }

  createWindow(attributes: ReturnType<WindowBuilder["build"]>): MockWindow {
    this.lastTitle = attributes.title;
    return new MockWindow(new WindowId(7));
  }

  pumpEvents() {
    return [];
  }
}

describe("window-core", () => {
  it("converts logical sizes into physical sizes", () => {
    expect(new LogicalSize(400, 300).toPhysical(2)).toEqual(
      new PhysicalSize(800, 600),
    );
  });

  it("exposes a neutral modifier-state default", () => {
    expect(defaultModifiersState()).toEqual({
      shift: false,
      control: false,
      alt: false,
      meta: false,
    });
  });

  it("rejects non-positive or non-finite scale factors", () => {
    expect(() => new LogicalSize(100, 50).toPhysical(0)).toThrow(
      "scale factors must be finite positive numbers",
    );
    expect(() => new PhysicalSize(200, 100).toLogical(Number.NaN)).toThrow(
      "scale factors must be finite positive numbers",
    );
  });

  it("rejects scaled dimensions that overflow u32", () => {
    expect(() => new LogicalSize(0xffff_ffff, 10).toPhysical(2)).toThrow(
      "scaled dimensions must fit into u32",
    );
  });

  it("builds the shared pointer-button variants", () => {
    expect(PointerButtons.primary()).toEqual({ kind: "primary" });
    expect(PointerButtons.secondary()).toEqual({ kind: "secondary" });
    expect(PointerButtons.middle()).toEqual({ kind: "middle" });
    expect(PointerButtons.other(4)).toEqual({ kind: "other", value: 4 });
  });

  it("rejects blank element ids", () => {
    expect(() =>
      new WindowBuilder().mountTarget(MountTargets.elementId("   ")).build(),
    ).toThrow("element ids must not be blank");
  });

  it("rejects minimum sizes above the initial size", () => {
    expect(() =>
      new WindowBuilder()
        .initialSize(new LogicalSize(200, 100))
        .minSize(new LogicalSize(300, 100))
        .build(),
    ).toThrow("minimum size must not exceed the initial size");
  });

  it("builds valid browser attributes", () => {
    const attributes = new WindowBuilder()
      .title("Canvas Host")
      .initialSize(new LogicalSize(640, 480))
      .visible(false)
      .resizable(false)
      .decorations(false)
      .transparent(true)
      .mountTarget(MountTargets.browserBody())
      .preferredSurface(SurfacePreference.Canvas2D)
      .build();

    expect(attributes.title).toBe("Canvas Host");
    expect(attributes.visible).toBe(false);
    expect(attributes.resizable).toBe(false);
    expect(attributes.decorations).toBe(false);
    expect(attributes.transparent).toBe(true);
    expect(attributes.mountTarget.kind).toBe("browser-body");
    expect(attributes.preferredSurface).toBe(SurfacePreference.Canvas2D);
  });

  it("hands validated attributes to a backend", () => {
    const backend = new MockBackend();
    const windowHandle = new WindowBuilder()
      .title("Backend Window")
      .mountTarget(MountTargets.browserBody())
      .buildWith(backend);

    expect(backend.lastTitle).toBe("Backend Window");
    expect(windowHandle.id().value).toBe(7);
    expect(windowHandle.renderTarget().kind).toBe("browser-canvas");
  });
});
