import { act } from "react";
import { afterAll, expect, test, vi } from "vitest";

type HostRequest = {
  component: string;
  event: { type: string; value?: string };
};

const events: HostRequest[] = [];
let navigationDisabled = true;
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const props = (statusText: string) => ({
  props: {
    address: "http://venture.test/start",
    pageTitle: "Venture React acceptance",
    statusText,
    backDisabled: navigationDisabled,
    forwardDisabled: navigationDisabled,
    navigationDisabled,
    contentSurface: "React host surface",
  },
});

window.mosaicHost = {
  getProps: vi.fn(async () => props("Ready")),
  handleEvent: vi.fn(async (request: HostRequest) => {
    events.push(request);
    return request.event.type === "navigate"
      ? props("Navigated through MosaicHost")
      : undefined;
  }),
};

const textButton = (label: string): HTMLButtonElement => {
  const button = [...document.querySelectorAll("button")].find(
    candidate => candidate.textContent === label,
  );
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`missing native ${label} button`);
  }
  return button;
};

const flush = async () => {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
};

test("React and Electron renderer controls cross the Mosaic host seam", async () => {
  document.body.innerHTML = '<div id="root"></div>';
  await act(async () => {
    await import("./main");
  });
  await flush();

  expect(document.body.textContent).toContain("Venture React acceptance");
  expect(document.body.textContent).toContain("React host surface");
  for (const label of ["Back", "Forward", "Reload", "Go"]) {
    const button = textButton(label);
    expect(button.disabled).toBe(true);
    button.click();
  }
  const address = document.querySelector('input[type="text"]');
  expect(address).toBeInstanceOf(HTMLInputElement);
  expect((address as HTMLInputElement).readOnly).toBe(true);
  expect(events).toEqual([]);

  navigationDisabled = false;
  await act(async () => {
    window.dispatchEvent(new Event("mosaic-host-ready"));
  });
  await flush();

  const enabledAddress = document.querySelector('input[type="text"]') as HTMLInputElement;
  expect(enabledAddress.readOnly).toBe(false);
  await act(async () => {
    const valueSetter = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    valueSetter?.call(enabledAddress, "http://venture.test/next");
    enabledAddress.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await flush();
  expect(events[events.length - 1]).toEqual({
    component: "VentureChrome",
    event: { type: "addressChange", value: "http://venture.test/next" },
  });

  await act(async () => {
    enabledAddress.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("navigate");
  expect(document.body.textContent).toContain("Navigated through MosaicHost");

  await act(async () => {
    textButton("Go").click();
  });
  await flush();
  expect(events.filter(request => request.event.type === "navigate")).toHaveLength(2);
});

afterAll(() => {
  delete window.mosaicHost;
  delete window.__mosaicReactRoot;
});
