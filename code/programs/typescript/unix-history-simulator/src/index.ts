import "./style.css";
import { Session, type SyscallEvent } from "@coding-adventures/unix-history-simulator";

const session = new Session();
const form = document.querySelector<HTMLFormElement>("#command-form")!;
const command = document.querySelector<HTMLInputElement>("#command")!;
const output = document.querySelector("#output")!;
const prompt = document.querySelector("#prompt")!;
const trace = document.querySelector("#trace")!;
const emptyTrace = document.querySelector<HTMLElement>("#trace-empty")!;
const descriptors = document.querySelector("#descriptors")!;
const status = document.querySelector("#status")!;
const step = document.querySelector<HTMLButtonElement>("#step")!;
const run = document.querySelector<HTMLButtonElement>("#run")!;
let loadedCommand = "";

function describe(event: SyscallEvent): [string, string] {
  const arg = "path" in event ? event.path : String(event.fd);
  const call = `${event.call}(${arg})`;
  if ("error" in event && event.error) return [call, event.error];
  switch (event.call) {
    case "open":
      if ("inode" in event) return [call, `inode ${event.inode}, descriptor ${event.fd}, offset 0`];
      break;
    case "read":
      if ("bytes" in event) {
        const count = event.bytes === 0 ? "EOF · 0 bytes" : `${event.bytes} bytes`;
        return [call, `${count} · offset ${event.offset} to ${event.nextOffset}`];
      }
      break;
    case "close":
      return [call, "descriptor released"];
    case "list":
      if ("names" in event) return [call, `directory inode ${event.inode} · ${event.names.length} entries`];
      break;
  }
  return [call, ""];
}

function render() {
  prompt.textContent = loadedCommand ? `$ ${loadedCommand}` : "$";
  output.textContent = session.output;
  trace.replaceChildren(...session.events.map(event => {
    const item = document.createElement("li");
    const [call, detail] = describe(event);
    const label = document.createElement("code");
    label.textContent = call;
    const explanation = document.createElement("span");
    explanation.textContent = detail;
    item.append(label, explanation);
    return item;
  }));
  emptyTrace.hidden = session.events.length !== 0;
  emptyTrace.textContent = session.busy ? "Step to see the next kernel operation." : "No kernel operations to show.";
  descriptors.replaceChildren();
  session.kernel.descriptors.forEach((entry, fd) => {
    if (!entry) return;
    const item = document.createElement("li");
    item.textContent = `fd ${fd} · inode ${entry.inode} · offset ${entry.offset}`;
    descriptors.append(item);
  });
  if (!descriptors.childElementCount) {
    const item = document.createElement("li");
    item.textContent = "None";
    descriptors.append(item);
  }
  step.disabled = run.disabled = !session.busy;
  status.textContent = session.busy ? "Ready to step" : loadedCommand ? "Complete" : "Idle";
}

function load(text: string) {
  session.reset();
  loadedCommand = text.trim();
  command.value = text;
  session.submit(text);
  render();
}

form.addEventListener("submit", event => {
  event.preventDefault();
  load(command.value);
});
document.querySelectorAll<HTMLButtonElement>("[data-command]").forEach(button => {
  button.addEventListener("click", () => load(button.dataset.command!));
});
step.addEventListener("click", () => { session.step(); render(); });
run.addEventListener("click", () => { session.run(); render(); });
document.querySelector("#reset")!.addEventListener("click", () => {
  session.reset();
  loadedCommand = "";
  render();
});
load(command.value);
