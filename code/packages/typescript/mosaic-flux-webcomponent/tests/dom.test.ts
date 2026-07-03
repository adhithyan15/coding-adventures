// dom.test.ts — vanilla-DOM binding helpers (jsdom).

import { describe, it, expect } from "vitest";
import { MosaicStore } from "../src/store.js";
import type { MosaicAction } from "../src/action.js";
import {
  bindAttr,
  bindClass,
  bindList,
  bindStyle,
  bindText,
} from "../src/dom.js";

interface S {
  text: string;
  count: number;
  disabled: boolean;
  highlight: boolean;
  color: string;
  items: { id: string; label: string }[];
}

const initial: S = {
  text: "hello",
  count: 0,
  disabled: false,
  highlight: false,
  color: "red",
  items: [
    { id: "a", label: "Alpha" },
    { id: "b", label: "Beta" },
  ],
};

class SetText implements MosaicAction<S> {
  constructor(public readonly text: string) {}
  apply(s: S): S {
    return { ...s, text: this.text };
  }
}

class ToggleDisabled implements MosaicAction<S> {
  apply(s: S): S {
    return { ...s, disabled: !s.disabled };
  }
}

class ToggleHighlight implements MosaicAction<S> {
  apply(s: S): S {
    return { ...s, highlight: !s.highlight };
  }
}

class SetColor implements MosaicAction<S> {
  constructor(public readonly color: string) {}
  apply(s: S): S {
    return { ...s, color: this.color };
  }
}

class ClearColor implements MosaicAction<S> {
  apply(s: S): S {
    return { ...s, color: "" };
  }
}

class SetItems implements MosaicAction<S> {
  constructor(public readonly items: { id: string; label: string }[]) {}
  apply(s: S): S {
    return { ...s, items: this.items };
  }
}

const makeStore = (over?: Partial<S>): MosaicStore<S> =>
  new MosaicStore<S>({ initialState: { ...initial, ...over } });

describe("bindText", () => {
  it("sets initial textContent from selector", () => {
    const el = document.createElement("span");
    const store = makeStore();
    bindText(el, store, (s) => s.text);
    expect(el.textContent).toBe("hello");
  });

  it("updates textContent on store change", () => {
    const el = document.createElement("span");
    const store = makeStore();
    bindText(el, store, (s) => s.text);
    store.dispatch(new SetText("world"));
    expect(el.textContent).toBe("world");
  });

  it("unsubscribe stops updates", () => {
    const el = document.createElement("span");
    const store = makeStore();
    const unsub = bindText(el, store, (s) => s.text);
    unsub();
    store.dispatch(new SetText("ignored"));
    expect(el.textContent).toBe("hello");
  });
});

describe("bindAttr", () => {
  it("sets attribute when value is a string", () => {
    const el = document.createElement("button");
    const store = makeStore({ disabled: true });
    bindAttr(el, "disabled", store, (s) => (s.disabled ? "true" : null));
    expect(el.getAttribute("disabled")).toBe("true");
  });

  it("removes attribute when value is null", () => {
    const el = document.createElement("button");
    el.setAttribute("disabled", "true");
    const store = makeStore({ disabled: false });
    bindAttr(el, "disabled", store, (s) => (s.disabled ? "true" : null));
    expect(el.hasAttribute("disabled")).toBe(false);
  });

  it("toggles in response to dispatch", () => {
    const el = document.createElement("button");
    const store = makeStore();
    bindAttr(el, "disabled", store, (s) => (s.disabled ? "true" : null));
    expect(el.hasAttribute("disabled")).toBe(false);
    store.dispatch(new ToggleDisabled());
    expect(el.getAttribute("disabled")).toBe("true");
    store.dispatch(new ToggleDisabled());
    expect(el.hasAttribute("disabled")).toBe(false);
  });
});

describe("bindClass", () => {
  it("adds class when predicate is true", () => {
    const el = document.createElement("div");
    const store = makeStore({ highlight: true });
    bindClass(el, "highlighted", store, (s) => s.highlight);
    expect(el.classList.contains("highlighted")).toBe(true);
  });

  it("removes class when predicate becomes false", () => {
    const el = document.createElement("div");
    const store = makeStore({ highlight: true });
    bindClass(el, "highlighted", store, (s) => s.highlight);
    store.dispatch(new ToggleHighlight());
    expect(el.classList.contains("highlighted")).toBe(false);
  });

  it("supports multiple class names separated by space", () => {
    const el = document.createElement("div");
    const store = makeStore({ highlight: true });
    bindClass(el, "a b c", store, (s) => s.highlight);
    expect(el.classList.contains("a")).toBe(true);
    expect(el.classList.contains("b")).toBe(true);
    expect(el.classList.contains("c")).toBe(true);
    store.dispatch(new ToggleHighlight());
    expect(el.classList.contains("a")).toBe(false);
    expect(el.classList.contains("b")).toBe(false);
    expect(el.classList.contains("c")).toBe(false);
  });
});

describe("bindStyle", () => {
  it("sets style property", () => {
    const el = document.createElement("div");
    const store = makeStore();
    bindStyle(el, "color", store, (s) => s.color);
    expect(el.style.color).toBe("red");
    store.dispatch(new SetColor("blue"));
    expect(el.style.color).toBe("blue");
  });

  it("removes property when selector returns null", () => {
    const el = document.createElement("div");
    el.style.setProperty("color", "red");
    const store = makeStore();
    bindStyle(el, "color", store, (s) => (s.color === "" ? null : s.color));
    expect(el.style.color).toBe("red");
    store.dispatch(new ClearColor());
    expect(el.style.color).toBe("");
  });
});

describe("bindList", () => {
  it("renders initial list", () => {
    const container = document.createElement("ul");
    const store = makeStore();
    bindList(
      container,
      store,
      (s) => s.items,
      (item) => item.id,
      (item) => {
        const li = document.createElement("li");
        li.textContent = item.label;
        return li;
      },
    );
    expect(container.children.length).toBe(2);
    expect(container.children[0]?.textContent).toBe("Alpha");
    expect(container.children[1]?.textContent).toBe("Beta");
  });

  it("preserves DOM identity on stable keys", () => {
    const container = document.createElement("ul");
    const store = makeStore();
    bindList(
      container,
      store,
      (s) => s.items,
      (item) => item.id,
      (item) => {
        const li = document.createElement("li");
        li.textContent = item.label;
        return li;
      },
    );
    const liAlpha = container.children[0];
    const liBeta = container.children[1];
    // Re-dispatch with same items (different object refs, same ids)
    store.dispatch(
      new SetItems([
        { id: "a", label: "Alpha" },
        { id: "b", label: "Beta" },
      ]),
    );
    expect(container.children[0]).toBe(liAlpha);
    expect(container.children[1]).toBe(liBeta);
  });

  it("inserts new items", () => {
    const container = document.createElement("ul");
    const store = makeStore();
    bindList(
      container,
      store,
      (s) => s.items,
      (item) => item.id,
      (item) => {
        const li = document.createElement("li");
        li.textContent = item.label;
        return li;
      },
    );
    store.dispatch(
      new SetItems([
        { id: "a", label: "Alpha" },
        { id: "b", label: "Beta" },
        { id: "c", label: "Charlie" },
      ]),
    );
    expect(container.children.length).toBe(3);
    expect(container.children[2]?.textContent).toBe("Charlie");
  });

  it("removes items whose keys are gone", () => {
    const container = document.createElement("ul");
    const store = makeStore();
    bindList(
      container,
      store,
      (s) => s.items,
      (item) => item.id,
      (item) => {
        const li = document.createElement("li");
        li.textContent = item.label;
        return li;
      },
    );
    store.dispatch(new SetItems([{ id: "a", label: "Alpha" }]));
    expect(container.children.length).toBe(1);
    expect(container.children[0]?.textContent).toBe("Alpha");
  });

  it("reorders items on key shuffle", () => {
    const container = document.createElement("ul");
    const store = makeStore();
    bindList(
      container,
      store,
      (s) => s.items,
      (item) => item.id,
      (item) => {
        const li = document.createElement("li");
        li.textContent = item.label;
        return li;
      },
    );
    const originalA = container.children[0];
    const originalB = container.children[1];
    // Swap order
    store.dispatch(
      new SetItems([
        { id: "b", label: "Beta" },
        { id: "a", label: "Alpha" },
      ]),
    );
    // Same DOM nodes, swapped positions
    expect(container.children[0]).toBe(originalB);
    expect(container.children[1]).toBe(originalA);
  });
});
