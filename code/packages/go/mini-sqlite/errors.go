package minisqlite

import "fmt"

type InterfaceError struct{ Message string }

func (e *InterfaceError) Error() string { return e.Message }

type DatabaseError struct{ Message string }

func (e *DatabaseError) Error() string { return e.Message }

type DataError struct{ Message string }

func (e *DataError) Error() string { return e.Message }

type OperationalError struct{ Message string }

func (e *OperationalError) Error() string { return e.Message }

type IntegrityError struct{ Message string }

func (e *IntegrityError) Error() string { return e.Message }

type InternalError struct{ Message string }

func (e *InternalError) Error() string { return e.Message }

type ProgrammingError struct{ Message string }

func (e *ProgrammingError) Error() string { return e.Message }

type NotSupportedError struct{ Message string }

func (e *NotSupportedError) Error() string { return e.Message }

func translateError(err error) error {
	if err == nil {
		return nil
	}
	switch err.(type) {
	case *InterfaceError, *DatabaseError, *DataError, *OperationalError,
		*IntegrityError, *InternalError, *ProgrammingError, *NotSupportedError:
		return err
	}
	return &ProgrammingError{Message: fmt.Sprintf("%v", err)}
}
