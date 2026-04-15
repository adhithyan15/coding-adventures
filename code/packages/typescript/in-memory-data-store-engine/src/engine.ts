import { type DataStoreCommand, commandName } from "@coding-adventures/in-memory-data-store-protocol";
import { type RespValue, errorValue } from "@coding-adventures/resp-protocol";
import { Store } from "./store.js";
import { installDefaultCommands, isMutatingCommand, type CommandHandler } from "./commands.js";

const LAZY_EXPIRE_EXEMPT = new Set([
  "PING",
  "ECHO",
  "SELECT",
  "INFO",
  "DBSIZE",
  "FLUSHDB",
  "FLUSHALL",
  "KEYS",
]);

export interface DataStoreModule {
  register(engine: DataStoreEngine): void;
}

export class DataStoreEngine {
  private storeState: Store;
  private readonly handlers = new Map<string, CommandHandler>();

  constructor(store: Store = Store.empty()) {
    this.storeState = store;
    installDefaultCommands((name, handler) => {
      this.registerCommand(name, handler);
    });
  }

  get store(): Store {
    return this.storeState;
  }

  registerCommand(name: string, handler: CommandHandler): this {
    this.handlers.set(name.toUpperCase(), handler);
    return this;
  }

  registerModule(module: DataStoreModule): this {
    module.register(this);
    return this;
  }

  execute(command: DataStoreCommand | string[]): RespValue {
    const parts = Array.isArray(command) ? command : [command.name, ...command.args];
    if (parts.length === 0) {
      return errorValue("ERR empty command");
    }
    const name = commandName(parts);
    const handler = this.handlers.get(name);
    if (!handler) {
      return errorValue(`ERR unknown command '${name}'`);
    }

    const shouldLazyExpire = !LAZY_EXPIRE_EXEMPT.has(name) && parts.length > 1;
    const inputStore = shouldLazyExpire ? this.storeState.expireLazy(parts[1]) : this.storeState;
    const [nextStore, response] = handler(inputStore, parts.slice(1));
    this.storeState = nextStore;
    return response;
  }

  executeCommand(command: DataStoreCommand): RespValue {
    return this.execute(command);
  }

  executeParts(parts: string[]): RespValue {
    return this.execute(parts);
  }

  reset(store: Store): void {
    this.storeState = store;
  }
}

export function createDataStoreEngine(store: Store = Store.empty()): DataStoreEngine {
  return new DataStoreEngine(store);
}

export { isMutatingCommand };
