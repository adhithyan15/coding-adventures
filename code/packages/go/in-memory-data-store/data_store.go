package datastore

import (
	datastoreengine "github.com/adhithyan15/coding-adventures/code/packages/go/in-memory-data-store-engine"
	datastoreprotocol "github.com/adhithyan15/coding-adventures/code/packages/go/in-memory-data-store-protocol"
	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

type DataStore struct {
	backend datastoreengine.DataStoreBackend
}

func New() *DataStore {
	return &DataStore{backend: datastoreengine.New()}
}

func FromStore(store *datastoreengine.Store) *DataStore {
	return &DataStore{backend: datastoreengine.FromStore(store)}
}

func (s *DataStore) ExecuteFrame(frame datastoreprotocol.CommandFrame) resp.Value {
	return s.backend.ExecuteFrame(frame)
}

func (s *DataStore) ExecuteParts(parts [][]byte) resp.Value {
	return s.backend.ExecuteParts(parts)
}

func (s *DataStore) Store() *datastoreengine.Store {
	return s.backend.Store()
}

func (s *DataStore) ActiveExpireAll() {
	s.backend.ActiveExpireAll()
}
