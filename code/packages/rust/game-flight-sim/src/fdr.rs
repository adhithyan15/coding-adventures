//! # Black-Box Flight Data Recorder (FDR) & Deterministic Replay
//!
//! Provides a caller-owned, bounded-capacity [`BlackBoxRecorder`] with
//! zero-allocation steady-state frame recording, a hardened binary codec
//! (`encode_black_box_into` / `decode_black_box_from_bytes` / `decode_black_box_into`),
//! and a bit-exact replay verifier ([`replay_black_box_verify`]).
//!
//! ## Wire Format (`GFDR` v1)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ Header (80 bytes)                                                   │
//! │   magic[4]="GFDR" | version:u32 | rng_seed:u64 | terrain_seed:u64   │
//! │   preset:u8 | mode:u8 | reserved[2]=0 | fixed_dt_ns:u64             │
//! │   init_alt_m:f64 | init_ias_mps:f64 | init_fuel_kg:f64 | gust:f64   │
//! │   initial_digest_fnv1a:u64 | frame_count:u32                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ Frames (frame_count * 152 bytes)                                    │
//! │   tick:u64 | dt_ns:u64 | elevator:f64 | aileron:f64 | rudder:f64    │
//! │   throttle:f64 | flaps:f64 | brake:f64 | gear:u8 | phase:u8         │
//! │   flags:u8 | pad[5]=0 | pitch:f64 | roll:f64 | heading:f64          │
//! │   ias:f64 | msl:f64 | agl:f64 | vsi:f64 | g_load:f64 | fuel:f64     │
//! │   state_digest_fnv1a:u64                                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ Trailer (8 bytes): FNV-1a 64-bit checksum over Header + Frames      │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use cockpit_hud_paint::{
    AnnunciatorFlags, MAX_AIRSPEED_MPS, MAX_G_LOAD, MAX_RADAR_ALTITUDE_M, MAX_VERTICAL_SPEED_MPS,
};
use terrain_lod::{Heightfield, RunwaySpec};

use crate::validate::{
    canon_zero, check_finite, check_range, fnv1a_mix_bytes, fnv1a_mix_f64, fnv1a_mix_u64,
    validate_dt_ns, validate_fdr_blob_bytes, validate_fdr_frame_count, validate_input_frame,
    FNV_OFFSET,
};
use crate::{
    sim_state_validate, state_digest_fnv1a, step_in_place, AircraftPresetKind, FlightInputFrame,
    FlightPhase, FlightSimError, FlightSimState, InitialFlightMode, BLACK_BOX_FRAME_BYTES,
    BLACK_BOX_HEADER_BYTES, BLACK_BOX_MAGIC, BLACK_BOX_TRAILER_BYTES, BLACK_BOX_VERSION,
    MAX_ALTITUDE_ENVELOPE_M, MAX_BLACK_BOX_BYTES, MAX_BLACK_BOX_FRAMES, MAX_FUEL_KG, MAX_GUST_MPS,
    MIN_ALTITUDE_ENVELOPE_M,
};

/// Metadata header stored in a [`BlackBoxRecorder`] and serialized at the
/// start of a `GFDR` binary blob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlackBoxHeader {
    /// Binary format version ([`BLACK_BOX_VERSION`]).
    pub version: u32,
    /// Deterministic simulation PRNG seed (`rng::Pcg32`).
    pub rng_seed: u64,
    /// Terrain synthesis seed associated with the recorded world.
    pub terrain_seed: u64,
    /// Aircraft preset identifier.
    pub aircraft_preset: AircraftPresetKind,
    /// Initial flight mode (`OnRunway` or `LevelTrim`).
    pub initial_mode: InitialFlightMode,
    /// Fixed simulation step duration in nanoseconds.
    pub fixed_dt_ns: u64,
    /// Initial altitude (level trim) or runway elevation in meters.
    pub initial_altitude_m: f64,
    /// Initial airspeed in `m/s` (`0.0` on runway, trim speed in level flight).
    pub initial_airspeed_mps: f64,
    /// Initial fuel mass in `kg`.
    pub initial_fuel_kg: f64,
    /// Turbulence gust intensity in `m/s`.
    pub gust_intensity_mps: f64,
    /// Canonical 64-bit FNV-1a digest of the initial [`FlightSimState`].
    pub initial_digest_fnv1a: u64,
}

impl BlackBoxHeader {
    /// Validate every header field against domain and protocol invariants.
    pub fn header_validate(&self) -> Result<(), FlightSimError> {
        if self.version != BLACK_BOX_VERSION {
            return Err(FlightSimError::BlackBoxCorrupt);
        }
        let _ = validate_dt_ns(self.fixed_dt_ns)?;
        let _ = check_range(
            self.initial_altitude_m,
            MIN_ALTITUDE_ENVELOPE_M,
            MAX_ALTITUDE_ENVELOPE_M,
        )?;
        let _ = check_range(self.initial_airspeed_mps, 0.0, MAX_AIRSPEED_MPS)?;
        let _ = check_range(self.initial_fuel_kg, 0.0, MAX_FUEL_KG)?;
        let _ = check_range(self.gust_intensity_mps, 0.0, MAX_GUST_MPS)?;
        Ok(())
    }
}

/// One recorded simulation tick in a [`BlackBoxRecorder`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlackBoxFrame {
    /// Monotonic simulation tick index (`1..=MAX_BLACK_BOX_FRAMES`).
    pub tick: u64,
    /// Pilot control inputs applied for this tick.
    pub input: FlightInputFrame,
    /// Resulting flight phase after the step.
    pub phase: FlightPhase,
    /// Active HUD annunciator flags after the step.
    pub annunciators: AnnunciatorFlags,
    /// Pitch angle in radians (`[-pi, pi]`).
    pub pitch_rad: f64,
    /// Roll (bank) angle in radians (`[-pi, pi]`).
    pub roll_rad: f64,
    /// Magnetic/world heading in radians (`[0, 2*pi]`).
    pub heading_rad: f64,
    /// Indicated airspeed in `m/s`.
    pub ias_mps: f64,
    /// Altitude above mean sea level in meters.
    pub msl_m: f64,
    /// Height above ground level in meters.
    pub agl_m: f64,
    /// Vertical speed in `m/s`.
    pub vsi_mps: f64,
    /// Normal load factor in `g`.
    pub g_load: f64,
    /// Remaining fuel mass in `kg`.
    pub fuel_kg: f64,
    /// Canonical 64-bit FNV-1a digest of the post-step [`FlightSimState`].
    pub state_digest_fnv1a: u64,
}

impl BlackBoxFrame {
    /// Validate every scalar, input, and enum tag in this recorded frame.
    pub fn frame_validate(&self) -> Result<(), FlightSimError> {
        let _ = validate_input_frame(&self.input)?;
        let two_pi = 2.0 * trig::PI + 1e-9;
        let _ = check_range(self.pitch_rad, -trig::PI - 1e-9, trig::PI + 1e-9)?;
        let _ = check_range(self.roll_rad, -trig::PI - 1e-9, trig::PI + 1e-9)?;
        let _ = check_range(self.heading_rad, 0.0, two_pi)?;
        let _ = check_range(self.ias_mps, 0.0, MAX_AIRSPEED_MPS)?;
        let _ = check_range(self.msl_m, MIN_ALTITUDE_ENVELOPE_M, MAX_ALTITUDE_ENVELOPE_M)?;
        let _ = check_range(self.agl_m, 0.0, MAX_RADAR_ALTITUDE_M)?;
        let _ = check_range(
            self.vsi_mps,
            -MAX_VERTICAL_SPEED_MPS,
            MAX_VERTICAL_SPEED_MPS,
        )?;
        let _ = check_range(self.g_load, -MAX_G_LOAD, MAX_G_LOAD)?;
        let _ = check_range(self.fuel_kg, 0.0, MAX_FUEL_KG)?;
        Ok(())
    }
}

/// Caller-owned, pre-allocated black-box Flight Data Recorder (`VER07` zero-alloc).
#[derive(Clone, Debug, PartialEq)]
pub struct BlackBoxRecorder {
    header: BlackBoxHeader,
    capacity: usize,
    frames: Vec<BlackBoxFrame>,
}

impl BlackBoxRecorder {
    /// Allocate a [`BlackBoxRecorder`] with pre-reserved capacity `<= MAX_BLACK_BOX_FRAMES`
    /// so steady-state recording via [`step_in_place`] performs `0` heap allocations.
    pub fn recorder_with_capacity(
        header: BlackBoxHeader,
        capacity: usize,
    ) -> Result<Self, FlightSimError> {
        header.header_validate()?;
        let _ = validate_fdr_frame_count(capacity)?;
        Ok(Self {
            header,
            capacity,
            frames: Vec::with_capacity(capacity),
        })
    }

    /// Number of recorded frames currently stored.
    #[inline]
    pub fn recorder_len(&self) -> usize {
        self.frames.len()
    }

    /// Pre-allocated frame capacity (`<= MAX_BLACK_BOX_FRAMES`).
    #[inline]
    pub const fn recorder_capacity(&self) -> usize {
        self.capacity
    }

    /// Return `true` when zero frames have been recorded.
    #[inline]
    pub fn recorder_is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Return `true` when `recorder_len() >= recorder_capacity()`.
    #[inline]
    pub fn recorder_is_full(&self) -> bool {
        self.frames.len() >= self.capacity
    }

    /// Reset recorded frames to empty while retaining the allocated buffer and
    /// updating the header's initial digest/parameters if needed.
    pub fn recorder_clear(&mut self, header: BlackBoxHeader) -> Result<(), FlightSimError> {
        header.header_validate()?;
        self.header = header;
        self.frames.clear();
        Ok(())
    }

    /// Borrow the recorder's [`BlackBoxHeader`].
    #[inline]
    pub const fn recorder_header(&self) -> &BlackBoxHeader {
        &self.header
    }

    /// Borrow all recorded [`BlackBoxFrame`] entries in tick order.
    #[inline]
    pub fn recorder_frames(&self) -> &[BlackBoxFrame] {
        &self.frames
    }

    /// Exact encoded byte length required by [`encode_black_box_into`].
    pub fn recorder_encoded_len(&self) -> Result<usize, FlightSimError> {
        let count = self.frames.len();
        let _ = validate_fdr_frame_count(count)?;
        let payload = count
            .checked_mul(BLACK_BOX_FRAME_BYTES)
            .ok_or(FlightSimError::BlackBoxCorrupt)?;
        BLACK_BOX_HEADER_BYTES
            .checked_add(payload)
            .and_then(|n| n.checked_add(BLACK_BOX_TRAILER_BYTES))
            .ok_or(FlightSimError::BlackBoxCorrupt)
    }

    /// Compute a canonical 64-bit FNV-1a digest over the header and all recorded frames.
    pub fn recorder_digest_fnv1a(&self) -> Result<u64, FlightSimError> {
        self.header.header_validate()?;
        let mut h = FNV_OFFSET;
        h = fnv1a_mix_u64(h, u64::from(self.header.version));
        h = fnv1a_mix_u64(h, self.header.rng_seed);
        h = fnv1a_mix_u64(h, self.header.terrain_seed);
        h = fnv1a_mix_bytes(
            h,
            &[
                self.header.aircraft_preset.preset_to_u8(),
                self.header.initial_mode.initial_mode_to_u8(),
            ],
        );
        h = fnv1a_mix_u64(h, self.header.fixed_dt_ns);
        h = fnv1a_mix_f64(h, self.header.initial_altitude_m)?;
        h = fnv1a_mix_f64(h, self.header.initial_airspeed_mps)?;
        h = fnv1a_mix_f64(h, self.header.initial_fuel_kg)?;
        h = fnv1a_mix_f64(h, self.header.gust_intensity_mps)?;
        h = fnv1a_mix_u64(h, self.header.initial_digest_fnv1a);
        h = fnv1a_mix_u64(h, self.frames.len() as u64);

        for frame in &self.frames {
            frame.frame_validate()?;
            h = fnv1a_mix_u64(h, frame.tick);
            h = fnv1a_mix_u64(h, frame.input.dt_ns);
            h = fnv1a_mix_f64(h, frame.input.elevator)?;
            h = fnv1a_mix_f64(h, frame.input.aileron)?;
            h = fnv1a_mix_f64(h, frame.input.rudder)?;
            h = fnv1a_mix_f64(h, frame.input.throttle)?;
            h = fnv1a_mix_f64(h, frame.input.flaps)?;
            h = fnv1a_mix_f64(h, frame.input.brake)?;
            h = fnv1a_mix_bytes(
                h,
                &[
                    u8::from(frame.input.gear_down),
                    frame.phase.phase_to_u8(),
                    pack_annunciators(frame.annunciators),
                ],
            );
            h = fnv1a_mix_f64(h, frame.pitch_rad)?;
            h = fnv1a_mix_f64(h, frame.roll_rad)?;
            h = fnv1a_mix_f64(h, frame.heading_rad)?;
            h = fnv1a_mix_f64(h, frame.ias_mps)?;
            h = fnv1a_mix_f64(h, frame.msl_m)?;
            h = fnv1a_mix_f64(h, frame.agl_m)?;
            h = fnv1a_mix_f64(h, frame.vsi_mps)?;
            h = fnv1a_mix_f64(h, frame.g_load)?;
            h = fnv1a_mix_f64(h, frame.fuel_kg)?;
            h = fnv1a_mix_u64(h, frame.state_digest_fnv1a);
        }
        Ok(h)
    }

    /// Push a validated frame when capacity permits (internal crate helper).
    pub(crate) fn push_frame(&mut self, frame: BlackBoxFrame) -> Result<(), FlightSimError> {
        if self.frames.len() >= self.capacity || self.frames.len() >= MAX_BLACK_BOX_FRAMES {
            return Err(FlightSimError::BlackBoxFull);
        }
        frame.frame_validate()?;
        self.frames.push(frame);
        Ok(())
    }
}

/// Summary returned by [`replay_black_box_verify`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayVerifyReport {
    /// Total frames replayed and verified bit-for-bit.
    pub frames_verified: usize,
    /// Final simulation tick reached.
    pub final_tick: u64,
    /// Final canonical 64-bit FNV-1a [`FlightSimState`] digest.
    pub final_digest_fnv1a: u64,
}

#[inline]
fn pack_annunciators(flags: AnnunciatorFlags) -> u8 {
    u8::from(flags.stall)
        | (u8::from(flags.overspeed) << 1)
        | (u8::from(flags.terrain) << 2)
        | (u8::from(flags.pull_up) << 3)
}

#[inline]
fn unpack_annunciators(bits: u8) -> Result<AnnunciatorFlags, FlightSimError> {
    if bits > 0x0f {
        return Err(FlightSimError::BlackBoxCorrupt);
    }
    Ok(AnnunciatorFlags {
        stall: (bits & 0x01) != 0,
        overspeed: (bits & 0x02) != 0,
        terrain: (bits & 0x04) != 0,
        pull_up: (bits & 0x08) != 0,
    })
}

struct ByteWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ByteWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn write_bytes(&mut self, src: &[u8]) -> Result<(), FlightSimError> {
        let end = self
            .pos
            .checked_add(src.len())
            .ok_or(FlightSimError::BufferTooSmall)?;
        if end > self.buf.len() {
            return Err(FlightSimError::BufferTooSmall);
        }
        self.buf[self.pos..end].copy_from_slice(src);
        self.pos = end;
        Ok(())
    }

    fn write_u32(&mut self, v: u32) -> Result<(), FlightSimError> {
        self.write_bytes(&v.to_le_bytes())
    }

    fn write_u64(&mut self, v: u64) -> Result<(), FlightSimError> {
        self.write_bytes(&v.to_le_bytes())
    }

    fn write_f64(&mut self, v: f64) -> Result<(), FlightSimError> {
        let clean = check_finite(v)?;
        self.write_u64(clean.to_bits())
    }
}

struct ByteReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn read_slice(&mut self, count: usize) -> Result<&'a [u8], FlightSimError> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or(FlightSimError::BlackBoxCorrupt)?;
        if end > self.buf.len() {
            return Err(FlightSimError::BlackBoxCorrupt);
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, FlightSimError> {
        let s = self.read_slice(1)?;
        Ok(s[0])
    }

    fn read_u32(&mut self) -> Result<u32, FlightSimError> {
        let s = self.read_slice(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, FlightSimError> {
        let s = self.read_slice(8)?;
        Ok(u64::from_le_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }

    fn read_f64(&mut self) -> Result<f64, FlightSimError> {
        let bits = self.read_u64()?;
        let val = f64::from_bits(bits);
        check_finite(val).map_err(|_| FlightSimError::BlackBoxCorrupt)
    }
}

/// Encode `recorder` into caller-owned `out` with zero heap allocations.
///
/// Atomic on `Err`: validates the header, every recorded frame, and `out.len()`
/// **before** writing any byte to `out`.
pub fn encode_black_box_into(
    recorder: &BlackBoxRecorder,
    out: &mut [u8],
) -> Result<usize, FlightSimError> {
    recorder.header.header_validate()?;
    let frame_count = recorder.frames.len();
    let _ = validate_fdr_frame_count(frame_count)?;
    for frame in &recorder.frames {
        frame.frame_validate()?;
    }
    let total_len = recorder.recorder_encoded_len()?;
    if out.len() < total_len {
        return Err(FlightSimError::BufferTooSmall);
    }

    let mut w = ByteWriter::new(&mut out[..total_len]);
    w.write_bytes(&BLACK_BOX_MAGIC)?;
    w.write_u32(recorder.header.version)?;
    w.write_u64(recorder.header.rng_seed)?;
    w.write_u64(recorder.header.terrain_seed)?;
    w.write_bytes(&[
        recorder.header.aircraft_preset.preset_to_u8(),
        recorder.header.initial_mode.initial_mode_to_u8(),
        0,
        0,
    ])?;
    w.write_u64(recorder.header.fixed_dt_ns)?;
    w.write_f64(recorder.header.initial_altitude_m)?;
    w.write_f64(recorder.header.initial_airspeed_mps)?;
    w.write_f64(recorder.header.initial_fuel_kg)?;
    w.write_f64(recorder.header.gust_intensity_mps)?;
    w.write_u64(recorder.header.initial_digest_fnv1a)?;
    w.write_u32(frame_count as u32)?;

    for frame in &recorder.frames {
        w.write_u64(frame.tick)?;
        w.write_u64(frame.input.dt_ns)?;
        w.write_f64(frame.input.elevator)?;
        w.write_f64(frame.input.aileron)?;
        w.write_f64(frame.input.rudder)?;
        w.write_f64(frame.input.throttle)?;
        w.write_f64(frame.input.flaps)?;
        w.write_f64(frame.input.brake)?;
        w.write_bytes(&[
            u8::from(frame.input.gear_down),
            frame.phase.phase_to_u8(),
            pack_annunciators(frame.annunciators),
            0,
            0,
            0,
            0,
            0,
        ])?;
        w.write_f64(frame.pitch_rad)?;
        w.write_f64(frame.roll_rad)?;
        w.write_f64(frame.heading_rad)?;
        w.write_f64(frame.ias_mps)?;
        w.write_f64(frame.msl_m)?;
        w.write_f64(frame.agl_m)?;
        w.write_f64(frame.vsi_mps)?;
        w.write_f64(frame.g_load)?;
        w.write_f64(frame.fuel_kg)?;
        w.write_u64(frame.state_digest_fnv1a)?;
    }

    let body_len = total_len - BLACK_BOX_TRAILER_BYTES;
    let checksum = fnv1a_mix_bytes(FNV_OFFSET, &out[..body_len]);
    out[body_len..total_len].copy_from_slice(&checksum.to_le_bytes());
    Ok(total_len)
}

fn parse_header_and_verify_checksum(
    bytes: &[u8],
) -> Result<(BlackBoxHeader, usize), FlightSimError> {
    if bytes.len() < BLACK_BOX_HEADER_BYTES + BLACK_BOX_TRAILER_BYTES
        || bytes.len() > MAX_BLACK_BOX_BYTES
    {
        return Err(FlightSimError::BlackBoxCorrupt);
    }
    if bytes[0..4] != BLACK_BOX_MAGIC {
        return Err(FlightSimError::BlackBoxCorrupt);
    }

    let mut r = ByteReader::new(&bytes[4..BLACK_BOX_HEADER_BYTES]);
    let version = r.read_u32()?;
    if version != BLACK_BOX_VERSION {
        return Err(FlightSimError::BlackBoxCorrupt);
    }
    let rng_seed = r.read_u64()?;
    let terrain_seed = r.read_u64()?;
    let preset_raw = r.read_u8()?;
    let mode_raw = r.read_u8()?;
    let rsv0 = r.read_u8()?;
    let rsv1 = r.read_u8()?;
    if rsv0 != 0 || rsv1 != 0 {
        return Err(FlightSimError::BlackBoxCorrupt);
    }
    let aircraft_preset = AircraftPresetKind::preset_from_u8(preset_raw)?;
    let initial_mode = InitialFlightMode::initial_mode_from_u8(mode_raw)?;
    let fixed_dt_ns = r.read_u64()?;
    let initial_altitude_m = canon_zero(r.read_f64()?);
    let initial_airspeed_mps = canon_zero(r.read_f64()?);
    let initial_fuel_kg = canon_zero(r.read_f64()?);
    let gust_intensity_mps = canon_zero(r.read_f64()?);
    let initial_digest_fnv1a = r.read_u64()?;
    let frame_count = r.read_u32()? as usize;

    let _ = validate_fdr_blob_bytes(bytes.len(), frame_count)?;

    let header = BlackBoxHeader {
        version,
        rng_seed,
        terrain_seed,
        aircraft_preset,
        initial_mode,
        fixed_dt_ns,
        initial_altitude_m,
        initial_airspeed_mps,
        initial_fuel_kg,
        gust_intensity_mps,
        initial_digest_fnv1a,
    };
    header
        .header_validate()
        .map_err(|_| FlightSimError::BlackBoxCorrupt)?;

    let body_len = bytes.len() - BLACK_BOX_TRAILER_BYTES;
    let stored_checksum = u64::from_le_bytes([
        bytes[body_len],
        bytes[body_len + 1],
        bytes[body_len + 2],
        bytes[body_len + 3],
        bytes[body_len + 4],
        bytes[body_len + 5],
        bytes[body_len + 6],
        bytes[body_len + 7],
    ]);
    let computed_checksum = fnv1a_mix_bytes(FNV_OFFSET, &bytes[..body_len]);
    if stored_checksum != computed_checksum {
        return Err(FlightSimError::BlackBoxCorrupt);
    }

    Ok((header, frame_count))
}

fn decode_one_frame(chunk: &[u8]) -> Result<BlackBoxFrame, FlightSimError> {
    let mut r = ByteReader::new(chunk);
    let tick = r.read_u64()?;
    let dt_ns = r.read_u64()?;
    let elevator = canon_zero(r.read_f64()?);
    let aileron = canon_zero(r.read_f64()?);
    let rudder = canon_zero(r.read_f64()?);
    let throttle = canon_zero(r.read_f64()?);
    let flaps = canon_zero(r.read_f64()?);
    let brake = canon_zero(r.read_f64()?);
    let gear_raw = r.read_u8()?;
    let phase_raw = r.read_u8()?;
    let flags_raw = r.read_u8()?;
    let p0 = r.read_u8()?;
    let p1 = r.read_u8()?;
    let p2 = r.read_u8()?;
    let p3 = r.read_u8()?;
    let p4 = r.read_u8()?;
    if gear_raw > 1 || p0 != 0 || p1 != 0 || p2 != 0 || p3 != 0 || p4 != 0 {
        return Err(FlightSimError::BlackBoxCorrupt);
    }
    let phase = FlightPhase::phase_from_u8(phase_raw)?;
    let annunciators = unpack_annunciators(flags_raw)?;

    let pitch_rad = canon_zero(r.read_f64()?);
    let roll_rad = canon_zero(r.read_f64()?);
    let heading_rad = canon_zero(r.read_f64()?);
    let ias_mps = canon_zero(r.read_f64()?);
    let msl_m = canon_zero(r.read_f64()?);
    let agl_m = canon_zero(r.read_f64()?);
    let vsi_mps = canon_zero(r.read_f64()?);
    let g_load = canon_zero(r.read_f64()?);
    let fuel_kg = canon_zero(r.read_f64()?);
    let state_digest_fnv1a = r.read_u64()?;

    let frame = BlackBoxFrame {
        tick,
        input: FlightInputFrame {
            dt_ns,
            elevator,
            aileron,
            rudder,
            throttle,
            flaps,
            brake,
            gear_down: gear_raw == 1,
        },
        phase,
        annunciators,
        pitch_rad,
        roll_rad,
        heading_rad,
        ias_mps,
        msl_m,
        agl_m,
        vsi_mps,
        g_load,
        fuel_kg,
        state_digest_fnv1a,
    };
    frame
        .frame_validate()
        .map_err(|_| FlightSimError::BlackBoxCorrupt)?;
    Ok(frame)
}

/// Decode a hardened `GFDR` binary blob into a caller-owned [`BlackBoxRecorder`]
/// with zero heap allocations.
///
/// Atomic on `Err`: validates the header, FNV-1a checksum, capacity, and every
/// frame **before** modifying `out`.
pub fn decode_black_box_into(
    bytes: &[u8],
    out: &mut BlackBoxRecorder,
) -> Result<usize, FlightSimError> {
    let (header, frame_count) = parse_header_and_verify_checksum(bytes)?;
    if frame_count > out.capacity {
        return Err(FlightSimError::BlackBoxFull);
    }
    let payload = &bytes[BLACK_BOX_HEADER_BYTES..bytes.len() - BLACK_BOX_TRAILER_BYTES];
    let (chunks, rem) = payload.as_chunks::<BLACK_BOX_FRAME_BYTES>();
    if !rem.is_empty() || chunks.len() != frame_count {
        return Err(FlightSimError::BlackBoxCorrupt);
    }

    // Pass 1: validate every frame before mutating `out`.
    for (idx, chunk) in chunks.iter().enumerate() {
        let frame = decode_one_frame(chunk)?;
        let expected_tick = (idx as u64) + 1;
        if frame.tick != expected_tick {
            return Err(FlightSimError::BlackBoxCorrupt);
        }
    }

    // Pass 2: commit into pre-allocated storage (0 heap allocations).
    out.header = header;
    out.frames.clear();
    for chunk in chunks {
        let frame = decode_one_frame(chunk)?;
        out.frames.push(frame);
    }
    Ok(frame_count)
}

/// Decode a hardened `GFDR` binary blob into a newly allocated [`BlackBoxRecorder`].
///
/// Rejects truncated, oversized, bad-magic, bad-checksum, or NaN/out-of-bounds
/// payloads before allocating frame storage (CWE-20 / CWE-125 / CWE-400 / CWE-502).
pub fn decode_black_box_from_bytes(bytes: &[u8]) -> Result<BlackBoxRecorder, FlightSimError> {
    let (header, frame_count) = parse_header_and_verify_checksum(bytes)?;
    let payload = &bytes[BLACK_BOX_HEADER_BYTES..bytes.len() - BLACK_BOX_TRAILER_BYTES];
    let (chunks, rem) = payload.as_chunks::<BLACK_BOX_FRAME_BYTES>();
    if !rem.is_empty() || chunks.len() != frame_count {
        return Err(FlightSimError::BlackBoxCorrupt);
    }

    for (idx, chunk) in chunks.iter().enumerate() {
        let frame = decode_one_frame(chunk)?;
        let expected_tick = (idx as u64) + 1;
        if frame.tick != expected_tick {
            return Err(FlightSimError::BlackBoxCorrupt);
        }
    }

    let mut recorder = BlackBoxRecorder::recorder_with_capacity(header, frame_count)?;
    for chunk in chunks {
        let frame = decode_one_frame(chunk)?;
        recorder.push_frame(frame)?;
    }
    Ok(recorder)
}

/// Replay every recorded [`BlackBoxFrame`] starting from `initial_state` and
/// verify bit-exact [`state_digest_fnv1a`] and telemetry parity on every tick.
pub fn replay_black_box_verify(
    initial_state: &FlightSimState,
    heightfield: &Heightfield<'_>,
    runway: &RunwaySpec,
    recorder: &BlackBoxRecorder,
) -> Result<ReplayVerifyReport, FlightSimError> {
    sim_state_validate(initial_state)?;
    recorder.header.header_validate()?;
    let init_digest = state_digest_fnv1a(initial_state)?;
    if init_digest != recorder.header.initial_digest_fnv1a {
        return Err(FlightSimError::ReplayMismatch);
    }

    let mut sim = initial_state.clone();
    for (idx, expected) in recorder.frames.iter().enumerate() {
        expected.frame_validate()?;
        let expected_tick = (idx as u64) + 1;
        if expected.tick != expected_tick {
            return Err(FlightSimError::ReplayMismatch);
        }

        let report = step_in_place(&mut sim, heightfield, runway, &expected.input, None)?;
        if report.tick != expected.tick
            || report.phase != expected.phase
            || report.state_digest_fnv1a != expected.state_digest_fnv1a
            || sim.fuel_kg.to_bits() != expected.fuel_kg.to_bits()
            || sim.last_agl_m.to_bits() != expected.agl_m.to_bits()
            || sim.last_g_load.to_bits() != expected.g_load.to_bits()
            || sim.last_vertical_speed_mps.to_bits() != expected.vsi_mps.to_bits()
            || sim.last_annunciators != expected.annunciators
        {
            return Err(FlightSimError::ReplayMismatch);
        }
    }

    let final_digest = state_digest_fnv1a(&sim)?;
    Ok(ReplayVerifyReport {
        frames_verified: recorder.frames.len(),
        final_tick: sim.tick,
        final_digest_fnv1a: final_digest,
    })
}
