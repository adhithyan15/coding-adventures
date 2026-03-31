-- Tests for coding_adventures.wave
--
-- This test suite covers all exported functions of the wave module:
--   sine_wave, cosine_wave, square_wave, sawtooth_wave, triangle_wave,
--   dc_offset, add_waves, scale_wave, mix_waves
--
-- Test strategy:
--   - Verify basic output shapes (correct length, type)
--   - Spot-check key sample values against known analytical results
--   - Test edge cases (zero amplitude, zero samples, error conditions)
--   - Verify mathematical relationships (e.g., sine² + cos² = amplitude²)

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local wave = require("coding_adventures.wave")

-- ---------------------------------------------------------------------------
-- Helper: approximate equality for floating-point comparisons
-- We use an absolute tolerance of 1e-9, which is tight enough to catch bugs
-- but loose enough to tolerate accumulated floating-point rounding.
-- ---------------------------------------------------------------------------
local EPSILON = 1e-9

local function approx_eq(a, b, tol)
    tol = tol or EPSILON
    return math.abs(a - b) <= tol
end

describe("wave", function()

    -- -----------------------------------------------------------------------
    -- Meta / version
    -- -----------------------------------------------------------------------

    it("has VERSION 0.1.0", function()
        assert.equals("0.1.0", wave.VERSION)
    end)

    it("exports TWO_PI constant", function()
        assert.is_not_nil(wave.TWO_PI)
        assert.is_true(approx_eq(wave.TWO_PI, 6.283185307179586, 1e-12))
    end)

    -- -----------------------------------------------------------------------
    -- sine_wave — basic checks
    -- -----------------------------------------------------------------------

    describe("sine_wave", function()

        it("returns correct number of samples", function()
            local s = wave.sine_wave(1.0, 1.0, 0, 8, 8)
            assert.equals(8, #s)
        end)

        it("starts near zero for phase=0", function()
            -- sin(0) = 0
            local s = wave.sine_wave(440, 1.0, 0, 44100, 1)
            assert.is_true(approx_eq(s[1], 0.0, 1e-9))
        end)

        it("reaches maximum at quarter-period", function()
            -- For frequency=1, sample_rate=4: sample index 1 (i=1) is at t=0.25s
            -- angle = 2*pi*1*0.25 = pi/2 → sin(pi/2) = 1.0
            local s = wave.sine_wave(1.0, 1.0, 0, 4, 5)
            assert.is_true(approx_eq(s[2], 1.0, 1e-9), "sample[2] should be ~1.0")
        end)

        it("returns near zero at half-period", function()
            -- At t=0.5, angle = 2*pi*1*0.5 = pi → sin(pi) ≈ 0
            local s = wave.sine_wave(1.0, 1.0, 0, 4, 5)
            assert.is_true(approx_eq(s[3], 0.0, 1e-9), "sample[3] should be ~0")
        end)

        it("reaches minimum at three-quarter period", function()
            -- At t=0.75, angle = 2*pi*1*0.75 = 3*pi/2 → sin(3*pi/2) = -1.0
            local s = wave.sine_wave(1.0, 1.0, 0, 4, 5)
            assert.is_true(approx_eq(s[4], -1.0, 1e-9), "sample[4] should be ~-1.0")
        end)

        it("scales by amplitude", function()
            local s = wave.sine_wave(1.0, 3.5, 0, 4, 3)
            -- sample[2] is quarter-period → 3.5 * 1 = 3.5
            assert.is_true(approx_eq(s[2], 3.5, 1e-9))
        end)

        it("phase offset shifts wave", function()
            -- With phase = pi/2 (quarter turn), sine becomes cosine
            local trig = require("coding_adventures.trig")
            local phase = trig.PI / 2
            local s = wave.sine_wave(1.0, 1.0, phase, 4, 2)
            -- sample[1] at t=0: sin(0 + pi/2) = cos(0) = 1.0
            assert.is_true(approx_eq(s[1], 1.0, 1e-9))
        end)

        it("pythagorean identity: sin² + cos² = amplitude²", function()
            -- For any sample, sine_wave[i]² + cosine_wave[i]² ≈ amplitude²
            local amp = 2.5
            local sr  = 44100
            local n   = 100
            local sv  = wave.sine_wave(440, amp, 0, sr, n)
            local cv  = wave.cosine_wave(440, amp, 0, sr, n)
            for i = 1, n do
                local identity = sv[i] * sv[i] + cv[i] * cv[i]
                assert.is_true(approx_eq(identity, amp * amp, 1e-8),
                    string.format("Pythagorean identity failed at sample %d: %g", i, identity))
            end
        end)

        it("returns empty array for zero samples", function()
            local s = wave.sine_wave(440, 1.0, 0, 44100, 0)
            assert.equals(0, #s)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- cosine_wave
    -- -----------------------------------------------------------------------

    describe("cosine_wave", function()

        it("returns correct number of samples", function()
            local s = wave.cosine_wave(1.0, 1.0, 0, 8, 8)
            assert.equals(8, #s)
        end)

        it("starts at amplitude for phase=0 (cos(0) = 1)", function()
            local amp = 1.5
            local s = wave.cosine_wave(440, amp, 0, 44100, 1)
            assert.is_true(approx_eq(s[1], amp, 1e-9))
        end)

        it("is zero at quarter-period", function()
            -- At t=0.25 (quarter period for f=1, sr=4): cos(pi/2) = 0
            local s = wave.cosine_wave(1.0, 1.0, 0, 4, 5)
            assert.is_true(approx_eq(s[2], 0.0, 1e-9))
        end)

        it("is -amplitude at half-period", function()
            -- cos(pi) = -1
            local s = wave.cosine_wave(1.0, 1.0, 0, 4, 5)
            assert.is_true(approx_eq(s[3], -1.0, 1e-9))
        end)

        it("scales by amplitude", function()
            local amp = 4.2
            local s = wave.cosine_wave(1.0, amp, 0, 44100, 1)
            assert.is_true(approx_eq(s[1], amp, 1e-9))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- square_wave
    -- -----------------------------------------------------------------------

    describe("square_wave", function()

        it("returns correct number of samples", function()
            local s = wave.square_wave(1.0, 1.0, 8, 8)
            assert.equals(8, #s)
        end)

        it("all samples are exactly ±amplitude", function()
            local amp = 2.0
            local s = wave.square_wave(440, amp, 44100, 1000)
            for i = 1, #s do
                local v = s[i]
                assert.is_true(v == amp or v == -amp,
                    string.format("sample %d = %g is not ±%g", i, v, amp))
            end
        end)

        it("first sample is positive (starts in positive half-cycle)", function()
            -- At t=0, sin(0) = 0 which is >= 0, so we expect +amplitude
            local amp = 1.0
            local s = wave.square_wave(1.0, amp, 4, 4)
            assert.equals(amp, s[1])
        end)

        it("switches sign at half-period", function()
            -- f=1, sr=4: half-period is at index 3 (t=0.5, angle=pi → sin<0)
            local amp = 1.0
            local s = wave.square_wave(1.0, amp, 4, 4)
            -- index 1,2: positive; index 3,4: negative
            assert.equals(amp, s[1])
            assert.equals(amp, s[2])
            assert.equals(-amp, s[3])
            assert.equals(-amp, s[4])
        end)

    end)

    -- -----------------------------------------------------------------------
    -- sawtooth_wave
    -- -----------------------------------------------------------------------

    describe("sawtooth_wave", function()

        it("returns correct number of samples", function()
            local s = wave.sawtooth_wave(1.0, 1.0, 8, 8)
            assert.equals(8, #s)
        end)

        it("all samples are within [-amplitude, +amplitude]", function()
            local amp = 1.5
            local s   = wave.sawtooth_wave(440, amp, 44100, 1000)
            for i = 1, #s do
                assert.is_true(s[i] >= -amp - 1e-9 and s[i] <= amp + 1e-9,
                    string.format("sample %d = %g out of range [-%g, %g]", i, s[i], amp, amp))
            end
        end)

        it("at t=0, value is 0", function()
            -- floor(0 * f + 0.5) = 0; 2*amp*(0 - 0) = 0
            local s = wave.sawtooth_wave(1.0, 1.0, 100, 1)
            assert.is_true(approx_eq(s[1], 0.0, 1e-9))
        end)

        it("scales with amplitude", function()
            local amp = 3.0
            local s   = wave.sawtooth_wave(440, amp, 44100, 100)
            for i = 1, #s do
                assert.is_true(s[i] >= -amp - 1e-9 and s[i] <= amp + 1e-9)
            end
        end)

    end)

    -- -----------------------------------------------------------------------
    -- triangle_wave
    -- -----------------------------------------------------------------------

    describe("triangle_wave", function()

        it("returns correct number of samples", function()
            local s = wave.triangle_wave(1.0, 1.0, 8, 8)
            assert.equals(8, #s)
        end)

        it("all samples are within [-amplitude, +amplitude]", function()
            local amp = 2.0
            local s   = wave.triangle_wave(440, amp, 44100, 1000)
            for i = 1, #s do
                assert.is_true(s[i] >= -amp - 1e-9 and s[i] <= amp + 1e-9,
                    string.format("sample %d = %g out of range", i, s[i]))
            end
        end)

        it("at t=0, value is -amplitude (starts at trough)", function()
            -- phase_frac(0) = 0 - floor(0.5) = 0 - 0 = 0
            -- triangle = amp * (2*(2*|0|) - 1) = amp * (0 - 1) = -amp
            local amp = 1.0
            local s   = wave.triangle_wave(1.0, amp, 100, 1)
            assert.is_true(approx_eq(s[1], -amp, 1e-9))
        end)

        it("is symmetric about zero over full period", function()
            -- Sum of samples over exactly one period averages to 0 for a
            -- symmetric triangle wave. Use many samples to reduce edge effects.
            local sr  = 1000
            local f   = 5
            local n   = sr  -- exactly 1 second = 5 full periods
            local amp = 1.0
            local s   = wave.triangle_wave(f, amp, sr, n)
            local sum = 0
            for i = 1, n do sum = sum + s[i] end
            -- Mean should be close to zero
            assert.is_true(math.abs(sum / n) < 0.01, "triangle wave should be symmetric")
        end)

    end)

    -- -----------------------------------------------------------------------
    -- dc_offset
    -- -----------------------------------------------------------------------

    describe("dc_offset", function()

        it("returns correct number of samples", function()
            local s = wave.dc_offset(5.0, 10)
            assert.equals(10, #s)
        end)

        it("all samples equal the given value", function()
            local val = 3.14
            local s   = wave.dc_offset(val, 20)
            for i = 1, 20 do
                assert.equals(val, s[i])
            end
        end)

        it("works with zero value", function()
            local s = wave.dc_offset(0, 5)
            for i = 1, 5 do assert.equals(0, s[i]) end
        end)

        it("works with negative value", function()
            local s = wave.dc_offset(-2.5, 3)
            for i = 1, 3 do assert.equals(-2.5, s[i]) end
        end)

        it("returns empty array for zero samples", function()
            local s = wave.dc_offset(1.0, 0)
            assert.equals(0, #s)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- add_waves
    -- -----------------------------------------------------------------------

    describe("add_waves", function()

        it("sums two waves element-by-element", function()
            local a = {1, 2, 3, 4}
            local b = {5, 6, 7, 8}
            local r = wave.add_waves(a, b)
            assert.same({6, 8, 10, 12}, r)
        end)

        it("returns same length as inputs", function()
            local a = {0.1, 0.2, 0.3}
            local b = {0.4, 0.5, 0.6}
            assert.equals(3, #wave.add_waves(a, b))
        end)

        it("works with negative values", function()
            local a = {1.0, -1.0}
            local b = {-1.0, 1.0}
            local r = wave.add_waves(a, b)
            assert.is_true(approx_eq(r[1], 0.0))
            assert.is_true(approx_eq(r[2], 0.0))
        end)

        it("superposition: adding in-phase doubles amplitude", function()
            local n  = 50
            local sr = 44100
            local s1 = wave.sine_wave(440, 1.0, 0, sr, n)
            local s2 = wave.sine_wave(440, 1.0, 0, sr, n)
            local r  = wave.add_waves(s1, s2)
            -- Each sample should equal 2x the original
            for i = 1, n do
                assert.is_true(approx_eq(r[i], 2 * s1[i], 1e-9))
            end
        end)

        it("superposition: adding out-of-phase cancels (destructive interference)", function()
            local trig_mod = require("coding_adventures.trig")
            local n   = 50
            local sr  = 44100
            local s1  = wave.sine_wave(440, 1.0, 0, sr, n)
            local s2  = wave.sine_wave(440, 1.0, trig_mod.PI, sr, n)  -- 180° phase shift
            local r   = wave.add_waves(s1, s2)
            for i = 1, n do
                assert.is_true(approx_eq(r[i], 0.0, 1e-8),
                    string.format("sample %d: %g should be ~0", i, r[i]))
            end
        end)

        it("errors when waves have different lengths", function()
            local a = {1, 2, 3}
            local b = {1, 2}
            assert.has_error(function() wave.add_waves(a, b) end)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- scale_wave
    -- -----------------------------------------------------------------------

    describe("scale_wave", function()

        it("multiplies each sample by scalar", function()
            local s = {1, 2, 3, 4, 5}
            local r = wave.scale_wave(s, 2)
            assert.same({2, 4, 6, 8, 10}, r)
        end)

        it("returns same length as input", function()
            local s = {1.0, 2.0, 3.0}
            assert.equals(3, #wave.scale_wave(s, 1.0))
        end)

        it("scaling by 0 gives all zeros", function()
            local s = wave.sine_wave(440, 1.0, 0, 44100, 10)
            local r = wave.scale_wave(s, 0)
            for i = 1, #r do
                assert.equals(0, r[i])
            end
        end)

        it("scaling by -1 inverts the wave", function()
            local s = wave.sine_wave(440, 1.0, 0, 44100, 20)
            local r = wave.scale_wave(s, -1)
            for i = 1, #s do
                assert.is_true(approx_eq(r[i], -s[i], 1e-15))
            end
        end)

        it("scaling by 1 leaves wave unchanged", function()
            local s = wave.sine_wave(440, 2.0, 0, 44100, 20)
            local r = wave.scale_wave(s, 1)
            for i = 1, #s do
                assert.equals(s[i], r[i])
            end
        end)

    end)

    -- -----------------------------------------------------------------------
    -- mix_waves
    -- -----------------------------------------------------------------------

    describe("mix_waves", function()

        it("mixing one wave returns that wave", function()
            local s = wave.sine_wave(440, 1.0, 0, 44100, 10)
            local m = wave.mix_waves({s})
            for i = 1, #s do assert.equals(s[i], m[i]) end
        end)

        it("mixing two identical waves doubles amplitude", function()
            local s = wave.sine_wave(440, 1.0, 0, 44100, 10)
            local m = wave.mix_waves({s, s})
            for i = 1, #s do
                assert.is_true(approx_eq(m[i], 2 * s[i], 1e-15))
            end
        end)

        it("mixing three waves sums all", function()
            local a = {1, 2, 3}
            local b = {4, 5, 6}
            local c = {7, 8, 9}
            local r = wave.mix_waves({a, b, c})
            assert.same({12, 15, 18}, r)
        end)

        it("errors on empty wave list", function()
            assert.has_error(function() wave.mix_waves({}) end)
        end)

        it("errors when waves have different lengths", function()
            local a = {1, 2, 3}
            local b = {1, 2}
            assert.has_error(function() wave.mix_waves({a, b}) end)
        end)

        it("mixing with dc_offset shifts the wave", function()
            local s   = wave.sine_wave(440, 1.0, 0, 44100, 10)
            local dc  = wave.dc_offset(0.5, 10)
            local mix = wave.mix_waves({s, dc})
            for i = 1, 10 do
                assert.is_true(approx_eq(mix[i], s[i] + 0.5, 1e-15))
            end
        end)

    end)

end)
