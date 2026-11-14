package legacy

import (
	"fmt"
	"strings"
	"testing"
)

// checkExactKeys walks the elements of a list only when they can hold keys: a
// list of strings is not copied element by element (a record of tens of
// thousands of paths would double its memory), and a list of objects still is.
func TestPolicyDecodeLargeStringListsAndStrictObjectLists(t *testing.T) {
	type item struct {
		Name string `json:"name"`
	}
	type doc struct {
		Paths []string `json:"paths"`
		Items []item   `json:"items"`
		Ptrs  []*item  `json:"ptrs"`
	}
	var paths []string
	for i := 0; i < 100000; i++ {
		paths = append(paths, fmt.Sprintf(`"code/p/%06d"`, i))
	}
	good := `{"paths":[` + strings.Join(paths, ",") + `],"items":[{"name":"a"}],"ptrs":[{"name":"b"}]}`
	var d doc
	if err := PolicyDecode([]byte(good), &d); err != nil || len(d.Paths) != 100000 {
		t.Fatalf("%v", err)
	}
	for name, bad := range map[string]string{
		"a duplicate key in a struct element":   `{"paths":[],"items":[{"name":"a","name":"b"}],"ptrs":[]}`,
		"a case variant in a struct element":    `{"paths":[],"items":[{"Name":"a"}],"ptrs":[]}`,
		"a duplicate key in a pointer element":  `{"paths":[],"items":[],"ptrs":[{"name":"a","name":"b"}]}`,
		"a duplicate key beside a long list":    `{"paths":[` + strings.Join(paths[:1000], ",") + `],"paths":[],"items":[],"ptrs":[]}`,
		"an unknown key in an element":          `{"paths":[],"items":[{"nope":1}],"ptrs":[]}`,
		"a number where the list wants strings": `{"paths":[1],"items":[],"ptrs":[]}`,
	} {
		var d doc
		if err := PolicyDecode([]byte(bad), &d); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}
