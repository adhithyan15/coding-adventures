/** Typed failures shared by the public simulator and internal timing model. */
export class HddError extends Error {
  constructor(message: string) {
    super(message);
    this.name = new.target.name;
  }
}

export class InvalidLbaError extends HddError {}
export class InvalidSectorDataError extends HddError {}
export class ClockOverflowError extends HddError {}
