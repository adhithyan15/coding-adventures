package ftlnandrelocation

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestRestoreQuiescentValidationOrderAndZeroResults(t *testing.T) {
	media, snapshot := restoreFixtureEvidence()
	badPageWidth := media
	badPageWidth.PageBytes = 0
	badMapShape := snapshot
	badMapShape.LogicalToPhysical = nil
	badBlock := cloneRestoreMedia(media)
	badBlock.Blocks[0].Pages = badBlock.Blocks[0].Pages[:1]
	badCount := cloneRestoreMedia(media)
	badCount.Blocks[0].CompletedErases = 4
	badCount.Blocks[0].IncompleteErase = true
	badCount.Blocks[0].Pages[0].Bytes[0] = 0
	badIncomplete := cloneRestoreMedia(media)
	badIncomplete.Blocks[0].IncompleteErase = true
	badIncomplete.Blocks[0].Pages[0].Bytes = []byte{0}
	badPage := cloneRestoreMedia(media)
	badPage.Blocks[0].Pages[0].Bytes = []byte{0, 0xff}
	badRole := cloneRestoreMedia(media)
	badRole.Blocks[0].Pages[0].Programmed = true

	cases := []struct {
		name     string
		media    MediaObservation
		snapshot ftlpagemap.Snapshot
		wantErr  error
	}{
		{"zero block count", MediaObservation{}, snapshot, ErrInvalidBlockGeometry},
		{"zero page extent", MediaObservation{Blocks: []nandblockerasecuts.Block{{}}, PagesPerBlock: 0}, snapshot, ErrInvalidBlockGeometry},
		{"one physical page delegates to page map", MediaObservation{Blocks: []nandblockerasecuts.Block{{Pages: []nandblockerasecuts.Page{{Bytes: []byte{0xff}}}}}, PagesPerBlock: 1, LogicalPages: 1}, ftlpagemap.Snapshot{}, ftlpagemap.ErrInvalidGeometry},
		{"checked product exceeds limit", MediaObservation{Blocks: make([]nandblockerasecuts.Block, 3), PagesPerBlock: 64}, snapshot, ErrInvalidBlockGeometry},
		{"map error precedes page width", badPageWidth, badMapShape, ftlpagemap.ErrInvalidSnapshotShape},
		{"map geometry precedes page width", badPageWidth, ftlpagemap.Snapshot{LogicalToPhysical: []int64{0}, PhysicalStatus: []ftlpagemap.PhysicalStatus{ftlpagemap.Live, ftlpagemap.Free}}, ErrInvalidPageSize},
		{"erase ceiling after map", MediaObservation{LogicalPages: 1, PagesPerBlock: 2, PageBytes: 2, MaxCompletedErases: maxSafeEraseCount + 1, Blocks: media.Blocks}, snapshot, nandblockerasecuts.ErrInvalidEraseCount},
		{"block shape precedes count and page faults", badBlock, snapshot, ErrInvalidMediaBlock},
		{"block count precedes incomplete and pages", badCount, snapshot, nandblockerasecuts.ErrInvalidEraseCount},
		{"incomplete block precedes page faults", badIncomplete, snapshot, ErrIncompleteMediaBlock},
		{"page width", func() MediaObservation {
			m := cloneRestoreMedia(media)
			m.Blocks[0].Pages[0].Bytes = []byte{0xff}
			return m
		}(), snapshot, ErrInvalidPageSize},
		{"clear marker requires erased bytes", badPage, snapshot, nandpageprogramcuts.ErrInvalidPageState},
		{"free role requires clear marker", badRole, snapshot, ErrMediaRoleMismatch},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			got, err := RestoreQuiescent(test.media, test.snapshot)
			if !errors.Is(err, test.wantErr) || !reflect.DeepEqual(got, Device{}) {
				t.Fatalf("RestoreQuiescent() error=%v device=%+v, want zero device and %v", err, got, test.wantErr)
			}
		})
	}

	media, snapshot = restoreFixtureEvidence()
	snapshot.Reservation = &ftlpagemap.Reservation{Logical: 0, Physical: 0}
	if got, err := RestoreQuiescent(media, snapshot); !errors.Is(err, ftlpagemap.ErrReservationPending) || !reflect.DeepEqual(got, Device{}) {
		t.Fatalf("pending map restore = %+v/%v, want zero/reservation refusal", got, err)
	}
}

func TestRestoreQuiescentIndependentPhysicalRoleModel(t *testing.T) {
	const physicalCount = 4
	statuses := []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Live, ftlpagemap.Stale}
	checked := 0
	for roleCode := 0; roleCode < 81; roleCode++ {
		roles := make([]ftlpagemap.PhysicalStatus, physicalCount)
		value := roleCode
		for index := range roles {
			roles[index] = statuses[value%len(statuses)]
			value /= len(statuses)
		}
		for logicalPhysical := int64(-1); logicalPhysical < physicalCount; logicalPhysical++ {
			snapshot := ftlpagemap.Snapshot{
				LogicalToPhysical: []int64{logicalPhysical},
				PhysicalStatus:    append([]ftlpagemap.PhysicalStatus(nil), roles...),
			}
			if _, err := ftlpagemap.Restore(1, physicalCount, snapshot); err != nil {
				continue
			}
			base := MediaObservation{
				LogicalPages:       1,
				PagesPerBlock:      2,
				PageBytes:          2,
				MaxCompletedErases: 2,
				Blocks: []nandblockerasecuts.Block{
					{Pages: []nandblockerasecuts.Page{{Bytes: []byte{0xff, 0xff}}, {Bytes: []byte{0xff, 0xff}}}},
					{Pages: []nandblockerasecuts.Page{{Bytes: []byte{0xff, 0xff}}, {Bytes: []byte{0xff, 0xff}}}},
				},
			}
			for physical, role := range roles {
				base.Blocks[physical/2].Pages[physical%2].Programmed = role != ftlpagemap.Free
			}
			for markerMask := 0; markerMask < 16; markerMask++ {
				for changedByte := -1; changedByte < physicalCount; changedByte++ {
					candidate := cloneRestoreMedia(base)
					for physical := 0; physical < physicalCount; physical++ {
						candidate.Blocks[physical/2].Pages[physical%2].Programmed = markerMask&(1<<physical) != 0
					}
					if changedByte >= 0 {
						candidate.Blocks[changedByte/2].Pages[changedByte%2].Bytes[0] = 0
					}
					wantErr := modelRestoreQuiescentError(candidate, snapshot)
					got, gotErr := RestoreQuiescent(candidate, snapshot)
					if !sameRestoreError(gotErr, wantErr) {
						t.Fatalf("roles=%v logical=%d markers=%04b changed=%d error=%v want=%v", roles, logicalPhysical, markerMask, changedByte, gotErr, wantErr)
					}
					if wantErr != nil {
						if !reflect.DeepEqual(got, Device{}) {
							t.Fatalf("refusal returned nonzero device for roles=%v markers=%04b", roles, markerMask)
						}
					} else {
						assertRestoredEvidence(t, got, candidate, snapshot)
					}
					checked++
				}
			}
		}
	}
	if checked < 1500 {
		t.Fatalf("independent role model checked only %d candidates", checked)
	}
}

func TestRestoreQuiescentContinuesRealWriteCopyEraseHistory(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 8, 2, 2, 5)
	if err != nil {
		t.Fatal(err)
	}
	payloads := [][]byte{{0x10, 0x11}, {0x20, 0x21}, {0x30, 0x31}}
	device, _, err = device.Write(0, payloads[0], Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	oldOwner, present, err := device.ObserveOwner(0)
	if err != nil || !present {
		t.Fatalf("initial owner = %+v/%t/%v", oldOwner, present, err)
	}
	device, _, err = device.Write(0, payloads[1], Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(1, payloads[2], Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	active, err := device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	active, copyReceipt, err := active.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || !copyReceipt.Receipt.Published || !copyReceipt.Receipt.Acknowledged {
		t.Fatalf("evacuation copy receipt=%+v err=%v", copyReceipt, err)
	}
	device, eraseReceipt, err := active.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.AfterMedia})
	if err != nil || !eraseReceipt.Released || eraseReceipt.Acknowledged {
		t.Fatalf("completed lost-ack erase receipt=%+v err=%v", eraseReceipt, err)
	}
	media, err := device.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	mapSnapshot := device.table.Snapshot()
	opened, err := RestoreQuiescent(media, mapSnapshot)
	if err != nil {
		t.Fatalf("restore real history: %v", err)
	}
	assertRestoredEvidence(t, opened, media, mapSnapshot)
	if opened.evacuationActive {
		t.Fatal("restored device retained an evacuation lifecycle")
	}
	if _, active := opened.EvacuationProgress(); active {
		t.Fatal("restored device reports an active evacuation")
	}
	if opened.MatchesOwner(oldOwner) {
		t.Fatal("owner observation from the prior process matched the restored device")
	}
	for logical, expected := range [][]byte{payloads[1], payloads[2]} {
		got, present, readErr := opened.Read(int64(logical))
		if readErr != nil || !present || !reflect.DeepEqual(got, expected) {
			t.Fatalf("restored read %d = %x/%t/%v, want %x", logical, got, present, readErr, expected)
		}
	}
	firstOwner, present, err := opened.ObserveOwner(0)
	if err != nil || !present || !opened.MatchesOwner(firstOwner) {
		t.Fatalf("restored current owner = %+v/%t/%v", firstOwner, present, err)
	}
	openedAgain, err := RestoreQuiescent(media, mapSnapshot)
	if err != nil {
		t.Fatal(err)
	}
	if openedAgain.MatchesOwner(firstOwner) {
		t.Fatal("restoring the same evidence reused an owner identity")
	}

	// Continue ordinary data operations and a completed stale-only erase.
	opened, _, err = opened.Write(0, []byte{0x40, 0x41}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatalf("write after restore: %v", err)
	}
	got, present, err := opened.Read(0)
	if err != nil || !present || !reflect.DeepEqual(got, []byte{0x40, 0x41}) {
		t.Fatalf("read after restored write = %x/%t/%v", got, present, err)
	}
	owner1, present, err := opened.ObserveOwner(1)
	if err != nil || !present {
		t.Fatalf("owner for erase setup = %+v/%t/%v", owner1, present, err)
	}
	opened, err = opened.InvalidateOwner(owner1)
	if err != nil {
		t.Fatal(err)
	}
	opened, eraseReceipt, err = opened.EraseBlock(1, EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil || !eraseReceipt.Released || !eraseReceipt.Acknowledged {
		t.Fatalf("post-restore erase = %+v/%v", eraseReceipt, err)
	}
	got, present, err = opened.Read(0)
	if err != nil || !present || !reflect.DeepEqual(got, []byte{0x40, 0x41}) {
		t.Fatalf("neighbor read after erase = %x/%t/%v", got, present, err)
	}
}

func TestRestoreQuiescentDetachedAtMaximumGeometry(t *testing.T) {
	device, err := NewWithEraseBlocks(64, 128, maxPageSize, 1, 3)
	if err != nil {
		t.Fatal(err)
	}
	media, err := device.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	snapshot := device.table.Snapshot()
	opened, err := RestoreQuiescent(media, snapshot)
	if err != nil {
		t.Fatalf("maximum restore: %v", err)
	}
	media.Blocks[0].Pages[0].Bytes[0] = 0
	snapshot.LogicalToPhysical[0] = 0
	first := opened.Snapshot()
	if first.Pages[0].Bytes[0] != 0xff || first.Map.LogicalToPhysical[0] != -1 {
		t.Fatal("mutating restore inputs changed the reopened device")
	}
	first.Pages[0].Bytes[0] = 0
	firstMedia, err := opened.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	firstMedia.Blocks[0].Pages[0].Bytes[1] = 0
	second := opened.Snapshot()
	secondMedia, err := opened.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	if second.Pages[0].Bytes[0] != 0xff || secondMedia.Blocks[0].Pages[0].Bytes[1] != 0xff {
		t.Fatal("mutating reopened snapshots changed retained physical bytes")
	}
}

func TestSeededRestoreQuiescentRoundTripsMatchPhysicalEvidence(t *testing.T) {
	random := rand.New(rand.NewSource(0x3B3B))
	for trial := 0; trial < 300; trial++ {
		device, err := NewWithEraseBlocks(3, 8, 2, 2, 6)
		if err != nil {
			t.Fatal(err)
		}
		for step := 0; step < 8; step++ {
			logical := int64(random.Intn(3))
			payload := []byte{byte(random.Intn(256)), byte(random.Intn(256))}
			cut := []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1}, {Kind: nandpageprogramcuts.AfterMedia}, {Kind: nandpageprogramcuts.Complete}}[random.Intn(4)]
			device, _, err = device.Write(logical, payload, cut)
			if err != nil && !errors.Is(err, ftlpagemap.ErrNoFreePage) {
				t.Fatalf("seeded write trial=%d step=%d: %v", trial, step, err)
			}
		}
		media, err := device.ObserveMedia()
		if err != nil {
			t.Fatal(err)
		}
		snapshot := device.table.Snapshot()
		next, err := RestoreQuiescent(media, snapshot)
		if err != nil {
			t.Fatalf("seeded restore trial=%d: %v", trial, err)
		}
		assertRestoredEvidence(t, next, media, snapshot)
	}
}

func FuzzRestoreQuiescentEvidence(f *testing.F) {
	for _, seed := range [][]byte{{}, {0}, {1, 2, 3}, {0xff, 0xff, 0xff, 0xff}, {7, 3, 2, 1, 0}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 32 {
			return
		}
		device, err := NewWithEraseBlocks(1, 4, 2, 2, 3)
		if err != nil {
			t.Fatal(err)
		}
		if len(commands) > 0 && commands[0]&1 != 0 {
			device, _, err = device.Write(0, []byte{0xaa, 0xff}, Cut{Kind: nandpageprogramcuts.Complete})
			if err != nil {
				t.Fatal(err)
			}
		}
		media, err := device.ObserveMedia()
		if err != nil {
			t.Fatal(err)
		}
		snapshot := device.table.Snapshot()
		if len(commands) > 1 {
			switch commands[1] % 8 {
			case 0:
				media.PagesPerBlock = 0
			case 1:
				media.PageBytes = 0
			case 2:
				media.Blocks[0].CompletedErases = media.MaxCompletedErases + 1
			case 3:
				media.Blocks[0].IncompleteErase = true
			case 4:
				media.Blocks[0].Pages[0].Programmed = true
			case 5:
				media.Blocks[0].Pages[0].Bytes[0] = 0
			case 6:
				snapshot.PhysicalStatus[0] = ftlpagemap.Reserved
			case 7:
				media.Blocks[0].Pages = media.Blocks[0].Pages[:1]
			}
		}
		wantErr := modelRestoreQuiescentError(media, snapshot)
		got, gotErr := RestoreQuiescent(media, snapshot)
		if !sameRestoreError(gotErr, wantErr) {
			t.Fatalf("runtime error=%v, independent model=%v, commands=%v", gotErr, wantErr, commands)
		}
		if wantErr == nil {
			assertRestoredEvidence(t, got, media, snapshot)
		} else if !reflect.DeepEqual(got, Device{}) {
			t.Fatal("refusal returned a nonzero device")
		}
	})
}

func restoreFixtureEvidence() (MediaObservation, ftlpagemap.Snapshot) {
	return MediaObservation{
			LogicalPages: 1, PagesPerBlock: 2, PageBytes: 2, MaxCompletedErases: 2,
			Blocks: []nandblockerasecuts.Block{{Pages: []nandblockerasecuts.Page{
				{Bytes: []byte{0xff, 0xff}}, {Bytes: []byte{0xff, 0xff}},
			}}},
		}, ftlpagemap.Snapshot{
			LogicalToPhysical: []int64{-1},
			PhysicalStatus:    []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Free},
		}
}

func cloneRestoreMedia(media MediaObservation) MediaObservation {
	clone := MediaObservation{
		LogicalPages: media.LogicalPages, PagesPerBlock: media.PagesPerBlock,
		PageBytes: media.PageBytes, MaxCompletedErases: media.MaxCompletedErases,
		Blocks: make([]nandblockerasecuts.Block, len(media.Blocks)),
	}
	for blockIndex, block := range media.Blocks {
		clone.Blocks[blockIndex] = nandblockerasecuts.Block{
			CompletedErases: block.CompletedErases, IncompleteErase: block.IncompleteErase,
			Pages: make([]nandblockerasecuts.Page, len(block.Pages)),
		}
		for pageIndex, page := range block.Pages {
			clone.Blocks[blockIndex].Pages[pageIndex] = nandblockerasecuts.Page{
				Bytes: append([]byte(nil), page.Bytes...), Programmed: page.Programmed,
			}
		}
	}
	return clone
}

func modelRestoreQuiescentError(media MediaObservation, snapshot ftlpagemap.Snapshot) error {
	blockCount := int64(len(media.Blocks))
	if blockCount < 1 || blockCount > maxRestoredPhysicalPages || media.PagesPerBlock < 1 ||
		media.PagesPerBlock > maxRestoredPhysicalPages || blockCount > maxRestoredPhysicalPages/media.PagesPerBlock {
		return ErrInvalidBlockGeometry
	}
	physicalCount := blockCount * media.PagesPerBlock
	if _, err := ftlpagemap.Restore(media.LogicalPages, physicalCount, snapshot); err != nil {
		return err
	}
	if media.PageBytes < 1 || media.PageBytes > maxPageSize {
		return ErrInvalidPageSize
	}
	if media.MaxCompletedErases > maxSafeEraseCount {
		return nandblockerasecuts.ErrInvalidEraseCount
	}
	for blockIndex, block := range media.Blocks {
		if len(block.Pages) != int(media.PagesPerBlock) {
			return ErrInvalidMediaBlock
		}
		if block.CompletedErases > media.MaxCompletedErases {
			return nandblockerasecuts.ErrInvalidEraseCount
		}
		if block.IncompleteErase {
			return ErrIncompleteMediaBlock
		}
		for pageIndex, page := range block.Pages {
			if len(page.Bytes) != media.PageBytes {
				return ErrInvalidPageSize
			}
			if !page.Programmed {
				for _, value := range page.Bytes {
					if value != 0xff {
						return nandpageprogramcuts.ErrInvalidPageState
					}
				}
			}
			physical := int64(blockIndex)*media.PagesPerBlock + int64(pageIndex)
			switch snapshot.PhysicalStatus[int(physical)] {
			case ftlpagemap.Free:
				if page.Programmed {
					return ErrMediaRoleMismatch
				}
			case ftlpagemap.Live, ftlpagemap.Stale:
				if !page.Programmed {
					return ErrMediaRoleMismatch
				}
			}
		}
	}
	return nil
}

func sameRestoreError(got, want error) bool {
	if got == nil || want == nil {
		return got == want
	}
	return errors.Is(got, want) && errors.Is(want, got)
}

func assertRestoredEvidence(t *testing.T, got Device, media MediaObservation, snapshot ftlpagemap.Snapshot) {
	t.Helper()
	gotSnapshot := got.Snapshot()
	if !reflect.DeepEqual(gotSnapshot.Map, snapshot) {
		t.Fatalf("restored page map=%+v, want %+v", gotSnapshot.Map, snapshot)
	}
	gotMedia, err := got.ObserveMedia()
	if err != nil || !reflect.DeepEqual(gotMedia, media) {
		t.Fatalf("restored physical media=%+v/%v, want %+v", gotMedia, err, media)
	}
}
