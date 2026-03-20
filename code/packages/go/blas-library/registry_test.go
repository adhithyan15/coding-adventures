package blaslibrary

import (
	"fmt"
	"sort"
	"testing"
)

// =========================================================================
// Registry tests
// =========================================================================

func TestNewBackendRegistry(t *testing.T) {
	r := NewBackendRegistry()
	if r == nil {
		t.Fatal("expected non-nil registry")
	}
	if len(r.backends) != 0 {
		t.Errorf("expected empty backends map, got %d entries", len(r.backends))
	}
}

func TestRegisterAndGet(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("test", func() (BlasBackend, error) {
		return &fakeBackend{name: "test"}, nil
	})
	b, err := r.Get("test")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if b.Name() != "test" {
		t.Errorf("expected name 'test', got %q", b.Name())
	}
}

func TestGetUnregistered(t *testing.T) {
	r := NewBackendRegistry()
	_, err := r.Get("nonexistent")
	if err == nil {
		t.Fatal("expected error for unregistered backend")
	}
}

func TestGetFactoryError(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("broken", func() (BlasBackend, error) {
		return nil, fmt.Errorf("initialization failed")
	})
	_, err := r.Get("broken")
	if err == nil {
		t.Fatal("expected error from factory")
	}
}

func TestGetBest(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("cpu", func() (BlasBackend, error) {
		return &fakeBackend{name: "cpu"}, nil
	})
	b, err := r.GetBest()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if b.Name() != "cpu" {
		t.Errorf("expected 'cpu', got %q", b.Name())
	}
}

func TestGetBestPriority(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("cpu", func() (BlasBackend, error) {
		return &fakeBackend{name: "cpu"}, nil
	})
	r.Register("cuda", func() (BlasBackend, error) {
		return &fakeBackend{name: "cuda"}, nil
	})
	b, err := r.GetBest()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// CUDA has higher priority than CPU
	if b.Name() != "cuda" {
		t.Errorf("expected 'cuda' (higher priority), got %q", b.Name())
	}
}

func TestGetBestSkipsFailures(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("cuda", func() (BlasBackend, error) {
		return nil, fmt.Errorf("no NVIDIA GPU")
	})
	r.Register("cpu", func() (BlasBackend, error) {
		return &fakeBackend{name: "cpu"}, nil
	})
	b, err := r.GetBest()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if b.Name() != "cpu" {
		t.Errorf("expected 'cpu' fallback, got %q", b.Name())
	}
}

func TestGetBestNoBackends(t *testing.T) {
	r := NewBackendRegistry()
	_, err := r.GetBest()
	if err == nil {
		t.Fatal("expected error when no backends registered")
	}
}

func TestGetBestAllFail(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("cuda", func() (BlasBackend, error) {
		return nil, fmt.Errorf("no GPU")
	})
	_, err := r.GetBest()
	if err == nil {
		t.Fatal("expected error when all backends fail")
	}
}

func TestListAvailable(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("cpu", func() (BlasBackend, error) {
		return &fakeBackend{name: "cpu"}, nil
	})
	r.Register("cuda", func() (BlasBackend, error) {
		return &fakeBackend{name: "cuda"}, nil
	})
	available := r.ListAvailable()
	sort.Strings(available)
	if len(available) != 2 {
		t.Fatalf("expected 2 backends, got %d", len(available))
	}
	if available[0] != "cpu" || available[1] != "cuda" {
		t.Errorf("expected [cpu, cuda], got %v", available)
	}
}

func TestSetPriority(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("cpu", func() (BlasBackend, error) {
		return &fakeBackend{name: "cpu"}, nil
	})
	r.Register("cuda", func() (BlasBackend, error) {
		return &fakeBackend{name: "cuda"}, nil
	})
	// Reverse priority: CPU first
	r.SetPriority([]string{"cpu", "cuda"})
	b, err := r.GetBest()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if b.Name() != "cpu" {
		t.Errorf("expected 'cpu' with custom priority, got %q", b.Name())
	}
}

func TestCreateBlas_Auto(t *testing.T) {
	// Save and restore global registry
	old := GlobalRegistry
	defer func() { GlobalRegistry = old }()

	GlobalRegistry = NewBackendRegistry()
	GlobalRegistry.Register("cpu", func() (BlasBackend, error) {
		return &fakeBackend{name: "cpu"}, nil
	})
	b, err := CreateBlas("auto")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if b.Name() != "cpu" {
		t.Errorf("expected 'cpu', got %q", b.Name())
	}
}

func TestCreateBlas_Specific(t *testing.T) {
	old := GlobalRegistry
	defer func() { GlobalRegistry = old }()

	GlobalRegistry = NewBackendRegistry()
	GlobalRegistry.Register("test", func() (BlasBackend, error) {
		return &fakeBackend{name: "test"}, nil
	})
	b, err := CreateBlas("test")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if b.Name() != "test" {
		t.Errorf("expected 'test', got %q", b.Name())
	}
}

func TestCreateBlas_NotFound(t *testing.T) {
	old := GlobalRegistry
	defer func() { GlobalRegistry = old }()

	GlobalRegistry = NewBackendRegistry()
	_, err := CreateBlas("nonexistent")
	if err == nil {
		t.Fatal("expected error for unregistered backend")
	}
}

func TestDefaultPriority(t *testing.T) {
	expected := []string{"cuda", "metal", "vulkan", "opencl", "webgpu", "opengl", "cpu"}
	if len(defaultPriority) != len(expected) {
		t.Fatalf("expected %d priorities, got %d", len(expected), len(defaultPriority))
	}
	for i, name := range expected {
		if defaultPriority[i] != name {
			t.Errorf("priority[%d]: expected %q, got %q", i, name, defaultPriority[i])
		}
	}
}

func TestRegistryOverwrite(t *testing.T) {
	r := NewBackendRegistry()
	r.Register("test", func() (BlasBackend, error) {
		return &fakeBackend{name: "v1"}, nil
	})
	r.Register("test", func() (BlasBackend, error) {
		return &fakeBackend{name: "v2"}, nil
	})
	b, _ := r.Get("test")
	if b.Name() != "v2" {
		t.Errorf("expected overwritten backend 'v2', got %q", b.Name())
	}
}

// =========================================================================
// fakeBackend -- minimal test implementation
// =========================================================================

type fakeBackend struct {
	name string
}

func (f *fakeBackend) Name() string                 { return f.name }
func (f *fakeBackend) DeviceName() string            { return "Fake Device" }
func (f *fakeBackend) Saxpy(alpha float32, x, y Vector) (Vector, error) {
	return Vector{}, nil
}
func (f *fakeBackend) Sdot(x, y Vector) (float32, error)            { return 0, nil }
func (f *fakeBackend) Snrm2(x Vector) float32                       { return 0 }
func (f *fakeBackend) Sscal(alpha float32, x Vector) Vector          { return Vector{} }
func (f *fakeBackend) Sasum(x Vector) float32                        { return 0 }
func (f *fakeBackend) Isamax(x Vector) int                           { return 0 }
func (f *fakeBackend) Scopy(x Vector) Vector                         { return Vector{} }
func (f *fakeBackend) Sswap(x, y Vector) (Vector, Vector, error)     { return Vector{}, Vector{}, nil }
func (f *fakeBackend) Sgemv(trans Transpose, alpha float32, a Matrix, x Vector, beta float32, y Vector) (Vector, error) {
	return Vector{}, nil
}
func (f *fakeBackend) Sger(alpha float32, x, y Vector, a Matrix) (Matrix, error) {
	return Matrix{}, nil
}
func (f *fakeBackend) Sgemm(transA, transB Transpose, alpha float32, a, b Matrix, beta float32, c Matrix) (Matrix, error) {
	return Matrix{}, nil
}
func (f *fakeBackend) Ssymm(side Side, alpha float32, a, b Matrix, beta float32, c Matrix) (Matrix, error) {
	return Matrix{}, nil
}
func (f *fakeBackend) SgemmBatched(transA, transB Transpose, alpha float32, aList, bList []Matrix, beta float32, cList []Matrix) ([]Matrix, error) {
	return nil, nil
}
