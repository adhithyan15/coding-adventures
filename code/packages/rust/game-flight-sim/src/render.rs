//! # Integrated 3D Terrain + Cockpit HUD Pipeline (`VER07` Zero-Allocation)
//!
//! Unites [`terrain_lod`] (`GAME42`), [`render3d_gpu_bridge`] (`GAME44`), and
//! [`cockpit_hud_paint`] (`GAME43`) over caller-owned [`FlightRenderScratch`],
//! [`PixelContainer`], [`DepthBufferMut`], and optional retained [`PaintScene`].
//!
//! All validation runs **before** any buffer mutation, guaranteeing
//! byte-for-byte atomicity on `Err` and `0` heap allocations in steady-state
//! rendering.

use aero_flight_model::ISA_SEA_LEVEL_DENSITY_KG_M3;
use cockpit_hud_paint::{
    build_hud_frame_into, paint_scene_digest_fnv1a, update_paint_scene_into,
    verify_paint_scene_finite, AnnunciatorFlags, FlightTelemetry, HudConfig, HudFrameSummary,
    HudScratch, MAX_AIRSPEED_MPS, MAX_ALTITUDE_MSL_M, MAX_G_LOAD, MAX_MACH, MAX_RADAR_ALTITUDE_M,
    MAX_VERTICAL_SPEED_MPS, MIN_ALTITUDE_MSL_M,
};
use math3d::{Mat4, Vec3};
use paint_instructions::{PaintInstruction, PaintScene};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    execute_render_pass_into, BackendPreference, CameraUniforms, ClipSpace, DepthBufferMut,
    DrawCall, LightRig, MaterialBinding, MeshSlice, RenderBridgeScratch, RenderExecutionReport,
    RenderPass, RenderTargetSpec, ShadingMode, Vertex, MAX_MATRIX_ENTRY, MAX_TOTAL_TRIANGLES,
    MAX_VERTICES_PER_MESH,
};
use terrain_lod::{
    emit_mesh_into, refine_into, Heightfield, LodParams, TerrainScratch, MAX_HEIGHT,
    MAX_OUTPUT_TRIANGLES, MAX_QUADTREE_NODES,
};

use crate::validate::{
    canon_zero, check_finite, check_mat4_bridge_bounded, check_range, fnv1a_mix_u64,
    validate_render_dim, FNV_OFFSET,
};
use crate::{sim_state_validate, CameraMode, FlightSimError, FlightSimState};

/// Expected instruction slot count in a retained `cockpit-hud-paint` [`PaintScene`].
const HUD_SCENE_SKELETON_LEN: usize = 55;

/// Minimum pre-allocated capacity for retained `PaintScene` string fields.
const HUD_STR_CAP_COLOR: usize = 16;
const HUD_STR_CAP_ID: usize = 32;
const HUD_STR_CAP_TEXT: usize = 32;
const HUD_PATH_CAP_POLY: usize = 16;
const HUD_PATH_CAP_PITCH: usize = 512;
const HUD_PATH_CAP_TICKS: usize = 512;

/// Caller-owned, pre-allocated scratch buffers for zero-allocation 3D terrain
/// and cockpit HUD rendering (`VER07`).
#[derive(Debug)]
pub struct FlightRenderScratch {
    /// Quadtree LOD refinement scratch (`GAME42`).
    pub terrain_scratch: TerrainScratch,
    /// Pre-allocated terrain vertex staging buffer.
    pub terrain_vertices: Vec<Vertex>,
    /// Pre-allocated terrain triangle index buffer (`u32`).
    pub terrain_indices: Vec<u32>,
    /// Software/GPU-bridge rasterization scratch (`GAME44`).
    pub bridge_scratch: RenderBridgeScratch,
    /// Cockpit HUD primitive staging scratch (`GAME43`).
    pub hud_scratch: HudScratch,
}

impl FlightRenderScratch {
    /// Allocate a [`FlightRenderScratch`] sized for the given terrain,
    /// framebuffer, and HUD budgets so steady-state [`render_frame_into`]
    /// performs `0` heap allocations.
    pub fn flight_scratch_with_capacity(
        max_nodes: usize,
        max_vertices: usize,
        max_triangles: usize,
        fb_width: u32,
        fb_height: u32,
        hud_capacity: usize,
    ) -> Result<Self, FlightSimError> {
        if max_nodes == 0 || max_nodes > MAX_QUADTREE_NODES {
            return Err(FlightSimError::BudgetExceeded);
        }
        if max_vertices == 0 || max_vertices > MAX_VERTICES_PER_MESH {
            return Err(FlightSimError::BudgetExceeded);
        }
        if max_triangles == 0
            || max_triangles > MAX_OUTPUT_TRIANGLES
            || max_triangles > MAX_TOTAL_TRIANGLES
        {
            return Err(FlightSimError::BudgetExceeded);
        }
        let _ = validate_render_dim(fb_width)?;
        let _ = validate_render_dim(fb_height)?;
        if !(cockpit_hud_paint::MIN_PRIMITIVES..=cockpit_hud_paint::MAX_PRIMITIVES)
            .contains(&hud_capacity)
        {
            return Err(FlightSimError::BudgetExceeded);
        }

        let index_len = max_triangles
            .checked_mul(3)
            .ok_or(FlightSimError::BudgetExceeded)?;

        let terrain_scratch = TerrainScratch::scratch_with_capacity(max_nodes)?;
        let default_vertex =
            Vertex::vertex_new(Vec3::zero(), Vec3::unit_y(), [0.3, 0.5, 0.3], 0.0, 0.0);
        let terrain_vertices = vec![default_vertex; max_vertices];
        let terrain_indices = vec![0_u32; index_len];
        let bridge_scratch =
            RenderBridgeScratch::bridge_scratch_with_capacity(max_vertices, fb_width, fb_height)?;
        let hud_scratch = HudScratch::hud_scratch_with_capacity(hud_capacity);

        Ok(Self {
            terrain_scratch,
            terrain_vertices,
            terrain_indices,
            bridge_scratch,
            hud_scratch,
        })
    }

    /// Clear transient scratch state while keeping all heap capacities intact.
    pub fn flight_scratch_reset(&mut self) {
        self.bridge_scratch.bridge_scratch_reset();
        self.hud_scratch.hud_scratch_clear();
    }

    /// Capacity of the pre-allocated terrain vertex buffer.
    #[inline]
    pub fn flight_scratch_vertex_capacity(&self) -> usize {
        self.terrain_vertices.len()
    }

    /// Capacity of the pre-allocated terrain index buffer (in `u32` elements).
    #[inline]
    pub fn flight_scratch_index_capacity(&self) -> usize {
        self.terrain_indices.len()
    }

    /// Capacity of the pre-allocated HUD primitive scratch buffer.
    #[inline]
    pub fn flight_scratch_hud_capacity(&self) -> usize {
        self.hud_scratch.hud_scratch_capacity()
    }
}

/// Combined summary and digests returned by [`render_frame_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightFrameReport {
    /// 3D terrain render-pass execution report (`GAME44`).
    pub render_report: RenderExecutionReport,
    /// Cockpit HUD primitive summary (`GAME43`).
    pub hud_summary: HudFrameSummary,
    /// Combined canonical 64-bit FNV-1a digest across 3D color, depth, HUD, and
    /// optional retained `PaintScene`.
    pub combined_digest_fnv1a: u64,
}

/// Construct [`FlightTelemetry`] from [`FlightSimState`] and [`Heightfield`],
/// enriched with [`aero_flight_model`] `AirData` / `IsaSample` and exact
/// terrain AGL.
pub fn telemetry_from_sim_state(
    state: &FlightSimState,
    heightfield: &Heightfield<'_>,
) -> Result<FlightTelemetry, FlightSimError> {
    sim_state_validate(state)?;
    let spec = heightfield.spec();
    let _ = Heightfield::from_heights(spec, heightfield.heights())?;
    let terrain_y =
        check_finite(heightfield.height_at(state.body.position.x, state.body.position.z)?)?;

    let mut telemetry = FlightTelemetry::telemetry_from_rigid_body(
        &state.body,
        &state.mass,
        state.last_isa.speed_of_sound_mps,
        terrain_y,
    )?;

    let tas = state.last_air_data.true_airspeed_mps;
    let rho_ratio = (state.last_isa.density_kg_m3 / ISA_SEA_LEVEL_DENSITY_KG_M3).max(0.0);
    let ias = canon_zero((tas * trig::sqrt(rho_ratio)).clamp(0.0, MAX_AIRSPEED_MPS));
    let msl = canon_zero(
        (state.body.position.y + state.env.ground_elevation_m)
            .clamp(MIN_ALTITUDE_MSL_M, MAX_ALTITUDE_MSL_M),
    );
    let agl = canon_zero((state.body.position.y - terrain_y).clamp(0.0, MAX_RADAR_ALTITUDE_M));

    telemetry.indicated_airspeed_mps = ias;
    telemetry.true_airspeed_mps = canon_zero(tas.clamp(0.0, MAX_AIRSPEED_MPS));
    telemetry.mach = canon_zero(state.last_air_data.mach.clamp(0.0, MAX_MACH));
    telemetry.altitude_msl_m = msl;
    telemetry.radar_altitude_agl_m = agl;
    telemetry.vertical_speed_mps = canon_zero(
        state
            .last_vertical_speed_mps
            .clamp(-MAX_VERTICAL_SPEED_MPS, MAX_VERTICAL_SPEED_MPS),
    );
    telemetry.aoa_rad = canon_zero(state.last_air_data.alpha_rad);
    telemetry.sideslip_rad = canon_zero(state.last_air_data.beta_rad);
    telemetry.g_load = canon_zero(state.last_g_load.clamp(-MAX_G_LOAD, MAX_G_LOAD));
    telemetry.telemetry_validate()?;
    Ok(telemetry)
}

/// Evaluate [`AnnunciatorFlags`] for `telemetry` against `hud_config` thresholds.
pub(crate) fn compute_annunciators(
    telemetry: &FlightTelemetry,
    hud_config: &HudConfig,
) -> AnnunciatorFlags {
    let airborne_like = telemetry.radar_altitude_agl_m > 3.0;
    let stall = airborne_like
        && (telemetry.aoa_rad >= hud_config.aoa_stall_rad
            || telemetry.indicated_airspeed_mps < hud_config.ias_stall_mps);
    let overspeed = telemetry.indicated_airspeed_mps > hud_config.ias_overspeed_mps
        || telemetry.mach > hud_config.mach_overspeed;
    let pull_up = airborne_like
        && telemetry.radar_altitude_agl_m < hud_config.pull_up_agl_m
        && telemetry.vertical_speed_mps < -2.5;
    let terrain = airborne_like
        && telemetry.radar_altitude_agl_m < hud_config.terrain_caution_agl_m
        && telemetry.vertical_speed_mps < -0.5;
    AnnunciatorFlags {
        stall,
        overspeed,
        terrain,
        pull_up,
    }
}

/// Build bounded cockpit or chase [`CameraUniforms`] from `state` and `target`.
///
/// Enforces `Mat4` entry bounds `<= render3d_gpu_bridge::MAX_MATRIX_ENTRY`
/// before returning.
pub fn build_flight_camera(
    state: &FlightSimState,
    target: &RenderTargetSpec,
    hud_config: &HudConfig,
    mode: CameraMode,
) -> Result<CameraUniforms, FlightSimError> {
    sim_state_validate(state)?;
    target.target_spec_validate()?;
    hud_config.hud_config_validate()?;

    let q = state.body.orientation.quat_normalize()?;
    let fwd = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0))?.normalize()?;
    let up = q.rotate_vec3(Vec3::new(0.0, 1.0, 0.0))?.normalize()?;

    let (eye, look_target) = match mode {
        CameraMode::Cockpit => {
            let eye_off = q.rotate_vec3(Vec3::new(0.6, 0.65, 0.0))?;
            let eye = state.body.position.vec3_add(eye_off)?;
            let look_ahead = fwd.vec3_scale(100.0)?;
            (eye, eye.vec3_add(look_ahead)?)
        }
        CameraMode::Chase => {
            let back = fwd.vec3_scale(-16.0)?;
            let lift = Vec3::new(0.0, 4.5, 0.0);
            let eye = state.body.position.vec3_add(back)?.vec3_add(lift)?;
            let ahead = fwd.vec3_scale(20.0)?;
            (eye, state.body.position.vec3_add(ahead)?)
        }
    };

    let view = Mat4::look_at(eye, look_target, up)?;
    let fov_rad = trig::radians(hud_config.fov_vertical_deg);
    let aspect = (target.width as f64) / (target.height as f64);
    let proj = match target.clip_space {
        ClipSpace::OpenGl => Mat4::perspective(fov_rad, aspect, 0.5, 5_000.0)?,
        ClipSpace::ZeroToOne => Mat4::perspective_zo(fov_rad, aspect, 0.5, 5_000.0)?,
    };
    let combined = proj.mul(&view)?;
    check_mat4_bridge_bounded(&view)?;
    check_mat4_bridge_bounded(&proj)?;
    check_mat4_bridge_bounded(&combined)?;

    let cam = CameraUniforms::camera_from_view_proj(view, proj, eye)?;
    cam.camera_validate()?;
    if cam.camera_position.x.abs() > MAX_MATRIX_ENTRY
        || cam.camera_position.y.abs() > MAX_MATRIX_ENTRY
        || cam.camera_position.z.abs() > MAX_MATRIX_ENTRY
    {
        return Err(FlightSimError::ValueOutOfBounds);
    }
    Ok(cam)
}

#[inline]
fn depth_buffer_len(depth: &DepthBufferMut<'_>) -> usize {
    match depth {
        DepthBufferMut::F32(s) => s.len(),
        DepthBufferMut::F64(s) => s.len(),
    }
}

#[inline]
fn check_opt_str_cap(s: &Option<String>, min_cap: usize) -> bool {
    match s {
        Some(inner) => inner.capacity() >= min_cap,
        None => false,
    }
}

fn validate_paint_scene_ready(scene: &PaintScene) -> Result<(), FlightSimError> {
    verify_paint_scene_finite(scene)?;
    if scene.background.capacity() < HUD_STR_CAP_COLOR
        || scene.instructions.len() != HUD_SCENE_SKELETON_LEN
    {
        return Err(FlightSimError::BufferTooSmall);
    }
    for (idx, instr) in scene.instructions.iter().enumerate() {
        let ok = match idx {
            0 | 3 | 12 | 14 | 22 | 24 | 32 | 35 | 46 | 48 | 50 | 52 => {
                if let PaintInstruction::Rect(r) = instr {
                    check_opt_str_cap(&r.base.id, HUD_STR_CAP_ID)
                        && check_opt_str_cap(&r.fill, HUD_STR_CAP_COLOR)
                        && check_opt_str_cap(&r.stroke, HUD_STR_CAP_COLOR)
                } else {
                    false
                }
            }
            4 | 33 | 37 => {
                if let PaintInstruction::Line(l) = instr {
                    check_opt_str_cap(&l.base.id, HUD_STR_CAP_ID)
                        && l.stroke.capacity() >= HUD_STR_CAP_COLOR
                } else {
                    false
                }
            }
            10 => {
                if let PaintInstruction::Ellipse(e) = instr {
                    check_opt_str_cap(&e.base.id, HUD_STR_CAP_ID)
                        && check_opt_str_cap(&e.fill, HUD_STR_CAP_COLOR)
                        && check_opt_str_cap(&e.stroke, HUD_STR_CAP_COLOR)
                } else {
                    false
                }
            }
            1 | 2 | 5 | 6 | 7 | 8 | 9 | 11 | 13 | 23 | 36 => {
                if let PaintInstruction::Path(p) = instr {
                    let min_cmds = match idx {
                        1 | 2 => HUD_PATH_CAP_POLY,
                        5 | 6 => HUD_PATH_CAP_PITCH,
                        13 | 23 | 36 => HUD_PATH_CAP_TICKS,
                        _ => 64,
                    };
                    check_opt_str_cap(&p.base.id, HUD_STR_CAP_ID)
                        && check_opt_str_cap(&p.fill, HUD_STR_CAP_COLOR)
                        && check_opt_str_cap(&p.stroke, HUD_STR_CAP_COLOR)
                        && p.commands.capacity() >= min_cmds
                } else {
                    false
                }
            }
            _ => {
                if let PaintInstruction::Text(t) = instr {
                    check_opt_str_cap(&t.base.id, HUD_STR_CAP_ID)
                        && t.text.capacity() >= HUD_STR_CAP_TEXT
                        && check_opt_str_cap(&t.fill, HUD_STR_CAP_COLOR)
                } else {
                    false
                }
            }
        };
        if !ok {
            return Err(FlightSimError::BufferTooSmall);
        }
    }
    Ok(())
}

fn terrain_vertex_color(y: f64, normal_y: f64) -> [f64; 3] {
    let slope = (1.0 - normal_y.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if y > 220.0 {
        [0.86, 0.88, 0.92]
    } else if slope > 0.25 {
        [0.48, 0.44, 0.38]
    } else if y < 12.0 {
        [0.34, 0.42, 0.30]
    } else {
        [0.24, 0.50, 0.24]
    }
}

/// Render one 3D terrain + cockpit HUD frame into caller-owned buffers with
/// zero heap allocations (`VER07`).
///
/// Strictly atomic on `Err`: validates `state`, `heightfield`, `lod_params`,
/// `hud_config`, `target`, `backend_pref`, `framebuffer`, `depth`, `scratch`,
/// and optional `paint_scene` **before** mutating any buffer.
#[allow(clippy::too_many_arguments)]
pub fn render_frame_into(
    state: &FlightSimState,
    heightfield: &Heightfield<'_>,
    lod_params: &LodParams,
    hud_config: &HudConfig,
    target: &RenderTargetSpec,
    backend_pref: BackendPreference,
    camera_mode: CameraMode,
    framebuffer: &mut PixelContainer,
    depth: DepthBufferMut<'_>,
    scratch: &mut FlightRenderScratch,
    paint_scene: Option<&mut PaintScene>,
) -> Result<FlightFrameReport, FlightSimError> {
    // 1. Upfront atomic validation (zero mutations).
    sim_state_validate(state)?;
    let spec = heightfield.spec();
    let _ = Heightfield::from_heights(spec, heightfield.heights())?;
    for &h in heightfield.heights() {
        let _ = check_range(h, -MAX_HEIGHT, MAX_HEIGHT)?;
    }

    let validated_lod = LodParams::lod_params_new(
        lod_params.camera_pos,
        lod_params.max_screen_error,
        lod_params.projection_scale,
        lod_params.max_depth,
        lod_params.max_nodes,
        lod_params.max_triangles,
        lod_params.frustum,
    )?;

    hud_config.hud_config_validate()?;
    target.target_spec_validate()?;
    let _ = validate_render_dim(target.width)?;
    let _ = validate_render_dim(target.height)?;

    if matches!(backend_pref, BackendPreference::RequireHardware(_)) {
        return Err(FlightSimError::BackendUnavailable);
    }

    let pixel_count = (target.width as usize)
        .checked_mul(target.height as usize)
        .ok_or(FlightSimError::InvalidDimensions)?;
    let byte_count = pixel_count
        .checked_mul(4)
        .ok_or(FlightSimError::InvalidDimensions)?;

    if framebuffer.width != target.width
        || framebuffer.height != target.height
        || framebuffer.data.len() != byte_count
        || depth_buffer_len(&depth) != pixel_count
    {
        return Err(FlightSimError::InvalidDimensions);
    }

    let v_count = heightfield.vertex_count();
    if v_count > MAX_VERTICES_PER_MESH
        || validated_lod.max_triangles > MAX_OUTPUT_TRIANGLES
        || validated_lod.max_triangles > MAX_TOTAL_TRIANGLES
    {
        return Err(FlightSimError::BudgetExceeded);
    }
    let max_idx_needed = validated_lod
        .max_triangles
        .checked_mul(3)
        .ok_or(FlightSimError::BudgetExceeded)?;

    if scratch.terrain_scratch.scratch_capacity() < validated_lod.max_nodes
        || scratch.terrain_vertices.len() < v_count
        || scratch.terrain_indices.len() < max_idx_needed
        || scratch.bridge_scratch.bridge_scratch_vertex_capacity() < v_count
        || scratch.bridge_scratch.bridge_scratch_pixel_byte_capacity() < byte_count
        || scratch.hud_scratch.hud_scratch_capacity() < hud_config.max_primitives
    {
        return Err(FlightSimError::BufferTooSmall);
    }

    let telemetry = telemetry_from_sim_state(state, heightfield)?;
    let camera = build_flight_camera(state, target, hud_config, camera_mode)?;
    let light = LightRig::light_rig_default();
    light.light_rig_validate()?;

    if let Some(ref scene) = paint_scene {
        validate_paint_scene_ready(scene)?;
    }

    // `build_hud_frame_into` runs a dry pass first, leaving `hud_scratch` untouched on Err.
    let hud_summary = build_hud_frame_into(&telemetry, hud_config, &mut scratch.hud_scratch)?;

    // 2. Populate terrain mesh vertices into pre-allocated scratch (0 allocs).
    let side = heightfield.grid_side();
    let inv_side_m1 = if side > 1 {
        1.0 / ((side - 1) as f64)
    } else {
        1.0
    };
    for iz in 0..side {
        let v_tex = (iz as f64) * inv_side_m1;
        for ix in 0..side {
            let u_tex = (ix as f64) * inv_side_m1;
            let pos = heightfield.vertex_position(ix, iz)?;
            let norm = heightfield.vertex_normal(ix, iz)?;
            let col = terrain_vertex_color(pos.y, norm.y);
            let idx = iz * side + ix;
            scratch.terrain_vertices[idx] = Vertex::vertex_new(pos, norm, col, u_tex, v_tex);
        }
    }

    // 3. Refine LOD & emit triangle indices into pre-allocated scratch (0 allocs).
    let _lod_stats = refine_into(heightfield, &validated_lod, &mut scratch.terrain_scratch)?;
    let emitted_tris = emit_mesh_into(
        heightfield,
        &scratch.terrain_scratch,
        &mut scratch.terrain_indices,
        validated_lod.max_triangles,
    )?;
    let emitted_idx_count = emitted_tris
        .checked_mul(3)
        .ok_or(FlightSimError::BudgetExceeded)?;

    // 4. Execute 3D terrain render pass via render3d-gpu-bridge (0 allocs).
    let meshes = [MeshSlice::mesh_slice_new(
        &scratch.terrain_vertices[..v_count],
        &scratch.terrain_indices[..emitted_idx_count],
    )?];
    let materials = [MaterialBinding::material_new(ShadingMode::Gouraud)];
    let draw_calls = [DrawCall::draw_call_new(
        0,
        0,
        Mat4::mat4_identity(),
        0,
        emitted_idx_count,
    )?];
    let pass = RenderPass {
        target: *target,
        camera,
        light,
        meshes: &meshes,
        materials: &materials,
        draw_calls: &draw_calls,
    };
    let render_report = execute_render_pass_into(
        &pass,
        backend_pref,
        framebuffer,
        depth,
        &mut scratch.bridge_scratch,
    )?;

    // 5. Update optional retained PaintScene (0 allocs) and compute combined digest.
    let mut combined = FNV_OFFSET;
    combined = fnv1a_mix_u64(combined, render_report.frame_digest);
    combined = fnv1a_mix_u64(combined, render_report.depth_digest);
    combined = fnv1a_mix_u64(combined, hud_summary.frame_digest_fnv1a);

    if let Some(scene) = paint_scene {
        let _scene_summary = update_paint_scene_into(&telemetry, hud_config, scene)?;
        let scene_digest = paint_scene_digest_fnv1a(scene)?;
        combined = fnv1a_mix_u64(combined, scene_digest);
    }

    Ok(FlightFrameReport {
        render_report,
        hud_summary,
        combined_digest_fnv1a: combined,
    })
}
