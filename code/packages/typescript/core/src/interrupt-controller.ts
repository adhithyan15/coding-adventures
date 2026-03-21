/**
 * InterruptController -- routes interrupts to cores.
 *
 * An interrupt is a signal that temporarily diverts the CPU from its current
 * work to handle an urgent event (timer, keyboard, network, IPI, syscall).
 *
 * This is a simplified shell -- it queues interrupts and routes them to
 * specific cores, but does not model priorities or masking.
 */

/** An interrupt waiting to be delivered. */
export interface PendingInterrupt {
  /** Identifies the interrupt source (e.g., timer=0, keyboard=1). */
  interruptID: number;
  /** Which core should handle it. -1 means "route to any available core". */
  targetCore: number;
}

/** Records a core acknowledging an interrupt. */
export interface AcknowledgedInterrupt {
  coreID: number;
  interruptID: number;
}

export class InterruptController {
  private _pending: PendingInterrupt[] = [];
  private _acknowledged: AcknowledgedInterrupt[] = [];
  private _numCores: number;

  constructor(numCores: number) {
    this._numCores = numCores;
  }

  /** Queues an interrupt for delivery. */
  raiseInterrupt(interruptID: number, targetCore: number): void {
    if (targetCore === -1) targetCore = 0;
    if (targetCore >= this._numCores) targetCore = 0;
    this._pending.push({ interruptID, targetCore });
  }

  /** Records that a core has begun handling an interrupt. */
  acknowledge(coreID: number, interruptID: number): void {
    this._acknowledged.push({ coreID, interruptID });

    // Remove from pending (first match only).
    let removed = false;
    this._pending = this._pending.filter(p => {
      if (!removed && p.interruptID === interruptID && p.targetCore === coreID) {
        removed = true;
        return false;
      }
      return true;
    });
  }

  /** Returns all pending interrupts targeted at a specific core. */
  pendingForCore(coreID: number): PendingInterrupt[] {
    return this._pending.filter(p => p.targetCore === coreID);
  }

  /** Returns the total number of pending (unacknowledged) interrupts. */
  pendingCount(): number {
    return this._pending.length;
  }

  /** Returns the total number of acknowledged interrupts. */
  acknowledgedCount(): number {
    return this._acknowledged.length;
  }

  /** Clears all pending and acknowledged interrupts. */
  reset(): void {
    this._pending = [];
    this._acknowledged = [];
  }
}
