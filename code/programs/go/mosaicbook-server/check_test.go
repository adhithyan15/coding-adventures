package main

import (
	"context"
	"errors"
	"strings"
	"sync"
	"testing"
)

func TestCheckDiscoveredStoriesRejectsMissingAndInvalidFiles(t *testing.T) {
	components := []Component{
		{ID: "Missing", Stories: []Story{{Name: "Default", Fixtures: map[string]interface{}{}}}},
		{
			ID:           "Broken",
			StoriesPath:  "Broken.stories.json",
			Stories:      []Story{{Name: "Default", Fixtures: map[string]interface{}{}}},
			StoriesError: "invalid character",
		},
	}
	called := false
	_, err := checkDiscoveredStories(
		context.Background(),
		components,
		[]string{"html"},
		1,
		nil,
		func(context.Context, Component, string, *Story) error {
			called = true
			return nil
		},
	)
	if err == nil {
		t.Fatal("expected catalogue error")
	}
	for _, want := range []string{"Broken: invalid character", "Missing: missing explicit .stories.json file"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q missing %q", err, want)
		}
	}
	if called {
		t.Fatal("compiler must not run when the catalogue itself is invalid")
	}
}

func TestCheckDiscoveredStoriesCompilesEveryStoryAndBackend(t *testing.T) {
	components := []Component{
		{
			ID:          "Button",
			StoriesPath: "Button.stories.json",
			Stories: []Story{
				{Name: "Primary", Fixtures: map[string]interface{}{"variant": "primary"}},
				{Name: "Danger", Fixtures: map[string]interface{}{"variant": "danger"}},
			},
		},
		{
			ID:          "Card",
			StoriesPath: "Card.stories.json",
			Stories:     []Story{{Name: "Default", Fixtures: map[string]interface{}{}}},
		},
	}
	backends := []string{"html", "react", "webcomponent"}
	var lock sync.Mutex
	seen := map[string]bool{}
	summary, err := checkDiscoveredStories(
		context.Background(),
		components,
		backends,
		3,
		nil,
		func(_ context.Context, component Component, backend string, story *Story) error {
			lock.Lock()
			seen[component.ID+"/"+story.Name+"/"+backend] = true
			lock.Unlock()
			return nil
		},
	)
	if err != nil {
		t.Fatalf("check: %v", err)
	}
	if summary.Components != 2 || summary.Stories != 3 || summary.Compilations != 9 {
		t.Fatalf("unexpected summary: %+v", summary)
	}
	if len(seen) != summary.Compilations {
		t.Fatalf("compiled %d unique tasks, want %d: %v", len(seen), summary.Compilations, seen)
	}
}

func TestCheckDiscoveredStoriesReportsCompileFailuresDeterministically(t *testing.T) {
	components := []Component{{
		ID:          "Button",
		StoriesPath: "Button.stories.json",
		Stories:     []Story{{Name: "Default", Fixtures: map[string]interface{}{}}},
	}}
	_, err := checkDiscoveredStories(
		context.Background(),
		components,
		[]string{"html", "react"},
		2,
		nil,
		func(_ context.Context, _ Component, backend string, _ *Story) error {
			return errors.New("broken " + backend)
		},
	)
	if err == nil {
		t.Fatal("expected compilation error")
	}
	want := "Button story \"Default\" on html: broken html\nButton story \"Default\" on react: broken react"
	if !strings.Contains(err.Error(), want) {
		t.Fatalf("errors must follow task order, got %q", err)
	}
}

func TestCheckDiscoveredStoriesRejectsInvalidWorkerCount(t *testing.T) {
	_, err := checkDiscoveredStories(context.Background(), []Component{{ID: "Button"}}, []string{"html"}, 0, nil, nil)
	if err == nil || !strings.Contains(err.Error(), "workers must be at least 1") {
		t.Fatalf("expected worker-count error, got %v", err)
	}
}

func TestStoryCheckBackendsCoversEveryBrowserEmitter(t *testing.T) {
	want := []string{"html", "webcomponent", "react"}
	if strings.Join(storyCheckBackends, ",") != strings.Join(want, ",") {
		t.Fatalf("story check backends = %v, want %v", storyCheckBackends, want)
	}
}

func TestCheckDiscoveredStoriesAcceptsOnlyMatchingRecordedDegradation(t *testing.T) {
	component := Component{ID: "Surface", StoriesPath: "Surface.stories.json", Stories: []Story{{Name: "Default", Fixtures: map[string]interface{}{}}}}
	recorded := []recordedDegradation{{
		Component: "Surface", Story: "Default", Backend: "html", ErrorContains: "$mosaic-child-slot", Issue: 14685,
	}}
	summary, err := checkDiscoveredStories(context.Background(), []Component{component}, []string{"html"}, 1, recorded,
		func(context.Context, Component, string, *Story) error {
			return errors.New("unsupported $mosaic-child-slot")
		})
	if err != nil {
		t.Fatalf("matching recorded degradation: %v", err)
	}
	if summary.RecordedDegradations != 1 {
		t.Fatalf("recorded degradations = %d, want 1", summary.RecordedDegradations)
	}
}

func TestCheckDiscoveredStoriesRejectsStaleRecordedDegradation(t *testing.T) {
	component := Component{ID: "Surface", StoriesPath: "Surface.stories.json", Stories: []Story{{Name: "Default", Fixtures: map[string]interface{}{}}}}
	recorded := []recordedDegradation{{
		Component: "Surface", Story: "Default", Backend: "html", ErrorContains: "$mosaic-child-slot", Issue: 14685,
	}}
	_, err := checkDiscoveredStories(context.Background(), []Component{component}, []string{"html"}, 1, recorded,
		func(context.Context, Component, string, *Story) error { return nil })
	if err == nil || !strings.Contains(err.Error(), "recorded degradation no longer matched") {
		t.Fatalf("expected stale degradation error, got %v", err)
	}
}
