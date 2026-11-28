package storagewalredoplan

import (
	"errors"
	"math"
	"reflect"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
)

var planLayout = storagepagelayout.Layout{
	Geometry:      blockdevicecontract.Geometry{BlockCount: 16, BytesPerBlock: 4},
	FirstBlock:    2,
	PageCount:     3,
	BlocksPerPage: 1,
}

func begin(lsn, tx uint64) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: lsn, TxID: tx}
}

func write(lsn, tx uint64, page int64, image ...byte) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: lsn, TxID: tx, PageID: page, Image: append([]byte(nil), image...)}
}

func commit(lsn, tx uint64) storagewalrecord.Record {
	return storagewalrecord.Record{Kind: storagewalrecord.CommitTransaction, LSN: lsn, TxID: tx}
}

func TestPlanInterleavedTransactionsByCommitAndWriteOrder(t *testing.T) {
	input := []storagewalrecord.Record{
		begin(1, 10), begin(2, 20),
		write(3, 10, 1, 1, 1, 1, 1),
		write(4, 20, 2, 2, 2, 2, 2),
		write(5, 10, 1, 3, 3, 3, 3),
		commit(6, 20), write(7, 10, 0, 4, 4, 4, 4), commit(8, 10),
	}
	plan, err := Plan(planLayout, input)
	if err != nil {
		t.Fatal(err)
	}
	want := []RedoWrite{
		{Record: input[3], CommitLSN: 6},
		{Record: input[2], CommitLSN: 8},
		{Record: input[4], CommitLSN: 8},
		{Record: input[6], CommitLSN: 8},
	}
	if !samePlan(plan, want) {
		t.Fatalf("Plan=%+v want=%+v", plan, want)
	}
	if got := []uint64{plan[0].CommitLSN, plan[1].CommitLSN, plan[2].CommitLSN, plan[3].CommitLSN}; !reflect.DeepEqual(got, []uint64{6, 8, 8, 8}) {
		t.Fatalf("commit order=%v", got)
	}
	if plan[1].Record.LSN != 3 || plan[2].Record.LSN != 5 {
		t.Fatalf("per-transaction source order changed: %+v", plan)
	}
}

func TestPlanOrdersSingleWriteTransactionsByCommit(t *testing.T) {
	input := []storagewalrecord.Record{
		begin(1, 10), write(2, 10, 0, 1, 1, 1, 1),
		begin(3, 20), write(4, 20, 1, 2, 2, 2, 2),
		commit(5, 20), commit(6, 10),
	}
	plan, err := Plan(planLayout, input)
	want := []RedoWrite{{Record: input[3], CommitLSN: 5}, {Record: input[1], CommitLSN: 6}}
	if err != nil || !samePlan(plan, want) {
		t.Fatalf("Plan=(%+v,%v) want=%+v", plan, err, want)
	}
}

func TestPlanEmptyAndIncompleteTransactionsSelectNothing(t *testing.T) {
	for _, input := range [][]storagewalrecord.Record{
		nil,
		{begin(1, 1), commit(2, 1)},
		{begin(1, 1), write(2, 1, 0, 1, 2, 3, 4)},
		{begin(1, 1), write(2, 1, 0, 1, 2, 3, 4), begin(3, 2), commit(4, 2)},
	} {
		plan, err := Plan(planLayout, input)
		if err != nil || plan != nil {
			t.Fatalf("Plan(%v)=(%v,%v), want nil,nil", input, plan, err)
		}
	}
}

func TestPlanExactValidationOrderAndNilPlanOnEveryRefusal(t *testing.T) {
	badLayout := storagepagelayout.Layout{Geometry: blockdevicecontract.Geometry{}, FirstBlock: -1}
	if plan, err := Plan(badLayout, []storagewalrecord.Record{{Kind: 255}}); plan != nil || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("layout-first=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{begin(1, 7), begin(2, 7)}); plan != nil || !errors.Is(err, ErrTransactionExists) {
		t.Fatalf("duplicate begin=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{write(1, 7, -1, 1)}); plan != nil || !errors.Is(err, ErrTransactionInactive) {
		t.Fatalf("inactive write before page=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{commit(1, 7)}); plan != nil || !errors.Is(err, ErrTransactionInactive) {
		t.Fatalf("inactive commit=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{{Kind: 99}}); plan != nil || !errors.Is(err, ErrRecordKind) {
		t.Fatalf("unknown kind=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{begin(1, 7), write(2, 7, -1, 1)}); plan != nil || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
		t.Fatalf("page before image width=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{begin(1, 7), write(2, 7, 1, 1, 2, 3)}); plan != nil || !errors.Is(err, ErrImageWidth) {
		t.Fatalf("image width=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{begin(1, 7), write(2, 7, 1, 1, 2, 3, 4), commit(3, 7), write(4, 7, 0, 1, 2, 3, 4)}); plan != nil || !errors.Is(err, ErrTransactionInactive) {
		t.Fatalf("write after commit=(%v,%v)", plan, err)
	}
	if plan, err := Plan(planLayout, []storagewalrecord.Record{begin(1, 7), write(2, 7, math.MaxInt64, 1, 2, 3, 4)}); plan != nil || !errors.Is(err, storagepagelayout.ErrInvalidPage) {
		t.Fatalf("signed page endpoint=(%v,%v)", plan, err)
	}
}

func TestPlanOwnsSelectedPageImages(t *testing.T) {
	input := []storagewalrecord.Record{begin(1, 4), write(2, 4, 1, 1, 2, 3, 4), commit(3, 4)}
	plan, err := Plan(planLayout, input)
	if err != nil || len(plan) != 1 {
		t.Fatalf("Plan=(%+v,%v)", plan, err)
	}
	plan[0].Record.Image[0] = 99
	if input[1].Image[0] != 1 {
		t.Fatalf("plan aliases input: %v", input[1].Image)
	}
	input[1].Image[1] = 88
	if plan[0].Record.Image[1] != 2 {
		t.Fatalf("input mutation changed plan: %v", plan[0].Record.Image)
	}
}

func TestPlanConsumesAlreadyAdmittedLSNsWithoutReranking(t *testing.T) {
	input := []storagewalrecord.Record{begin(30, 9), write(47, 9, 1, 1, 2, 3, 4), commit(99, 9)}
	plan, err := Plan(planLayout, input)
	if err != nil || len(plan) != 1 || plan[0].Record.LSN != 47 || plan[0].CommitLSN != 99 {
		t.Fatalf("Plan=(%+v,%v), expected the original admitted LSN fields", plan, err)
	}
}

func TestFiniteHistoriesAgainstSeparateTransactionTable(t *testing.T) {
	choices := [][]storagewalrecord.Record{
		{begin(1, 1)}, {begin(1, 2)},
		{write(1, 1, 0, 1, 2, 3, 4)}, {write(1, 2, 1, 5, 6, 7, 8)},
		{commit(1, 1)}, {commit(1, 2)},
		{{Kind: 255, LSN: 1}},
		{write(1, 1, -1, 9)}, {write(1, 1, 1, 9)},
	}
	for code := 0; code < 9*9*9*9; code++ {
		value := code
		input := make([]storagewalrecord.Record, 4)
		for index := range input {
			choice := choices[value%len(choices)][0]
			value /= len(choices)
			choice.LSN = uint64(index + 1)
			input[index] = choice
		}
		want, wantErr := modelPlan(planLayout, input)
		got, gotErr := Plan(planLayout, input)
		if !sameError(gotErr, wantErr) || !samePlan(got, want) {
			t.Fatalf("history=%+v got=(%+v,%v) want=(%+v,%v)", input, got, gotErr, want, wantErr)
		}
	}
}

func TestSeededHistoriesAgainstSeparateTransactionTable(t *testing.T) {
	for seed := uint32(1); seed <= 1_000; seed++ {
		state := seed
		input := make([]storagewalrecord.Record, 16)
		for index := range input {
			state = state*1664525 + 1013904223
			kind := state % 7
			tx := uint64((state>>8)%3 + 1)
			switch kind {
			case 0:
				input[index] = begin(uint64(index+1), tx)
			case 1, 2, 3:
				input[index] = write(uint64(index+1), tx, int64((state>>16)%5)-1, byte(state), byte(state>>8), byte(state>>16), byte(state>>24))
			case 4, 5:
				input[index] = commit(uint64(index+1), tx)
			default:
				input[index] = storagewalrecord.Record{Kind: 99, LSN: uint64(index + 1), TxID: tx}
			}
		}
		want, wantErr := modelPlan(planLayout, input)
		got, gotErr := Plan(planLayout, input)
		if !sameError(gotErr, wantErr) || !samePlan(got, want) {
			t.Fatalf("seed=%d got=(%+v,%v) want=(%+v,%v)", seed, got, gotErr, want, wantErr)
		}
	}
}

func FuzzPlanHistory(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9})
	f.Add([]byte{0xff, 1, 1, 3, 0, 2, 2, 1, 5, 4, 4, 0})
	f.Add([]byte{2, 2, 0, 4, 4, 1, 0, 2, 1, 3, 5})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 128 {
			t.Skip()
		}
		records := make([]storagewalrecord.Record, len(input))
		for index, value := range input {
			lsn := uint64(index + 1)
			tx := uint64((value>>3)%4 + 1)
			switch value % 7 {
			case 0:
				records[index] = begin(lsn, tx)
			case 1:
				records[index] = commit(lsn, tx)
			case 2, 3, 4:
				page := int64((value>>5)%5) - 1
				records[index] = write(lsn, tx, page, value, value^0x5a, value+1, value-1)
			case 5:
				records[index] = storagewalrecord.Record{Kind: 255, LSN: lsn, TxID: tx}
			case 6:
				records[index] = write(lsn, tx, int64((value>>4)%4), value)
			}
		}
		want, wantErr := modelPlan(planLayout, records)
		got, gotErr := Plan(planLayout, records)
		if !sameError(gotErr, wantErr) || !samePlan(got, want) {
			t.Fatalf("input=%v got=(%+v,%v) want=(%+v,%v)", input, got, gotErr, want, wantErr)
		}
	})
}

// modelPlan is a separate flat-table oracle; it deliberately copies input
// images into table rows and selects them only when a commit is observed.
func modelPlan(layout storagepagelayout.Layout, records []storagewalrecord.Record) ([]RedoWrite, error) {
	if err := layout.Validate(); err != nil {
		return nil, err
	}
	type modelTx struct {
		begun, active bool
		rows          []RedoWrite
	}
	table := map[uint64]modelTx{}
	order := make([]uint64, 0)
	for _, record := range records {
		switch record.Kind {
		case storagewalrecord.BeginTransaction:
			if table[record.TxID].begun {
				return nil, ErrTransactionExists
			}
			table[record.TxID] = modelTx{begun: true, active: true}
		case storagewalrecord.WritePageImage:
			row, ok := table[record.TxID]
			if !ok || !row.active {
				return nil, ErrTransactionInactive
			}
			page, err := layout.Span(record.PageID)
			if err != nil {
				return nil, err
			}
			if len(record.Image) != int(page.BytesPerPage) {
				return nil, ErrImageWidth
			}
			owned := record
			owned.Image = append([]byte{}, record.Image...)
			row.rows = append(row.rows, RedoWrite{Record: owned})
			table[record.TxID] = row
		case storagewalrecord.CommitTransaction:
			row, ok := table[record.TxID]
			if !ok || !row.active {
				return nil, ErrTransactionInactive
			}
			row.active = false
			for index := range row.rows {
				row.rows[index].CommitLSN = record.LSN
			}
			table[record.TxID] = row
			order = append(order, record.TxID)
		default:
			return nil, ErrRecordKind
		}
	}
	var result []RedoWrite
	for _, tx := range order {
		result = append(result, table[tx].rows...)
	}
	return result, nil
}

func sameError(got, want error) bool {
	if got == nil || want == nil {
		return got == nil && want == nil
	}
	return errors.Is(got, want) && errors.Is(want, got)
}

func samePlan(got, want []RedoWrite) bool {
	if len(got) != len(want) {
		return false
	}
	for index := range got {
		left, right := got[index], want[index]
		if left.CommitLSN != right.CommitLSN || left.Record.Kind != right.Record.Kind || left.Record.LSN != right.Record.LSN || left.Record.TxID != right.Record.TxID || left.Record.PageID != right.Record.PageID || string(left.Record.Image) != string(right.Record.Image) {
			return false
		}
	}
	return true
}
