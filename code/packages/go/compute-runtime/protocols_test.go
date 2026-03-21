package computeruntime

import "testing"

// =========================================================================
// DeviceType tests
// =========================================================================

func TestDeviceTypeString(t *testing.T) {
	tests := []struct {
		dt   DeviceType
		want string
	}{
		{DeviceTypeGPU, "gpu"},
		{DeviceTypeTPU, "tpu"},
		{DeviceTypeNPU, "npu"},
		{DeviceType(99), "unknown"},
	}
	for _, tt := range tests {
		if got := tt.dt.String(); got != tt.want {
			t.Errorf("DeviceType(%d).String() = %q, want %q", tt.dt, got, tt.want)
		}
	}
}

// =========================================================================
// QueueType tests
// =========================================================================

func TestQueueTypeString(t *testing.T) {
	tests := []struct {
		qt   QueueType
		want string
	}{
		{QueueTypeCompute, "compute"},
		{QueueTypeTransfer, "transfer"},
		{QueueTypeComputeTransfer, "compute_transfer"},
		{QueueType(99), "unknown"},
	}
	for _, tt := range tests {
		if got := tt.qt.String(); got != tt.want {
			t.Errorf("QueueType(%d).String() = %q, want %q", tt.qt, got, tt.want)
		}
	}
}

func TestParseQueueType(t *testing.T) {
	tests := []struct {
		input string
		want  QueueType
	}{
		{"compute", QueueTypeCompute},
		{"transfer", QueueTypeTransfer},
		{"compute_transfer", QueueTypeComputeTransfer},
		{"unknown_value", QueueTypeComputeTransfer},
	}
	for _, tt := range tests {
		if got := ParseQueueType(tt.input); got != tt.want {
			t.Errorf("ParseQueueType(%q) = %d, want %d", tt.input, got, tt.want)
		}
	}
}

// =========================================================================
// MemoryType tests
// =========================================================================

func TestMemoryTypeFlags(t *testing.T) {
	// Single flags
	dl := MemoryTypeDeviceLocal
	if !dl.Has(MemoryTypeDeviceLocal) {
		t.Error("DEVICE_LOCAL should have DEVICE_LOCAL")
	}
	if dl.Has(MemoryTypeHostVisible) {
		t.Error("DEVICE_LOCAL should not have HOST_VISIBLE")
	}

	// Combined flags
	unified := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	if !unified.Has(MemoryTypeDeviceLocal) {
		t.Error("unified should have DEVICE_LOCAL")
	}
	if !unified.Has(MemoryTypeHostVisible) {
		t.Error("unified should have HOST_VISIBLE")
	}
	if !unified.Has(MemoryTypeHostCoherent) {
		t.Error("unified should have HOST_COHERENT")
	}
	if unified.Has(MemoryTypeHostCached) {
		t.Error("unified should not have HOST_CACHED")
	}
}

func TestMemoryTypeString(t *testing.T) {
	none := MemoryType(0)
	if got := none.String(); got != "NONE" {
		t.Errorf("MemoryType(0).String() = %q, want %q", got, "NONE")
	}

	dl := MemoryTypeDeviceLocal
	if got := dl.String(); got != "DEVICE_LOCAL" {
		t.Errorf("DEVICE_LOCAL.String() = %q, want %q", got, "DEVICE_LOCAL")
	}

	combined := MemoryTypeHostVisible | MemoryTypeHostCoherent
	got := combined.String()
	if got != "HOST_VISIBLE | HOST_COHERENT" {
		t.Errorf("HOST_VISIBLE|HOST_COHERENT.String() = %q, want %q", got, "HOST_VISIBLE | HOST_COHERENT")
	}
}

// =========================================================================
// BufferUsage tests
// =========================================================================

func TestBufferUsageFlags(t *testing.T) {
	usage := BufferUsageStorage | BufferUsageTransferDst
	if !usage.Has(BufferUsageStorage) {
		t.Error("should have STORAGE")
	}
	if !usage.Has(BufferUsageTransferDst) {
		t.Error("should have TRANSFER_DST")
	}
	if usage.Has(BufferUsageTransferSrc) {
		t.Error("should not have TRANSFER_SRC")
	}
}

// =========================================================================
// PipelineStage tests
// =========================================================================

func TestPipelineStageString(t *testing.T) {
	tests := []struct {
		ps   PipelineStage
		want string
	}{
		{PipelineStageTopOfPipe, "top_of_pipe"},
		{PipelineStageCompute, "compute"},
		{PipelineStageTransfer, "transfer"},
		{PipelineStageHost, "host"},
		{PipelineStageBottomOfPipe, "bottom_of_pipe"},
		{PipelineStage(99), "unknown"},
	}
	for _, tt := range tests {
		if got := tt.ps.String(); got != tt.want {
			t.Errorf("PipelineStage(%d).String() = %q, want %q", tt.ps, got, tt.want)
		}
	}
}

// =========================================================================
// CommandBufferState tests
// =========================================================================

func TestCommandBufferStateString(t *testing.T) {
	tests := []struct {
		s    CommandBufferState
		want string
	}{
		{CommandBufferStateInitial, "initial"},
		{CommandBufferStateRecording, "recording"},
		{CommandBufferStateRecorded, "recorded"},
		{CommandBufferStatePending, "pending"},
		{CommandBufferStateComplete, "complete"},
		{CommandBufferState(99), "unknown"},
	}
	for _, tt := range tests {
		if got := tt.s.String(); got != tt.want {
			t.Errorf("CommandBufferState(%d).String() = %q, want %q", tt.s, got, tt.want)
		}
	}
}

// =========================================================================
// RuntimeEventType tests
// =========================================================================

func TestRuntimeEventTypeString(t *testing.T) {
	if got := RuntimeEventSubmit.String(); got != "SUBMIT" {
		t.Errorf("RuntimeEventSubmit.String() = %q, want %q", got, "SUBMIT")
	}
	if got := RuntimeEventMemoryTransfer.String(); got != "MEMORY_TRANSFER" {
		t.Errorf("RuntimeEventMemoryTransfer.String() = %q, want %q", got, "MEMORY_TRANSFER")
	}
	if got := RuntimeEventType(99).String(); got != "UNKNOWN" {
		t.Errorf("RuntimeEventType(99).String() = %q, want %q", got, "UNKNOWN")
	}
}

// =========================================================================
// Data structure tests
// =========================================================================

func TestQueueFamily(t *testing.T) {
	qf := QueueFamily{QueueType: QueueTypeCompute, Count: 4}
	if qf.QueueType != QueueTypeCompute {
		t.Errorf("QueueType = %v, want %v", qf.QueueType, QueueTypeCompute)
	}
	if qf.Count != 4 {
		t.Errorf("Count = %d, want %d", qf.Count, 4)
	}
}

func TestDefaultDeviceLimits(t *testing.T) {
	limits := DefaultDeviceLimits()
	if limits.MaxWorkgroupSize[0] != 1024 {
		t.Errorf("MaxWorkgroupSize[0] = %d, want %d", limits.MaxWorkgroupSize[0], 1024)
	}
	if limits.MaxBufferSize != 2*1024*1024*1024 {
		t.Errorf("MaxBufferSize = %d, want %d", limits.MaxBufferSize, 2*1024*1024*1024)
	}
	if limits.MaxPushConstantSize != 128 {
		t.Errorf("MaxPushConstantSize = %d, want %d", limits.MaxPushConstantSize, 128)
	}
}

func TestMemoryHeap(t *testing.T) {
	heap := MemoryHeap{Size: 1024, Flags: MemoryTypeDeviceLocal}
	if heap.Size != 1024 {
		t.Errorf("Size = %d, want %d", heap.Size, 1024)
	}
}

func TestDefaultDescriptorBinding(t *testing.T) {
	b := DefaultDescriptorBinding(0)
	if b.Binding != 0 {
		t.Errorf("Binding = %d, want 0", b.Binding)
	}
	if b.Type != "storage" {
		t.Errorf("Type = %q, want %q", b.Type, "storage")
	}
	if b.Count != 1 {
		t.Errorf("Count = %d, want 1", b.Count)
	}
}

func TestDefaultPipelineBarrier(t *testing.T) {
	pb := DefaultPipelineBarrier()
	if pb.SrcStage != PipelineStageTopOfPipe {
		t.Errorf("SrcStage = %v, want %v", pb.SrcStage, PipelineStageTopOfPipe)
	}
	if pb.DstStage != PipelineStageBottomOfPipe {
		t.Errorf("DstStage = %v, want %v", pb.DstStage, PipelineStageBottomOfPipe)
	}
}

func TestRuntimeTraceFormat(t *testing.T) {
	trace := RuntimeTrace{
		TimestampCycles: 100,
		EventType:       RuntimeEventSubmit,
		Description:     "CB#0 to compute queue",
	}
	got := trace.Format()
	want := "[T=100 cycles] SUBMIT -- CB#0 to compute queue"
	if got != want {
		t.Errorf("Format() = %q, want %q", got, want)
	}

	// Without description
	trace2 := RuntimeTrace{
		TimestampCycles: 0,
		EventType:       RuntimeEventBarrier,
	}
	got2 := trace2.Format()
	want2 := "[T=0 cycles] BARRIER"
	if got2 != want2 {
		t.Errorf("Format() = %q, want %q", got2, want2)
	}
}

func TestRuntimeStatsUpdateUtilization(t *testing.T) {
	stats := &RuntimeStats{
		TotalDeviceCycles: 80,
		TotalIdleCycles:   20,
	}
	stats.UpdateUtilization()
	if stats.GPUUtilization != 0.8 {
		t.Errorf("GPUUtilization = %f, want %f", stats.GPUUtilization, 0.8)
	}

	// Zero total
	stats2 := &RuntimeStats{}
	stats2.UpdateUtilization()
	if stats2.GPUUtilization != 0.0 {
		t.Errorf("GPUUtilization = %f, want %f", stats2.GPUUtilization, 0.0)
	}
}

func TestRecordedCommand(t *testing.T) {
	cmd := RecordedCommand{
		Command: "dispatch",
		Args:    map[string]interface{}{"group_x": 4},
	}
	if cmd.Command != "dispatch" {
		t.Errorf("Command = %q, want %q", cmd.Command, "dispatch")
	}
}

func TestAccessFlags(t *testing.T) {
	if AccessFlagsNone != 0 {
		t.Errorf("AccessFlagsNone = %d, want 0", AccessFlagsNone)
	}
	if AccessFlagsShaderRead == 0 {
		t.Error("AccessFlagsShaderRead should be non-zero")
	}
}
