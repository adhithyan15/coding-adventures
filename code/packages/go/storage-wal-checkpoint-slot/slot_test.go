package storagewalcheckpointslot

import (
	"bytes"
	"encoding/hex"
	"errors"
	"fmt"
	"testing"

	hddblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
	storagewalcheckpointrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record"
)

func record(phase storagewalcheckpointrecord.Phase, epoch, previous uint64) storagewalcheckpointrecord.Record {
	return storagewalcheckpointrecord.Record{Phase: phase, Epoch: epoch, PreviousLSN: previous}
}

func cuts() []struct {
	name string
	cut  hddwritecrashcuts.Cut
} {
	values := []struct {
		name string
		cut  hddwritecrashcuts.Cut
	}{
		{"before", hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}},
		{"after", hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}},
		{"acknowledged", hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}},
	}
	for prefix := 1; prefix < hddwritecrashcuts.BytesPerSector; prefix++ {
		value := prefix
		values = append(values, struct {
			name string
			cut  hddwritecrashcuts.Cut
		}{
			name: fmt.Sprintf("prefix-%d", prefix),
			cut:  hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value},
		})
	}
	return values
}

func frameAt(image [][]byte, first int) []byte {
	frame := make([]byte, 0, storagewalcheckpointrecord.FrameBytes)
	for i := 0; i < storagewalcheckpointrecord.FrameBytes/hddwritecrashcuts.BytesPerSector; i++ {
		frame = append(frame, image[first+i]...)
	}
	return frame
}

func putFrame(image [][]byte, first int, frame []byte) {
	for i := 0; i < storagewalcheckpointrecord.FrameBytes/hddwritecrashcuts.BytesPerSector; i++ {
		copy(image[first+i], frame[i*hddwritecrashcuts.BytesPerSector:(i+1)*hddwritecrashcuts.BytesPerSector])
	}
}

func blankImage() [][]byte {
	image := make([][]byte, hddwritecrashcuts.SectorCount)
	for i := range image {
		image[i] = make([]byte, hddwritecrashcuts.BytesPerSector)
	}
	return image
}

func apply(t *testing.T, before [][]byte, lba int64, data []byte, cut hddwritecrashcuts.Cut) [][]byte {
	t.Helper()
	result, err := hddwritecrashcuts.Apply(before, lba, data, cut)
	if err != nil {
		t.Fatalf("physical write lba %d cut %+v: %v", lba, cut, err)
	}
	return result.Image
}

func TestStageMatchesFinalFrameWithOnlyStagedGuardChange(t *testing.T) {
	record := record(storagewalcheckpointrecord.Resetting, 1, 0)
	final, err := storagewalcheckpointrecord.Encode(record)
	if err != nil {
		t.Fatal(err)
	}
	staged, err := Stage(record)
	if err != nil {
		t.Fatal(err)
	}
	want, _ := hex.DecodeString("444252460100000018000000444243500101a50001000000000000000000000000000000aeb4e057")
	if len(staged) != 40 || !bytes.Equal(staged, want) {
		t.Fatalf("staged=%x", staged)
	}
	if staged[GuardOffset] != StagedGuard || !bytes.Equal(staged[:GuardOffset], final[:GuardOffset]) || !bytes.Equal(staged[GuardOffset+1:], final[GuardOffset+1:]) {
		t.Fatal("staging changed bytes beyond the publication guard")
	}
	staged[0] ^= 0xff
	again, err := Stage(record)
	if err != nil || !bytes.Equal(again, want) {
		t.Fatalf("Stage does not return fresh bytes: %x %v", again, err)
	}
}

func TestStagePreservesRecordRefusal(t *testing.T) {
	frame, err := Stage(record(0, 0, ^uint64(0)))
	if frame != nil || !errors.Is(err, storagewalcheckpointrecord.ErrPhase) {
		t.Fatalf("Stage=(%x,%v)", frame, err)
	}
}

func TestInspectAllGuardBytesAndSlotKinds(t *testing.T) {
	for value := 0; value < 256; value++ {
		t.Run(fmt.Sprintf("guard-%02x", value), func(t *testing.T) {
			slot := make([]byte, storagewalcheckpointrecord.FrameBytes)
			slot[GuardOffset] = byte(value)
			state, got, err := Inspect(slot)
			switch byte(value) {
			case 0:
				if state != Vacant || got != (storagewalcheckpointrecord.Record{}) || err != nil {
					t.Fatalf("vacant=(%d,%+v,%v)", state, got, err)
				}
			case StagedGuard:
				if state != Staged || got != (storagewalcheckpointrecord.Record{}) || err != nil {
					t.Fatalf("staged=(%d,%+v,%v)", state, got, err)
				}
			default:
				if state != 0 || got != (storagewalcheckpointrecord.Record{}) || !errors.Is(err, ErrSlotGuard) {
					t.Fatalf("guard refusal=(%d,%+v,%v)", state, got, err)
				}
			}
		})
	}
	publishedRecord := record(storagewalcheckpointrecord.Ready, 7, 12)
	published, err := storagewalcheckpointrecord.Encode(publishedRecord)
	if err != nil {
		t.Fatal(err)
	}
	state, got, err := Inspect(published)
	if err != nil || state != Published || got != publishedRecord {
		t.Fatalf("published=(%d,%+v,%v)", state, got, err)
	}
}

func TestInspectWidthAndPublishedLowerErrors(t *testing.T) {
	for _, width := range []int{0, 39, 41} {
		state, record, err := Inspect(make([]byte, width))
		if state != 0 || record != (storagewalcheckpointrecord.Record{}) || !errors.Is(err, ErrSlotWidth) {
			t.Fatalf("width %d=(%d,%+v,%v)", width, state, record, err)
		}
	}
	published, _ := storagewalcheckpointrecord.Encode(record(storagewalcheckpointrecord.Ready, 1, 0))
	published[20] ^= 1
	state, record, err := Inspect(published)
	if state != 0 || record != (storagewalcheckpointrecord.Record{}) || !errors.Is(err, storagerecordframe.ErrChecksum) {
		t.Fatalf("lower published error=(%d,%+v,%v)", state, record, err)
	}
}

func TestPhysicalInactiveSlotGuardBodyAndFinalByteCuts(t *testing.T) {
	const slotFirst = 27
	const guardSector = slotFirst + GuardOffset/hddwritecrashcuts.BytesPerSector
	const guardWithinSector = GuardOffset % hddwritecrashcuts.BytesPerSector
	oldRecord := record(storagewalcheckpointrecord.Ready, 3, 8)
	newRecord := record(storagewalcheckpointrecord.Resetting, 4, 11)
	oldFrame, _ := storagewalcheckpointrecord.Encode(oldRecord)
	stagedFrame, err := Stage(newRecord)
	if err != nil {
		t.Fatal(err)
	}
	finalFrame, _ := storagewalcheckpointrecord.Encode(newRecord)

	initial := blankImage()
	putFrame(initial, slotFirst, oldFrame)
	guardSectorData := append([]byte(nil), oldFrame[2*hddwritecrashcuts.BytesPerSector:3*hddwritecrashcuts.BytesPerSector]...)
	guardSectorData[guardWithinSector] = StagedGuard
	for _, choice := range cuts() {
		t.Run("guard-first/"+choice.name, func(t *testing.T) {
			image := apply(t, initial, guardSector, guardSectorData, choice.cut)
			state, got, err := Inspect(frameAt(image, slotFirst))
			if err != nil {
				t.Fatal(err)
			}
			if state == Published {
				if got != oldRecord {
					t.Fatalf("old published record=%+v", got)
				}
			} else if state != Staged || got != (storagewalcheckpointrecord.Record{}) {
				t.Fatalf("guard cut exposed state=(%d,%+v)", state, got)
			}
		})
	}

	stagedImage := apply(t, initial, guardSector, guardSectorData, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged})
	for sector := 0; sector < storagewalcheckpointrecord.FrameBytes/hddwritecrashcuts.BytesPerSector; sector++ {
		for _, choice := range cuts() {
			name := fmt.Sprintf("body/sector-%d/%s", sector, choice.name)
			t.Run(name, func(t *testing.T) {
				image := apply(t, stagedImage, int64(slotFirst+sector), stagedFrame[sector*8:(sector+1)*8], choice.cut)
				state, got, err := Inspect(frameAt(image, slotFirst))
				if err != nil || state != Staged || got != (storagewalcheckpointrecord.Record{}) {
					t.Fatalf("body cut=(%d,%+v,%v)", state, got, err)
				}
			})
		}
		stagedImage = apply(t, stagedImage, int64(slotFirst+sector), stagedFrame[sector*8:(sector+1)*8], hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged})
	}
	if !bytes.Equal(frameAt(stagedImage, slotFirst), stagedFrame) {
		t.Fatal("body staging bytes differ from staged frame")
	}
	finalSector := append([]byte(nil), finalFrame[2*8:3*8]...)
	for _, choice := range cuts() {
		t.Run("guard-clear-last/"+choice.name, func(t *testing.T) {
			image := apply(t, stagedImage, guardSector, finalSector, choice.cut)
			state, got, err := Inspect(frameAt(image, slotFirst))
			if err != nil {
				t.Fatal(err)
			}
			if state == Staged {
				if got != (storagewalcheckpointrecord.Record{}) {
					t.Fatalf("staged exposed record %+v", got)
				}
			} else if state != Published || got != newRecord {
				t.Fatalf("final guard cut=(%d,%+v)", state, got)
			}
		})
	}
}

func TestPhysicalHDDInactiveSlotGuardBodyFinalOrder(t *testing.T) {
	const slotFirst = int64(27)
	const otherFirst = int64(22)
	device := hddblockdevice.New()
	oldRecord := record(storagewalcheckpointrecord.Ready, 7, 19)
	newRecord := record(storagewalcheckpointrecord.Resetting, 8, 20)
	otherRecord := record(storagewalcheckpointrecord.Ready, 6, 13)
	oldFrame, _ := storagewalcheckpointrecord.Encode(oldRecord)
	stagedFrame, err := Stage(newRecord)
	if err != nil {
		t.Fatal(err)
	}
	otherFrame, _ := storagewalcheckpointrecord.Encode(otherRecord)
	writeFrame := func(first int64, frame []byte) {
		t.Helper()
		for i := 0; i < len(frame)/hddwritecrashcuts.BytesPerSector; i++ {
			if err := device.WriteBlock(first+int64(i), frame[i*8:(i+1)*8]); err != nil {
				t.Fatal(err)
			}
		}
	}
	readFrame := func(first int64) []byte {
		t.Helper()
		var frame []byte
		for i := int64(0); i < 5; i++ {
			sector, err := device.ReadBlock(first + i)
			if err != nil {
				t.Fatal(err)
			}
			frame = append(frame, sector...)
		}
		return frame
	}
	writeFrame(slotFirst, oldFrame)
	writeFrame(otherFirst, otherFrame)
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	guardSector := append([]byte(nil), oldFrame[16:24]...)
	guardSector[2] = StagedGuard
	if err := device.WriteBlock(slotFirst+2, guardSector); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	state, value, err := Inspect(readFrame(slotFirst))
	if err != nil || state != Staged || value != (storagewalcheckpointrecord.Record{}) {
		t.Fatalf("guard durable=(%d,%+v,%v)", state, value, err)
	}
	writeFrame(slotFirst, stagedFrame)
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	state, value, err = Inspect(readFrame(slotFirst))
	if err != nil || state != Staged || value != (storagewalcheckpointrecord.Record{}) {
		t.Fatalf("staged body durable=(%d,%+v,%v)", state, value, err)
	}
	finalSector := append([]byte(nil), stagedFrame[16:24]...)
	finalSector[2] = 0
	if err := device.WriteBlock(slotFirst+2, finalSector); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	state, value, err = Inspect(readFrame(slotFirst))
	if err != nil || state != Published || value != newRecord {
		t.Fatalf("published body=(%d,%+v,%v)", state, value, err)
	}
	state, value, err = Inspect(readFrame(otherFirst))
	if err != nil || state != Published || value != otherRecord {
		t.Fatalf("neighbor slot changed=(%d,%+v,%v)", state, value, err)
	}
}

func FuzzInspectCheckpointSlots(f *testing.F) {
	good, _ := Stage(record(storagewalcheckpointrecord.Ready, 2, 1))
	f.Add(good)
	f.Add(make([]byte, storagewalcheckpointrecord.FrameBytes))
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, slot []byte) {
		if len(slot) > 128 {
			t.Skip()
		}
		state, record, err := Inspect(slot)
		if err != nil {
			return
		}
		switch state {
		case Vacant, Staged:
			if record != (storagewalcheckpointrecord.Record{}) {
				t.Fatalf("nonpublished state exposes record %+v", record)
			}
		case Published:
			frame, err := storagewalcheckpointrecord.Encode(record)
			if err != nil || !bytes.Equal(frame, slot) {
				t.Fatalf("published round-trip=(%x,%v)", frame, err)
			}
		default:
			t.Fatalf("unknown successful state %d", state)
		}
	})
}
