// Selection coordinates refer to rendered body rows and cells. Reveal only
// within the nearest scroll frame; do not move document scroll or keyboard focus.
export function mosaic$revealTableCell(table: HTMLTableElement | null, row: number, col: number): void {
  if (!table || !Number.isInteger(row) || !Number.isInteger(col) || row < 0 || col < 0) return;
  const renderedRow = table.tBodies[0]?.rows[row];
  const cell = renderedRow && Array.from(renderedRow.cells).filter(cell => cell.tagName === 'TD')[col];
  if (!cell) return;
  for (let frame = table.parentElement; frame && frame !== document.body; frame = frame?.parentElement ?? null) {
    const style = getComputedStyle(frame);
    if (!/(auto|scroll)/.test(`${style.overflowX} ${style.overflowY}`)) continue;
    const bounds = frame.getBoundingClientRect();
    const target = cell.getBoundingClientRect();
    const head = table.tHead;
    const inset = head && getComputedStyle(head).position === 'sticky' ? head.getBoundingClientRect().height : 0;
    const top = bounds.top + frame.clientTop + inset;
    const bottom = bounds.top + frame.clientTop + frame.clientHeight;
    let left = bounds.left + frame.clientLeft;
    let right = left + frame.clientWidth;
    // Logical leading headers may be on either side. Reserve their actual
    // visible rectangles instead of making assumptions about scrollLeft in RTL.
    for (const header of Array.from(renderedRow?.cells ?? [])) {
      if (header.tagName !== 'TH' || getComputedStyle(header).position !== 'sticky') continue;
      const rect = header.getBoundingClientRect();
      if (getComputedStyle(table).direction === 'rtl') right = Math.min(right, rect.left);
      else left = Math.max(left, rect.right);
    }
    if (target.top < top) frame.scrollTop += target.top - top;
    else if (target.bottom > bottom) frame.scrollTop += target.bottom - bottom;
    if (target.left < left) frame.scrollLeft += target.left - left;
    else if (target.right > right) frame.scrollLeft += target.right - right;
    return;
  }
}
