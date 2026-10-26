/** Train XOR, or resume a portable checkpoint supplied on stdin with --resume. */
import { readFileSync } from "node:fs";
import { DenseNetwork } from "../src/index.js";

const inputs = [[0, 0], [0, 1], [1, 0], [1, 1]], targets = [[0], [1], [1], [0]];
const resume = process.argv.includes("--resume");
const model = resume ? DenseNetwork.fromCheckpoint(JSON.parse(readFileSync(0, "utf8")))
  : DenseNetwork.seeded([2, 4, 1], ["tanh", "sigmoid"], 7);
const losses = model.fit(inputs, targets, resume ? 1 : 3000, 1);
console.log(JSON.stringify({ checkpoint: model.toCheckpoint(), predictions: model.predict(inputs),
  initial_loss: losses[0], final_loss: losses.at(-1) }));
