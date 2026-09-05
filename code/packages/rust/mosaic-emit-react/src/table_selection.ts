// Selection coordinates refer to rendered body rows and cells. Reveal only
// within the nearest scroll frame; do not move document scroll or keyboard focus.
function mosaic$revealTableCell(table: HTMLTableElement | null, row: number, col: number): void {
  if (!table || !Number.isInteger(row) || !Number.isInteger(col) || row < 0 || col < 0) return;
  const cell = table.tBodies[0]?.rows[row]?.cells[col];
  if (!cell) return;
  for (let frame = table.parentElement; frame && frame !== document.body; frame = frame.parentElement) {
    const style = getComputedStyle(frame);
    if (!/(auto|scroll)/.test(`${style.overflowX} ${style.overflowY}`)) continue;
    const bounds = frame.getBoundingClientRect();
    const target = cell.getBoundingClientRect();
    const head = table.tHead;
    const inset = head && getComputedStyle(head).position === 'sticky' ? head.getBoundingClientRect().height : 0;
    const top = bounds.top + frame.clientTop + inset;
    const bottom = bounds.top + frame.clientTop + frame.clientHeight;
    const left = bounds.left + frame.clientLeft;
    const right = left + frame.clientWidth;
    if (target.top < top) frame.scrollTop += target.top - top;
    else if (target.bottom > bottom) frame.scrollTop += target.bottom - bottom;
    if (target.left < left) frame.scrollLeft += target.left - left;
    else if (target.right > right) frame.scrollLeft += target.right - right;
    return;
  }
}

