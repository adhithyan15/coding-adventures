package legacy

import (
	"bytes"
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
)

// policyDecode decodes a committed policy file into v more strictly than
// strictDecode: on top of refusing unknown fields and trailing data it refuses
// a duplicate key in any object (Go's decoder lets the last one win) and a key
// whose spelling is not EXACTLY a field's JSON name (Go's decoder matches
// field names case-insensitively, so `"Pins"` would otherwise be accepted).
// Policy files are PR-editable, so what a reviewer reads must be what is parsed.
func policyDecode(data []byte, v any) error {
	if err := strictDecode(data, v); err != nil {
		return err
	}
	return checkExactKeys(data, reflect.TypeOf(v))
}

// jsonName is the JSON object key a struct field answers to.
func jsonName(f reflect.StructField) (string, bool) {
	tag := f.Tag.Get("json")
	if tag == "-" || !f.IsExported() {
		return "", false
	}
	name, _, _ := strings.Cut(tag, ",")
	if name == "" {
		name = f.Name
	}
	return name, true
}

func checkExactKeys(raw []byte, t reflect.Type) error {
	for t.Kind() == reflect.Pointer {
		t = t.Elem()
	}
	raw = bytes.TrimSpace(raw)
	if len(raw) == 0 || string(raw) == "null" {
		return nil
	}
	switch t.Kind() {
	case reflect.Struct:
		keys, vals, err := objectMembers(raw)
		if err != nil {
			return err
		}
		fields := map[string]reflect.Type{}
		for i := 0; i < t.NumField(); i++ {
			if n, ok := jsonName(t.Field(i)); ok {
				fields[n] = t.Field(i).Type
			}
		}
		seen := map[string]bool{}
		for i, k := range keys {
			if seen[k] {
				return fmt.Errorf("duplicate key %q", short(k))
			}
			seen[k] = true
			ft, ok := fields[k]
			if !ok {
				return fmt.Errorf("key %q is not a field name (names are case-exact)", short(k))
			}
			if err := checkExactKeys(vals[i], ft); err != nil {
				return err
			}
		}
	case reflect.Map:
		keys, vals, err := objectMembers(raw)
		if err != nil {
			return err
		}
		seen := map[string]bool{}
		for i, k := range keys {
			if seen[k] {
				return fmt.Errorf("duplicate key %q", short(k))
			}
			seen[k] = true
			if err := checkExactKeys(vals[i], t.Elem()); err != nil {
				return err
			}
		}
	case reflect.Slice:
		if raw[0] != '[' {
			return nil // json.RawMessage and friends are checked by their owners
		}
		var items []json.RawMessage
		if err := json.Unmarshal(raw, &items); err != nil {
			return err
		}
		for _, it := range items {
			if err := checkExactKeys(it, t.Elem()); err != nil {
				return err
			}
		}
	}
	return nil
}

// PolicyDecode is policyDecode for other Kiln packages: the strict decoder
// (unknown fields, trailing data, duplicate keys and case-inexact keys all
// refused) that KILN04 stage 4's record formats share with the policy files,
// so a record file and a policy file can never be read two different ways.
func PolicyDecode(data []byte, v any) error { return policyDecode(data, v) }
