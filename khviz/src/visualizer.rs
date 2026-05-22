use crate::audio::AudioLevels;
use crate::eel::{EelEnv, parse};
use crate::preset::Preset;
use crate::renderer::{Renderer, MESH_W, MESH_H};

pub struct Visualizer {
    renderer: Renderer,
    audio: AudioLevels,
    preset: Option<Preset>,

    eel_env: EelEnv,
    wave_envs: [EelEnv; 8],
    shape_envs: [EelEnv; 4],
    regs: [f64; 100],

    time_domain: Vec<f32>,
    freq: Vec<f32>,
    sample_rate: f32,
    fft_size: usize,

    start_time: f64,
    last_time: f64,
    frame: u64,
    fps: f32,
    fps_accum: f32,
    fps_count: u32,

    texture_ratio: f32,

    // Preset blending
    blend_from_tex: Option<web_sys::WebGlTexture>,
    blend_remaining: f32,
    blend_total: f32,
}

impl Visualizer {
    pub fn new(
        gl: web_sys::WebGl2RenderingContext,
        width: u32,
        height: u32,
        mesh_width: usize,
        mesh_height: usize,
        texture_ratio: f32,
        fxaa: bool,
    ) -> Result<Self, String> {
        let _ = (mesh_width, mesh_height); // will use MESH_W / MESH_H constants for now
        let renderer = Renderer::new(gl, width, height, texture_ratio, fxaa, None)?;

        Ok(Self {
            renderer,
            audio: AudioLevels::new(),
            preset: None,
            eel_env: EelEnv::new(),
            wave_envs: Default::default(),
            shape_envs: Default::default(),
            regs: [0.0; 100],
            time_domain: vec![0.0; 2048],
            freq: vec![0.0; 2048],
            sample_rate: 44100.0,
            fft_size: 2048,
            start_time: 0.0,
            last_time: 0.0,
            frame: 0,
            fps: 60.0,
            fps_accum: 0.0,
            fps_count: 0,
            texture_ratio,
            blend_from_tex: None,
            blend_remaining: 0.0,
            blend_total: 0.0,
        })
    }

    pub fn load_preset(&mut self, preset: Preset, blend_time: f32) {
        // Reset EEL envs for new preset
        self.eel_env = EelEnv::new();
        for env in &mut self.wave_envs {
            *env = EelEnv::new();
        }
        for env in &mut self.shape_envs {
            *env = EelEnv::new();
        }
        self.regs = [0.0; 100];

        // Run init_eqs_eel
        if !preset.init_eqs_eel.is_empty() {
            let ast = parse(&preset.init_eqs_eel);
            self.eel_env.run(&ast, &mut self.regs);
        }
        for (i, wp) in preset.waves.iter().enumerate() {
            if !wp.init_eqs_eel.is_empty() && i < 8 {
                let ast = parse(&wp.init_eqs_eel);
                self.wave_envs[i].run(&ast, &mut self.regs);
            }
        }
        for (i, sp) in preset.shapes.iter().enumerate() {
            if !sp.init_eqs_eel.is_empty() && i < 4 {
                let ast = parse(&sp.init_eqs_eel);
                self.shape_envs[i].run(&ast, &mut self.regs);
            }
        }

        let _ = blend_time; // TODO: blend support
        let _ = self.renderer.load_preset(&preset);
        self.preset = Some(preset);
    }

    pub fn set_audio_data(&mut self, time_domain: &[f32], freq: &[f32]) {
        self.time_domain.clear();
        self.time_domain.extend_from_slice(time_domain);
        self.freq.clear();
        self.freq.extend_from_slice(freq);
        self.fft_size = freq.len() * 2;
    }

    pub fn set_renderer_size(&mut self, width: u32, height: u32) {
        let _ = self.renderer.resize(width, height, self.texture_ratio);
    }

    pub fn render(&mut self, now_ms: f64) {
        if self.start_time == 0.0 {
            self.start_time = now_ms;
            self.last_time = now_ms;
        }

        let elapsed = (now_ms - self.start_time) / 1000.0;
        let dt = ((now_ms - self.last_time) / 1000.0) as f32;
        self.last_time = now_ms;

        // FPS smoothing
        self.fps_accum += if dt > 0.0 { 1.0 / dt } else { 60.0 };
        self.fps_count += 1;
        if self.fps_count >= 10 {
            self.fps = self.fps_accum / self.fps_count as f32;
            self.fps_accum = 0.0;
            self.fps_count = 0;
        }

        // Update audio levels
        self.audio.update(&self.freq, self.sample_rate, self.fft_size, self.fps);

        // Update animated noise textures
        self.renderer.textures.update_animated(&self.renderer.gl, dt as f64);

        if let Some(preset) = &self.preset.clone() {
            let bv = &preset.base_vals;

            // Set EEL inputs
            let audio = &self.audio;
            self.eel_env.set_audio(
                audio.bass() as f64, audio.mid() as f64, audio.treb() as f64, audio.vol() as f64,
                audio.bass_att() as f64, audio.mid_att() as f64, audio.treb_att() as f64, audio.vol_att() as f64,
            );
            self.eel_env.set_time(elapsed, self.fps as f64, self.frame);

            // Copy base_vals into EEL env so frame_eqs can override
            macro_rules! copy_bv {
                ($($field:ident),*) => {
                    $(self.eel_env.set(stringify!($field), bv.$field as f64);)*
                };
            }
            copy_bv!(
                zoom, zoomexp, rot, warp, warpscale, warpanimspeed,
                cx, cy, dx, dy, sx, sy,
                wave_r, wave_g, wave_b, wave_a, wave_x, wave_y, wave_scale,
                wave_smoothing, wave_mystery, wave_mode,
                decay, gammaadj
            );

            // Run frame_eqs_eel
            if !preset.frame_eqs_eel.is_empty() {
                let ast = parse(&preset.frame_eqs_eel);
                self.eel_env.run(&ast, &mut self.regs);
            }

            self.renderer.render_frame(
                preset,
                &self.audio,
                &mut self.eel_env,
                &mut self.wave_envs,
                &mut self.shape_envs,
                &mut self.regs,
                elapsed,
                self.fps as f64,
                self.frame,
                dt.max(1.0 / 144.0),
                &self.time_domain.clone(),
                &self.freq.clone(),
            );
        }

        self.frame += 1;
    }

    pub fn load_extra_images(&mut self, image_data: &wasm_bindgen::JsValue) -> Result<(), String> {
        self.renderer
            .textures
            .load_extra_images(&self.renderer.gl, image_data)
    }
}
