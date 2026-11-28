package storagewalpreparecommit

import (
	"errors"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalredoplan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-plan"
)

func layouts() (storagepagelayout.Layout, storagepagelayout.Layout) { return layoutsForLogPages(4) }

func layoutsForLogPages(logPages int64) (storagepagelayout.Layout, storagepagelayout.Layout) {
	g := blockdevicecontract.Geometry{BlockCount: logPages*7 + 4, BytesPerBlock: 8}
	return storagepagelayout.Layout{Geometry: g, FirstBlock: 0, PageCount: logPages, BlocksPerPage: 7}, storagepagelayout.Layout{Geometry: g, FirstBlock: logPages * 7, PageCount: 4, BlocksPerPage: 1}
}

func rec(kind storagewalrecord.Kind, lsn, tx uint64, page int64, image []byte) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: kind, LSN: lsn, TxID: tx, PageID: page, Image: image}
}

func TestPrepareRanksAndEncodesEveryProposedRecord(t *testing.T) {
	log, data := layoutsForLogPages(6)
	prefix := []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 50, 0, nil), rec(storagewalrecord.WritePageImage, 2, 50, 0, []byte{1, 2, 3, 4, 5, 6, 7, 8})}
	images := []PageImage{{PageID: 1, Image: []byte{10, 11, 12, 13, 14, 15, 16, 17}}, {PageID: 1, Image: []byte{20, 21, 22, 23, 24, 25, 26, 27}}}
	got, err := Prepare(log, data, prefix, 60, images)
	if err != nil {
		t.Fatal(err)
	}
	want := []storagewalrecord.Record{
		rec(storagewalrecord.BeginTransaction, 3, 60, 0, nil),
		rec(storagewalrecord.WritePageImage, 4, 60, 1, images[0].Image),
		rec(storagewalrecord.WritePageImage, 5, 60, 1, images[1].Image),
		rec(storagewalrecord.CommitTransaction, 6, 60, 0, nil),
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("records=%+v want %+v", got, want)
	}
	for index, record := range got {
		page, err := storagewalpageslot.EncodePage(56, record)
		if err != nil {
			t.Fatalf("record[%d] encode: %v", index, err)
		}
		decoded, present, err := storagewalpageslot.DecodePage(page)
		if err != nil || !present || !reflect.DeepEqual(decoded, record) {
			t.Fatalf("record[%d] round trip=(%+v,%v,%v)", index, decoded, present, err)
		}
	}
	// The existing incomplete transaction remains in the prefix and the new
	// committed writes are exactly the planner's detached tail.
	combined := append(append([]storagewalrecord.Record(nil), prefix...), got...)
	plan, err := storagewalredoplan.Plan(data, combined)
	if err != nil || len(plan) != 2 {
		t.Fatalf("combined plan=%+v err=%v", plan, err)
	}
	if plan[0].Record.LSN != 4 || plan[1].Record.LSN != 5 {
		t.Fatalf("plan order=%+v", plan)
	}
}

func TestPrepareZeroImageCommitAndExactCapacity(t *testing.T) {
	log, data := layouts()
	got, err := Prepare(log, data, nil, 7, nil)
	want := []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 7, 0, nil), rec(storagewalrecord.CommitTransaction, 2, 7, 0, nil)}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("zero-image=(%+v,%v)", got, err)
	}

	// Exactly three free slots for begin, one image and commit.
	prefix := []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 8, 0, nil)}
	images := []PageImage{{PageID: 2, Image: []byte{8, 7, 6, 5, 4, 3, 2, 1}}}
	got, err = Prepare(log, data, prefix, 9, images)
	if err != nil || len(got) != 3 || got[2].Kind != storagewalrecord.CommitTransaction || got[2].LSN != 4 {
		t.Fatalf("exact fit=(%+v,%v)", got, err)
	}
	// Only two slots remain, so omitting the commit reservation is observable.
	shortLog := log
	shortLog.PageCount = 3
	if records, err := Prepare(shortLog, data, prefix, 9, images); records != nil || !errors.Is(err, ErrInsufficientSlots) {
		t.Fatalf("short reservation=(%+v,%v)", records, err)
	}
}

func TestPreparePreservesFirstRefusal(t *testing.T) {
	log, data := layouts()
	badLog, badData := log, data
	badLog.PageCount = 0
	badData.FirstBlock = -1
	if records, err := Prepare(badLog, badData, nil, 0, nil); records != nil || !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("log-before-data=(%+v,%v)", records, err)
	}
	if records, err := Prepare(log, badData, nil, 0, nil); records != nil || !errors.Is(err, storagepagelayout.ErrInvalidLayout) {
		t.Fatalf("bad data=(%+v,%v)", records, err)
	}
	if records, err := Prepare(log, data, nil, 0, nil); records != nil || !errors.Is(err, storagewalrecord.ErrTxID) {
		t.Fatalf("invalid txid=(%+v,%v)", records, err)
	}
	if records, err := Prepare(log, data, nil, 7, []PageImage{{PageID: -1, Image: make([]byte, 8)}}); records != nil || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
		t.Fatalf("invalid page=(%+v,%v)", records, err)
	}
	if records, err := Prepare(log, data, nil, 7, []PageImage{{PageID: 0, Image: make([]byte, 7)}}); records != nil || !errors.Is(err, storagewalredoplan.ErrImageWidth) {
		t.Fatalf("wrong image width=(%+v,%v)", records, err)
	}
	if records, err := Prepare(log, data, nil, 7, []PageImage{{PageID: 0, Image: nil}}); records != nil || !errors.Is(err, storagewalredoplan.ErrImageWidth) {
		t.Fatalf("invalid record image=(%+v,%v)", records, err)
	}
}

func TestPrepareValidatesCombinedExistingHistory(t *testing.T) {
	log, data := layoutsForLogPages(6)
	images := []PageImage{{PageID: 0, Image: []byte{1, 2, 3, 4, 5, 6, 7, 8}}}
	cases := []struct {
		name   string
		prefix []storagewalrecord.Record
		txID   uint64
		want   error
	}{
		{name: "existing transaction id", prefix: []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 4, 0, nil)}, txID: 4, want: storagewalredoplan.ErrTransactionExists},
		{name: "existing invalid record grammar", prefix: []storagewalrecord.Record{rec(storagewalrecord.WritePageImage, 1, 4, 0, images[0].Image)}, txID: 5, want: storagewalredoplan.ErrTransactionInactive},
		{name: "incomplete prior different transaction", prefix: []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 4, 0, nil)}, txID: 5},
		{name: "already committed id cannot repeat", prefix: []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 4, 0, nil), rec(storagewalrecord.CommitTransaction, 2, 4, 0, nil)}, txID: 4, want: storagewalredoplan.ErrTransactionExists},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, err := Prepare(log, data, tc.prefix, tc.txID, images)
			if tc.want != nil {
				if got != nil || !errors.Is(err, tc.want) {
					t.Fatalf("Prepare=(%+v,%v), want %v", got, err, tc.want)
				}
				return
			}
			if err != nil || len(got) != 3 {
				t.Fatalf("Prepare=(%+v,%v)", got, err)
			}
		})
	}
}

func TestPrepareOwnsImagesAndPreservesInputs(t *testing.T) {
	log, data := layoutsForLogPages(6)
	image := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	prefixImage := []byte{9, 8, 7, 6, 5, 4, 3, 2}
	prefix := []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, 5, 0, nil), rec(storagewalrecord.WritePageImage, 2, 5, 0, prefixImage)}
	images := []PageImage{{PageID: 1, Image: image}}
	prefixBefore := append([]storagewalrecord.Record(nil), prefix...)
	prefixBefore[1].Image = append([]byte(nil), prefix[1].Image...)
	prefixImageBefore, imageBefore := append([]byte(nil), prefixImage...), append([]byte(nil), image...)
	got, err := Prepare(log, data, prefix, 6, images)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(prefix, prefixBefore) || string(prefix[1].Image) != string(prefixImageBefore) {
		t.Fatal("prefix input changed during preparation")
	}
	image[0], prefixImage[0] = 99, 99
	if string(got[1].Image) != string(imageBefore) {
		t.Fatalf("returned image aliases input: %v", got[1].Image)
	}
	got[1].Image[1] = 88
	if images[0].Image[1] != imageBefore[1] {
		t.Fatal("returned image aliases image input")
	}
}

func TestPrepareDenseTransactionGenerator(t *testing.T) {
	log, data := layouts()
	decisions, accepted := 0, 0
	for prefixLength := 0; prefixLength <= 2; prefixLength++ {
		for imageCount := 0; imageCount <= 2; imageCount++ {
			for pageID := int64(0); pageID < 5; pageID++ {
				prefix := make([]storagewalrecord.Record, 0, prefixLength)
				for index := 0; index < prefixLength; index++ {
					if index == 0 {
						prefix = append(prefix, rec(storagewalrecord.BeginTransaction, 1, 50, 0, nil))
					} else {
						prefix = append(prefix, rec(storagewalrecord.WritePageImage, 2, 50, 0, make([]byte, 8)))
					}
				}
				images := make([]PageImage, imageCount)
				for index := range images {
					images[index] = PageImage{PageID: pageID, Image: []byte{byte(index + 1), 2, 3, 4, 5, 6, 7, 8}}
				}
				proposed, err := Prepare(log, data, prefix, 51, images)
				decisions++
				wantErr := (imageCount > 0 && pageID >= data.PageCount) || prefixLength+imageCount+2 > int(log.PageCount)
				if wantErr && err == nil {
					t.Fatalf("accepted invalid generated input prefix=%d images=%d page=%d", prefixLength, imageCount, pageID)
				}
				if wantErr && proposed != nil {
					t.Fatalf("refusal returned records=%+v", proposed)
				}
				if !wantErr {
					if err != nil {
						t.Fatalf("valid generated input refused: %v", err)
					}
					accepted++
					if len(proposed) != imageCount+2 || proposed[0].LSN != uint64(prefixLength+1) || proposed[len(proposed)-1].LSN != uint64(prefixLength+imageCount+2) {
						t.Fatalf("generated ranks=%+v", proposed)
					}
				}
			}
		}
	}
	t.Logf("generator decisions=%d accepted=%d", decisions, accepted)
}

func FuzzPrepareRecords(f *testing.F) {
	for _, seed := range [][]byte{{0}, {1, 0, 0, 0, 0, 0, 0, 0}, {2, 1, 2, 3, 4, 5, 6, 7, 8}, {3, 3, 1, 255, 9, 8, 7, 6, 5, 4, 3, 2}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) == 0 {
			t.Skip()
		}
		log, data := layouts()
		prefix := []storagewalrecord.Record(nil)
		prefixMode := raw[0] % 4
		txID := uint64(raw[0]>>2%250 + 1)
		switch prefixMode {
		case 1:
			prefix = []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, txID+1, 0, nil)}
		case 2:
			prefix = []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, txID, 0, nil)}
		case 3:
			prefix = []storagewalrecord.Record{rec(storagewalrecord.BeginTransaction, 1, txID+1, 0, nil), rec(storagewalrecord.CommitTransaction, 2, txID+1, 0, nil)}
		}
		count := int(raw[0] >> 4 % 3)
		images := make([]PageImage, count)
		for index := range images {
			pageID := int64(0)
			if index+1 < len(raw) {
				pageID = int64(raw[index+1] % 5)
			}
			image := make([]byte, 8)
			for offset := range image {
				if index+1+offset < len(raw) {
					image[offset] = raw[index+1+offset]
				}
			}
			images[index] = PageImage{PageID: pageID, Image: image}
		}
		got, err := Prepare(log, data, prefix, txID, images)
		if len(prefix)+count+2 > int(log.PageCount) {
			if got != nil || !errors.Is(err, ErrInsufficientSlots) {
				t.Fatalf("insufficient slots=(%+v,%v)", got, err)
			}
			return
		}
		if prefixMode == 2 {
			if got != nil || !errors.Is(err, storagewalredoplan.ErrTransactionExists) {
				t.Fatalf("duplicate tx=(%+v,%v)", got, err)
			}
			return
		}
		invalidPage := false
		for _, image := range images {
			invalidPage = invalidPage || image.PageID >= data.PageCount
		}
		if invalidPage {
			if got != nil || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
				t.Fatalf("invalid page=(%+v,%v)", got, err)
			}
			return
		}
		if err != nil {
			t.Fatalf("generated valid transaction refused: %v", err)
		}
		if len(got) != count+2 || got[0].Kind != storagewalrecord.BeginTransaction || got[0].LSN != uint64(len(prefix)+1) || got[len(got)-1].Kind != storagewalrecord.CommitTransaction {
			t.Fatalf("result=%+v", got)
		}
		for i := range images {
			if got[i+1].PageID != images[i].PageID || string(got[i+1].Image) != string(images[i].Image) {
				t.Fatalf("image order mismatch result=%+v", got)
			}
		}
	})
}
