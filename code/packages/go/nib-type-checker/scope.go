package nibtypechecker

type SymbolRecord struct {
	Name         string
	NibType      NibType
	HasType      bool
	IsConst      bool
	IsStatic     bool
	IsFn         bool
	FnParams     [][2]any
	FnReturnType NibType
	HasReturn    bool
}

type ScopeChain struct {
	scopes []map[string]SymbolRecord
}

func NewScopeChain() *ScopeChain {
	return &ScopeChain{scopes: []map[string]SymbolRecord{{}}}
}

func (s *ScopeChain) Push() {
	s.scopes = append(s.scopes, map[string]SymbolRecord{})
}

func (s *ScopeChain) Pop() {
	if len(s.scopes) > 1 {
		s.scopes = s.scopes[:len(s.scopes)-1]
	}
}

func (s *ScopeChain) Define(name string, symbol SymbolRecord) {
	s.scopes[len(s.scopes)-1][name] = symbol
}

func (s *ScopeChain) DefineGlobal(name string, symbol SymbolRecord) {
	s.scopes[0][name] = symbol
}

func (s *ScopeChain) Lookup(name string) (SymbolRecord, bool) {
	for index := len(s.scopes) - 1; index >= 0; index-- {
		if symbol, ok := s.scopes[index][name]; ok {
			return symbol, true
		}
	}
	return SymbolRecord{}, false
}
