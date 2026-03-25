import { Parser } from "@coding-adventures/cli-builder";
import { ParseErrors } from "@coding-adventures/cli-builder";
import * as fs from "fs";
import * as path from "path";
import * as url from "url";

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));

function wrapText(text: string, width: number): string[] {
  if (text.length <= width) return [text];
  
  const lines: string[] = [];
  const words = text.split(/\s+/);
  if (words.length === 0) return [""];
  
  let currentLine = "";
  for (const word of words) {
    if (currentLine.length + word.length + 1 <= width) {
      currentLine = currentLine === "" ? word : currentLine + " " + word;
    } else {
      lines.push(currentLine);
      currentLine = word;
    }
  }
  if (currentLine !== "") lines.push(currentLine);
  return lines;
}

function formatBubble(lines: string[], isThink: boolean): string {
  if (lines.length === 0) return "";
  
  const maxLen = Math.max(...lines.map(l => l.length));
  const borderTop = " " + "_".repeat(maxLen + 2);
  const borderBottom = " " + "-".repeat(maxLen + 2);
  
  const result: string[] = [borderTop];
  
  if (lines.length === 1) {
    const start = isThink ? "(" : "<";
    const end = isThink ? ")" : ">";
    result.push(`${start} ${lines[0].padEnd(maxLen)} ${end}`);
  } else {
    lines.forEach((line, i) => {
      let start: string, end: string;
      if (isThink) {
        start = "(";
        end = ")";
      } else if (i === 0) {
        start = "/";
        end = "\\";
      } else if (i === lines.length - 1) {
        start = "\\";
        end = "/";
      } else {
        start = "|";
        end = "|";
      }
      result.push(`${start} ${line.padEnd(maxLen)} ${end}`);
    });
  }
  
  result.push(borderBottom);
  return result.join("\n");
}

function loadCow(cowName: string, root: string): string {
  let cowPath = path.join(root, "code", "specs", "cows", `${cowName}.cow`);
  if (!fs.existsSync(cowPath)) {
    cowPath = path.join(root, "code", "specs", "cows", "default.cow");
  }
  
  const content = fs.readFileSync(cowPath, "utf-8");
  
  // Simple parser for $the_cow = <<EOC; ... EOC
  const match = content.match(/<<EOC;\n([\s\S]*?)EOC/);
  if (match) {
    return match[1];
  }
  return content;
}

function findRoot(): string {
  let curr = __dirname;
  for (let i = 0; i < 10; i++) {
    if (fs.existsSync(path.join(curr, "code", "specs", "cowsay.json"))) {
      return curr;
    }
    const parent = path.dirname(curr);
    if (parent === curr) break;
    curr = parent;
  }
  return __dirname;
}

async function main() {
  const root = findRoot();
  const specPath = path.join(root, "code", "specs", "cowsay.json");
  
  const parser = new Parser(specPath, process.argv);
  
  try {
    const result = parser.parse();
    
    if ("text" in result && !("flags" in result)) {
      process.stdout.write(result.text + "\n");
      process.exit(0);
    }
    
    if ("version" in result && !("flags" in result)) {
      process.stdout.write(result.version + "\n");
      process.exit(0);
    }
    
    // ParseResult
    const r = result as any;
    const flags = r.flags;
    const args = r.arguments;
    
    // Handle message
    let message = "";
    const messageParts = args["message"] || [];
    if (messageParts.length === 0) {
      if (!process.stdin.isTTY) {
        message = fs.readFileSync(0, "utf-8").trim();
      } else {
        return;
      }
    } else {
      message = messageParts.join(" ");
    }
    
    if (!message) return;
    
    // Handle modes
    let eyes = flags["eyes"] || "oo";
    let tongue = flags["tongue"] || "  ";
    
    if (flags["borg"]) eyes = "==";
    if (flags["dead"]) {
      eyes = "XX";
      tongue = "U ";
    }
    if (flags["greedy"]) eyes = "$$";
    if (flags["paranoid"]) eyes = "@@";
    if (flags["stoned"]) {
      eyes = "xx";
      tongue = "U ";
    }
    if (flags["tired"]) eyes = "--";
    if (flags["wired"]) eyes = "OO";
    if (flags["youthful"]) eyes = "..";
    
    // Force 2 chars
    eyes = (eyes + "  ").substring(0, 2);
    tongue = (tongue + "  ").substring(0, 2);
    
    // Handle wrapping
    let lines: string[] = [];
    if (flags["nowrap"]) {
      lines = message.split("\n");
    } else {
      const width = flags["width"] || 40;
      message.split("\n").forEach(line => {
        if (line === "") {
          lines.push("");
        } else {
          lines.push(...wrapText(line, width));
        }
      });
    }
    
    // Handle speech vs thought
    let isThink = flags["think"] || path.basename(process.argv[1] || "").includes("cowthink");
    const thoughts = isThink ? "o" : "\\";
    
    // Generate bubble
    const bubble = formatBubble(lines, isThink);
    
    // Load and render cow
    const cowTemplate = loadCow(flags["cowfile"] || "default", root);
    
    // Replace placeholders
    let cow = cowTemplate
      .replace(/\$eyes/g, eyes)
      .replace(/\$tongue/g, tongue)
      .replace(/\$thoughts/g, thoughts);
      
    // Final unescape
    cow = cow.replace(/\\\\/g, "\\");
    
    console.log(bubble);
    console.log(cow);
    
  } catch (e) {
    if (e instanceof ParseErrors) {
      for (const err of e.errors) {
        console.error(`error: ${err.message}`);
      }
      process.exit(1);
    }
    throw e;
  }
}

main();
