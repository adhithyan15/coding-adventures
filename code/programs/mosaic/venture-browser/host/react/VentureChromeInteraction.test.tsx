import { act } from "react";
import { afterAll, expect, test, vi } from "vitest";

type HostRequest = {
  component: string;
  event: { type: string; value?: string };
};

const events: HostRequest[] = [];
let navigationDisabled = true;
let stopDisabled = true;
let bookmarked = false;
let bookmarksOpen = false;
let historyOpen = false;
let historyPosition = 2;
let findOpen = false;
let findQuery = "";
let findResultLabel = "";
let zoomPercent = 100;
let pageInfoOpen = false;
let viewSourceOpen = false;
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const props = (statusText: string) => ({
  props: {
    address: "http://venture.test/start",
    pageTitle: "Venture React acceptance",
    statusText,
    backDisabled: navigationDisabled,
    forwardDisabled: navigationDisabled,
    stopDisabled,
    bookmarkLabel: bookmarked ? "Remove Bookmark" : "Bookmark",
    bookmarkDisabled: navigationDisabled,
    bookmarksLabel: `Bookmarks (${bookmarked ? 1 : 0})`,
    bookmarksDisabled: navigationDisabled || !bookmarked,
    bookmarksOpen,
    bookmarksPosition: bookmarked ? "1 of 1" : "",
    bookmarksTitle: bookmarked ? "Venture React acceptance" : "",
    bookmarksAddress: bookmarked ? "http://venture.test/start" : "",
    bookmarksPreviousDisabled: true,
    bookmarksNextDisabled: true,
    bookmarksNavigateDisabled: navigationDisabled || !bookmarked,
    historyLabel: "History (2)",
    historyDisabled: navigationDisabled,
    historyOpen,
    historyPosition: `${historyPosition} of 2`,
    historyAddress: historyPosition === 1 ? "http://venture.test/previous" : "http://venture.test/start",
    historyPreviousDisabled: navigationDisabled,
    historyNextDisabled: navigationDisabled,
    historyNavigateDisabled: navigationDisabled || historyPosition === 2,
    copyAddressDisabled: navigationDisabled,
    openPageDisabled: navigationDisabled,
    savePageDisabled: navigationDisabled,
    printPageDisabled: navigationDisabled,
    sharePageDisabled: navigationDisabled,
    pageInfoDisabled: navigationDisabled,
    pageInfoOpen,
    pageInfoTitle: "Venture React acceptance",
    pageInfoAddress: "http://venture.test/final",
    pageInfoRequestedAddress: "http://venture.test/start",
    pageInfoStatus: "HTTP 200",
    pageInfoResources: "Images: 2 (0 failed)  Stylesheets: 1 (0 failed)",
    zoomLabel: `${zoomPercent}%`,
    zoomOutDisabled: navigationDisabled || zoomPercent === 50,
    zoomResetDisabled: navigationDisabled || zoomPercent === 100,
    zoomInDisabled: navigationDisabled || zoomPercent === 200,
    viewSourceDisabled: navigationDisabled,
    viewSourceOpen,
    viewSourceTitle: "Venture React acceptance",
    viewSourceAddress: "http://venture.test/final",
    viewSourceContent: "<html><title>Venture React acceptance</title></html>",
    findOpen,
    findQuery,
    findResultLabel,
    findDisabled: navigationDisabled,
    navigationDisabled,
    contentSurface: "React host surface",
  },
});

window.mosaicHost = {
  getProps: vi.fn(async () => props("Ready")),
  handleEvent: vi.fn(async (request: HostRequest) => {
    events.push(request);
    if (request.event.type === "toggleBookmark") {
      bookmarked = !bookmarked;
      bookmarksOpen = bookmarksOpen && bookmarked;
      return props("Bookmark persisted through MosaicHost");
    }
    if (request.event.type === "bookmarksOpen") {
      bookmarksOpen = true;
      return props("Bookmarks opened through MosaicHost");
    }
    if (request.event.type === "bookmarksClose") {
      bookmarksOpen = false;
      return props("Bookmarks closed through MosaicHost");
    }
    if (request.event.type === "bookmarksNavigate") {
      bookmarksOpen = false;
      return props("Bookmark opened through MosaicHost");
    }
    if (request.event.type === "historyOpen") {
      historyOpen = true;
      return props("History opened through MosaicHost");
    }
    if (request.event.type === "historyPrevious") {
      historyPosition = 1;
      return props("History selection changed through MosaicHost");
    }
    if (request.event.type === "historyClose" || request.event.type === "historyNavigate") {
      historyOpen = false;
      return props("History closed through MosaicHost");
    }
    if (request.event.type === "findChange") {
      findQuery = request.event.value ?? "";
      findResultLabel = findQuery ? "1 of 2" : "";
      return props("Find updated through MosaicHost");
    }
    if (request.event.type === "findOpen") {
      findOpen = true;
      return props("Find opened through MosaicHost");
    }
    if (request.event.type === "findClose") {
      findOpen = false;
      findQuery = "";
      findResultLabel = "";
      return props("Find closed through MosaicHost");
    }
    if (request.event.type === "findNext") {
      findResultLabel = "2 of 2";
      return props("Find advanced through MosaicHost");
    }
    if (request.event.type === "zoomIn") {
      zoomPercent += 25;
      return props("Zoom updated through MosaicHost");
    }
    if (request.event.type === "zoomReset") {
      zoomPercent = 100;
      return props("Zoom reset through MosaicHost");
    }
    if (request.event.type === "pageInfo") {
      pageInfoOpen = true;
      return props("Page information shown through MosaicHost");
    }
    if (request.event.type === "pageInfoClose") {
      pageInfoOpen = false;
      return props("Page information closed through MosaicHost");
    }
    if (request.event.type === "viewSource") {
      viewSourceOpen = true;
      return props("Page source shown through MosaicHost");
    }
    if (request.event.type === "viewSourceClose") {
      viewSourceOpen = false;
      return props("Page source closed through MosaicHost");
    }
    if (request.event.type === "stop") {
      stopDisabled = true;
      return props("Loading stopped through MosaicHost");
    }
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
  for (const label of ["Back", "Forward", "Reload", "Stop", "Bookmark", "Bookmarks (0)", "History (2)", "Copy", "New Window", "Save", "Print", "Share", "Info", "Zoom Out", "100%", "Zoom In", "Source", "Find", "Go"]) {
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

  stopDisabled = false;
  await act(async () => {
    window.dispatchEvent(new Event("mosaic-host-ready"));
  });
  await flush();
  await act(async () => {
    textButton("Stop").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("stop");
  expect(textButton("Stop").disabled).toBe(true);
  expect(document.body.textContent).toContain("Loading stopped through MosaicHost");

  await act(async () => {
    textButton("Zoom In").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("zoomIn");
  expect(document.body.textContent).toContain("125%");
  await act(async () => {
    textButton("125%").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("zoomReset");

  await act(async () => {
    textButton("Bookmark").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("toggleBookmark");
  expect(document.body.textContent).toContain("Remove Bookmark");
  expect(document.body.textContent).toContain("Bookmark persisted through MosaicHost");

  await act(async () => {
    textButton("Bookmarks (1)").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("bookmarksOpen");
  expect(document.body.textContent).toContain("1 of 1");
  expect(document.body.textContent).toContain("http://venture.test/start");
  await act(async () => {
    textButton("Close").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("bookmarksClose");

  await act(async () => {
    textButton("History (2)").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("historyOpen");
  expect(document.body.textContent).toContain("2 of 2");
  await act(async () => {
    textButton("Previous").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("historyPrevious");
  expect(document.body.textContent).toContain("http://venture.test/previous");
  await act(async () => {
    textButton("Open").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("historyNavigate");

  await act(async () => {
    textButton("Copy").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("copyAddress");

  await act(async () => {
    textButton("New Window").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("openPageInNewWindow");

  await act(async () => {
    textButton("Save").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("savePage");

  await act(async () => {
    textButton("Print").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("printPage");

  await act(async () => {
    textButton("Share").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("sharePage");

  await act(async () => {
    textButton("Info").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("pageInfo");
  expect(document.body.textContent).toContain("Venture React acceptance");
  expect(document.body.textContent).toContain("HTTP 200");
  await act(async () => {
    textButton("Close").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("pageInfoClose");
  expect(document.body.textContent).not.toContain("Images: 2 (0 failed)");

  await act(async () => {
    textButton("Source").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("viewSource");
  expect(document.body.textContent).toContain("<html><title>Venture React acceptance</title></html>");
  expect(document.body.textContent).toContain("http://venture.test/final");
  await act(async () => {
    textButton("Copy Source").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("viewSourceCopy");
  expect(document.body.textContent).toContain("<html><title>Venture React acceptance</title></html>");
  await act(async () => {
    textButton("Close Source").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("viewSourceClose");
  expect(document.body.textContent).not.toContain("<html><title>Venture React acceptance</title></html>");

  expect(document.querySelector('input[placeholder="Find in page"]')).toBeNull();
  await act(async () => {
    textButton("Find").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("findOpen");
  const find = [...document.querySelectorAll('input[type="text"]')].find(
    candidate =>
      candidate instanceof HTMLInputElement && candidate.placeholder === "Find in page",
  ) as HTMLInputElement;
  await act(async () => {
    const valueSetter = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    valueSetter?.call(find, "venture");
    find.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await flush();
  expect(events[events.length - 1]?.event).toEqual({ type: "findChange", value: "venture" });
  expect(document.body.textContent).toContain("1 of 2");
  await act(async () => {
    textButton("Next").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("findNext");
  expect(document.body.textContent).toContain("2 of 2");
  await act(async () => {
    textButton("Close Find").click();
  });
  await flush();
  expect(events[events.length - 1]?.event.type).toBe("findClose");
  expect(document.querySelector('input[placeholder="Find in page"]')).toBeNull();

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
