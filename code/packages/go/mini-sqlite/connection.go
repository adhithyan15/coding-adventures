package minisqlite

const (
	APILevel     = "2.0"
	ThreadSafety = 1
	ParamStyle   = "qmark"
)

type Options struct {
	Autocommit bool
}

type Connection struct {
	db         *inMemoryDatabase
	autocommit bool
	snapshot   snapshot
	closed     bool
}

func Connect(database string, options ...Options) (*Connection, error) {
	if database != ":memory:" {
		return nil, &NotSupportedError{Message: "Go mini-sqlite supports only :memory: in Level 0"}
	}
	opts := Options{}
	if len(options) > 0 {
		opts = options[0]
	}
	return &Connection{
		db:         newInMemoryDatabase(),
		autocommit: opts.Autocommit,
	}, nil
}

func (c *Connection) Cursor() (*Cursor, error) {
	if err := c.assertOpen(); err != nil {
		return nil, err
	}
	return &Cursor{conn: c, rowcount: -1}, nil
}

func (c *Connection) Execute(sql string, params ...any) (*Cursor, error) {
	cur, err := c.Cursor()
	if err != nil {
		return nil, err
	}
	return cur.Execute(sql, params...)
}

func (c *Connection) Executemany(sql string, seq [][]any) (*Cursor, error) {
	cur, err := c.Cursor()
	if err != nil {
		return nil, err
	}
	return cur.Executemany(sql, seq)
}

func (c *Connection) Commit() error {
	if err := c.assertOpen(); err != nil {
		return err
	}
	c.snapshot = nil
	return nil
}

func (c *Connection) Rollback() error {
	if err := c.assertOpen(); err != nil {
		return err
	}
	if c.snapshot != nil {
		c.db.restore(c.snapshot)
		c.snapshot = nil
	}
	return nil
}

func (c *Connection) Close() error {
	if c.closed {
		return nil
	}
	if c.snapshot != nil {
		c.db.restore(c.snapshot)
	}
	c.snapshot = nil
	c.closed = true
	return nil
}

func (c *Connection) executeBound(sql string, params []any) (*statementResult, error) {
	if err := c.assertOpen(); err != nil {
		return nil, err
	}
	bound, err := bindParameters(sql, params)
	if err != nil {
		return nil, err
	}
	switch firstKeyword(bound) {
	case "BEGIN":
		c.ensureSnapshot()
		return emptyStatementResult(), nil
	case "COMMIT":
		return emptyStatementResult(), c.Commit()
	case "ROLLBACK":
		return emptyStatementResult(), c.Rollback()
	case "SELECT":
		return c.db.selectSQL(bound)
	case "CREATE":
		c.ensureSnapshot()
		stmt, err := parseCreate(bound)
		if err != nil {
			return nil, err
		}
		return c.db.create(stmt)
	case "DROP":
		c.ensureSnapshot()
		stmt, err := parseDrop(bound)
		if err != nil {
			return nil, err
		}
		return c.db.drop(stmt)
	case "INSERT":
		c.ensureSnapshot()
		stmt, err := parseInsert(bound)
		if err != nil {
			return nil, err
		}
		return c.db.insert(stmt)
	case "UPDATE":
		c.ensureSnapshot()
		stmt, err := parseUpdate(bound)
		if err != nil {
			return nil, err
		}
		return c.db.update(stmt)
	case "DELETE":
		c.ensureSnapshot()
		stmt, err := parseDelete(bound)
		if err != nil {
			return nil, err
		}
		return c.db.delete(stmt)
	default:
		return nil, &ProgrammingError{Message: "unsupported SQL statement"}
	}
}

func (c *Connection) ensureSnapshot() {
	if !c.autocommit && c.snapshot == nil {
		c.snapshot = c.db.snapshot()
	}
}

func (c *Connection) assertOpen() error {
	if c.closed {
		return &ProgrammingError{Message: "connection is closed"}
	}
	return nil
}
