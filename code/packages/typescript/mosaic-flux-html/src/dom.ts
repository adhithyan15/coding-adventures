// dom.ts — vanilla-DOM binding helpers.
//
// The HTML emitter produces static-ish DOM trees where dynamic
// content comes from store selectors. These helpers wire a selector
// to a DOM update so the emitter doesn't have to write subscribe +
// callback boilerplate everywhere.
//
// Each bind* function returns an unsubscribe function for cleanup.
// Callers (typically the emitted hydrator code) accumulate them and
// invoke all unsubscribes on detach.
//
// Why no virtual DOM, no diffing engine, no template compiler:
// strict-Flux + fine-grained subscriptions makes targeted DOM updates
// trivial. A text node bound to a state slice updates that text node
// ONLY when the slice changes. A 100-row list bound to a selector
// updates ONLY the rows whose data changed (when bindList is given a
// key function). The whole-DOM-diff approach React/Vue/etc. take is
// solving a problem we don't have because our updates are already
// scoped.

import type { MosaicStore } from "./store.js";

/**
 * Bind the textContent of an element to a string selector.
 *
 * Updates only when the selected string changes (by Object.is).
 */
export function bindText<State>(
  el: Element,
  store: MosaicStore<State>,
  selector: (state: State) => string,
): () => void {
  el.textContent = store.select(selector);
  return store.subscribe(selector, (value) => {
    el.textContent = value;
  });
}

/**
 * Bind an element attribute to a string selector. Setting the
 * attribute to `null` removes it (matching the convention where
 * absence is meaningful, e.g., `disabled`, `aria-expanded`).
 */
export function bindAttr<State>(
  el: Element,
  attrName: string,
  store: MosaicStore<State>,
  selector: (state: State) => string | null,
): () => void {
  applyAttr(el, attrName, store.select(selector));
  return store.subscribe(selector, (value) => applyAttr(el, attrName, value));
}

function applyAttr(el: Element, attrName: string, value: string | null): void {
  if (value === null) {
    el.removeAttribute(attrName);
  } else {
    el.setAttribute(attrName, value);
  }
}

/**
 * Bind a class to a boolean selector. Adds the class when the
 * predicate is true; removes it when false. Multiple class names
 * separated by spaces are supported.
 */
export function bindClass<State>(
  el: Element,
  className: string,
  store: MosaicStore<State>,
  predicate: (state: State) => boolean,
): () => void {
  applyClass(el, className, store.select(predicate));
  return store.subscribe(predicate, (value) => applyClass(el, className, value));
}

function applyClass(el: Element, className: string, present: boolean): void {
  const classes = className.split(/\s+/).filter(Boolean);
  for (const c of classes) {
    if (present) {
      el.classList.add(c);
    } else {
      el.classList.remove(c);
    }
  }
}

/**
 * Bind an inline style property to a selector. Setting the value to
 * `null` removes the property.
 */
export function bindStyle<State>(
  el: HTMLElement,
  prop: string,
  store: MosaicStore<State>,
  selector: (state: State) => string | null,
): () => void {
  applyStyle(el, prop, store.select(selector));
  return store.subscribe(selector, (value) => applyStyle(el, prop, value));
}

function applyStyle(el: HTMLElement, prop: string, value: string | null): void {
  if (value === null) {
    el.style.removeProperty(prop);
  } else {
    el.style.setProperty(prop, value);
  }
}

/**
 * Bind a container's children to a list selector with key-based
 * reconciliation.
 *
 * On each list change, the helper computes a key-keyed diff and:
 *   - inserts a freshly-rendered child for new keys
 *   - moves existing children to match new ordering
 *   - removes children whose keys are gone
 *   - leaves children with unchanged keys in place (no re-render)
 *
 * This is the cheapest list update strategy that maintains DOM
 * identity for unchanged items (so focus, selection, scroll, and
 * event listeners survive).
 *
 * @param container - The element whose children mirror the list
 * @param store - The MosaicStore providing reactivity
 * @param listSelector - Returns the current list
 * @param keyFn - Returns a stable string key per item (must be unique)
 * @param renderItem - Renders a fresh DOM node for an item
 * @returns Unsubscribe function
 */
export function bindList<State, Item>(
  container: Element,
  store: MosaicStore<State>,
  listSelector: (state: State) => ReadonlyArray<Item>,
  keyFn: (item: Item, index: number) => string,
  renderItem: (item: Item) => Node,
): () => void {
  // Map from key → existing rendered DOM node
  const existing = new Map<string, Node>();

  const reconcile = (list: ReadonlyArray<Item>): void => {
    const seen = new Set<string>();
    let cursor: Node | null = container.firstChild;
    list.forEach((item, index) => {
      const key = keyFn(item, index);
      seen.add(key);
      let node = existing.get(key);
      if (node === undefined) {
        node = renderItem(item);
        existing.set(key, node);
      }
      if (cursor === node) {
        cursor = node.nextSibling;
      } else {
        container.insertBefore(node, cursor);
        // Note: insertBefore moves the node if it's already mounted.
        // cursor is unchanged because we inserted *before* it.
      }
    });
    // Remove anything past the cursor that no longer belongs.
    while (cursor !== null) {
      const next: Node | null = cursor.nextSibling;
      container.removeChild(cursor);
      cursor = next;
    }
    // Clean up the existing-map for keys no longer in the list.
    for (const k of existing.keys()) {
      if (!seen.has(k)) existing.delete(k);
    }
  };

  reconcile(store.select(listSelector));
  return store.subscribe(listSelector, reconcile);
}
