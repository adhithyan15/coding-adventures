// element.ts — MosaicHostElement base class + defineMosaicElement
// helper for custom-element lifecycle integration.
//
// The HTML emitter targets a flat DOM tree where the host wires
// bindings on initialisation. The Web Component emitter targets the
// same architecture but encapsulated inside a custom element with
// (typically) a shadow root. The lifecycle is different:
//
//   1. The element is constructed (constructor)
//   2. The browser inserts it into the DOM (connectedCallback)
//   3. The element may move within the DOM (disconnect+connect again)
//   4. The browser removes it from the DOM (disconnectedCallback)
//
// MosaicHostElement standardises the binding lifecycle:
//
//   * Subclass extends MosaicHostElement
//   * Subclass overrides bindStore(store) to wire bindings; it
//     accumulates returned unsubscribe fns via this.track(...)
//   * The base class invokes bindStore in connectedCallback when a
//     store has been set, and tears down all tracked unsubscribes
//     in disconnectedCallback
//
// This means subclasses never have to remember to manage cleanup —
// every binding registered via this.track() is automatically
// disposed when the element leaves the DOM, eliminating a common
// memory-leak class.

import type { MosaicStore } from "./store.js";
import type { MosaicAction } from "./action.js";

/**
 * Cleanup function returned by binding helpers.
 */
export type Unsubscribe = () => void;

/**
 * Base class for Mosaic-aware custom elements.
 *
 * Lifecycle:
 *
 *   const el = new SomeMosaicElement();         // constructor
 *   el.store = myStore;                          // set the store
 *   document.body.appendChild(el);               // → connectedCallback → bindStore
 *   document.body.removeChild(el);               // → disconnectedCallback → cleanup
 *
 * Or via attribute / property:
 *
 *   const el = document.createElement("my-mosaic-element");
 *   (el as SomeMosaicElement).store = myStore;
 *   document.body.appendChild(el);
 *
 * Subclasses override bindStore(store) to register bindings.
 * Use this.track(unsubscribe) for each binding so it's cleaned up
 * automatically when the element disconnects.
 */
export abstract class MosaicHostElement<State> extends HTMLElement {
  #store: MosaicStore<State> | null = null;
  #unsubscribes: Unsubscribe[] = [];
  #connected = false;
  #shadow: ShadowRoot | null = null;

  /**
   * The bound store. Setting this property after the element has
   * connected triggers a rebind (old bindings disposed, bindStore
   * re-invoked).
   */
  get store(): MosaicStore<State> | null {
    return this.#store;
  }

  set store(value: MosaicStore<State> | null) {
    if (this.#store === value) return;
    if (this.#connected) {
      this.#cleanup();
    }
    this.#store = value;
    if (this.#connected && value !== null) {
      this.bindStore(value);
    }
  }

  /**
   * Attach an open shadow root if one hasn't been attached yet.
   * Returns the shadow root. Idempotent — calling repeatedly returns
   * the same root.
   *
   * Subclasses typically call this once in connectedCallback (before
   * super.connectedCallback(), or via override of bindStore).
   */
  attachShadowIfNeeded(): ShadowRoot {
    if (this.#shadow !== null) {
      return this.#shadow;
    }
    // shadowRoot may already exist if a subclass attached one
    // independently; respect that.
    if (this.shadowRoot !== null) {
      this.#shadow = this.shadowRoot;
      return this.#shadow;
    }
    this.#shadow = this.attachShadow({ mode: "open" });
    return this.#shadow;
  }

  /**
   * Register an unsubscribe to be invoked on disconnect. Returns the
   * unsubscribe for chaining or immediate invocation if desired.
   */
  track(unsubscribe: Unsubscribe): Unsubscribe {
    this.#unsubscribes.push(unsubscribe);
    return unsubscribe;
  }

  /**
   * Dispatch shortcut. Throws if no store is bound.
   */
  dispatch<A extends MosaicAction<State>>(action: A): void {
    if (this.#store === null) {
      throw new Error(
        `${this.constructor.name}: cannot dispatch without a store. Set element.store first.`,
      );
    }
    this.#store.dispatch(action);
  }

  /**
   * Subclasses implement this to wire bindings against the store.
   * Use this.track(...) for every unsubscribe returned by a binding
   * helper.
   *
   * Called once per connect-with-store cycle: when the element is
   * connected with a store set, or when the store is reassigned
   * while connected.
   */
  protected abstract bindStore(store: MosaicStore<State>): void;

  /**
   * Native lifecycle hook. Subclasses calling super.connectedCallback
   * preserve the auto-bind behaviour. Subclasses that don't call
   * super must implement their own bindStore invocation.
   */
  connectedCallback(): void {
    this.#connected = true;
    if (this.#store !== null) {
      this.bindStore(this.#store);
    }
  }

  /**
   * Native lifecycle hook. Disposes all bindings registered via
   * this.track().
   */
  disconnectedCallback(): void {
    this.#connected = false;
    this.#cleanup();
  }

  #cleanup(): void {
    const toRun = this.#unsubscribes;
    this.#unsubscribes = [];
    for (const u of toRun) {
      try {
        u();
      } catch {
        // A bad unsubscribe shouldn't prevent the others from running.
        // This is rare (most just remove from a Set), but defending
        // here means a buggy custom binding can't leak the rest.
      }
    }
  }
}

/**
 * Register a custom element class with `customElements.define`.
 *
 * Calling defineMosaicElement is idempotent per tag name: if the
 * tag is already registered (for example because a previous bundle
 * ran), the existing definition wins and no error is thrown.
 *
 * @param tagName - The custom element tag (must contain a hyphen)
 * @param elementClass - The class extending MosaicHostElement
 * @param options - Optional ElementDefinitionOptions (e.g., extends)
 */
export function defineMosaicElement(
  tagName: string,
  elementClass: CustomElementConstructor,
  options?: ElementDefinitionOptions,
): void {
  // customElements.get returns the existing constructor if any.
  // We don't overwrite — the first definition wins. This matches
  // the browser's behaviour (a second define() throws), but we
  // swallow the duplicate-registration to keep idempotent dev-time
  // builds working under HMR/module re-execution.
  if (typeof customElements === "undefined") {
    throw new Error(
      "defineMosaicElement: customElements is not available in this environment.",
    );
  }
  if (customElements.get(tagName) !== undefined) {
    return;
  }
  customElements.define(tagName, elementClass, options);
}
