// element.test.ts — MosaicHostElement + defineMosaicElement.

import { describe, it, expect, vi, beforeEach } from "vitest";
import { MosaicStore } from "../src/store.js";
import type { MosaicAction } from "../src/action.js";
import {
  MosaicHostElement,
  defineMosaicElement,
} from "../src/element.js";
import { bindText } from "../src/dom.js";

interface S {
  text: string;
  count: number;
}

class SetText implements MosaicAction<S> {
  constructor(public readonly text: string) {}
  apply(s: S): S {
    return { ...s, text: this.text };
  }
}

class Increment implements MosaicAction<S> {
  apply(s: S): S {
    return { ...s, count: s.count + 1 };
  }
}

const initial: S = { text: "hello", count: 0 };

/**
 * Test subclass: renders a single text span bound to state.text in
 * its shadow root, plus exposes a publicDispatch wrapper so tests
 * can call dispatch from outside.
 */
class TextElement extends MosaicHostElement<S> {
  span: HTMLSpanElement;
  bindStoreCallCount = 0;

  constructor() {
    super();
    const shadow = this.attachShadowIfNeeded();
    this.span = document.createElement("span");
    shadow.appendChild(this.span);
  }

  protected bindStore(store: MosaicStore<S>): void {
    this.bindStoreCallCount++;
    this.track(bindText(this.span, store, (s) => s.text));
  }

  publicDispatch(action: MosaicAction<S>): void {
    this.dispatch(action);
  }
}

let tagCounter = 0;
const uniqueTag = (): string => `mosaic-test-${++tagCounter}-${Date.now()}`;

describe("MosaicHostElement", () => {
  let tagName: string;
  let store: MosaicStore<S>;

  beforeEach(() => {
    tagName = uniqueTag();
    customElements.define(tagName, class extends TextElement {});
    store = new MosaicStore<S>({ initialState: { ...initial } });
  });

  it("does not bind before connection", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    // Not connected yet
    expect(el.bindStoreCallCount).toBe(0);
  });

  it("invokes bindStore on connection when store is set", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    expect(el.bindStoreCallCount).toBe(1);
    expect(el.span.textContent).toBe("hello");
  });

  it("does not invoke bindStore on connection without a store", () => {
    const el = document.createElement(tagName) as TextElement;
    document.body.appendChild(el);
    expect(el.bindStoreCallCount).toBe(0);
    el.remove();
  });

  it("invokes bindStore when store is set AFTER connection", () => {
    const el = document.createElement(tagName) as TextElement;
    document.body.appendChild(el);
    expect(el.bindStoreCallCount).toBe(0);
    el.store = store;
    expect(el.bindStoreCallCount).toBe(1);
  });

  it("rebinds when store is reassigned while connected", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    expect(el.bindStoreCallCount).toBe(1);
    const store2 = new MosaicStore<S>({ initialState: { text: "two", count: 0 } });
    el.store = store2;
    expect(el.bindStoreCallCount).toBe(2);
    expect(el.span.textContent).toBe("two");
  });

  it("disposes bindings on disconnect", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    store.dispatch(new SetText("world"));
    expect(el.span.textContent).toBe("world");
    el.remove();
    // After disconnect, further dispatches should NOT update the
    // element's text content (the binding has been disposed).
    store.dispatch(new SetText("ignored"));
    expect(el.span.textContent).toBe("world");
  });

  it("reconnect after disconnect rebinds (no stale bindings)", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    expect(el.bindStoreCallCount).toBe(1);
    el.remove();
    document.body.appendChild(el);
    expect(el.bindStoreCallCount).toBe(2);
    store.dispatch(new SetText("after-reconnect"));
    expect(el.span.textContent).toBe("after-reconnect");
  });

  it("attachShadowIfNeeded is idempotent", () => {
    const el = document.createElement(tagName) as TextElement;
    const shadow1 = el.attachShadowIfNeeded();
    const shadow2 = el.attachShadowIfNeeded();
    expect(shadow1).toBe(shadow2);
  });

  it("attachShadowIfNeeded respects pre-existing shadow root", () => {
    // Define a separate test class that DOESN'T attach a shadow in
    // its constructor, to exercise the "subclass attached one
    // independently" branch.
    class BareElement extends MosaicHostElement<S> {
      protected bindStore(_store: MosaicStore<S>): void {
        // no-op
      }
    }
    const bareTag = uniqueTag();
    customElements.define(bareTag, BareElement);
    const el = document.createElement(bareTag) as BareElement;
    const preAttached = el.attachShadow({ mode: "open" });
    const fromHelper = el.attachShadowIfNeeded();
    expect(fromHelper).toBe(preAttached);
  });

  it("dispatch shortcut delegates to the store", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    el.publicDispatch(new SetText("from-dispatch"));
    expect(store.state.text).toBe("from-dispatch");
    expect(el.span.textContent).toBe("from-dispatch");
  });

  it("dispatch throws when no store is bound", () => {
    const el = document.createElement(tagName) as TextElement;
    expect(() => el.publicDispatch(new Increment())).toThrow(/cannot dispatch/);
  });

  it("setting store to the same reference is a no-op", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    const before = el.bindStoreCallCount;
    el.store = store;
    expect(el.bindStoreCallCount).toBe(before);
  });

  it("setting store to null disposes existing bindings while connected", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    document.body.appendChild(el);
    store.dispatch(new SetText("present"));
    expect(el.span.textContent).toBe("present");
    el.store = null;
    store.dispatch(new SetText("after-null"));
    expect(el.span.textContent).toBe("present");
  });

  it("a bad unsubscribe doesn't prevent other unsubscribes from running", () => {
    class BadUnsubElement extends MosaicHostElement<S> {
      otherUnsubRan = false;
      protected bindStore(_s: MosaicStore<S>): void {
        this.track(() => {
          throw new Error("first one throws");
        });
        this.track(() => {
          this.otherUnsubRan = true;
        });
      }
    }
    const badTag = uniqueTag();
    customElements.define(badTag, BadUnsubElement);
    const el = document.createElement(badTag) as BadUnsubElement;
    el.store = store;
    document.body.appendChild(el);
    el.remove(); // triggers cleanup
    expect(el.otherUnsubRan).toBe(true);
  });

  it("store getter returns null before any store is assigned", () => {
    const el = document.createElement(tagName) as TextElement;
    expect(el.store).toBe(null);
  });

  it("store getter returns the assigned store", () => {
    const el = document.createElement(tagName) as TextElement;
    el.store = store;
    expect(el.store).toBe(store);
  });
});

describe("defineMosaicElement", () => {
  it("registers a class with customElements.define", () => {
    const tag = uniqueTag();
    class E extends MosaicHostElement<S> {
      protected bindStore(_s: MosaicStore<S>): void {}
    }
    defineMosaicElement(tag, E);
    expect(customElements.get(tag)).toBe(E);
  });

  it("is idempotent — second call with same tag is a no-op", () => {
    const tag = uniqueTag();
    class E1 extends MosaicHostElement<S> {
      protected bindStore(_s: MosaicStore<S>): void {}
    }
    class E2 extends MosaicHostElement<S> {
      protected bindStore(_s: MosaicStore<S>): void {}
    }
    defineMosaicElement(tag, E1);
    defineMosaicElement(tag, E2); // should not throw
    // First definition wins
    expect(customElements.get(tag)).toBe(E1);
  });

  it("throws when customElements is undefined", () => {
    const original = (globalThis as { customElements?: unknown }).customElements;
    (globalThis as { customElements?: unknown }).customElements = undefined;
    try {
      class E extends MosaicHostElement<S> {
        protected bindStore(_s: MosaicStore<S>): void {}
      }
      expect(() => defineMosaicElement(uniqueTag(), E)).toThrow(
        /customElements is not available/,
      );
    } finally {
      (globalThis as { customElements?: unknown }).customElements = original;
    }
  });

  it("accepts ElementDefinitionOptions third argument", () => {
    const tag = uniqueTag();
    class E extends MosaicHostElement<S> {
      protected bindStore(_s: MosaicStore<S>): void {}
    }
    const spy = vi.spyOn(customElements, "define");
    defineMosaicElement(tag, E, {});
    expect(spy).toHaveBeenCalledWith(tag, E, {});
    spy.mockRestore();
  });
});
