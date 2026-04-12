package datastoreengine

import (
	"strings"
	"sync"
	"sync/atomic"

	datastoreprotocol "github.com/adhithyan15/coding-adventures/code/packages/go/in-memory-data-store-protocol"
	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

type CommandHandler func(store *Store, args [][]byte) (*Store, resp.Value)

type CommandRegistration struct {
	Handler        CommandHandler
	Mutating       bool
	SkipLazyExpire bool
}

type DataStoreBackend interface {
	ExecuteFrame(frame datastoreprotocol.CommandFrame) resp.Value
	ExecuteParts(parts [][]byte) resp.Value
	Store() *Store
	ActiveExpireAll()
}

type DataStoreEngine struct {
	mu       sync.Mutex
	store    *Store
	commands map[string]CommandRegistration
	frozen   atomic.Bool
}

func New() *DataStoreEngine {
	engine := &DataStoreEngine{
		store:    NewStore(),
		commands: make(map[string]CommandRegistration),
	}
	registerBuiltinCommands(engine)
	return engine
}

func FromStore(store *Store) *DataStoreEngine {
	engine := &DataStoreEngine{
		store:    store.Clone(),
		commands: make(map[string]CommandRegistration),
	}
	registerBuiltinCommands(engine)
	return engine
}

func (e *DataStoreEngine) RegisterCommand(name string, mutating bool, skipLazyExpire bool, handler CommandHandler) {
	if e.frozen.Load() {
		panic("cannot register commands on a frozen engine")
	}
	e.mu.Lock()
	defer e.mu.Unlock()
	e.commands[strings.ToUpper(name)] = CommandRegistration{
		Handler:        handler,
		Mutating:       mutating,
		SkipLazyExpire: skipLazyExpire,
	}
}

func (e *DataStoreEngine) Freeze() {
	e.frozen.Store(true)
}

func (e *DataStoreEngine) IsFrozen() bool {
	return e.frozen.Load()
}

func (e *DataStoreEngine) ExecuteFrame(frame datastoreprotocol.CommandFrame) resp.Value {
	_, response := e.executeWithDB(e.currentDB(), frame.Command, frame.Args, true)
	return response
}

func (e *DataStoreEngine) ExecuteParts(parts [][]byte) resp.Value {
	frame, ok := datastoreprotocol.FromParts(parts)
	if !ok {
		return resp.ErrorValue("ERR empty command")
	}
	return e.ExecuteFrame(frame)
}

func (e *DataStoreEngine) Store() *Store {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.store.Clone()
}

func (e *DataStoreEngine) ActiveExpireAll() {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.store = e.store.ActiveExpireAll()
}

func (e *DataStoreEngine) executeWithDB(dbIndex int, command string, args [][]byte, recordAOF bool) (int, resp.Value) {
	e.mu.Lock()
	registration, ok := e.commands[strings.ToUpper(command)]
	if !ok {
		e.mu.Unlock()
		return dbIndex, resp.ErrorValue("ERR unknown command '" + strings.ToUpper(command) + "'")
	}

	store := e.store.Clone().WithActiveDB(dbIndex)
	e.mu.Unlock()

	if !registration.SkipLazyExpire && len(args) > 0 {
		store = store.ExpireLazy(args[0])
	}

	newStore, response := registration.Handler(store, args)
	if newStore == nil {
		newStore = store
	}

	e.mu.Lock()
	e.store = newStore
	activeDB := newStore.ActiveDB
	e.mu.Unlock()

	return activeDB, response
}

func (e *DataStoreEngine) currentDB() int {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.store.ActiveDB
}
