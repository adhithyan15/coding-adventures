package fparithmetic

// Pipelined floating-point arithmetic -- the bridge to GPU architecture.
//
// === Why Pipelining? ===
//
// Imagine a car factory with a single worker who does everything: welds the
// frame, installs the engine, paints the body, mounts the wheels, inspects
// the result. One car takes 5 hours. Want 100 cars? That's 500 hours.
//
// Now imagine a factory with 5 stations, each doing one step. The first car
// still takes 5 hours to pass through all 5 stations. But while it moves to
// station 2, a NEW car enters station 1. After the initial 5-hour fill-up
// time, a finished car rolls off the line every HOUR -- 5x throughput!
//
// This is pipelining, and it's exactly how GPUs achieve massive throughput.
//
// === Latency vs Throughput ===
//
//	Latency:     Time for ONE operation to complete start-to-finish.
//	Throughput:  How many operations complete per unit time.
//
// For a 5-stage pipeline:
//
//	Latency = 5 clock cycles (one operation still takes 5 cycles)
//	Throughput = 1 result per clock cycle (after pipeline fills up)
//
// === Pipeline Timing Diagram ===
//
//	Clock:  1    2    3    4    5    6    7    8
//	--------------------------------------------
//	Stage1: [A1] [B1] [C1] [D1]  -    -    -    -
//	Stage2:  -   [A2] [B2] [C2] [D2]  -    -    -
//	Stage3:  -    -   [A3] [B3] [C3] [D3]  -    -
//	Stage4:  -    -    -   [A4] [B4] [C4] [D4]  -
//	Stage5:  -    -    -    -   [A5] [B5] [C5] [D5]
//
// === How This Connects to GPUs ===
//
// A modern GPU has thousands of CUDA cores, each containing pipelined FP units.
// With 5000 cores each running pipelined FP at 1.5 GHz:
//
//	5000 cores x 1 result/cycle x 1.5 GHz = 7.5 TFLOPS
//
// === Go Concurrency Advantage ===
//
// Go's goroutines and channels are a natural fit for pipeline simulation.
// Each pipeline uses a mutex-protected array of stage data, advanced on
// each rising clock edge via a clock listener callback. This mirrors how
// real hardware pipeline registers capture data on each clock edge.

import (
	"sync"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// stageData -- intermediate pipeline data passed between stages
// =========================================================================

// stageData holds the intermediate computation state as it flows through
// pipeline stages. In hardware, this data lives in pipeline registers --
// banks of D flip-flops that capture values on each clock edge.
//
// The "special" field handles bypass cases (NaN, Inf, Zero) that skip
// the normal computation stages. When special is non-nil, stages simply
// pass the data through without processing.
type stageData struct {
	// If non-nil, this is a pre-computed result (NaN, Inf, zero) that
	// bypasses the normal pipeline stages. In hardware, this is a
	// multiplexer that selects between the normal computation path
	// and the special-value bypass.
	special *FloatBits

	// Normal computation fields. Different stages use different subsets.
	signA, signB     int
	expA, expB       int
	mantA, mantB     int
	guardBits        int
	resultSign       int
	resultMant       int
	resultExp        int
	product          int
	productSign      int
	productLeading   int
	cSign            int
	expC             int
	mantC            int
	productExp       int
	cAligned         int
}

// =========================================================================
// PipelinedFPAdder -- 5-stage pipelined floating-point adder
// =========================================================================

// PipelinedFPAdder is a 5-stage pipelined floating-point adder driven by a clock.
//
// In real GPU hardware, the FP adder is pipelined so that while one
// addition is being normalized (stage 4), a newer addition is being
// aligned (stage 2), and an even newer one is being unpacked (stage 1).
//
// === Pipeline Stages ===
//
//	Stage 1: UNPACK    -- Extract sign, exponent, mantissa. Handle specials.
//	Stage 2: ALIGN     -- Compare exponents, shift smaller mantissa right.
//	Stage 3: ADD/SUB   -- Add or subtract aligned mantissas.
//	Stage 4: NORMALIZE -- Shift result to get leading 1 in correct position.
//	Stage 5: ROUND     -- Apply round-to-nearest-even, pack into FloatBits.
type PipelinedFPAdder struct {
	Clk        *clock.Clock
	Fmt        FloatFormat
	Results    []*FloatBits
	CycleCount int

	mu            sync.Mutex
	stages        [5]*stageData
	inputsPending []addInput
}

type addInput struct {
	a, b FloatBits
}

// NewPipelinedFPAdder creates a new 5-stage pipelined adder and registers
// it as a listener on the given clock. On each rising edge, the pipeline
// advances one stage -- just like hardware pipeline registers capturing
// data on the clock edge.
func NewPipelinedFPAdder(clk *clock.Clock, fmt FloatFormat) *PipelinedFPAdder {
	p := &PipelinedFPAdder{
		Clk: clk,
		Fmt: fmt,
	}
	clk.RegisterListener(p.onClockEdge)
	return p
}

// Submit queues a new addition (a + b) to enter the pipeline on the next
// rising clock edge. In hardware, this is the dispatch unit loading
// operands into the pipeline's input register.
func (p *PipelinedFPAdder) Submit(a, b FloatBits) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.inputsPending = append(p.inputsPending, addInput{a, b})
}

// onClockEdge advances the pipeline on rising clock edges.
//
// This is the heart of the pipeline simulation. On every rising edge:
//  1. Shift all stages forward: stage[i] = process(stage[i-1])
//  2. Load new input into stage 0 (if pending)
//  3. Collect output from the last stage (if any)
func (p *PipelinedFPAdder) onClockEdge(edge clock.ClockEdge) {
	if !edge.IsRising {
		return
	}

	p.mu.Lock()
	defer p.mu.Unlock()

	p.CycleCount++

	// Shift pipeline forward (from end to avoid overwriting)
	for i := 4; i > 0; i-- {
		p.stages[i] = p.adderProcessStage(i, p.stages[i-1])
	}

	// Load new input
	if len(p.inputsPending) > 0 {
		inp := p.inputsPending[0]
		p.inputsPending = p.inputsPending[1:]
		p.stages[0] = p.adderStageUnpack(inp.a, inp.b)
	} else {
		p.stages[0] = nil
	}

	// Collect output from last stage
	if p.stages[4] != nil {
		if p.stages[4].special != nil {
			result := *p.stages[4].special
			p.Results = append(p.Results, &result)
		}
		p.stages[4] = nil
	}
}

func (p *PipelinedFPAdder) adderProcessStage(stageNum int, input *stageData) *stageData {
	if input == nil {
		return nil
	}
	switch stageNum {
	case 1:
		return p.adderStageAlign(input)
	case 2:
		return p.adderStageAdd(input)
	case 3:
		return p.adderStageNormalize(input)
	case 4:
		return p.adderStageRoundPack(input)
	}
	return nil
}

// Stage 0: UNPACK -- extract fields and detect special values.
func (p *PipelinedFPAdder) adderStageUnpack(a, b FloatBits) *stageData {
	f := p.Fmt

	// Special value detection
	if IsNaN(a) || IsNaN(b) {
		nan := makeNaN(f)
		return &stageData{special: &nan}
	}
	aInf, bInf := IsInf(a), IsInf(b)
	if aInf && bInf {
		if a.Sign == b.Sign {
			inf := makeInf(a.Sign, f)
			return &stageData{special: &inf}
		}
		nan := makeNaN(f)
		return &stageData{special: &nan}
	}
	if aInf {
		return &stageData{special: &a}
	}
	if bInf {
		return &stageData{special: &b}
	}

	aZero, bZero := IsZero(a), IsZero(b)
	if aZero && bZero {
		z := makeZero(a.Sign&b.Sign, f)
		return &stageData{special: &z}
	}
	if aZero {
		return &stageData{special: &b}
	}
	if bZero {
		return &stageData{special: &a}
	}

	// Normal extraction
	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)

	if expA != 0 {
		mantA = (1 << f.MantissaBits) | mantA
	} else {
		expA = 1
	}
	if expB != 0 {
		mantB = (1 << f.MantissaBits) | mantB
	} else {
		expB = 1
	}

	guardBits := 3
	mantA <<= guardBits
	mantB <<= guardBits

	return &stageData{
		signA: a.Sign, signB: b.Sign,
		expA: expA, expB: expB,
		mantA: mantA, mantB: mantB,
		guardBits: guardBits,
	}
}

// Stage 1: ALIGN -- shift the smaller mantissa right.
func (p *PipelinedFPAdder) adderStageAlign(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	expA, expB := data.expA, data.expB
	mantA, mantB := data.mantA, data.mantB
	guardBits := data.guardBits

	var resultExp int
	if expA >= expB {
		expDiff := expA - expB
		if expDiff > 0 {
			if expDiff < (f.MantissaBits + 1 + guardBits) {
				shiftedOut := mantB & ((1 << expDiff) - 1)
				mantB >>= expDiff
				if shiftedOut != 0 {
					mantB |= 1
				}
			} else {
				sticky := 0
				if mantB != 0 {
					sticky = 1
				}
				mantB >>= expDiff
				if sticky != 0 {
					mantB |= 1
				}
			}
		}
		resultExp = expA
	} else {
		expDiff := expB - expA
		if expDiff > 0 {
			if expDiff < (f.MantissaBits + 1 + guardBits) {
				shiftedOut := mantA & ((1 << expDiff) - 1)
				mantA >>= expDiff
				if shiftedOut != 0 {
					mantA |= 1
				}
			} else {
				sticky := 0
				if mantA != 0 {
					sticky = 1
				}
				mantA >>= expDiff
				if sticky != 0 {
					mantA |= 1
				}
			}
		}
		resultExp = expB
	}

	return &stageData{
		signA: data.signA, signB: data.signB,
		mantA: mantA, mantB: mantB,
		resultExp: resultExp, guardBits: guardBits,
	}
}

// Stage 2: ADD/SUB -- add or subtract aligned mantissas.
func (p *PipelinedFPAdder) adderStageAdd(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	var resultMant, resultSign int
	if data.signA == data.signB {
		resultMant = data.mantA + data.mantB
		resultSign = data.signA
	} else {
		if data.mantA >= data.mantB {
			resultMant = data.mantA - data.mantB
			resultSign = data.signA
		} else {
			resultMant = data.mantB - data.mantA
			resultSign = data.signB
		}
	}

	if resultMant == 0 {
		z := makeZero(0, p.Fmt)
		return &stageData{special: &z}
	}

	return &stageData{
		resultSign: resultSign, resultMant: resultMant,
		resultExp: data.resultExp, guardBits: data.guardBits,
	}
}

// Stage 3: NORMALIZE -- shift result to correct position.
func (p *PipelinedFPAdder) adderStageNormalize(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	resultMant := data.resultMant
	resultExp := data.resultExp
	guardBits := data.guardBits
	normalPos := f.MantissaBits + guardBits
	leadingPos := bitLength(resultMant) - 1

	if leadingPos > normalPos {
		shiftAmount := leadingPos - normalPos
		lostBits := resultMant & ((1 << shiftAmount) - 1)
		resultMant >>= shiftAmount
		if lostBits != 0 {
			resultMant |= 1
		}
		resultExp += shiftAmount
	} else if leadingPos < normalPos {
		shiftAmount := normalPos - leadingPos
		if resultExp-shiftAmount >= 1 {
			resultMant <<= shiftAmount
			resultExp -= shiftAmount
		} else {
			actualShift := resultExp - 1
			if actualShift > 0 {
				resultMant <<= actualShift
			}
			resultExp = 0
		}
	}

	return &stageData{
		resultSign: data.resultSign, resultMant: resultMant,
		resultExp: resultExp, guardBits: guardBits,
	}
}

// Stage 4: ROUND & PACK -- apply rounding and produce FloatBits.
func (p *PipelinedFPAdder) adderStageRoundPack(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	resultMant := data.resultMant
	resultExp := data.resultExp
	resultSign := data.resultSign
	guardBits := data.guardBits

	guard := (resultMant >> (guardBits - 1)) & 1
	roundBit := (resultMant >> (guardBits - 2)) & 1
	stickyBit := resultMant & ((1 << (guardBits - 2)) - 1)
	if stickyBit != 0 {
		stickyBit = 1
	}

	resultMant >>= guardBits

	if guard == 1 {
		if roundBit == 1 || stickyBit == 1 {
			resultMant++
		} else if (resultMant & 1) == 1 {
			resultMant++
		}
	}

	if resultMant >= (1 << (f.MantissaBits + 1)) {
		resultMant >>= 1
		resultExp++
	}

	maxExp := (1 << f.ExponentBits) - 1
	if resultExp >= maxExp {
		inf := makeInf(resultSign, f)
		return &stageData{special: &inf}
	}
	if resultExp <= 0 {
		if resultExp < -(f.MantissaBits) {
			z := makeZero(resultSign, f)
			return &stageData{special: &z}
		}
		shift := 1 - resultExp
		resultMant >>= shift
		resultExp = 0
	}

	if resultExp > 0 {
		resultMant &= (1 << f.MantissaBits) - 1
	}

	result := FloatBits{
		Sign:     resultSign,
		Exponent: IntToBitsMSB(resultExp, f.ExponentBits),
		Mantissa: IntToBitsMSB(resultMant, f.MantissaBits),
		Fmt:      f,
	}
	return &stageData{special: &result}
}

// =========================================================================
// PipelinedFPMultiplier -- 4-stage pipelined floating-point multiplier
// =========================================================================

// PipelinedFPMultiplier is a 4-stage pipelined floating-point multiplier.
//
// Multiplication is simpler than addition because there's no alignment step.
//
//	Stage 1: UNPACK + SIGN + EXPONENT
//	Stage 2: MULTIPLY MANTISSAS
//	Stage 3: NORMALIZE
//	Stage 4: ROUND & PACK
type PipelinedFPMultiplier struct {
	Clk        *clock.Clock
	Fmt        FloatFormat
	Results    []*FloatBits
	CycleCount int

	mu            sync.Mutex
	stages        [4]*stageData
	inputsPending []addInput
}

// NewPipelinedFPMultiplier creates a new 4-stage pipelined multiplier.
func NewPipelinedFPMultiplier(clk *clock.Clock, fmt FloatFormat) *PipelinedFPMultiplier {
	p := &PipelinedFPMultiplier{
		Clk: clk,
		Fmt: fmt,
	}
	clk.RegisterListener(p.onClockEdge)
	return p
}

// Submit queues a new multiplication (a * b).
func (p *PipelinedFPMultiplier) Submit(a, b FloatBits) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.inputsPending = append(p.inputsPending, addInput{a, b})
}

func (p *PipelinedFPMultiplier) onClockEdge(edge clock.ClockEdge) {
	if !edge.IsRising {
		return
	}

	p.mu.Lock()
	defer p.mu.Unlock()

	p.CycleCount++

	for i := 3; i > 0; i-- {
		p.stages[i] = p.mulProcessStage(i, p.stages[i-1])
	}

	if len(p.inputsPending) > 0 {
		inp := p.inputsPending[0]
		p.inputsPending = p.inputsPending[1:]
		p.stages[0] = p.mulStageUnpackExp(inp.a, inp.b)
	} else {
		p.stages[0] = nil
	}

	if p.stages[3] != nil {
		if p.stages[3].special != nil {
			result := *p.stages[3].special
			p.Results = append(p.Results, &result)
		}
		p.stages[3] = nil
	}
}

func (p *PipelinedFPMultiplier) mulProcessStage(stageNum int, input *stageData) *stageData {
	if input == nil {
		return nil
	}
	switch stageNum {
	case 1:
		return p.mulStageMultiply(input)
	case 2:
		return p.mulStageNormalize(input)
	case 3:
		return p.mulStageRoundPack(input)
	}
	return nil
}

// Stage 0: UNPACK + SIGN + EXPONENT
func (p *PipelinedFPMultiplier) mulStageUnpackExp(a, b FloatBits) *stageData {
	f := p.Fmt
	resultSign := a.Sign ^ b.Sign

	if IsNaN(a) || IsNaN(b) {
		nan := makeNaN(f)
		return &stageData{special: &nan}
	}

	aInf, bInf := IsInf(a), IsInf(b)
	aZero, bZero := IsZero(a), IsZero(b)

	if (aInf && bZero) || (bInf && aZero) {
		nan := makeNaN(f)
		return &stageData{special: &nan}
	}
	if aInf || bInf {
		inf := makeInf(resultSign, f)
		return &stageData{special: &inf}
	}
	if aZero || bZero {
		z := makeZero(resultSign, f)
		return &stageData{special: &z}
	}

	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)

	if expA != 0 {
		mantA = (1 << f.MantissaBits) | mantA
	} else {
		expA = 1
	}
	if expB != 0 {
		mantB = (1 << f.MantissaBits) | mantB
	} else {
		expB = 1
	}

	return &stageData{
		resultSign: resultSign,
		resultExp:  expA + expB - f.Bias,
		mantA:      mantA,
		mantB:      mantB,
	}
}

// Stage 1: MULTIPLY MANTISSAS
func (p *PipelinedFPMultiplier) mulStageMultiply(data *stageData) *stageData {
	if data.special != nil {
		return data
	}
	return &stageData{
		resultSign: data.resultSign,
		resultExp:  data.resultExp,
		product:    data.mantA * data.mantB,
	}
}

// Stage 2: NORMALIZE
func (p *PipelinedFPMultiplier) mulStageNormalize(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	product := data.product
	resultExp := data.resultExp

	productLeading := bitLength(product) - 1
	normalPos := 2 * f.MantissaBits

	if productLeading > normalPos {
		resultExp += productLeading - normalPos
	} else if productLeading < normalPos {
		resultExp -= normalPos - productLeading
	}

	return &stageData{
		resultSign:     data.resultSign,
		resultExp:      resultExp,
		product:        product,
		productLeading: productLeading,
	}
}

// Stage 3: ROUND & PACK
func (p *PipelinedFPMultiplier) mulStageRoundPack(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	resultSign := data.resultSign
	resultExp := data.resultExp
	product := data.product
	productLeading := data.productLeading

	roundPos := productLeading - f.MantissaBits

	var resultMant int
	if roundPos > 0 {
		guard := (product >> (roundPos - 1)) & 1
		var roundBit, sticky int
		if roundPos >= 2 {
			roundBit = (product >> (roundPos - 2)) & 1
			if product&((1<<(roundPos-2))-1) != 0 {
				sticky = 1
			}
		}
		resultMant = product >> roundPos
		if guard == 1 {
			if roundBit == 1 || sticky == 1 {
				resultMant++
			} else if (resultMant & 1) == 1 {
				resultMant++
			}
		}
		if resultMant >= (1 << (f.MantissaBits + 1)) {
			resultMant >>= 1
			resultExp++
		}
	} else if roundPos == 0 {
		resultMant = product
	} else {
		resultMant = product << (-roundPos)
	}

	maxExp := (1 << f.ExponentBits) - 1
	if resultExp >= maxExp {
		inf := makeInf(resultSign, f)
		return &stageData{special: &inf}
	}
	if resultExp <= 0 {
		if resultExp < -(f.MantissaBits) {
			z := makeZero(resultSign, f)
			return &stageData{special: &z}
		}
		shift := 1 - resultExp
		resultMant >>= shift
		resultExp = 0
	}
	if resultExp > 0 {
		resultMant &= (1 << f.MantissaBits) - 1
	}

	result := FloatBits{
		Sign:     resultSign,
		Exponent: IntToBitsMSB(resultExp, f.ExponentBits),
		Mantissa: IntToBitsMSB(resultMant, f.MantissaBits),
		Fmt:      f,
	}
	return &stageData{special: &result}
}

// =========================================================================
// PipelinedFMA -- 6-stage pipelined fused multiply-add
// =========================================================================

// PipelinedFMA is a 6-stage pipelined fused multiply-add unit.
//
// FMA computes a * b + c with a single rounding step. It's the most
// important operation in machine learning because the dot product is
// just a chain of FMAs.
//
//	Stage 1: UNPACK all three operands
//	Stage 2: MULTIPLY a * b mantissas (full precision!)
//	Stage 3: ALIGN product with c
//	Stage 4: ADD product + c
//	Stage 5: NORMALIZE
//	Stage 6: ROUND & PACK (single rounding step!)
type PipelinedFMA struct {
	Clk        *clock.Clock
	Fmt        FloatFormat
	Results    []*FloatBits
	CycleCount int

	mu            sync.Mutex
	stages        [6]*stageData
	inputsPending []fmaInput
}

type fmaInput struct {
	a, b, c FloatBits
}

// NewPipelinedFMA creates a new 6-stage pipelined FMA unit.
func NewPipelinedFMA(clk *clock.Clock, fmt FloatFormat) *PipelinedFMA {
	p := &PipelinedFMA{
		Clk: clk,
		Fmt: fmt,
	}
	clk.RegisterListener(p.onClockEdge)
	return p
}

// Submit queues a new FMA operation (a * b + c).
func (p *PipelinedFMA) Submit(a, b, c FloatBits) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.inputsPending = append(p.inputsPending, fmaInput{a, b, c})
}

func (p *PipelinedFMA) onClockEdge(edge clock.ClockEdge) {
	if !edge.IsRising {
		return
	}

	p.mu.Lock()
	defer p.mu.Unlock()

	p.CycleCount++

	for i := 5; i > 0; i-- {
		p.stages[i] = p.fmaProcessStage(i, p.stages[i-1])
	}

	if len(p.inputsPending) > 0 {
		inp := p.inputsPending[0]
		p.inputsPending = p.inputsPending[1:]
		p.stages[0] = p.fmaStageUnpack(inp.a, inp.b, inp.c)
	} else {
		p.stages[0] = nil
	}

	if p.stages[5] != nil {
		if p.stages[5].special != nil {
			result := *p.stages[5].special
			p.Results = append(p.Results, &result)
		}
		p.stages[5] = nil
	}
}

func (p *PipelinedFMA) fmaProcessStage(stageNum int, input *stageData) *stageData {
	if input == nil {
		return nil
	}
	switch stageNum {
	case 1:
		return p.fmaStageMultiply(input)
	case 2:
		return p.fmaStageAlign(input)
	case 3:
		return p.fmaStageAdd(input)
	case 4:
		return p.fmaStageNormalize(input)
	case 5:
		return p.fmaStageRoundPack(input)
	}
	return nil
}

// Stage 0: UNPACK all three operands
func (p *PipelinedFMA) fmaStageUnpack(a, b, c FloatBits) *stageData {
	f := p.Fmt

	if IsNaN(a) || IsNaN(b) || IsNaN(c) {
		nan := makeNaN(f)
		return &stageData{special: &nan}
	}

	aInf, bInf, cInf := IsInf(a), IsInf(b), IsInf(c)
	aZero, bZero := IsZero(a), IsZero(b)
	productSign := a.Sign ^ b.Sign

	if (aInf && bZero) || (bInf && aZero) {
		nan := makeNaN(f)
		return &stageData{special: &nan}
	}
	if aInf || bInf {
		if cInf && productSign != c.Sign {
			nan := makeNaN(f)
			return &stageData{special: &nan}
		}
		inf := makeInf(productSign, f)
		return &stageData{special: &inf}
	}
	if aZero || bZero {
		if IsZero(c) {
			z := makeZero(productSign&c.Sign, f)
			return &stageData{special: &z}
		}
		cCopy := c
		return &stageData{special: &cCopy}
	}
	if cInf {
		cCopy := c
		return &stageData{special: &cCopy}
	}

	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)
	expC := BitsMSBToInt(c.Exponent)
	mantC := BitsMSBToInt(c.Mantissa)

	if expA != 0 {
		mantA = (1 << f.MantissaBits) | mantA
	} else {
		expA = 1
	}
	if expB != 0 {
		mantB = (1 << f.MantissaBits) | mantB
	} else {
		expB = 1
	}
	if expC != 0 {
		mantC = (1 << f.MantissaBits) | mantC
	} else {
		expC = 1
	}

	return &stageData{
		productSign: productSign,
		cSign:       c.Sign,
		expA:        expA,
		expB:        expB,
		mantA:       mantA,
		mantB:       mantB,
		expC:        expC,
		mantC:       mantC,
	}
}

// Stage 1: MULTIPLY a * b (full precision)
func (p *PipelinedFMA) fmaStageMultiply(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	product := data.mantA * data.mantB
	productExp := data.expA + data.expB - f.Bias

	productLeading := bitLength(product) - 1
	normalProductPos := 2 * f.MantissaBits

	if productLeading > normalProductPos {
		productExp += productLeading - normalProductPos
	} else if productLeading < normalProductPos {
		productExp -= normalProductPos - productLeading
	}

	return &stageData{
		productSign:    data.productSign,
		cSign:          data.cSign,
		product:        product,
		productExp:     productExp,
		productLeading: productLeading,
		expC:           data.expC,
		mantC:          data.mantC,
	}
}

// Stage 2: ALIGN product with c
func (p *PipelinedFMA) fmaStageAlign(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	product := data.product
	productExp := data.productExp
	productLeading := data.productLeading
	mantC := data.mantC

	expDiff := productExp - data.expC

	cScaleShift := productLeading - f.MantissaBits
	var cAligned int
	if cScaleShift >= 0 {
		cAligned = mantC << cScaleShift
	} else {
		cAligned = mantC >> (-cScaleShift)
	}

	var resultExp int
	if expDiff >= 0 {
		cAligned >>= expDiff
		resultExp = productExp
	} else {
		product >>= (-expDiff)
		resultExp = data.expC
	}

	return &stageData{
		productSign:    data.productSign,
		cSign:          data.cSign,
		product:        product,
		cAligned:       cAligned,
		resultExp:      resultExp,
		productLeading: productLeading,
	}
}

// Stage 3: ADD product + c
func (p *PipelinedFMA) fmaStageAdd(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	product := data.product
	cAligned := data.cAligned

	var resultMant, resultSign int
	if data.productSign == data.cSign {
		resultMant = product + cAligned
		resultSign = data.productSign
	} else {
		if product >= cAligned {
			resultMant = product - cAligned
			resultSign = data.productSign
		} else {
			resultMant = cAligned - product
			resultSign = data.cSign
		}
	}

	if resultMant == 0 {
		z := makeZero(0, f)
		return &stageData{special: &z}
	}

	return &stageData{
		resultSign:     resultSign,
		resultMant:     resultMant,
		resultExp:      data.resultExp,
		productLeading: data.productLeading,
	}
}

// Stage 4: NORMALIZE
func (p *PipelinedFMA) fmaStageNormalize(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	resultMant := data.resultMant
	resultExp := data.resultExp
	productLeading := data.productLeading
	targetPos := productLeading
	if targetPos < f.MantissaBits {
		targetPos = f.MantissaBits
	}

	resultLeading := bitLength(resultMant) - 1
	if resultLeading > targetPos {
		shift := resultLeading - targetPos
		resultExp += shift
	} else if resultLeading < targetPos {
		shiftNeeded := targetPos - resultLeading
		resultExp -= shiftNeeded
	}

	return &stageData{
		resultSign: data.resultSign,
		resultMant: resultMant,
		resultExp:  resultExp,
	}
}

// Stage 5: ROUND & PACK (single rounding step)
func (p *PipelinedFMA) fmaStageRoundPack(data *stageData) *stageData {
	if data.special != nil {
		return data
	}

	f := p.Fmt
	resultSign := data.resultSign
	resultExp := data.resultExp
	resultMant := data.resultMant

	resultLeading := bitLength(resultMant) - 1
	roundPos := resultLeading - f.MantissaBits

	if roundPos > 0 {
		guard := (resultMant >> (roundPos - 1)) & 1
		var roundBit, sticky int
		if roundPos >= 2 {
			roundBit = (resultMant >> (roundPos - 2)) & 1
			if resultMant&((1<<(roundPos-2))-1) != 0 {
				sticky = 1
			}
		}
		resultMant >>= roundPos
		if guard == 1 {
			if roundBit == 1 || sticky == 1 {
				resultMant++
			} else if (resultMant & 1) == 1 {
				resultMant++
			}
		}
		if resultMant >= (1 << (f.MantissaBits + 1)) {
			resultMant >>= 1
			resultExp++
		}
	} else if roundPos < 0 {
		resultMant <<= (-roundPos)
	}

	maxExp := (1 << f.ExponentBits) - 1
	if resultExp >= maxExp {
		inf := makeInf(resultSign, f)
		return &stageData{special: &inf}
	}
	if resultExp <= 0 {
		if resultExp < -(f.MantissaBits) {
			z := makeZero(resultSign, f)
			return &stageData{special: &z}
		}
		shift := 1 - resultExp
		resultMant >>= shift
		resultExp = 0
	}
	if resultExp > 0 {
		resultMant &= (1 << f.MantissaBits) - 1
	}

	result := FloatBits{
		Sign:     resultSign,
		Exponent: IntToBitsMSB(resultExp, f.ExponentBits),
		Mantissa: IntToBitsMSB(resultMant, f.MantissaBits),
		Fmt:      f,
	}
	return &stageData{special: &result}
}

// =========================================================================
// FPUnit -- a complete floating-point unit with all three pipelines
// =========================================================================

// FPUnit is a complete floating-point unit with pipelined adder, multiplier,
// and FMA. This is what sits inside every GPU core.
//
//	+--------------------------------------------------+
//	|                    FP Unit                        |
//	|                                                  |
//	|   +-----------------------------+                |
//	|   |  Pipelined FP Adder (5)     |                |
//	|   +-----------------------------+                |
//	|                                                  |
//	|   +-----------------------------+                |
//	|   |  Pipelined FP Multiplier (4)|                |
//	|   +-----------------------------+                |
//	|                                                  |
//	|   +-----------------------------+                |
//	|   |  Pipelined FMA Unit (6)     |                |
//	|   +-----------------------------+                |
//	|                                                  |
//	|   All three share the same clock signal          |
//	+--------------------------------------------------+
//
// A modern GPU like the NVIDIA RTX 4090 has 16,384 CUDA cores, each
// containing an FP unit like this. Running at ~2.5 GHz:
//
//	16,384 cores x 2 FLOPs/cycle (FMA) x 2.52 GHz = 82.6 TFLOPS
type FPUnit struct {
	Clk        *clock.Clock
	Fmt        FloatFormat
	Adder      *PipelinedFPAdder
	Multiplier *PipelinedFPMultiplier
	Fma        *PipelinedFMA
}

// NewFPUnit creates a complete floating-point unit with all three pipelines
// sharing the same clock.
func NewFPUnit(clk *clock.Clock, fmt FloatFormat) *FPUnit {
	return &FPUnit{
		Clk:        clk,
		Fmt:        fmt,
		Adder:      NewPipelinedFPAdder(clk, fmt),
		Multiplier: NewPipelinedFPMultiplier(clk, fmt),
		Fma:        NewPipelinedFMA(clk, fmt),
	}
}

// Tick runs the clock for n complete cycles. Each full cycle consists of a
// rising edge (where pipeline stages advance) and a falling edge (idle half).
func (u *FPUnit) Tick(n int) {
	for i := 0; i < n; i++ {
		u.Clk.FullCycle()
	}
}
