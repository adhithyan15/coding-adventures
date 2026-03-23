import { MemoryStorage } from "../memory-storage.js";
import { runStorageTests } from "./storage.shared.js";

runStorageTests("MemoryStorage", () => new MemoryStorage([
  { name: "items", keyPath: "id" },
  { name: "other", keyPath: "id" },
]));
