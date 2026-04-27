export interface ExampleProgram {
  readonly id: string;
  readonly name: string;
  readonly summary: string;
  readonly source: string;
}

export const EXAMPLES: readonly ExampleProgram[] = [
  {
    id: "adder",
    name: "Add Two Digits",
    summary: "Function calls, typed locals, and a returned u4 value.",
    source: `fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() {
    let result: u4 = add(3, 4);
}`,
  },
  {
    id: "loop",
    name: "Count Up",
    summary: "A small counted loop that accumulates a running total.",
    source: `fn main() {
    let total: u4 = 0;

    for i in 0..4 {
        total = total +% 1;
    }
}`,
  },
  {
    id: "branch",
    name: "Tiny Branch",
    summary: "Conditional logic with explicit bool values.",
    source: `fn choose(flag: bool) -> u4 {
    if flag {
        return 9;
    }

    return 2;
}

fn main() {
    let picked: u4 = choose(true);
}`,
  },
];
