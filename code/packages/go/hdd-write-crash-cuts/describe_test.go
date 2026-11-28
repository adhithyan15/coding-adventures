package hddwritecrashcuts

import (
	"errors"
	"reflect"
	"testing"
)

var (
	describedPrefix int
	describedAck    bool
	describedErr    error
)

func descriptorOracle(cut Cut) (int, bool, error) {
	switch cut.Kind {
	case BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return 0, false, ErrInvalidPrefix
		}
		return 0, false, nil
	case DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix > 7 {
			return 0, false, ErrInvalidPrefix
		}
		return *cut.WrittenPrefix, false, nil
	case AfterMediaWriteBeforeResponse:
		if cut.WrittenPrefix != nil {
			return 0, false, ErrInvalidPrefix
		}
		return 8, false, nil
	case Acknowledged:
		if cut.WrittenPrefix != nil {
			return 0, false, ErrInvalidPrefix
		}
		return 8, true, nil
	default:
		return 0, false, ErrInvalidCut
	}
}

func TestDescribeCrossesEveryKindAndNativePrefixShape(t *testing.T) {
	maximum := int(^uint(0) >> 1)
	minimum := -maximum - 1
	prefixes := []*int{
		nil,
		intPointer(minimum), intPointer(-1), intPointer(0),
		intPointer(1), intPointer(2), intPointer(3), intPointer(4),
		intPointer(5), intPointer(6), intPointer(7), intPointer(8),
		intPointer(maximum),
	}
	kinds := []CutKind{
		BeforeMediaWrite, DuringMediaWrite, AfterMediaWriteBeforeResponse,
		Acknowledged, CutKind("unknown"),
	}
	image := patternedImage(0x29)
	data := []byte{0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7}
	checked := 0
	for _, kind := range kinds {
		for _, prefix := range prefixes {
			cut := Cut{Kind: kind, WrittenPrefix: prefix}
			wantBytes, wantAck, wantErr := descriptorOracle(cut)
			gotBytes, gotAck, gotErr := Describe(cut)
			if gotBytes != wantBytes || gotAck != wantAck || !errors.Is(gotErr, wantErr) {
				t.Fatalf("Describe(%q,%v)=(%d,%t,%v), want (%d,%t,%v)", kind, pointerValue(prefix), gotBytes, gotAck, gotErr, wantBytes, wantAck, wantErr)
			}
			result, applyErr := Apply(image, 17, data, cut)
			if !errors.Is(applyErr, wantErr) {
				t.Fatalf("Apply(%q,%v) error=%v, Describe model=%v", kind, pointerValue(prefix), applyErr, wantErr)
			}
			if wantErr != nil {
				if !reflect.DeepEqual(result, Result{}) || gotBytes != 0 || gotAck {
					t.Fatalf("refusal was not empty: result=%+v description=(%d,%t)", result, gotBytes, gotAck)
				}
			} else if result.PersistedPrefixBytes != gotBytes || (result.Receipt != nil) != gotAck {
				t.Fatalf("Apply and Describe disagree for %q: result=%+v description=(%d,%t)", kind, result, gotBytes, gotAck)
			}
			checked++
		}
	}
	if checked != len(kinds)*len(prefixes) {
		t.Fatalf("descriptor combinations=%d", checked)
	}
	t.Logf("checked %d descriptor-kind and native-int prefix combinations", checked)
}

func pointerValue(value *int) any {
	if value == nil {
		return nil
	}
	return *value
}

func TestDescribeAllocatesNothing(t *testing.T) {
	valid := Cut{Kind: DuringMediaWrite, WrittenPrefix: intPointer(7)}
	invalid := Cut{Kind: DuringMediaWrite, WrittenPrefix: intPointer(0)}
	if allocations := testing.AllocsPerRun(1_000, func() {
		describedPrefix, describedAck, describedErr = Describe(valid)
	}); allocations != 0 {
		t.Fatalf("valid Describe allocated %v times", allocations)
	}
	if allocations := testing.AllocsPerRun(1_000, func() {
		describedPrefix, describedAck, describedErr = Describe(invalid)
	}); allocations != 0 {
		t.Fatalf("refused Describe allocated %v times", allocations)
	}
}

func FuzzDescribeCut(f *testing.F) {
	f.Add("before_media_write", 0, false)
	f.Add("during_media_write", 7, true)
	f.Add("during_media_write", 0, true)
	f.Add("after_media_write_before_response", 8, true)
	f.Add("acknowledged", 0, false)
	f.Add("unknown", -1, true)
	f.Fuzz(func(t *testing.T, kind string, prefix int, present bool) {
		var written *int
		if present {
			written = &prefix
		}
		cut := Cut{Kind: CutKind(kind), WrittenPrefix: written}
		wantBytes, wantAck, wantErr := descriptorOracle(cut)
		gotBytes, gotAck, gotErr := Describe(cut)
		if gotBytes != wantBytes || gotAck != wantAck || !errors.Is(gotErr, wantErr) {
			t.Fatalf("Describe(%q,%v)=(%d,%t,%v), want (%d,%t,%v)", kind, pointerValue(written), gotBytes, gotAck, gotErr, wantBytes, wantAck, wantErr)
		}
	})
}

func TestDescribeLeavesExistingApplyBehaviorUnchanged(t *testing.T) {
	image := patternedImage(0x10)
	data := []byte{0, 1, 2, 3, 4, 5, 6, 7}
	for _, cut := range allCuts() {
		before, ack, err := Describe(cut)
		result, applyErr := Apply(image, 3, data, cut)
		if err != nil || applyErr != nil || result.PersistedPrefixBytes != before || (result.Receipt != nil) != ack {
			t.Fatalf("cut=%+v Describe=(%d,%t,%v) Apply=(%+v,%v)", cut, before, ack, err, result, applyErr)
		}
	}
	if !reflect.DeepEqual(image, patternedImage(0x10)) {
		t.Fatal("Describe or Apply changed the input during comparison")
	}
}
