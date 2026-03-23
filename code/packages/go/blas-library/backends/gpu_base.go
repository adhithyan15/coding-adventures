// GPU Backend Base -- shared logic for all six GPU-accelerated backends.
//
// # Why a Base Struct for GPU Backends?
//
// All six GPU backends (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL) follow
// the same pattern for every BLAS operation:
//
//  1. Upload input data to device memory via the vendor API
//  2. Compute the result using the CPU reference (CpuBlas)
//  3. Upload the result to device memory, download it back
//  4. Free device memory
//  5. Return new Matrix/Vector objects
//
// Since our device simulators operate synchronously and kernel execution is
// simplified, the GPU backends perform the actual arithmetic on the CPU side
// but still exercise the full GPU memory pipeline (allocate, upload, download).
// This demonstrates the complete GPU programming pattern without requiring
// a full GPU instruction compiler.
//
// # The Template Method Pattern (Go-style)
//
// In Go, we use embedding + interfaces instead of inheritance. The gpuBase
// struct provides all BLAS operations. Each GPU backend embeds gpuBase and
// provides a gpuMemory implementation with three methods:
//
//	upload(data []byte) (handle interface{}, err error)   -- upload bytes to device
//	download(handle interface{}, size int) ([]byte, error) -- download bytes from device
//	free(handle interface{}) error                         -- free device memory
//
// This is the Template Method design pattern adapted to Go's composition model.
package backends

import (
	"encoding/binary"
	"math"

	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
)

// =========================================================================
// gpuMemory -- the interface each GPU backend must implement
// =========================================================================

// gpuMemory defines the three template methods each GPU backend must provide.
//
// These correspond to the fundamental GPU memory operations:
//
//	upload:   CPU -> GPU (host to device)
//	download: GPU -> CPU (device to host)
//	free:     release GPU memory
//
// Every vendor API expresses these differently (cudaMalloc vs MTLBuffer vs
// VkDeviceMemory), but the semantics are identical.
type gpuMemory interface {
	upload(data []byte) (interface{}, error)
	download(handle interface{}, size int) ([]byte, error)
	free(handle interface{}) error
}

// =========================================================================
// gpuBase -- the shared BLAS implementation for all GPU backends
// =========================================================================

// gpuBase provides all BLAS and ML operations by delegating arithmetic to
// CpuBlas and wrapping every call with GPU memory operations.
//
// Each GPU backend struct embeds gpuBase and provides a gpuMemory
// implementation. The result is that every GPU backend produces results
// identical to CpuBlas (correctness guarantee) while fully exercising
// the vendor-specific GPU memory pipeline.
type gpuBase struct {
	cpu *CpuBlas
	mem gpuMemory
}

// newGpuBase creates a new gpuBase with the given memory implementation.
func newGpuBase(mem gpuMemory) gpuBase {
	return gpuBase{
		cpu: &CpuBlas{},
		mem: mem,
	}
}

// =========================================================================
// Serialization helpers -- convert between Go types and byte slices
// =========================================================================

// float32sToBytes converts a slice of float32 values to little-endian bytes.
//
// Each float32 is 4 bytes in IEEE 754 format. We use little-endian because
// that is what x86, ARM, and all major GPU architectures use natively.
//
// Example:
//
//	[]float32{1.0, 2.0} -> 8 bytes (little-endian IEEE 754)
func float32sToBytes(data []float32) []byte {
	buf := make([]byte, len(data)*4)
	for i, v := range data {
		binary.LittleEndian.PutUint32(buf[i*4:], math.Float32bits(v))
	}
	return buf
}

// bytesToFloat32s converts little-endian bytes back to float32 values.
//
// The inverse of float32sToBytes. Used when downloading results from the GPU.
func bytesToFloat32s(data []byte, count int) []float32 {
	result := make([]float32, count)
	for i := 0; i < count; i++ {
		result[i] = math.Float32frombits(binary.LittleEndian.Uint32(data[i*4:]))
	}
	return result
}

// =========================================================================
// GPU round-trip helpers -- exercise the full GPU pipeline
// =========================================================================

// gpuRoundTripVector uploads a vector to GPU, downloads it back.
//
// This exercises the complete GPU memory pipeline:
//
//	1. Serialize vector data to bytes
//	2. Upload bytes to device memory (vendor-specific)
//	3. Download bytes from device memory
//	4. Free device memory
//	5. Deserialize bytes back to float32 values
//
// The result is numerically identical to the input -- the point is to
// exercise the GPU memory pipeline, not to compute anything.
func (g *gpuBase) gpuRoundTripVector(v blas.Vector) (blas.Vector, error) {
	data := float32sToBytes(v.Data)
	handle, err := g.mem.upload(data)
	if err != nil {
		return blas.Vector{}, err
	}
	result, err := g.mem.download(handle, len(data))
	if err != nil {
		return blas.Vector{}, err
	}
	_ = g.mem.free(handle)
	return blas.NewVector(bytesToFloat32s(result, v.Size)), nil
}

// gpuRoundTripMatrix uploads a matrix to GPU, downloads it back.
func (g *gpuBase) gpuRoundTripMatrix(m blas.Matrix) (blas.Matrix, error) {
	data := float32sToBytes(m.Data)
	handle, err := g.mem.upload(data)
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err := g.mem.download(handle, len(data))
	if err != nil {
		return blas.Matrix{}, err
	}
	_ = g.mem.free(handle)
	return blas.Matrix{
		Data:  bytesToFloat32s(result, m.Rows*m.Cols),
		Rows:  m.Rows,
		Cols:  m.Cols,
		Order: m.Order,
	}, nil
}

// =========================================================================
// LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
// =========================================================================

// Saxpy computes y = alpha * x + y via the GPU pipeline.
func (g *gpuBase) Saxpy(alpha float32, x, y blas.Vector) (blas.Vector, error) {
	hx, err := g.mem.upload(float32sToBytes(x.Data))
	if err != nil {
		return blas.Vector{}, err
	}
	hy, err := g.mem.upload(float32sToBytes(y.Data))
	if err != nil {
		return blas.Vector{}, err
	}
	result, err := g.cpu.Saxpy(alpha, x, y)
	if err != nil {
		return blas.Vector{}, err
	}
	result, err = g.gpuRoundTripVector(result)
	if err != nil {
		return blas.Vector{}, err
	}
	_ = g.mem.free(hx)
	_ = g.mem.free(hy)
	return result, nil
}

// Sdot computes the dot product via the GPU pipeline.
func (g *gpuBase) Sdot(x, y blas.Vector) (float32, error) {
	hx, err := g.mem.upload(float32sToBytes(x.Data))
	if err != nil {
		return 0, err
	}
	hy, err := g.mem.upload(float32sToBytes(y.Data))
	if err != nil {
		return 0, err
	}
	result, err := g.cpu.Sdot(x, y)
	_ = g.mem.free(hx)
	_ = g.mem.free(hy)
	return result, err
}

// Snrm2 computes the Euclidean norm via the GPU pipeline.
func (g *gpuBase) Snrm2(x blas.Vector) float32 {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Snrm2(x)
	_ = g.mem.free(hx)
	return result
}

// Sscal computes alpha * x via the GPU pipeline.
func (g *gpuBase) Sscal(alpha float32, x blas.Vector) blas.Vector {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Sscal(alpha, x)
	result, _ = g.gpuRoundTripVector(result)
	_ = g.mem.free(hx)
	return result
}

// Sasum computes the absolute sum via the GPU pipeline.
func (g *gpuBase) Sasum(x blas.Vector) float32 {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Sasum(x)
	_ = g.mem.free(hx)
	return result
}

// Isamax returns the index of the largest absolute value via the GPU pipeline.
func (g *gpuBase) Isamax(x blas.Vector) int {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Isamax(x)
	_ = g.mem.free(hx)
	return result
}

// Scopy deep copies a vector via the GPU pipeline (upload then download).
func (g *gpuBase) Scopy(x blas.Vector) blas.Vector {
	result, _ := g.gpuRoundTripVector(x)
	return result
}

// Sswap exchanges x and y via the GPU pipeline.
func (g *gpuBase) Sswap(x, y blas.Vector) (blas.Vector, blas.Vector, error) {
	hx, err := g.mem.upload(float32sToBytes(x.Data))
	if err != nil {
		return blas.Vector{}, blas.Vector{}, err
	}
	hy, err := g.mem.upload(float32sToBytes(y.Data))
	if err != nil {
		return blas.Vector{}, blas.Vector{}, err
	}
	newX, newY, err := g.cpu.Sswap(x, y)
	if err != nil {
		return blas.Vector{}, blas.Vector{}, err
	}
	_ = g.mem.free(hx)
	_ = g.mem.free(hy)
	newX, err = g.gpuRoundTripVector(newX)
	if err != nil {
		return blas.Vector{}, blas.Vector{}, err
	}
	newY, err = g.gpuRoundTripVector(newY)
	if err != nil {
		return blas.Vector{}, blas.Vector{}, err
	}
	return newX, newY, nil
}

// =========================================================================
// LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
// =========================================================================

// Sgemv computes y = alpha * op(A) * x + beta * y via the GPU pipeline.
func (g *gpuBase) Sgemv(
	trans blas.Transpose, alpha float32, a blas.Matrix, x blas.Vector,
	beta float32, y blas.Vector,
) (blas.Vector, error) {
	ha, err := g.mem.upload(float32sToBytes(a.Data))
	if err != nil {
		return blas.Vector{}, err
	}
	hx, err := g.mem.upload(float32sToBytes(x.Data))
	if err != nil {
		return blas.Vector{}, err
	}
	hy, err := g.mem.upload(float32sToBytes(y.Data))
	if err != nil {
		return blas.Vector{}, err
	}
	result, err := g.cpu.Sgemv(trans, alpha, a, x, beta, y)
	if err != nil {
		return blas.Vector{}, err
	}
	result, err = g.gpuRoundTripVector(result)
	if err != nil {
		return blas.Vector{}, err
	}
	_ = g.mem.free(ha)
	_ = g.mem.free(hx)
	_ = g.mem.free(hy)
	return result, nil
}

// Sger computes A = alpha * x * y^T + A via the GPU pipeline.
func (g *gpuBase) Sger(alpha float32, x, y blas.Vector, a blas.Matrix) (blas.Matrix, error) {
	ha, err := g.mem.upload(float32sToBytes(a.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	hx, err := g.mem.upload(float32sToBytes(x.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	hy, err := g.mem.upload(float32sToBytes(y.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err := g.cpu.Sger(alpha, x, y, a)
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	if err != nil {
		return blas.Matrix{}, err
	}
	_ = g.mem.free(ha)
	_ = g.mem.free(hx)
	_ = g.mem.free(hy)
	return result, nil
}

// =========================================================================
// LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
// =========================================================================

// Sgemm computes C = alpha * op(A) * op(B) + beta * C via the GPU pipeline.
func (g *gpuBase) Sgemm(
	transA, transB blas.Transpose, alpha float32,
	a, b blas.Matrix, beta float32, c blas.Matrix,
) (blas.Matrix, error) {
	ha, err := g.mem.upload(float32sToBytes(a.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	hb, err := g.mem.upload(float32sToBytes(b.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	hc, err := g.mem.upload(float32sToBytes(c.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err := g.cpu.Sgemm(transA, transB, alpha, a, b, beta, c)
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	if err != nil {
		return blas.Matrix{}, err
	}
	_ = g.mem.free(ha)
	_ = g.mem.free(hb)
	_ = g.mem.free(hc)
	return result, nil
}

// Ssymm computes symmetric matrix multiply via the GPU pipeline.
func (g *gpuBase) Ssymm(
	side blas.Side, alpha float32,
	a, b blas.Matrix, beta float32, c blas.Matrix,
) (blas.Matrix, error) {
	ha, err := g.mem.upload(float32sToBytes(a.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	hb, err := g.mem.upload(float32sToBytes(b.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	hc, err := g.mem.upload(float32sToBytes(c.Data))
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err := g.cpu.Ssymm(side, alpha, a, b, beta, c)
	if err != nil {
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	if err != nil {
		return blas.Matrix{}, err
	}
	_ = g.mem.free(ha)
	_ = g.mem.free(hb)
	_ = g.mem.free(hc)
	return result, nil
}

// SgemmBatched computes multiple independent GEMMs via the GPU pipeline.
func (g *gpuBase) SgemmBatched(
	transA, transB blas.Transpose, alpha float32,
	aList, bList []blas.Matrix, beta float32,
	cList []blas.Matrix,
) ([]blas.Matrix, error) {
	results := make([]blas.Matrix, len(aList))
	for i := range aList {
		r, err := g.Sgemm(transA, transB, alpha, aList[i], bList[i], beta, cList[i])
		if err != nil {
			return nil, err
		}
		results[i] = r
	}
	return results, nil
}

// =========================================================================
// ML EXTENSIONS: Activation Functions
// =========================================================================

// Relu computes ReLU via the GPU pipeline.
func (g *gpuBase) Relu(x blas.Matrix) blas.Matrix {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Relu(x)
	result, _ = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result
}

// Gelu computes GELU via the GPU pipeline.
func (g *gpuBase) Gelu(x blas.Matrix) blas.Matrix {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Gelu(x)
	result, _ = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result
}

// Sigmoid computes sigmoid via the GPU pipeline.
func (g *gpuBase) Sigmoid(x blas.Matrix) blas.Matrix {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Sigmoid(x)
	result, _ = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result
}

// TanhActivation computes tanh via the GPU pipeline.
func (g *gpuBase) TanhActivation(x blas.Matrix) blas.Matrix {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.TanhActivation(x)
	result, _ = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result
}

// Softmax computes softmax via the GPU pipeline.
func (g *gpuBase) Softmax(x blas.Matrix, axis int) blas.Matrix {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result := g.cpu.Softmax(x, axis)
	result, _ = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result
}

// LayerNorm computes layer normalization via the GPU pipeline.
func (g *gpuBase) LayerNorm(
	x blas.Matrix, gamma, beta blas.Vector, eps float32,
) (blas.Matrix, error) {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result, err := g.cpu.LayerNorm(x, gamma, beta, eps)
	if err != nil {
		_ = g.mem.free(hx)
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result, err
}

// BatchNorm computes batch normalization via the GPU pipeline.
func (g *gpuBase) BatchNorm(
	x blas.Matrix, gamma, beta, runningMean, runningVar blas.Vector,
	eps float32, training bool,
) (blas.Matrix, error) {
	hx, _ := g.mem.upload(float32sToBytes(x.Data))
	result, err := g.cpu.BatchNorm(x, gamma, beta, runningMean, runningVar, eps, training)
	if err != nil {
		_ = g.mem.free(hx)
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hx)
	return result, err
}

// Conv2d computes 2D convolution via the GPU pipeline.
func (g *gpuBase) Conv2d(
	input, weight blas.Matrix, bias *blas.Vector, stride, padding int,
) (blas.Matrix, error) {
	hi, _ := g.mem.upload(float32sToBytes(input.Data))
	hw, _ := g.mem.upload(float32sToBytes(weight.Data))
	result, err := g.cpu.Conv2d(input, weight, bias, stride, padding)
	if err != nil {
		_ = g.mem.free(hi)
		_ = g.mem.free(hw)
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hi)
	_ = g.mem.free(hw)
	return result, err
}

// Attention computes scaled dot-product attention via the GPU pipeline.
func (g *gpuBase) Attention(
	q, k, v blas.Matrix, mask *blas.Matrix, scale *float32,
) (blas.Matrix, error) {
	hq, _ := g.mem.upload(float32sToBytes(q.Data))
	hk, _ := g.mem.upload(float32sToBytes(k.Data))
	hv, _ := g.mem.upload(float32sToBytes(v.Data))
	result, err := g.cpu.Attention(q, k, v, mask, scale)
	if err != nil {
		_ = g.mem.free(hq)
		_ = g.mem.free(hk)
		_ = g.mem.free(hv)
		return blas.Matrix{}, err
	}
	result, err = g.gpuRoundTripMatrix(result)
	_ = g.mem.free(hq)
	_ = g.mem.free(hk)
	_ = g.mem.free(hv)
	return result, err
}
