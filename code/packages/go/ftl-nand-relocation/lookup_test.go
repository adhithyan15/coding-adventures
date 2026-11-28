package ftlnandrelocation

import (
	"bytes"
	"errors"
	"math"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestLookupRejectsMalformedDeviceBeforeLogicalAddress(t *testing.T) {
	var zero Device
	if physical, present, err := zero.Lookup(-1); physical != 0 || present || !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero-device Lookup(-1) = %d/%t/%v; want geometry refusal", physical, present, err)
	}
}

func TestLookupReturnsZeroForAbsence(t *testing.T) {
	device, err := New(1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	if physical, present, err := device.Lookup(0); physical != 0 || present || err != nil {
		t.Fatalf("absent Lookup(0) = %d/%t/%v; want zero/absent/nil", physical, present, err)
	}
	for _, logical := range []int64{-1, 1, math.MinInt64, math.MaxInt64} {
		if physical, present, err := device.Lookup(logical); physical != 0 || present || !errors.Is(err, ftlpagemap.ErrInvalidLogicalPage) {
			t.Errorf("Lookup(%d) = %d/%t/%v; want invalid logical address", logical, physical, present, err)
		}
	}
}

func TestLookupMarksPhysicalPageZeroPresent(t *testing.T) {
	device, err := New(1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, receipt, err := device.Write(0, []byte{0x12, 0x34}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt.Physical != 0 {
		t.Fatalf("initial write = %+v, %v", receipt, err)
	}
	if physical, present, err := device.Lookup(0); physical != 0 || !present || err != nil {
		t.Fatalf("mapped page-zero Lookup(0) = %d/%t/%v; want zero/present/nil", physical, present, err)
	}
	data, present, err := device.Read(0)
	if err != nil || !present || !bytes.Equal(data, []byte{0x12, 0x34}) {
		t.Fatalf("mapped physical zero read = %x/%t/%v", data, present, err)
	}
}

func TestLookupPreservesReadBytesAndDetachedOwnership(t *testing.T) {
	device, err := New(1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0xa5, 0x5a}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	if physical, present, err := device.Lookup(0); physical != 0 || !present || err != nil {
		t.Fatalf("Lookup before Read = %d/%t/%v", physical, present, err)
	}
	data, present, err := device.Read(0)
	if err != nil || !present || !bytes.Equal(data, []byte{0xa5, 0x5a}) {
		t.Fatalf("lookup-preserved read bytes = %x/%t/%v", data, present, err)
	}
	data[0] = 0
	again, againPresent, againErr := device.Read(0)
	if againErr != nil || !againPresent || !bytes.Equal(again, []byte{0xa5, 0x5a}) {
		t.Fatalf("lookup read output aliases device bytes: %x/%t/%v", again, againPresent, againErr)
	}
}

func TestLookupFollowsEveryOwnerAtMaximumLegacyGeometry(t *testing.T) {
	device, err := New(64, 128, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitial(64, 128, 1)
	initial := device
	var firstMapped Device
	for logical := int64(0); logical < 64; logical++ {
		payload := []byte{byte(logical + 1)}
		device, model = compareTransition(t, device, model, logical, payload, modelCut{kind: cutComplete})
		if logical == 0 {
			firstMapped = device
		}
	}

	for consumed := 0; consumed < 63; consumed++ {
		device, model = compareTransition(t, device, model, 0, []byte{0xff}, modelCut{kind: cutAfterPrefix, prefix: 0})
	}
	device, model = compareTransition(t, device, model, 0, []byte{0x7a}, modelCut{kind: cutComplete})
	if physical, present, err := device.Lookup(0); physical != 127 || !present || err != nil {
		t.Fatalf("upper physical endpoint Lookup(0) = %d/%t/%v; want 127/present/nil", physical, present, err)
	}
	for logical := int64(0); logical < 64; logical++ {
		physical, present, err := device.Lookup(logical)
		if err != nil || !present || physical != model.mapping[int(logical)] {
			t.Errorf("final Lookup(%d) = %d/%t/%v; want mapped owner %d", logical, physical, present, err, model.mapping[int(logical)])
		}
	}

	if physical, present, err := initial.Lookup(0); physical != 0 || present || err != nil {
		t.Fatalf("initial predecessor changed: Lookup(0) = %d/%t/%v", physical, present, err)
	}
	if physical, present, err := firstMapped.Lookup(0); physical != 0 || !present || err != nil {
		t.Fatalf("retained predecessor changed: Lookup(0) = %d/%t/%v", physical, present, err)
	}
	if data, present, err := firstMapped.Read(0); err != nil || !present || !bytes.Equal(data, []byte{1}) {
		t.Fatalf("retained predecessor read = %x/%t/%v", data, present, err)
	}
	assertDeviceEquals(t, device, model)
}
