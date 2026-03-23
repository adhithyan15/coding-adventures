import { describe, it, expect } from "vitest";
import { render, screen, act } from "@testing-library/react";
import "@testing-library/jest-dom";
import { Store } from "../store.js";
import { useStore } from "../use-store.js";
import type { Action } from "../types.js";

interface TestState {
  value: string;
}

const testReducer = (state: TestState, action: Action): TestState => {
  if (action.type === "SET") return { value: action.value as string };
  return state;
};

function TestComponent({ store }: { store: Store<TestState> }) {
  const state = useStore(store);
  return <div data-testid="value">{state.value}</div>;
}

describe("useStore", () => {
  it("renders initial state", () => {
    const store = new Store({ value: "hello" }, testReducer);
    render(<TestComponent store={store} />);
    expect(screen.getByTestId("value")).toHaveTextContent("hello");
  });

  it("re-renders on dispatch", () => {
    const store = new Store({ value: "hello" }, testReducer);
    render(<TestComponent store={store} />);
    act(() => store.dispatch({ type: "SET", value: "world" }));
    expect(screen.getByTestId("value")).toHaveTextContent("world");
  });

  it("does not re-render after unmount", () => {
    const store = new Store({ value: "hello" }, testReducer);
    const { unmount } = render(<TestComponent store={store} />);
    unmount();
    // Should not throw even though component is unmounted
    expect(() =>
      store.dispatch({ type: "SET", value: "world" }),
    ).not.toThrow();
  });
});
