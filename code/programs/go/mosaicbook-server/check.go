// check.go — bounded, non-interactive MosaicBook catalogue validation

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

var storyCheckBackends = []string{"html", "webcomponent", "react"}

type storyCheckSummary struct {
	Components           int
	Stories              int
	Compilations         int
	RecordedDegradations int
}

type storyCheckTask struct {
	component Component
	story     Story
	backend   string
	index     int
}

type storyCompileFunc func(context.Context, Component, string, *Story) error

type recordedDegradation struct {
	Component     string `json:"component"`
	Story         string `json:"story"`
	Backend       string `json:"backend"`
	ErrorContains string `json:"errorContains"`
	Issue         int    `json:"issue"`
}

type recordedDegradationsFile struct {
	Degradations []recordedDegradation `json:"degradations"`
}

func loadRecordedDegradations(path string) ([]recordedDegradation, error) {
	if path == "" {
		return nil, nil
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read recorded degradations: %w", err)
	}
	var file recordedDegradationsFile
	if err := json.Unmarshal(data, &file); err != nil {
		return nil, fmt.Errorf("parse recorded degradations: %w", err)
	}
	seen := make(map[string]bool, len(file.Degradations))
	for _, degradation := range file.Degradations {
		key := degradation.key()
		if degradation.Component == "" || degradation.Story == "" || degradation.Backend == "" || degradation.ErrorContains == "" || degradation.Issue < 1 {
			return nil, fmt.Errorf("recorded degradation entries require component, story, backend, errorContains, and a positive issue: %+v", degradation)
		}
		if seen[key] {
			return nil, fmt.Errorf("duplicate recorded degradation for %s", degradation.label())
		}
		seen[key] = true
	}
	return file.Degradations, nil
}

func (d recordedDegradation) key() string {
	return d.Component + "\x00" + d.Story + "\x00" + d.Backend
}

func (d recordedDegradation) label() string {
	return fmt.Sprintf("%s story %q on %s (#%d)", d.Component, d.Story, d.Backend, d.Issue)
}

// checkStories discovers the real catalogue, rejects missing or invalid story
// files, then compiles every explicit story through every browser backend.
// Known isolation gaps may be recorded against tracking issues; stale records
// fail so a fixed emitter cannot leave its exception behind indefinitely.
func (s *Server) checkStories(ctx context.Context, workers int, degradationsPath string) (storyCheckSummary, error) {
	components, err := s.discoverValidatedComponents()
	if err != nil {
		return storyCheckSummary{}, fmt.Errorf("discover components: %w", err)
	}
	degradations, err := loadRecordedDegradations(degradationsPath)
	if err != nil {
		return storyCheckSummary{}, err
	}
	return checkDiscoveredStories(ctx, components, storyCheckBackends, workers, degradations, s.compileStoryForCheck)
}

func checkDiscoveredStories(
	ctx context.Context,
	components []Component,
	backends []string,
	workers int,
	recorded []recordedDegradation,
	compile storyCompileFunc,
) (storyCheckSummary, error) {
	if workers < 1 {
		return storyCheckSummary{}, fmt.Errorf("workers must be at least 1")
	}
	if len(backends) == 0 {
		return storyCheckSummary{}, fmt.Errorf("at least one backend is required")
	}
	if len(components) == 0 {
		return storyCheckSummary{}, fmt.Errorf("no renderable Mosaic components discovered")
	}

	components = append([]Component(nil), components...)
	sort.Slice(components, func(i, j int) bool { return components[i].ID < components[j].ID })

	summary := storyCheckSummary{Components: len(components)}
	var inventoryProblems []string
	var tasks []storyCheckTask
	for _, component := range components {
		switch {
		case component.StoriesPath == "":
			inventoryProblems = append(inventoryProblems, fmt.Sprintf("%s: missing explicit .stories.json file", component.ID))
			continue
		case component.StoriesError != "":
			inventoryProblems = append(inventoryProblems, fmt.Sprintf("%s: %s", component.ID, component.StoriesError))
			continue
		case len(component.Stories) == 0:
			inventoryProblems = append(inventoryProblems, fmt.Sprintf("%s: stories file contains no stories", component.ID))
			continue
		}

		summary.Stories += len(component.Stories)
		for _, story := range component.Stories {
			for _, backend := range backends {
				tasks = append(tasks, storyCheckTask{
					component: component,
					story:     story,
					backend:   backend,
					index:     len(tasks),
				})
			}
		}
	}
	summary.Compilations = len(tasks)
	if len(inventoryProblems) > 0 {
		return summary, fmt.Errorf("invalid MosaicBook story catalogue:\n%s", strings.Join(inventoryProblems, "\n"))
	}

	if workers > len(tasks) {
		workers = len(tasks)
	}
	jobs := make(chan storyCheckTask)
	errorsByTask := make([]error, len(tasks))
	var group sync.WaitGroup
	for range workers {
		group.Add(1)
		go func() {
			defer group.Done()
			for task := range jobs {
				if err := compile(ctx, task.component, task.backend, &task.story); err != nil {
					errorsByTask[task.index] = fmt.Errorf(
						"%s story %q on %s: %w",
						task.component.ID,
						task.story.Name,
						task.backend,
						err,
					)
				}
			}
		}()
	}
	for _, task := range tasks {
		jobs <- task
	}
	close(jobs)
	group.Wait()

	var compileProblems []string
	recordedByKey := make(map[string]recordedDegradation, len(recorded))
	for _, degradation := range recorded {
		recordedByKey[degradation.key()] = degradation
	}
	matched := make(map[string]bool, len(recorded))
	for i, err := range errorsByTask {
		if err == nil {
			continue
		}
		task := tasks[i]
		key := recordedDegradation{Component: task.component.ID, Story: task.story.Name, Backend: task.backend}.key()
		if degradation, ok := recordedByKey[key]; ok && strings.Contains(err.Error(), degradation.ErrorContains) {
			matched[key] = true
			summary.RecordedDegradations++
			continue
		}
		compileProblems = append(compileProblems, err.Error())
	}
	for _, degradation := range recorded {
		if !matched[degradation.key()] {
			compileProblems = append(compileProblems, "recorded degradation no longer matched: "+degradation.label())
		}
	}
	if len(compileProblems) > 0 {
		return summary, fmt.Errorf("MosaicBook story compilation failed:\n%s", strings.Join(compileProblems, "\n"))
	}
	return summary, nil
}

func (s *Server) compileStoryForCheck(ctx context.Context, component Component, backend string, story *Story) error {
	ext := backendExtension(backend)
	tmp, err := os.CreateTemp("", "mosaicbook-check-*"+ext)
	if err != nil {
		return fmt.Errorf("create output file: %w", err)
	}
	outputPath := tmp.Name()
	if err := tmp.Close(); err != nil {
		os.Remove(outputPath) //nolint:errcheck
		return fmt.Errorf("close output file: %w", err)
	}
	defer os.Remove(outputPath) //nolint:errcheck
	return s.compileContext(ctx, component, backend, outputPath, story)
}
