package minisqlite

type ColumnDescription struct {
	Name string
}

type Cursor struct {
	conn        *Connection
	Description []ColumnDescription
	rowcount    int
	lastrowid   any
	arraysize   int
	rows        [][]any
	offset      int
	closed      bool
}

func (c *Cursor) Rowcount() int {
	return c.rowcount
}

func (c *Cursor) Lastrowid() any {
	return c.lastrowid
}

func (c *Cursor) Arraysize() int {
	if c.arraysize == 0 {
		return 1
	}
	return c.arraysize
}

func (c *Cursor) SetArraysize(size int) {
	if size > 0 {
		c.arraysize = size
	}
}

func (c *Cursor) Execute(sql string, params ...any) (*Cursor, error) {
	if err := c.assertOpen(); err != nil {
		return nil, err
	}
	result, err := c.conn.executeBound(sql, params)
	if err != nil {
		return nil, err
	}
	c.rows = result.rows
	c.offset = 0
	c.rowcount = result.rowsAffected
	c.Description = make([]ColumnDescription, len(result.columns))
	for i, column := range result.columns {
		c.Description[i] = ColumnDescription{Name: column}
	}
	return c, nil
}

func (c *Cursor) Executemany(sql string, seq [][]any) (*Cursor, error) {
	if err := c.assertOpen(); err != nil {
		return nil, err
	}
	total := 0
	for _, params := range seq {
		if _, err := c.Execute(sql, params...); err != nil {
			return nil, err
		}
		if c.rowcount > 0 {
			total += c.rowcount
		}
	}
	if len(seq) > 0 {
		c.rowcount = total
	}
	return c, nil
}

func (c *Cursor) Fetchone() ([]any, bool) {
	if c.closed || c.offset >= len(c.rows) {
		return nil, false
	}
	row := c.rows[c.offset]
	c.offset++
	return row, true
}

func (c *Cursor) Fetchmany(size int) [][]any {
	if c.closed {
		return nil
	}
	if size < 0 {
		size = c.Arraysize()
	}
	end := c.offset + size
	if end > len(c.rows) {
		end = len(c.rows)
	}
	rows := c.rows[c.offset:end]
	c.offset = end
	return rows
}

func (c *Cursor) Fetchall() [][]any {
	if c.closed {
		return nil
	}
	rows := c.rows[c.offset:]
	c.offset = len(c.rows)
	return rows
}

func (c *Cursor) Close() {
	c.closed = true
	c.rows = nil
	c.Description = nil
}

func (c *Cursor) assertOpen() error {
	if c.closed {
		return &ProgrammingError{Message: "cursor is closed"}
	}
	return nil
}
