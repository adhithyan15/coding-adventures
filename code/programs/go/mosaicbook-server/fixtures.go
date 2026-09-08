// fixtures.go — validate story fixtures against the component interface
//
// Story files are executable examples of a component's declared states. A
// misspelled slot or a value of the wrong type must therefore be an authoring
// error, not an ignored value that happens to render the default state.
//
// MosaicBook deliberately does not parse .mil itself. It asks mosaic-compile
// for `--describe` JSON, so slot names, list element types, and closed one-of
// sets come from the same mosmodel parser used by every real compilation.

package main

import (
	"encoding/json"
	"fmt"
	"os/exec"
	"sort"
	"strings"
)

type componentDescription struct {
	Component string            `json:"component"`
	Slots     []slotDescription `json:"slots"`
}

type slotDescription struct {
	Name string          `json:"name"`
	Type json.RawMessage `json:"type"`
}

type fixtureSlotType struct {
	kind  string
	inner *fixtureSlotType
	oneOf []string
}

// discoverValidatedComponents discovers components and annotates any invalid
// story fixtures through Component.StoriesError. Keeping discovery usable even
// when a story is broken lets the browser show the component and the error,
// while the forthcoming CI gate can reject the same catalogue.
func (s *Server) discoverValidatedComponents() ([]Component, error) {
	components, err := discoverComponents(s.root)
	if err != nil {
		return components, err
	}
	for i := range components {
		s.validateComponentStories(&components[i])
	}
	return components, nil
}

func (s *Server) validateComponentStories(component *Component) {
	if component.StoriesError != "" || !component.isThreeFile() || !hasStoryFixtures(component.Stories) {
		return
	}

	description, err := s.describeComponent(*component)
	if err != nil {
		component.StoriesError = fmt.Sprintf("cannot validate story fixtures: %v", err)
		return
	}
	if description.Component != componentNameFromID(component.ID) {
		component.StoriesError = fmt.Sprintf(
			"cannot validate story fixtures: interface describes component %q, expected %q",
			description.Component,
			componentNameFromID(component.ID),
		)
		return
	}
	if err := validateStoryFixtures(component.Stories, description); err != nil {
		component.StoriesError = err.Error()
	}
}

func hasStoryFixtures(stories []Story) bool {
	for _, story := range stories {
		if len(story.Fixtures) > 0 {
			return true
		}
	}
	return false
}

// describeComponent asks mosaic-compile to parse the .mil and print its typed
// surface. This is the load-bearing seam that prevents MosaicBook from growing
// a second, inevitably drifting interface parser.
func (s *Server) describeComponent(component Component) (componentDescription, error) {
	args := []string{"--interface", component.InterfacePath, "--describe"}
	cmd := exec.Command(s.compilerPath, args...)
	out := &cappedWriter{limit: maxCompilerOutputBytes}
	cmd.Stdout = out
	cmd.Stderr = out
	if err := cmd.Run(); err != nil {
		if isNotFound(err) {
			return componentDescription{}, fmt.Errorf("mosaic-compile binary %q not found on PATH", s.compilerPath)
		}
		if out.written > 0 {
			return componentDescription{}, fmt.Errorf("mosaic-compile --describe failed: %s", out.String())
		}
		return componentDescription{}, fmt.Errorf("mosaic-compile --describe exited with error: %w", err)
	}

	var description componentDescription
	if err := json.Unmarshal(out.buf, &description); err != nil {
		return componentDescription{}, fmt.Errorf("invalid mosaic-compile --describe JSON: %w", err)
	}
	return description, nil
}

func validateStoryFixtures(stories []Story, description componentDescription) error {
	slots := make(map[string]fixtureSlotType, len(description.Slots))
	for _, slot := range description.Slots {
		typeDescription, err := parseFixtureSlotType(slot.Type)
		if err != nil {
			return fmt.Errorf("cannot validate fixtures for slot %q: %w", slot.Name, err)
		}
		slots[slot.Name] = typeDescription
	}

	var problems []string
	for _, story := range stories {
		names := make([]string, 0, len(story.Fixtures))
		for name := range story.Fixtures {
			names = append(names, name)
		}
		sort.Strings(names)
		for _, name := range names {
			slotType, ok := slots[name]
			if !ok {
				problems = append(problems, fmt.Sprintf("story %q fixture %q names an undeclared slot", story.Name, name))
				continue
			}
			if err := validateFixtureValue(story.Fixtures[name], slotType, name); err != nil {
				problems = append(problems, fmt.Sprintf("story %q fixture %q: %v", story.Name, name, err))
			}
		}
	}
	if len(problems) > 0 {
		return fmt.Errorf("invalid story fixtures: %s", strings.Join(problems, "; "))
	}
	return nil
}

func parseFixtureSlotType(raw json.RawMessage) (fixtureSlotType, error) {
	var scalar string
	if err := json.Unmarshal(raw, &scalar); err == nil {
		return fixtureSlotType{kind: scalar}, nil
	}

	var compound map[string]json.RawMessage
	if err := json.Unmarshal(raw, &compound); err != nil || len(compound) != 1 {
		return fixtureSlotType{}, fmt.Errorf("unsupported slot type %s", string(raw))
	}
	if value, ok := compound["oneOf"]; ok {
		var values []string
		if err := json.Unmarshal(value, &values); err != nil || len(values) == 0 {
			return fixtureSlotType{}, fmt.Errorf("invalid one-of slot type %s", string(raw))
		}
		return fixtureSlotType{kind: "oneOf", oneOf: values}, nil
	}
	if value, ok := compound["list"]; ok {
		inner, err := parseFixtureSlotType(value)
		if err != nil {
			return fixtureSlotType{}, err
		}
		return fixtureSlotType{kind: "list", inner: &inner}, nil
	}
	if value, ok := compound["component"]; ok {
		var name string
		if err := json.Unmarshal(value, &name); err != nil || name == "" {
			return fixtureSlotType{}, fmt.Errorf("invalid component slot type %s", string(raw))
		}
		return fixtureSlotType{kind: "component"}, nil
	}
	return fixtureSlotType{}, fmt.Errorf("unsupported slot type %s", string(raw))
}

func validateFixtureValue(value interface{}, slotType fixtureSlotType, path string) error {
	switch slotType.kind {
	case "text", "image", "color":
		if _, ok := value.(string); !ok {
			return fmt.Errorf("expected %s at %s, got %s", slotType.kind, path, fixtureValueKind(value))
		}
	case "number":
		if _, ok := value.(float64); !ok {
			return fmt.Errorf("expected number at %s, got %s", path, fixtureValueKind(value))
		}
	case "bool":
		if _, ok := value.(bool); !ok {
			return fmt.Errorf("expected bool at %s, got %s", path, fixtureValueKind(value))
		}
	case "list":
		values, ok := value.([]interface{})
		if !ok {
			return fmt.Errorf("expected list at %s, got %s", path, fixtureValueKind(value))
		}
		for i, item := range values {
			if err := validateFixtureValue(item, *slotType.inner, fmt.Sprintf("%s[%d]", path, i)); err != nil {
				return err
			}
		}
	case "oneOf":
		selected, ok := value.(string)
		if !ok {
			return fmt.Errorf("expected one of [%s] at %s, got %s", strings.Join(slotType.oneOf, ", "), path, fixtureValueKind(value))
		}
		for _, allowed := range slotType.oneOf {
			if selected == allowed {
				return nil
			}
		}
		return fmt.Errorf("value %q is not one of [%s]", selected, strings.Join(slotType.oneOf, ", "))
	case "component", "node":
		if _, ok := value.(map[string]interface{}); !ok {
			return fmt.Errorf("expected object at %s, got %s", path, fixtureValueKind(value))
		}
	default:
		return fmt.Errorf("unsupported slot type %q", slotType.kind)
	}
	return nil
}

func fixtureValueKind(value interface{}) string {
	switch value.(type) {
	case nil:
		return "null"
	case string:
		return "string"
	case float64:
		return "number"
	case bool:
		return "bool"
	case []interface{}:
		return "list"
	case map[string]interface{}:
		return "object"
	default:
		return fmt.Sprintf("%T", value)
	}
}
