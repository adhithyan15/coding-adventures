package devicedriverframework

import (
	"bytes"
	"testing"
)

func TestSimulatedDiskDefaults(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 2048)
	if disk.Name != "disk0" {
		t.Errorf("Name = %q, want disk0", disk.Name)
	}
	if disk.Type != DeviceBlock {
		t.Errorf("Type = %v, want BLOCK", disk.Type)
	}
	if disk.Major != MajorDisk {
		t.Errorf("Major = %d, want %d", disk.Major, MajorDisk)
	}
	if disk.BlockSize() != 512 {
		t.Errorf("BlockSize = %d, want 512", disk.BlockSize())
	}
	if disk.TotalBlocks() != 2048 {
		t.Errorf("TotalBlocks = %d, want 2048", disk.TotalBlocks())
	}
	if disk.InterruptNumber != IntDisk {
		t.Errorf("InterruptNumber = %d, want %d", disk.InterruptNumber, IntDisk)
	}
}

func TestSimulatedDiskInit(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	disk.WriteBlock(0, bytes.Repeat([]byte{0xFF}, 512))
	disk.Init()
	if !disk.Initialized {
		t.Error("Should be initialized after Init()")
	}
	data, _ := disk.ReadBlock(0)
	if !bytes.Equal(data, make([]byte, 512)) {
		t.Error("Init() should zero the storage")
	}
}

func TestSimulatedDiskReadFreshDisk(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	data, err := disk.ReadBlock(0)
	if err != nil {
		t.Fatalf("ReadBlock(0) error: %v", err)
	}
	if len(data) != 512 {
		t.Errorf("ReadBlock returned %d bytes, want 512", len(data))
	}
	if !bytes.Equal(data, make([]byte, 512)) {
		t.Error("Fresh disk should return all zeros")
	}
}

func TestSimulatedDiskWriteThenRead(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	payload := make([]byte, 512)
	for i := range payload {
		payload[i] = byte(i % 256)
	}
	if err := disk.WriteBlock(2, payload); err != nil {
		t.Fatalf("WriteBlock error: %v", err)
	}
	data, err := disk.ReadBlock(2)
	if err != nil {
		t.Fatalf("ReadBlock error: %v", err)
	}
	if !bytes.Equal(data, payload) {
		t.Error("ReadBlock should return what was written")
	}
}

func TestSimulatedDiskWriteDoesNotAffectOtherBlocks(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	disk.WriteBlock(1, bytes.Repeat([]byte{0xAA}, 512))
	data0, _ := disk.ReadBlock(0)
	data2, _ := disk.ReadBlock(2)
	if !bytes.Equal(data0, make([]byte, 512)) {
		t.Error("Block 0 should be unaffected")
	}
	if !bytes.Equal(data2, make([]byte, 512)) {
		t.Error("Block 2 should be unaffected")
	}
}

func TestSimulatedDiskReadOutOfRange(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	_, err := disk.ReadBlock(4)
	if err == nil {
		t.Error("ReadBlock(4) should return error for 4-block disk")
	}
}

func TestSimulatedDiskReadNegative(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	_, err := disk.ReadBlock(-1)
	if err == nil {
		t.Error("ReadBlock(-1) should return error")
	}
}

func TestSimulatedDiskWriteOutOfRange(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	err := disk.WriteBlock(4, make([]byte, 512))
	if err == nil {
		t.Error("WriteBlock(4) should return error")
	}
}

func TestSimulatedDiskWriteWrongSize(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	err := disk.WriteBlock(0, make([]byte, 100))
	if err == nil {
		t.Error("WriteBlock with wrong size should return error")
	}
}

func TestSimulatedDiskWriteNegative(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	err := disk.WriteBlock(-1, make([]byte, 512))
	if err == nil {
		t.Error("WriteBlock(-1) should return error")
	}
}

func TestSimulatedDiskLastBlock(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	data := bytes.Repeat([]byte{0xFF}, 512)
	disk.WriteBlock(3, data)
	result, _ := disk.ReadBlock(3)
	if !bytes.Equal(result, data) {
		t.Error("Should be able to read/write the last block")
	}
}

func TestSimulatedDiskOverwrite(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	disk.WriteBlock(0, bytes.Repeat([]byte{0xAA}, 512))
	disk.WriteBlock(0, bytes.Repeat([]byte{0xBB}, 512))
	data, _ := disk.ReadBlock(0)
	if !bytes.Equal(data, bytes.Repeat([]byte{0xBB}, 512)) {
		t.Error("Overwriting should use the latest data")
	}
}

func TestSimulatedDiskStorage(t *testing.T) {
	disk := NewSimulatedDisk("disk0", 0, 512, 4)
	if len(disk.Storage()) != 4*512 {
		t.Errorf("Storage length = %d, want %d", len(disk.Storage()), 4*512)
	}
}
