package ftlnandrelocation

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

const (
	modelMaxPhysicalPages int64  = 128
	modelMaxPageBytes     int    = 65536
	modelMaxEraseCount    uint64 = 9007199254740991
)

func TestRestoreInterruptedValidationOrderAndZeroDevice(t *testing.T) {
	media, snapshot := interruptedRestoreExample()
	badMap := snapshot
	badMap.LogicalToPhysical = nil
	badWidth := cloneRestoreMedia(media)
	badWidth.PageBytes = 0
	badBlockShape := cloneRestoreMedia(media)
	badBlockShape.Blocks[0].Pages = badBlockShape.Blocks[0].Pages[:1]
	badCount := cloneRestoreMedia(media)
	badCount.Blocks[0].CompletedErases = 3
	badCount.Blocks[0].Pages[0].Bytes = []byte{0}
	equalBudget := cloneRestoreMedia(media)
	equalBudget.Blocks[0].CompletedErases = equalBudget.MaxCompletedErases
	equalBudget.Blocks[0].Pages[0].Bytes = []byte{0}
	badMarker := cloneRestoreMedia(media)
	badMarker.Blocks[0].Pages[0].Bytes[0] = 0
	noStale := cloneRestoreMedia(media)
	for index := range noStale.Blocks[0].Pages {
		noStale.Blocks[0].Pages[index].Programmed = false
	}
	noStaleSnapshot := ftlpagemap.Snapshot{
		LogicalToPhysical: []int64{-1},
		PhysicalStatus:    []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free},
	}
	live := cloneRestoreMedia(media)
	liveSnapshot := ftlpagemap.Snapshot{
		LogicalToPhysical: []int64{0},
		PhysicalStatus:    []ftlpagemap.PhysicalStatus{ftlpagemap.Live, ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Free},
	}
	clearAfterMarked := cloneRestoreMedia(media)
	clearAfterMarked.Blocks[0].Pages[2].Programmed = false

	cases := []struct {
		name     string
		media    MediaObservation
		snapshot ftlpagemap.Snapshot
		want     error
	}{
		{"geometry precedes invalid map", MediaObservation{}, badMap, ErrInvalidBlockGeometry},
		{"map validation precedes media width", badWidth, badMap, ftlpagemap.ErrInvalidSnapshotShape},
		{"page width precedes block scan", badWidth, snapshot, ErrInvalidPageSize},
		{"block shape precedes count and page faults", badBlockShape, snapshot, ErrInvalidMediaBlock},
		{"count above ceiling precedes page faults", badCount, snapshot, nandblockerasecuts.ErrInvalidEraseCount},
		{"incomplete count at ceiling has no remaining erase", equalBudget, snapshot, nandblockerasecuts.ErrEraseLimit},
		{"clear marker requires erased bytes", badMarker, snapshot, nandpageprogramcuts.ErrInvalidPageState},
		{"all-free interrupted block has no stale progress", noStale, noStaleSnapshot, ErrIncompleteMediaBlock},
		{"live role cannot be interrupted", live, liveSnapshot, ErrIncompleteMediaBlock},
		{"stale markers must form a feasible prefix", clearAfterMarked, snapshot, ErrIncompleteMediaBlock},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			got, err := RestoreWithInterruptedErases(test.media, test.snapshot)
			if !errors.Is(err, test.want) || !reflect.DeepEqual(got, Device{}) {
				t.Fatalf("restore = zero? %t / %v, want zero device and %v", reflect.DeepEqual(got, Device{}), err, test.want)
			}
		})
	}
	if got, err := RestoreQuiescent(media, snapshot); !errors.Is(err, ErrIncompleteMediaBlock) || !reflect.DeepEqual(got, Device{}) {
		t.Fatalf("quiescent restore accepted incomplete evidence: %v", err)
	}
}

func TestRestoreInterruptedExhaustiveSmallRoleAndMarkerModel(t *testing.T) {
	const pages = 4
	roles := []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Live, ftlpagemap.Stale}
	checked := 0
	valid := 0
	for roleCode := 0; roleCode < 81; roleCode++ {
		status := make([]ftlpagemap.PhysicalStatus, pages)
		remaining := roleCode
		mapping := []int64{-1, -1}
		owner := 0
		for page := range status {
			status[page] = roles[remaining%len(roles)]
			remaining /= len(roles)
			if status[page] == ftlpagemap.Live && owner < len(mapping) {
				mapping[owner] = int64(page)
				owner++
			}
		}
		snapshot := ftlpagemap.Snapshot{LogicalToPhysical: mapping, PhysicalStatus: status}
		for markerMask := 0; markerMask < 1<<pages; markerMask++ {
			for _, incomplete := range []bool{false, true} {
				media := MediaObservation{
					LogicalPages: 2, PagesPerBlock: pages, PageBytes: 1, MaxCompletedErases: 2,
					Blocks: []nandblockerasecuts.Block{{
						Pages:           make([]nandblockerasecuts.Page, pages),
						IncompleteErase: incomplete,
					}},
				}
				for page := range media.Blocks[0].Pages {
					media.Blocks[0].Pages[page] = nandblockerasecuts.Page{
						Bytes: []byte{0xff}, Programmed: markerMask&(1<<page) != 0,
					}
				}
				want := modelRestoreInterruptedError(media, snapshot)
				got, err := RestoreWithInterruptedErases(media, snapshot)
				if !sameRestoreError(err, want) {
					t.Fatalf("roles=%v markers=%04b incomplete=%t error=%v want=%v", status, markerMask, incomplete, err, want)
				}
				if want != nil {
					if !reflect.DeepEqual(got, Device{}) {
						t.Fatalf("refusal returned nonzero device for roles=%v markers=%04b", status, markerMask)
					}
				} else {
					valid++
					assertRestoredEvidence(t, got, media, snapshot)
				}
				checked++
			}
		}
	}
	if checked != 2592 || valid == 0 {
		t.Fatalf("checked=%d valid=%d; want complete 2,592-case role/marker domain", checked, valid)
	}
	t.Logf("checked %d status/marker combinations; %d valid", checked, valid)
}

func TestRestoreInterruptedHistoryRetriesFencesAndReleases(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 8, 2, 4, 4)
	if err != nil {
		t.Fatal(err)
	}
	allOnes := []byte{0xff, 0xff}
	device, _, err = device.Write(0, allOnes, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, allOnes, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(1, allOnes, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	owner0, present, err := device.ObserveOwner(0)
	if err != nil || !present {
		t.Fatalf("owner 0 = %+v/%t/%v", owner0, present, err)
	}
	owner1, present, err := device.ObserveOwner(1)
	if err != nil || !present {
		t.Fatalf("owner 1 = %+v/%t/%v", owner1, present, err)
	}
	device, err = device.InvalidateOwner(owner0)
	if err != nil {
		t.Fatal(err)
	}
	device, err = device.InvalidateOwner(owner1)
	if err != nil {
		t.Fatal(err)
	}
	device, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if err != nil || receipt.Released || receipt.Acknowledged {
		t.Fatalf("first prefix erase receipt=%+v err=%v", receipt, err)
	}
	media, err := device.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	if !media.Blocks[0].IncompleteErase || media.Blocks[0].Pages[0].Programmed || !media.Blocks[0].Pages[1].Programmed {
		t.Fatalf("first interrupted image=%+v", media.Blocks[0])
	}
	restored, err := RestoreWithInterruptedErases(media, device.table.Snapshot())
	if err != nil {
		t.Fatalf("restore first prefix: %v", err)
	}
	if restored.evacuationActive || restored.MatchesOwner(owner0) || restored.MatchesOwner(owner1) {
		t.Fatal("restore retained lifecycle or old process-local owner identity")
	}
	if len(restored.blocks) != 2 || !restored.blocks[0].IncompleteErase || restored.blocks[0].CompletedErases != 0 {
		t.Fatalf("restored block lost its incomplete-erase fence: %+v", restored.blocks)
	}
	assertRestoredEvidence(t, restored, media, device.table.Snapshot())
	before, _ := restored.ObserveMedia()
	if _, _, err := restored.ProgramUnownedPage(0, 3, allOnes, Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ErrEvacuationBlockIncomplete) {
		t.Fatalf("free page in interrupted block was programmable: %v", err)
	}
	if _, _, err := restored.CopyLivePage(0, ftlevacuationplan.Copy{}, Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ErrEvacuationBlockIncomplete) {
		t.Fatalf("copy bypassed interrupted block fence: %v", err)
	}
	written, writeReceipt, err := restored.Write(0, allOnes, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || writeReceipt.Physical != 4 || !writeReceipt.Published {
		t.Fatalf("write did not skip the interrupted block: %+v/%v", writeReceipt, err)
	}
	readBytes, present, readErr := written.Read(0)
	if readErr != nil || !present || !reflect.DeepEqual(readBytes, allOnes) {
		t.Fatalf("write outside interrupted block read=%x/%t/%v", readBytes, present, readErr)
	}
	afterRefusals, _ := restored.ObserveMedia()
	if !reflect.DeepEqual(before.Blocks[0], afterRefusals.Blocks[0]) {
		t.Fatal("refused page operations changed interrupted block")
	}

	shorter, shortReceipt, err := restored.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	if err != nil || shortReceipt.Released || !shorter.blocks[0].IncompleteErase || shorter.pages[0].Programmed {
		t.Fatalf("shorter retry lost earlier prefix progress: %+v/%+v/%v", shorter.blocks[0], shortReceipt, err)
	}
	longer, longReceipt, err := shorter.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 2})
	if err != nil || longReceipt.Released || !longer.blocks[0].IncompleteErase || longer.pages[1].Programmed {
		t.Fatalf("longer retry failed to extend prefix: %+v/%+v/%v", longer.blocks[0], longReceipt, err)
	}
	longMedia, err := longer.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	longerRestored, err := RestoreWithInterruptedErases(longMedia, longer.table.Snapshot())
	if err != nil {
		t.Fatalf("restore longer prefix: %v", err)
	}
	assertRestoredEvidence(t, longerRestored, longMedia, longer.table.Snapshot())

	completed, completion, err := longerRestored.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.AfterMedia})
	if err != nil || !completion.Released || completion.Acknowledged || completed.blocks[0].IncompleteErase || completed.blocks[0].CompletedErases != 1 {
		t.Fatalf("lost-ack media completion=%+v block=%+v err=%v", completion, completed.blocks[0], err)
	}
	completedMedia, err := completed.ObserveMedia()
	if err != nil {
		t.Fatal(err)
	}
	completedSnapshot := completed.table.Snapshot()
	quiescent, err := RestoreQuiescent(completedMedia, completedSnapshot)
	if err != nil {
		t.Fatalf("completed-media lost-ack restore: %v", err)
	}
	assertRestoredEvidence(t, quiescent, completedMedia, completedSnapshot)
	continued, writeReceipt, err := quiescent.Write(0, []byte{0x21, 0x43}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || writeReceipt.Physical != 0 {
		t.Fatalf("freed block allocation=%+v err=%v; want lowest released page 0", writeReceipt, err)
	}
	got, present, err := continued.Read(0)
	if err != nil || !present || !reflect.DeepEqual(got, []byte{0x21, 0x43}) {
		t.Fatalf("continued read=%x/%t/%v", got, present, err)
	}
}

func TestRestoreInterruptedSupportsMultipleBlocksAndDetachedInputs(t *testing.T) {
	media := MediaObservation{
		LogicalPages: 1, PagesPerBlock: 2, PageBytes: 1, MaxCompletedErases: 4,
		Blocks: []nandblockerasecuts.Block{
			{IncompleteErase: true, Pages: []nandblockerasecuts.Page{{Bytes: []byte{0xff}}, {Bytes: []byte{0x11}, Programmed: true}}},
			{IncompleteErase: true, Pages: []nandblockerasecuts.Page{{Bytes: []byte{0xff}, Programmed: true}, {Bytes: []byte{0x22}, Programmed: true}}},
		},
	}
	snapshot := ftlpagemap.Snapshot{
		LogicalToPhysical: []int64{-1},
		PhysicalStatus:    []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Stale},
	}
	device, err := RestoreWithInterruptedErases(media, snapshot)
	if err != nil {
		t.Fatalf("multiple feasible incomplete blocks refused: %v", err)
	}
	initial := device.Snapshot()
	media.Blocks[0].Pages[0].Bytes[0] = 0
	snapshot.PhysicalStatus[0] = ftlpagemap.Free
	if device.Snapshot().Pages[0].Bytes[0] != 0xff || device.Snapshot().Map.PhysicalStatus[0] != ftlpagemap.Stale {
		t.Fatal("restored device aliased input bytes or status arrays")
	}
	initial.Pages[0].Bytes[0] = 0
	observed, err := device.ObserveMedia()
	if err != nil || observed.Blocks[0].Pages[0].Bytes[0] != 0xff {
		t.Fatalf("snapshot bytes aliased restored device: %+v/%v", observed, err)
	}
}

func TestRestoreInterruptedRegeneratesOwnerIdentity(t *testing.T) {
	media := MediaObservation{
		LogicalPages: 1, PagesPerBlock: 2, PageBytes: 1, MaxCompletedErases: 3,
		Blocks: []nandblockerasecuts.Block{
			{IncompleteErase: true, Pages: []nandblockerasecuts.Page{{Bytes: []byte{0x11}, Programmed: true}, {Bytes: []byte{0xff}}}},
			{Pages: []nandblockerasecuts.Page{{Bytes: []byte{0x22}, Programmed: true}, {Bytes: []byte{0xff}}}},
		},
	}
	snapshot := ftlpagemap.Snapshot{
		LogicalToPhysical: []int64{2},
		PhysicalStatus: []ftlpagemap.PhysicalStatus{
			ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Live, ftlpagemap.Free,
		},
	}
	prior, err := NewWithEraseBlocks(1, 4, 1, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	prior, _, err = prior.Write(0, []byte{0x22}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	oldOwner, present, err := prior.ObserveOwner(0)
	if err != nil || !present {
		t.Fatalf("prior owner=%+v/%t/%v", oldOwner, present, err)
	}
	first, err := RestoreWithInterruptedErases(media, snapshot)
	if err != nil {
		t.Fatalf("first restore: %v", err)
	}
	firstOwner, present, err := first.ObserveOwner(0)
	if err != nil || !present || first.MatchesOwner(oldOwner) || !first.MatchesOwner(firstOwner) {
		t.Fatalf("first restored owner=%+v/%t/%v", firstOwner, present, err)
	}
	second, err := RestoreWithInterruptedErases(media, snapshot)
	if err != nil {
		t.Fatalf("second restore: %v", err)
	}
	if second.MatchesOwner(firstOwner) {
		t.Fatal("restoring identical interrupted evidence reused an owner identity")
	}
}

func TestFiveThousandSeededInterruptedEvidenceMatchesModel(t *testing.T) {
	random := rand.New(rand.NewSource(0x5e3b3c))
	accepted := 0
	refused := 0
	for trial := 0; trial < 5000; trial++ {
		media, snapshot := randomInterruptedEvidence(random)
		want := modelRestoreInterruptedError(media, snapshot)
		got, err := RestoreWithInterruptedErases(media, snapshot)
		if !sameRestoreError(err, want) {
			t.Fatalf("trial=%d error=%v want=%v media=%+v map=%+v", trial, err, want, media, snapshot)
		}
		if want != nil {
			refused++
			if !reflect.DeepEqual(got, Device{}) {
				t.Fatalf("trial=%d refusal returned nonzero device", trial)
			}
			continue
		}
		accepted++
		assertRestoredEvidence(t, got, media, snapshot)
		before := got.Snapshot()
		media.Blocks[0].Pages[0].Bytes[0] ^= 0xff
		if !reflect.DeepEqual(before, got.Snapshot()) {
			t.Fatalf("trial=%d restored bytes aliased input media", trial)
		}
	}
	if accepted == 0 || refused == 0 {
		t.Fatalf("seeded model reached accepted=%d refused=%d", accepted, refused)
	}
	t.Logf("seed 0x5e3b3c checked 5,000 raw restores: %d accepted, %d refused", accepted, refused)
}

func FuzzRestoreInterruptedEvidence(f *testing.F) {
	for _, seed := range [][]byte{
		{}, {0}, {0xff, 0, 1, 0}, {4, 7, 1, 1}, {0x35, 0xaa, 0x01, 0x03},
	} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 4 {
			raw = raw[:4]
		}
		random := rand.New(rand.NewSource(int64(len(raw)) * 97))
		for _, value := range raw {
			random.Seed(random.Int63() ^ int64(value))
		}
		media, snapshot := randomInterruptedEvidence(random)
		want := modelRestoreInterruptedError(media, snapshot)
		got, err := RestoreWithInterruptedErases(media, snapshot)
		if !sameRestoreError(err, want) {
			t.Fatalf("runtime error=%v model=%v raw=%x", err, want, raw)
		}
		if want == nil {
			assertRestoredEvidence(t, got, media, snapshot)
		} else if !reflect.DeepEqual(got, Device{}) {
			t.Fatal("refusal returned nonzero device")
		}
	})
}

func randomInterruptedEvidence(random *rand.Rand) (MediaObservation, ftlpagemap.Snapshot) {
	media := MediaObservation{
		LogicalPages: 2, PagesPerBlock: 4, PageBytes: 1, MaxCompletedErases: 2,
		Blocks: []nandblockerasecuts.Block{{Pages: make([]nandblockerasecuts.Page, 4)}},
	}
	snapshot := ftlpagemap.Snapshot{
		LogicalToPhysical: []int64{-1, -1},
		PhysicalStatus:    make([]ftlpagemap.PhysicalStatus, 4),
	}
	roles := []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Live, ftlpagemap.Stale}
	owner := 0
	for physical := range media.Blocks[0].Pages {
		role := roles[random.Intn(len(roles))]
		snapshot.PhysicalStatus[physical] = role
		if role == ftlpagemap.Live && owner < len(snapshot.LogicalToPhysical) {
			snapshot.LogicalToPhysical[owner] = int64(physical)
			owner++
		}
		page := nandblockerasecuts.Page{Bytes: []byte{0xff}, Programmed: role != ftlpagemap.Free}
		if page.Programmed {
			page.Bytes[0] = byte(random.Intn(256))
		}
		media.Blocks[0].Pages[physical] = page
	}
	media.Blocks[0].IncompleteErase = random.Intn(2) == 0
	media.Blocks[0].CompletedErases = uint64(random.Intn(4))
	return media, snapshot
}

func interruptedRestoreExample() (MediaObservation, ftlpagemap.Snapshot) {
	media := MediaObservation{
		LogicalPages: 1, PagesPerBlock: 4, PageBytes: 1, MaxCompletedErases: 2,
		Blocks: []nandblockerasecuts.Block{{IncompleteErase: true, Pages: []nandblockerasecuts.Page{
			{Bytes: []byte{0xff}}, {Bytes: []byte{0xff}, Programmed: true},
			{Bytes: []byte{0xff}, Programmed: true}, {Bytes: []byte{0xff}},
		}}},
	}
	snapshot := ftlpagemap.Snapshot{
		LogicalToPhysical: []int64{-1},
		PhysicalStatus:    []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Free},
	}
	return media, snapshot
}

func modelRestoreInterruptedError(media MediaObservation, snapshot ftlpagemap.Snapshot) error {
	blockCount := int64(len(media.Blocks))
	if blockCount < 1 || blockCount > modelMaxPhysicalPages || media.PagesPerBlock < 1 ||
		media.PagesPerBlock > modelMaxPhysicalPages || blockCount > modelMaxPhysicalPages/media.PagesPerBlock {
		return ErrInvalidBlockGeometry
	}
	physicalCount := blockCount * media.PagesPerBlock
	if _, err := ftlpagemap.Restore(media.LogicalPages, physicalCount, snapshot); err != nil {
		return err
	}
	if media.PageBytes < 1 || media.PageBytes > modelMaxPageBytes {
		return ErrInvalidPageSize
	}
	if media.MaxCompletedErases > modelMaxEraseCount {
		return nandblockerasecuts.ErrInvalidEraseCount
	}
	for blockIndex, block := range media.Blocks {
		if len(block.Pages) != int(media.PagesPerBlock) {
			return ErrInvalidMediaBlock
		}
		if block.CompletedErases > media.MaxCompletedErases {
			return nandblockerasecuts.ErrInvalidEraseCount
		}
		if block.IncompleteErase && block.CompletedErases == media.MaxCompletedErases {
			return nandblockerasecuts.ErrEraseLimit
		}
		stale := 0
		lastStale := -1
		sawSet := false
		impossible := false
		for index, page := range block.Pages {
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
			physical := blockIndex*int(media.PagesPerBlock) + index
			role := snapshot.PhysicalStatus[physical]
			if block.IncompleteErase {
				switch role {
				case ftlpagemap.Free:
					if page.Programmed {
						return ErrMediaRoleMismatch
					}
				case ftlpagemap.Live:
					return ErrIncompleteMediaBlock
				case ftlpagemap.Stale:
					stale++
					lastStale = index
					if page.Programmed {
						sawSet = true
					} else if sawSet {
						impossible = true
					}
				}
			} else {
				switch role {
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
		if block.IncompleteErase && (stale == 0 || impossible || (!sawSet && lastStale == int(media.PagesPerBlock)-1)) {
			return ErrIncompleteMediaBlock
		}
	}
	return nil
}
