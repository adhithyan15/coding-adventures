package main

import (
	"encoding/json"
	"path/filepath"
	"strings"
	"testing"
)

func describedComponent(t *testing.T, slotsJSON string) componentDescription {
	t.Helper()
	var description componentDescription
	data := []byte(`{"component":"Control","slots":` + slotsJSON + `}`)
	if err := json.Unmarshal(data, &description); err != nil {
		t.Fatalf("description JSON: %v", err)
	}
	return description
}

func TestValidateStoryFixturesRejectsUndeclaredSlot(t *testing.T) {
	description := describedComponent(t, `[{"name":"variant","type":{"oneOf":["primary","danger"]}}]`)
	stories := []Story{{Name: "Danger", Fixtures: map[string]interface{}{"varient": "danger"}}}

	err := validateStoryFixtures(stories, description)
	if err == nil || !strings.Contains(err.Error(), `fixture "varient" names an undeclared slot`) {
		t.Fatalf("expected undeclared-slot error, got %v", err)
	}
}

func TestValidateStoryFixturesChecksScalarAndListTypes(t *testing.T) {
	description := describedComponent(t, `[
		{"name":"disabled","type":"bool"},
		{"name":"count","type":"number"},
		{"name":"labels","type":{"list":"text"}},
		{"name":"matrix","type":{"list":{"list":"number"}}}
	]`)
	validJSON := `{"disabled":true,"count":3,"labels":["one","two"],"matrix":[[1,2],[3]]}`
	var valid map[string]interface{}
	if err := json.Unmarshal([]byte(validJSON), &valid); err != nil {
		t.Fatal(err)
	}
	if err := validateStoryFixtures([]Story{{Name: "Valid", Fixtures: valid}}, description); err != nil {
		t.Fatalf("valid fixtures rejected: %v", err)
	}

	invalidJSON := `{"disabled":"true","count":false,"labels":["one",2],"matrix":[[1],["two"]]}`
	var invalid map[string]interface{}
	if err := json.Unmarshal([]byte(invalidJSON), &invalid); err != nil {
		t.Fatal(err)
	}
	err := validateStoryFixtures([]Story{{Name: "Invalid", Fixtures: invalid}}, description)
	if err == nil {
		t.Fatal("expected type errors")
	}
	for _, want := range []string{"expected bool", "expected number", "labels[1]", "matrix[1][0]"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q missing %q", err, want)
		}
	}
}

func TestValidateStoryFixturesChecksOneOfMembership(t *testing.T) {
	description := describedComponent(t, `[{"name":"size","type":{"oneOf":["sm","md","lg"]}}]`)
	stories := []Story{
		{Name: "Medium", Fixtures: map[string]interface{}{"size": "md"}},
		{Name: "Typo", Fixtures: map[string]interface{}{"size": "medium"}},
	}

	err := validateStoryFixtures(stories, description)
	if err == nil || !strings.Contains(err.Error(), `value "medium" is not one of [sm, md, lg]`) {
		t.Fatalf("expected closed-set error, got %v", err)
	}
}

func TestValidateStoryFixturesAllowsUnsetSlots(t *testing.T) {
	description := describedComponent(t, `[{"name":"required-label","type":"text"}]`)
	stories := []Story{{Name: "Unset", Fixtures: map[string]interface{}{}}}
	if err := validateStoryFixtures(stories, description); err != nil {
		t.Fatalf("an unset slot is a legitimate story state: %v", err)
	}
}

func TestDiscoverValidatedComponentsSurfacesDescribeFailure(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Control", ".light.msl")
	mustWrite(t, filepath.Join(dir, "Control.stories.json"), `{
		"stories": [{"name":"Named","fixtures":{"label":"hello"}}]
	}`)

	server := &Server{root: dir, compilerPath: filepath.Join(dir, "missing-mosaic-compile")}
	components, err := server.discoverValidatedComponents()
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	if len(components) != 1 {
		t.Fatalf("expected one component, got %d", len(components))
	}
	if !strings.Contains(components[0].StoriesError, "cannot validate story fixtures") {
		t.Fatalf("describe failure must surface through StoriesError, got %q", components[0].StoriesError)
	}
	if len(components[0].Stories) != 1 || components[0].Stories[0].Name != "Named" {
		t.Fatalf("broken validation must not hide authored stories: %+v", components[0].Stories)
	}
}
