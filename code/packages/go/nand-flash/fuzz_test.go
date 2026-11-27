package nandflash

import (
	"errors"
	"testing"
)

// FuzzCommandStream bounds each generated history and checks every transition
// against a tiny, separately stored model. Ordinary go test runs the seeds;
// longer campaigns are invoked with Go's built-in coverage-guided fuzzer.
func FuzzCommandStream(f *testing.F) {
	f.Add([]byte{0x00, 0x40, 0x80, 0xC0})
	f.Add([]byte{0xF0, 0x3F, 0xA5, 0x59, 0xFF})
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 128 {
			return
		}
		device, err := New(Config{Geometry: Geometry{2, 2, 1}, MaxEraseCycles: ptrUint64(2)})
		if err != nil {
			t.Fatal(err)
		}
		model := newModel(2, 2, 1, 2)
		for step, command := range commands {
			block := int((command>>4)&3) - 1
			page := int(command&3) - 1
			switch command >> 6 {
			case 0: // Program: vary coordinates, nilness, length, and the byte.
				var data []byte
				switch (command >> 2) & 3 {
				case 0:
					data = nil
				case 1:
					data = []byte{}
				default:
					data = []byte{command ^ 0xA5}
				}
				want := expectedAddressError(block, page)
				if want == nil {
					switch {
					case data == nil:
						want = ErrPayload
					case len(data) != 1:
						want = ErrPayloadSize
					case model.programmed[block][page]:
						want = ErrProgrammed
					}
				}
				got := device.Program(block, page, data)
				if !sameError(got, want) {
					t.Fatalf("step %d Program(%d,%d) error=%v; want %v", step, block, page, got, want)
				}
				if want == nil {
					model.pages[block][page][0] = data[0]
					model.programmed[block][page] = true
				}
			case 1: // Erase the selected block or check that refusal is atomic.
				var want error
				if block < 0 || block >= 2 {
					want = ErrBlock
				} else if model.erases[block] >= model.limit {
					want = ErrEraseLimit
				}
				got := device.EraseBlock(block)
				if !sameError(got, want) {
					t.Fatalf("step %d EraseBlock(%d) error=%v; want %v", step, block, got, want)
				}
				if want == nil {
					model.erases[block]++
					for p := range model.pages[block] {
						model.pages[block][p][0] = 0xFF
						model.programmed[block][p] = false
					}
				}
			case 2: // Read a cell and compare both bytes and address failure.
				want := expectedAddressError(block, page)
				got, err := device.Read(block, page)
				if !sameError(err, want) {
					t.Fatalf("step %d Read(%d,%d) error=%v; want %v", step, block, page, err, want)
				}
				if want == nil && (len(got) != 1 || got[0] != model.pages[block][page][0]) {
					t.Fatalf("step %d Read(%d,%d) = %v; want %v", step, block, page, got, model.pages[block][page])
				}
			case 3: // Query markers through the same address validation order.
				want := expectedAddressError(block, page)
				got, err := device.IsProgrammed(block, page)
				if !sameError(err, want) {
					t.Fatalf("step %d IsProgrammed(%d,%d) error=%v; want %v", step, block, page, err, want)
				}
				if want == nil && got != model.programmed[block][page] {
					t.Fatalf("step %d marker = %v; want %v", step, got, model.programmed[block][page])
				}
			}
			compareModelState(t, device, model)
		}
	})
}

func expectedAddressError(block, page int) error {
	if block < 0 || block >= 2 {
		return ErrBlock
	}
	if page < 0 || page >= 2 {
		return ErrPage
	}
	return nil
}

// FuzzWideAddressAndPayload supplements the byte-command stream with full
// signed-width addresses and arbitrary bounded payload lengths.
func FuzzWideAddressAndPayload(f *testing.F) {
	f.Add(int64(0), int64(0), []byte{0xA5}, false)
	f.Add(int64(-1), int64(0), []byte{0xA5}, false)
	f.Add(int64(1), int64(-1), []byte{0xA5}, true)
	f.Add(int64(0), int64(0), []byte{}, false)
	f.Fuzz(func(t *testing.T, block64, page64 int64, payload []byte, nilPayload bool) {
		if len(payload) > 64 {
			return
		}
		block, blockOK := fitHostInt(block64)
		page, pageOK := fitHostInt(page64)
		if !blockOK || !pageOK {
			return
		}
		device, err := New(Config{Geometry: Geometry{2, 2, 1}, MaxEraseCycles: ptrUint64(2)})
		if err != nil {
			t.Fatal(err)
		}
		model := newModel(2, 2, 1, 2)
		data := append([]byte{}, payload...)
		if nilPayload {
			data = nil
		}
		want := expectedAddressError(block, page)
		if want == nil {
			switch {
			case data == nil:
				want = ErrPayload
			case len(data) != 1:
				want = ErrPayloadSize
			case model.programmed[block][page]:
				want = ErrProgrammed
			}
		}
		got := device.Program(block, page, data)
		if !sameError(got, want) {
			t.Fatalf("Program(%d,%d,len=%d,nil=%t) = %v; want %v", block, page, len(data), nilPayload, got, want)
		}
		if want == nil {
			model.pages[block][page][0] = data[0]
			model.programmed[block][page] = true
		}
		compareModelState(t, device, model)
	})
}

func fitHostInt(value int64) (int, bool) {
	converted := int(value)
	return converted, int64(converted) == value
}

func sameError(got, want error) bool {
	if want == nil {
		return got == nil
	}
	return errors.Is(got, want)
}
