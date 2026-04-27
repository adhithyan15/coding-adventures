package ge225simulator

import "fmt"

const (
	mask20    = (1 << 20) - 1
	dataMask  = (1 << 19) - 1
	signBit   = 1 << 19
	addrMask  = 0x1fff
	xMask     = 0x7fff
	nMask     = 0x3f
	wordBytes = 3
	maxXGroups = 32
)

const (
	opLDA = 0o00
	opADD = 0o01
	opSUB = 0o02
	opSTA = 0o03
	opBXL = 0o04
	opBXH = 0o05
	opLDX = 0o06
	opSPB = 0o07
	opDLD = 0o10
	opDAD = 0o11
	opDSU = 0o12
	opDST = 0o13
	opINX = 0o14
	opMPY = 0o15
	opDVD = 0o16
	opSTX = 0o17
	opEXT = 0o20
	opCAB = 0o21
	opDCB = 0o22
	opORY = 0o23
	opMOY = 0o24
	opRCD = 0o25
	opBRU = 0o26
	opSTO = 0o27
)

var baseOpcodeNames = map[int]string{
	opLDA: "LDA", opADD: "ADD", opSUB: "SUB", opSTA: "STA", opBXL: "BXL",
	opBXH: "BXH", opLDX: "LDX", opSPB: "SPB", opDLD: "DLD", opDAD: "DAD",
	opDSU: "DSU", opDST: "DST", opINX: "INX", opMPY: "MPY", opDVD: "DVD",
	opSTX: "STX", opEXT: "EXT", opCAB: "CAB", opDCB: "DCB", opORY: "ORY",
	opMOY: "MOY", opRCD: "RCD", opBRU: "BRU", opSTO: "STO",
}

var fixedWords = map[string]int{
	"OFF": 0o2500005, "TYP": 0o2500006, "TON": 0o2500007, "RCS": 0o2500011,
	"HPT": 0o2500016, "LDZ": 0o2504002, "LDO": 0o2504022, "LMO": 0o2504102,
	"CPL": 0o2504502, "NEG": 0o2504522, "CHS": 0o2504040, "NOP": 0o2504012,
	"LAQ": 0o2504001, "LQA": 0o2504004, "XAQ": 0o2504005, "MAQ": 0o2504006,
	"ADO": 0o2504032, "SBO": 0o2504112, "SET_DECMODE": 0o2506011, "SET_BINMODE": 0o2506012,
	"SXG": 0o2506013, "SET_PST": 0o2506015, "SET_PBK": 0o2506016,
	"BOD": 0o2514000, "BEV": 0o2516000, "BMI": 0o2514001, "BPL": 0o2516001,
	"BZE": 0o2514002, "BNZ": 0o2516002, "BOV": 0o2514003, "BNO": 0o2516003,
	"BPE": 0o2514004, "BPC": 0o2516004, "BNR": 0o2514005, "BNN": 0o2516005,
}

var fixedNames = func() map[int]string {
	out := map[int]string{}
	for name, word := range fixedWords {
		out[word] = name
	}
	return out
}()

var shiftBases = map[string]int{
	"SRA": 0o2510000, "SNA": 0o2510100, "SCA": 0o2510040, "SAN": 0o2510400,
	"SRD": 0o2511000, "NAQ": 0o2511100, "SCD": 0o2511200, "ANQ": 0o2511400,
	"SLA": 0o2512000, "SLD": 0o2512200, "NOR": 0o2513000, "DNO": 0o2513200,
}

var typewriterCodes = map[int]string{
	0o00: "0", 0o01: "1", 0o02: "2", 0o03: "3", 0o04: "4", 0o05: "5", 0o06: "6", 0o07: "7",
	0o10: "8", 0o11: "9", 0o13: "/", 0o21: "A", 0o22: "B", 0o23: "C", 0o24: "D", 0o25: "E",
	0o26: "F", 0o27: "G", 0o30: "H", 0o31: "I", 0o33: "-", 0o40: ".", 0o41: "J", 0o42: "K",
	0o43: "L", 0o44: "M", 0o45: "N", 0o46: "O", 0o47: "P", 0o50: "Q", 0o51: "R", 0o53: "$",
	0o60: " ", 0o62: "S", 0o63: "T", 0o64: "U", 0o65: "V", 0o66: "W", 0o67: "X", 0o70: "Y",
	0o71: "Z",
}

type Indicators struct {
	Carry       bool
	Zero        bool
	Negative    bool
	Overflow    bool
	ParityError bool
}

type State struct {
	A                      int
	Q                      int
	M                      int
	N                      int
	PC                     int
	IR                     int
	Indicators             Indicators
	Overflow               bool
	ParityError            bool
	DecimalMode            bool
	AutomaticInterruptMode bool
	SelectedXGroup         int
	NReady                 bool
	TypewriterPower        bool
	ControlSwitches        int
	XWords                 []int
	Halted                 bool
	Memory                 []int
}

type Trace struct {
	Address          int
	InstructionWord  int
	Mnemonic         string
	ABefore          int
	AAfter           int
	QBefore          int
	QAfter           int
	EffectiveAddress *int
}

type decodedInstruction struct {
	Mnemonic  string
	Opcode    *int
	Modifier  *int
	Address   *int
	Count     *int
	FixedWord bool
}

type Simulator struct {
	memorySize              int
	memory                  []int
	cardReaderQueue         [][]int
	a, q, m, n             int
	pc, ir                  int
	overflow, parityError   bool
	decimalMode             bool
	automaticInterruptMode  bool
	selectedXGroup          int
	nReady, typewriterPower bool
	typewriterOutput        []string
	controlSwitches         int
	halted                  bool
	xGroups                 [maxXGroups][4]int
}

func New(memoryWords int) *Simulator {
	if memoryWords <= 0 {
		panic("memoryWords must be positive")
	}
	return &Simulator{memorySize: memoryWords, memory: make([]int, memoryWords)}
}

func (s *Simulator) Reset() {
	s.a, s.q, s.m, s.n = 0, 0, 0, 0
	s.pc, s.ir = 0, 0
	s.overflow, s.parityError = false, false
	s.decimalMode, s.automaticInterruptMode = false, false
	s.selectedXGroup = 0
	s.nReady = true
	s.typewriterPower = false
	s.typewriterOutput = nil
	s.controlSwitches = 0
	s.halted = false
	for i := range s.xGroups {
		s.xGroups[i] = [4]int{}
	}
}

func (s *Simulator) GetState() State {
	mem := append([]int(nil), s.memory...)
	xWords := append([]int(nil), s.xGroups[s.selectedXGroup][:]...)
	return State{
		A: s.a, Q: s.q, M: s.m, N: s.n, PC: s.pc, IR: s.ir,
		Indicators: Indicators{
			Carry: s.overflow, Zero: s.a == 0, Negative: s.a&signBit != 0,
			Overflow: s.overflow, ParityError: s.parityError,
		},
		Overflow: s.overflow, ParityError: s.parityError, DecimalMode: s.decimalMode,
		AutomaticInterruptMode: s.automaticInterruptMode, SelectedXGroup: s.selectedXGroup,
		NReady: s.nReady, TypewriterPower: s.typewriterPower, ControlSwitches: s.controlSwitches,
		XWords: xWords, Halted: s.halted, Memory: mem,
	}
}

func (s *Simulator) SetControlSwitches(value int) { s.controlSwitches = value & mask20 }
func (s *Simulator) QueueCardReaderRecord(words []int) {
	record := make([]int, len(words))
	for i, word := range words { record[i] = word & mask20 }
	s.cardReaderQueue = append(s.cardReaderQueue, record)
}
func (s *Simulator) GetTypewriterOutput() string {
	out := ""
	for _, chunk := range s.typewriterOutput { out += chunk }
	return out
}

func (s *Simulator) LoadWords(words []int, startAddress int) error {
	for i, word := range words {
		if err := s.WriteWord(startAddress+i, word); err != nil { return err }
	}
	return nil
}

func (s *Simulator) ReadWord(address int) (int, error) {
	if err := s.checkAddress(address); err != nil { return 0, err }
	return s.memory[address], nil
}

func (s *Simulator) WriteWord(address int, value int) error {
	if err := s.checkAddress(address); err != nil { return err }
	s.memory[address] = value & mask20
	return nil
}

func EncodeInstruction(opcode, modifier, address int) (int, error) {
	if opcode < 0 || opcode > 0o37 { return 0, fmt.Errorf("opcode out of range: %d", opcode) }
	if modifier < 0 || modifier > 0o3 { return 0, fmt.Errorf("modifier out of range: %d", modifier) }
	if address < 0 || address > addrMask { return 0, fmt.Errorf("address out of range: %d", address) }
	return ((opcode & 0x1f) << 15) | ((modifier & 0x03) << 13) | (address & addrMask), nil
}

func DecodeInstruction(word int) (int, int, int) {
	normalized := word & mask20
	return (normalized >> 15) & 0x1f, (normalized >> 13) & 0x03, normalized & addrMask
}

func AssembleFixed(mnemonic string) (int, error) {
	word, ok := fixedWords[mnemonic]
	if !ok { return 0, fmt.Errorf("unknown fixed GE-225 instruction: %s", mnemonic) }
	return word, nil
}

func AssembleShift(mnemonic string, count int) (int, error) {
	if count < 0 || count > 0o37 { return 0, fmt.Errorf("shift count out of range: %d", count) }
	base, ok := shiftBases[mnemonic]
	if !ok { return 0, fmt.Errorf("unknown GE-225 shift instruction: %s", mnemonic) }
	return base | count, nil
}

func PackWords(words []int) []byte {
	blob := make([]byte, len(words)*wordBytes)
	for i, word := range words {
		normalized := word & mask20
		blob[i*wordBytes] = byte((normalized >> 16) & 0xff)
		blob[i*wordBytes+1] = byte((normalized >> 8) & 0xff)
		blob[i*wordBytes+2] = byte(normalized & 0xff)
	}
	return blob
}

func UnpackWords(program []byte) ([]int, error) {
	if len(program)%wordBytes != 0 { return nil, fmt.Errorf("GE-225 byte stream must be a multiple of %d bytes, got %d", wordBytes, len(program)) }
	words := make([]int, 0, len(program)/wordBytes)
	for i := 0; i < len(program); i += wordBytes {
		words = append(words, (int(program[i])<<16|int(program[i+1])<<8|int(program[i+2]))&mask20)
	}
	return words, nil
}

func (s *Simulator) DisassembleWord(word int) (string, error) {
	decoded, err := s.decodeWord(word)
	if err != nil { return "", err }
	if decoded.FixedWord {
		if decoded.Count == nil { return decoded.Mnemonic, nil }
		return fmt.Sprintf("%s %d", decoded.Mnemonic, *decoded.Count), nil
	}
	return fmt.Sprintf("%s 0x%03X,X%d", decoded.Mnemonic, *decoded.Address, *decoded.Modifier), nil
}

func (s *Simulator) Step() (Trace, error) {
	if s.halted { return Trace{}, fmt.Errorf("cannot step a halted GE-225 simulator") }
	pcBefore := s.pc
	word, _ := s.ReadWord(s.pc)
	s.ir = word
	s.pc = (s.pc + 1) % s.memorySize
	decoded, err := s.decodeWord(s.ir)
	if err != nil { return Trace{}, err }
	aBefore, qBefore := s.a, s.q
	var effectiveAddress *int
	if !decoded.FixedWord {
		address := *decoded.Address
		if decoded.Mnemonic != "BXL" && decoded.Mnemonic != "BXH" && decoded.Mnemonic != "LDX" &&
			decoded.Mnemonic != "SPB" && decoded.Mnemonic != "INX" && decoded.Mnemonic != "STX" && decoded.Mnemonic != "MOY" {
			ea := s.resolveEffectiveAddress(address, *decoded.Modifier)
			effectiveAddress = &ea
		}
		runAddr := address
		if effectiveAddress != nil { runAddr = *effectiveAddress }
		if err := s.executeMemoryReference(decoded.Mnemonic, *decoded.Modifier, runAddr, address, pcBefore); err != nil { return Trace{}, err }
	} else {
		if err := s.executeFixed(decoded); err != nil { return Trace{}, err }
	}
	mnemonic, _ := s.DisassembleWord(s.ir)
	return Trace{Address: pcBefore, InstructionWord: s.ir, Mnemonic: mnemonic, ABefore: aBefore, AAfter: s.a, QBefore: qBefore, QAfter: s.q, EffectiveAddress: effectiveAddress}, nil
}

func (s *Simulator) Run(maxSteps int) ([]Trace, error) {
	traces := []Trace{}
	for steps := 0; !s.halted && steps < maxSteps; steps++ {
		trace, err := s.Step()
		if err != nil { return traces, err }
		traces = append(traces, trace)
	}
	return traces, nil
}

func toSigned20(value int) int {
	word := value & mask20
	if word&signBit != 0 { return word - (1 << 20) }
	return word
}

func fromSigned20(value int) int { return value & mask20 }
func signOf(word int) int { if word&signBit != 0 { return 1 }; return 0 }
func withSign(word, sign int) int { return ((sign & 1) << 19) | (word & dataMask) }

func combineWords(high, low int) int64 { return (int64(high&mask20) << 20) | int64(low&mask20) }
func splitSigned40(value int64) (int, int) {
	const mask40 int64 = (1 << 40) - 1
	raw := value & mask40
	return int((raw >> 20) & mask20), int(raw & mask20)
}
func toSigned40(value int64) int64 { return (value << 24) >> 24 }

func arithCompare(left, right int) int {
	l, r := toSigned20(left), toSigned20(right)
	if l < r { return -1 }
	if l > r { return 1 }
	return 0
}
func arithCompareDouble(lh, ll, rh, rl int) int {
	left, right := toSigned40(combineWords(lh, ll)), toSigned40(combineWords(rh, rl))
	if left < right { return -1 }
	if left > right { return 1 }
	return 0
}

func (s *Simulator) getXWord(slot int) int { return s.xGroups[s.selectedXGroup][slot] & xMask }
func (s *Simulator) setXWord(slot, value int) { s.xGroups[s.selectedXGroup][slot] = value & xMask }

func (s *Simulator) executeMemoryReference(mnemonic string, modifier, effectiveOrRawAddress, rawAddress, pcBefore int) error {
	effectiveAddress := effectiveOrRawAddress % s.memorySize
	switch mnemonic {
	case "LDA":
		word, _ := s.ReadWord(effectiveAddress); s.m = word; s.a = word
	case "ADD":
		word, _ := s.ReadWord(effectiveAddress); s.m = word
		total := toSigned20(s.a) + toSigned20(s.m); s.a = fromSigned20(total); s.overflow = total < -(1<<19) || total > ((1<<19)-1)
	case "SUB":
		word, _ := s.ReadWord(effectiveAddress); s.m = word
		total := toSigned20(s.a) - toSigned20(s.m); s.a = fromSigned20(total); s.overflow = total < -(1<<19) || total > ((1<<19)-1)
	case "STA":
		return s.WriteWord(effectiveAddress, s.a)
	case "BXL":
		if s.getXWord(modifier)&addrMask >= rawAddress { s.pc = (s.pc + 1) % s.memorySize }
	case "BXH":
		if s.getXWord(modifier)&addrMask < rawAddress { s.pc = (s.pc + 1) % s.memorySize }
	case "LDX":
		word, _ := s.ReadWord(rawAddress % s.memorySize); s.setXWord(modifier, word)
	case "SPB":
		s.setXWord(modifier, pcBefore); s.pc = rawAddress % s.memorySize
	case "DLD":
		first, _ := s.ReadWord(effectiveAddress)
		if effectiveAddress&1 != 0 { s.a, s.q = first, first } else { second, _ := s.ReadWord((effectiveAddress + 1) % s.memorySize); s.a, s.q = first, second }
	case "DAD":
		left := toSigned40(combineWords(s.a, s.q))
		first, _ := s.ReadWord(effectiveAddress); second := first
		if effectiveAddress&1 == 0 { second, _ = s.ReadWord((effectiveAddress + 1) % s.memorySize) }
		total := left + toSigned40(combineWords(first, second)); s.a, s.q = splitSigned40(total); s.overflow = total < -(1<<39) || total > ((1<<39)-1)
	case "DSU":
		left := toSigned40(combineWords(s.a, s.q))
		first, _ := s.ReadWord(effectiveAddress); second := first
		if effectiveAddress&1 == 0 { second, _ = s.ReadWord((effectiveAddress + 1) % s.memorySize) }
		total := left - toSigned40(combineWords(first, second)); s.a, s.q = splitSigned40(total); s.overflow = total < -(1<<39) || total > ((1<<39)-1)
	case "DST":
		if effectiveAddress&1 != 0 { return s.WriteWord(effectiveAddress, s.q) }
		if err := s.WriteWord(effectiveAddress, s.a); err != nil { return err }
		return s.WriteWord((effectiveAddress+1)%s.memorySize, s.q)
	case "INX":
		s.setXWord(modifier, (s.getXWord(modifier)+rawAddress)&xMask)
	case "MPY":
		word, _ := s.ReadWord(effectiveAddress); s.m = word
		product := int64(toSigned20(s.q))*int64(toSigned20(s.m)) + int64(toSigned20(s.a)); s.a, s.q = splitSigned40(product); s.overflow = product < -(1<<39) || product > ((1<<39)-1)
	case "DVD":
		word, _ := s.ReadWord(effectiveAddress); s.m = word; divisor := int64(toSigned20(s.m))
		if divisor == 0 { return fmt.Errorf("GE-225 divide by zero") }
		aAbs := toSigned20(s.a); if aAbs < 0 { aAbs = -aAbs }
		dAbs := divisor; if dAbs < 0 { dAbs = -dAbs }
		if int64(aAbs) >= dAbs { s.overflow = true; return nil }
		dividend := toSigned40(combineWords(s.a, s.q)); absDividend := dividend; if absDividend < 0 { absDividend = -absDividend }
		quotientMag, remainderMag := absDividend/dAbs, absDividend%dAbs
		quotient := quotientMag; if (dividend < 0) != (divisor < 0) { quotient = -quotient }
		remainder := remainderMag; if quotient < 0 { remainder = -remainder }
		s.a, s.q = fromSigned20(int(quotient)), fromSigned20(int(remainder)); s.overflow = quotient < -(1<<19) || quotient > ((1<<19)-1)
	case "STX":
		return s.WriteWord(rawAddress%s.memorySize, s.getXWord(modifier))
	case "EXT":
		word, _ := s.ReadWord(effectiveAddress); s.m = word; s.a &= (^s.m) & mask20
	case "CAB":
		word, _ := s.ReadWord(effectiveAddress); s.m = word; relation := arithCompare(s.m, s.a)
		if relation == 0 { s.pc = (s.pc + 1) % s.memorySize } else if relation < 0 { s.pc = (s.pc + 2) % s.memorySize }
	case "DCB":
		first, _ := s.ReadWord(effectiveAddress); second := first; if effectiveAddress&1 == 0 { second, _ = s.ReadWord((effectiveAddress + 1) % s.memorySize) }
		relation := arithCompareDouble(first, second, s.a, s.q)
		if relation == 0 { s.pc = (s.pc + 1) % s.memorySize } else if relation < 0 { s.pc = (s.pc + 2) % s.memorySize }
	case "ORY":
		word, _ := s.ReadWord(effectiveAddress); return s.WriteWord(effectiveAddress, word|s.a)
	case "MOY":
		wordCount := -toSigned20(s.q); if wordCount < 0 { wordCount = 0 }
		destination := s.a & xMask
		for offset := 0; offset < wordCount; offset++ { word, _ := s.ReadWord((rawAddress + offset) % s.memorySize); _ = s.WriteWord((destination+offset)%s.memorySize, word) }
		s.setXWord(0, s.pc); s.a = 0
	case "RCD":
		if len(s.cardReaderQueue) == 0 { return fmt.Errorf("RCD executed with no queued card-reader record") }
		record := s.cardReaderQueue[0]; s.cardReaderQueue = s.cardReaderQueue[1:]
		for offset, word := range record { _ = s.WriteWord((effectiveAddress+offset)%s.memorySize, word) }
	case "BRU":
		s.pc = effectiveAddress
	case "STO":
		existing, _ := s.ReadWord(effectiveAddress); return s.WriteWord(effectiveAddress, (existing &^ addrMask) | (s.a & addrMask))
	default:
		return fmt.Errorf("unimplemented GE-225 memory-reference instruction: %s", mnemonic)
	}
	return nil
}

func (s *Simulator) executeFixed(decoded decodedInstruction) error {
	mnemonic, count := decoded.Mnemonic, 0
	if decoded.Count != nil { count = *decoded.Count }
	switch mnemonic {
	case "OFF":
		s.typewriterPower, s.nReady = false, true
	case "TYP":
		if !s.typewriterPower { s.nReady = false; return nil }
		code := s.n & nMask
		if code == 0o37 { s.typewriterOutput = append(s.typewriterOutput, "\r") } else if code == 0o76 { s.typewriterOutput = append(s.typewriterOutput, "\t") } else if code != 0o72 && code != 0o75 {
			char, ok := typewriterCodes[code]; if !ok { s.nReady = false; return nil }; s.typewriterOutput = append(s.typewriterOutput, char)
		}
		s.nReady = true
	case "TON":
		s.typewriterPower = true
	case "RCS":
		s.a |= s.controlSwitches
	case "HPT":
		s.nReady = false
	case "LDZ":
		s.a = 0
	case "LDO":
		s.a = 1
	case "LMO":
		s.a = mask20
	case "CPL":
		s.a = (^s.a) & mask20
	case "NEG":
		before := toSigned20(s.a); s.a = fromSigned20(-before); s.overflow = before == -(1<<19)
	case "CHS":
		s.a ^= signBit
	case "NOP":
	case "LAQ":
		s.a = s.q
	case "LQA":
		s.q = s.a
	case "XAQ":
		s.a, s.q = s.q, s.a
	case "MAQ":
		s.q = s.a; s.a = 0
	case "ADO":
		total := toSigned20(s.a) + 1; s.a = fromSigned20(total); s.overflow = total < -(1<<19) || total > ((1<<19)-1)
	case "SBO":
		total := toSigned20(s.a) - 1; s.a = fromSigned20(total); s.overflow = total < -(1<<19) || total > ((1<<19)-1)
	case "SET_DECMODE":
		s.decimalMode = true
	case "SET_BINMODE":
		s.decimalMode = false
	case "SXG":
		s.selectedXGroup = s.a & 0x1f
	case "SET_PST":
		s.automaticInterruptMode = true
	case "SET_PBK":
		s.automaticInterruptMode = false
	case "BOD", "BEV", "BMI", "BPL", "BZE", "BNZ", "BOV", "BNO", "BPE", "BPC", "BNR", "BNN":
		s.executeBranchTest(mnemonic)
	default:
		if _, ok := shiftBases[mnemonic]; ok { s.executeShift(mnemonic, count); return nil }
		return fmt.Errorf("unimplemented GE-225 fixed instruction: %s", mnemonic)
	}
	return nil
}

func (s *Simulator) executeBranchTest(mnemonic string) {
	cond := false
	switch mnemonic {
	case "BOD": cond = s.a&1 != 0
	case "BEV": cond = s.a&1 == 0
	case "BMI": cond = s.a&signBit != 0
	case "BPL": cond = s.a&signBit == 0
	case "BZE": cond = s.a == 0
	case "BNZ": cond = s.a != 0
	case "BOV": cond = s.overflow
	case "BNO": cond = !s.overflow
	case "BPE": cond = s.parityError
	case "BPC": cond = !s.parityError
	case "BNR": cond = s.nReady
	case "BNN": cond = !s.nReady
	}
	if mnemonic == "BOV" || mnemonic == "BNO" { s.overflow = false }
	if mnemonic == "BPE" || mnemonic == "BPC" { s.parityError = false }
	if !cond { s.pc = (s.pc + 1) % s.memorySize }
}

func (s *Simulator) executeShift(mnemonic string, count int) {
	if count == 0 {
		if mnemonic == "SRD" { s.q = withSign(s.q, signOf(s.a)) } else if mnemonic == "SLD" { s.a = withSign(s.a, signOf(s.q)) }
		return
	}
	aSign, aData := signOf(s.a), s.a&dataMask
	qSign, qData := signOf(s.q), s.q&dataMask
	switch mnemonic {
	case "SRA":
		s.a = fromSigned20(toSigned20(s.a) >> min(count, 19))
	case "SLA":
		s.overflow = (aData >> max(0, 19-count)) != 0; s.a = withSign((aData<<count)&dataMask, aSign)
	case "SCA":
		rotation := count % 19; if rotation != 0 { aData = ((aData >> rotation) | (aData << (19 - rotation))) & dataMask }; s.a = withSign(aData, aSign)
	case "SAN":
		fill := 0; if aSign == 1 { fill = (1 << count) - 1 }
		combined := ((aData & dataMask) << 6) | (s.n & nMask); combined = ((fill << 25) | combined) >> count
		s.a = withSign((combined>>6)&dataMask, aSign); s.n = combined & nMask
	case "SNA":
		combined := (((s.n & nMask) << 19) | aData) >> count; s.n = (combined >> 19) & nMask; s.a = withSign(combined&dataMask, aSign)
	case "SRD":
		value := combineWords(s.a, s.q) >> count; s.a = withSign(int((value>>20)&dataMask), aSign); s.q = withSign(int(value&dataMask), aSign)
	case "NAQ":
		combined := (((s.n & nMask) << 38) | ((aData & dataMask) << 19) | qData) >> count
		s.n = (combined >> 38) & nMask; s.a = withSign((combined>>19)&dataMask, aSign); s.q = withSign(combined&dataMask, aSign)
	case "SCD":
		rotation := count % 38; combined := ((aData & dataMask) << 19) | qData
		if rotation != 0 { combined = ((combined >> rotation) | (combined << (38 - rotation))) & ((1 << 38) - 1) }
		s.a = withSign((combined>>19)&dataMask, aSign); s.q = withSign(combined&dataMask, aSign)
	case "ANQ":
		for i := 0; i < count; i++ { bit := s.a & 1; s.a = fromSigned20(toSigned20(s.a) >> 1); qData = ((bit << 18) | ((s.q & dataMask) >> 1)) & dataMask; s.q = withSign(qData, aSign); s.n = ((bit << 5) | (s.n >> 1)) & nMask }
	case "SLD":
		combined := ((aData & dataMask) << 19) | qData; s.overflow = (combined >> max(0, 38-count)) != 0; combined = (combined << count) & ((1 << 38) - 1); s.a = withSign((combined>>19)&dataMask, qSign); s.q = withSign(combined&dataMask, qSign)
	case "NOR":
		shifts, targetBit := 0, 0; if aSign == 1 { targetBit = 1 }
		for shifts < count { lead := (aData >> 18) & 1; if lead != targetBit { break }; s.overflow = s.overflow || lead == 1; aData = (aData << 1) & dataMask; shifts++ }
		s.a = withSign(aData, aSign); s.setXWord(0, count-shifts)
	case "DNO":
		shifts, targetBit := 0, 0; if aSign == 1 { targetBit = 1 }
		combined := ((aData & dataMask) << 19) | qData
		for shifts < count { lead := (combined >> 37) & 1; if lead != targetBit { break }; s.overflow = s.overflow || lead == 1; combined = (combined << 1) & ((1 << 38) - 1); shifts++ }
		s.a = withSign((combined>>19)&dataMask, qSign); s.q = withSign(combined&dataMask, qSign); s.setXWord(0, count-shifts)
	}
}

func (s *Simulator) decodeWord(word int) (decodedInstruction, error) {
	normalized := word & mask20
	if name, ok := fixedNames[normalized]; ok { return decodedInstruction{Mnemonic: name, FixedWord: true}, nil }
	for name, base := range shiftBases {
		if (normalized & ^0o37) == base { count := normalized & 0o37; return decodedInstruction{Mnemonic: name, Count: &count, FixedWord: true}, nil }
	}
	opcode, modifier, address := DecodeInstruction(normalized); mnemonic, ok := baseOpcodeNames[opcode]
	if !ok { return decodedInstruction{}, fmt.Errorf("unknown GE-225 opcode field %o", opcode) }
	return decodedInstruction{Mnemonic: mnemonic, Opcode: &opcode, Modifier: &modifier, Address: &address, FixedWord: false}, nil
}

func (s *Simulator) resolveEffectiveAddress(address, modifier int) int {
	base := address % s.memorySize
	if modifier == 0 { return base }
	return (base + (s.getXWord(modifier) % s.memorySize)) % s.memorySize
}

func (s *Simulator) checkAddress(address int) error {
	if address < 0 || address >= s.memorySize { return fmt.Errorf("address out of range: %d", address) }
	return nil
}

func min(a, b int) int { if a < b { return a }; return b }
func max(a, b int) int { if a > b { return a }; return b }
