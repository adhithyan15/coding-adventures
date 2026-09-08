const mosaic$tableCapacities = new WeakMap<HTMLTableElement, { rows?: number; pitch?: number; wheelRows?: number; offset?: number; virtualPitch?: number; requestedOffset?: number; problem?: string }>();

// A callback-ref factory supports both React 18's null cleanup and React 19.
// Last capacity belongs to the DOM table, not the render's callback identity.
export function mosaic$tableCapacityRef(onRows: (rows: number) => void, reveal?: (table: HTMLTableElement) => void, window?: { offset: number; total: number; shift: (rows: number) => void }): (table: HTMLTableElement | null) => void {
  let dispose: (() => void) | undefined;
  return table => {
    dispose?.();
    dispose = undefined;
    if (!table) return;
    const state = mosaic$tableCapacities.get(table) ?? {};
    mosaic$tableCapacities.set(table, state);
    const warn = (problem: string) => {
      if (state.problem !== problem) console.warn(`Mosaic HostTable: ${problem}`);
      state.problem = problem;
    };
    if (typeof ResizeObserver === 'undefined') { warn('viewport capacity observation is unavailable'); return; }
    let frame = table.parentElement;
    while (frame && frame !== document.body && !/(auto|scroll)/.test(`${getComputedStyle(frame).overflowX} ${getComputedStyle(frame).overflowY}`)) frame = frame.parentElement;
    if (!frame || frame === document.body) { warn('viewport capacity requires a bounded scroll frame'); return; }
    const scrollFrame = frame;
    const body = Array.from(table.tBodies).find(section => !section.hasAttribute('data-mosaic-spacer'));
    const before = table.querySelector<HTMLTableRowElement>('tbody[data-mosaic-spacer="before"] > tr');
    const after = table.querySelector<HTMLTableRowElement>('tbody[data-mosaic-spacer="after"] > tr');
    const virtual = !!(window && before && after);
    if (!virtual) reveal?.(table);
    const maximum = () => Math.max(0, (window?.total ?? 0) - (body?.rows.length ?? 0));
    const origin = () => table.getBoundingClientRect().top - scrollFrame.getBoundingClientRect().top - scrollFrame.clientTop + scrollFrame.scrollTop;
    let live = true;
    let pending = 0;
    const measure = () => {
      pending = 0;
      state.pitch = undefined;
      if (!live || !table.isConnected || !scrollFrame.clientHeight) return;
      const rows = Array.from(body?.rows ?? []);
      if (!rows.length) return;
      const rects = rows.map(row => row.getBoundingClientRect());
      if (rects.some(rect => rect.height <= 0)) return;
      const spacing = getComputedStyle(table).borderCollapse === 'collapse' ? 0 : parseFloat(getComputedStyle(table).borderSpacing.split(' ').pop() ?? '0') || 0;
      // Collapsed table borders can add half a border to the first row's
      // interval. Use the median interval so showing/hiding a spacer does not
      // change the logical pitch or move the thumb at the workbook boundary.
      const intervals = rects.slice(1).map((rect, i) => rect.top - rects[i].top).sort((a, b) => a - b);
      const pitch = intervals.length ? intervals[Math.floor(intervals.length / 2)] : rects[0].height + spacing;
      if (pitch <= 0) return;
      if (rects.some((rect, i) => Math.abs(rect.height - rects[0].height) > 0.5 || (i > 0 && Math.abs(rect.top - rects[i - 1].top - pitch) > 0.5))) {
        warn('variable-height rows are not supported by uniform viewport capacity'); return;
      }
      const pinned = [table.tHead, table.tFoot].reduce((height, section) => height + (section && getComputedStyle(section).position === 'sticky' ? section.getBoundingClientRect().height : 0), 0);
      const available = scrollFrame.clientHeight - pinned;
      if (available <= 0) return;
      state.problem = undefined;
      state.pitch = pitch;
      if (virtual && window && before && after) {
        // Do not let browser scroll anchoring fight the authoritative row window.
        table.style.overflowAnchor = 'none';
        before.style.display = window.offset ? '' : 'none';
        after.style.display = window.total > window.offset + rows.length ? '' : 'none';
        before.style.height = `${window.offset * pitch}px`;
        after.style.height = `${Math.max(0, window.total - window.offset - rows.length) * pitch}px`;
        const fromScroll = state.requestedOffset === window.offset;
        if (!fromScroll && (state.offset !== window.offset || Math.abs((state.virtualPitch ?? pitch) - pitch) > 0.5)) scrollFrame.scrollTop = origin() + window.offset * pitch;
        state.offset = window.offset;
        state.virtualPitch = pitch;
        state.requestedOffset = undefined;
        if (!fromScroll) reveal?.(table);
      }
      // Include the partial trailing row in a physical scroll window.
      const capacity = Math.max(1, virtual ? Math.ceil(available / pitch) + 1 : Math.floor(available / pitch));
      if (capacity === state.rows) return;
      state.rows = capacity;
      onRows(capacity);
    };
    const schedule = () => { if (live && !pending) pending = requestAnimationFrame(measure); };
    const observer = new ResizeObserver(schedule);
    const wheel = (event: WheelEvent) => {
      if (!live || !window || !state.pitch || event.defaultPrevented || !event.cancelable || event.ctrlKey || event.metaKey || event.shiftKey || Math.abs(event.deltaX) >= Math.abs(event.deltaY)) return;
      if (!(event.target instanceof Element) || event.target.closest('table') !== table) return;
      if (event.target instanceof Element && event.target.closest('input, textarea, select, [contenteditable="true"]')) return;
      const limit = maximum();
      if ((event.deltaY < 0 && window.offset <= 0) || (event.deltaY > 0 && window.offset >= limit)) { state.wheelRows = 0; return; }
      const amount = event.deltaY * (event.deltaMode === 1 ? 1 : event.deltaMode === 2 ? (state.rows ?? 1) : 1 / state.pitch);
      if (!Number.isFinite(amount) || amount === 0) return;
      event.preventDefault();
      const previous = state.wheelRows ?? 0;
      const accumulated = (Math.sign(previous) === Math.sign(amount) ? previous : 0) + amount;
      const rows = Math.trunc(accumulated);
      state.wheelRows = accumulated - rows;
      if (rows) window.shift(rows);
    };
    const scroll = () => {
      if (!live || !virtual || !window || !state.pitch) return;
      const offset = Math.max(0, Math.min(maximum(), Math.floor((scrollFrame.scrollTop - origin()) / state.pitch)));
      const previous = state.requestedOffset ?? window.offset;
      if (offset === previous) return;
      state.requestedOffset = offset;
      window.shift(offset - previous);
    };
    if (window) scrollFrame.addEventListener('wheel', wheel, { passive: false });
    if (virtual) scrollFrame.addEventListener('scroll', scroll, { passive: true });
    observer.observe(scrollFrame);
    observer.observe(table);
    if (table.tHead) observer.observe(table.tHead);
    if (table.tFoot) observer.observe(table.tFoot);
    for (const row of Array.from(body?.rows ?? [])) observer.observe(row);
    schedule();
    dispose = () => { live = false; observer.disconnect(); scrollFrame.removeEventListener('wheel', wheel); scrollFrame.removeEventListener('scroll', scroll); if (pending) cancelAnimationFrame(pending); };
  };
}
