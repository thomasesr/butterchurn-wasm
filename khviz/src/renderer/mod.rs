pub mod blur;
pub mod comp;
pub mod gl;
pub mod motion;
pub mod output;
pub mod shapes;
pub mod waves;
pub mod warp;

use web_sys::{WebGl2RenderingContext as GL, WebGlFramebuffer, WebGlTexture};

use blur::BlurPass;
use comp::CompPass;
use gl::{create_fbo, create_texture_f32};
use motion::MotionRenderer;
use output::OutputPass;
use shapes::ShapeRenderer;
use waves::WaveRenderer;
use warp::WarpPass;

use crate::audio::AudioLevels;
use crate::eel::EelEnv;
use crate::preset::{BaseVals, Preset};
use crate::textures::Textures;

pub struct FrameBuffer {
    pub tex: WebGlTexture,
    pub fb: WebGlFramebuffer,
}

impl FrameBuffer {
    fn new(gl: &GL, w: i32, h: i32) -> Result<Self, String> {
        let tex = create_texture_f32(gl, w, h)?;
        let fb = create_fbo(gl, &tex)?;
        Ok(Self { tex, fb })
    }
}

pub struct Renderer {
    pub gl: GL,
    pub tex_w: i32,
    pub tex_h: i32,
    pub canvas_w: i32,
    pub canvas_h: i32,

    fb_main: [FrameBuffer; 2],
    fb_output: FrameBuffer,
    ping_idx: usize,

    warp: WarpPass,
    comp: CompPass,
    blur: BlurPass,
    waves: WaveRenderer,
    shapes: ShapeRenderer,
    motion: MotionRenderer,
    output: OutputPass,

    pub textures: Textures,

    pub rand_preset: [f32; 4],

    // cached last mesh sample UVs for motion vectors
    last_sample_uvs: Vec<f32>,
}

pub const MESH_W: usize = 48;
pub const MESH_H: usize = 36;

impl Renderer {
    pub fn new(
        gl: GL,
        canvas_w: u32,
        canvas_h: u32,
        texture_ratio: f32,
        fxaa: bool,
        preset: Option<&Preset>,
    ) -> Result<Self, String> {
        let tex_w = ((canvas_w as f32 * texture_ratio) as i32).max(1);
        let tex_h = ((canvas_h as f32 * texture_ratio) as i32).max(1);

        let fb_main = [
            FrameBuffer::new(&gl, tex_w, tex_h)?,
            FrameBuffer::new(&gl, tex_w, tex_h)?,
        ];
        let fb_output = FrameBuffer::new(&gl, tex_w, tex_h)?;

        // Seed both ping-pong buffers so the first warp pass has non-black to decay.
        for i in 0..2 {
            gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&fb_main[i].fb));
            gl.clear_color(0.04, 0.02, 0.08, 1.0);
            gl.clear(GL::COLOR_BUFFER_BIT);
        }
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);

        let (warp_body, comp_body) = if let Some(p) = preset {
            (p.warp_glsl.as_str(), p.comp_glsl.as_str())
        } else {
            ("", "")
        };

        let warp = WarpPass::new(&gl, MESH_W, MESH_H, warp_body)?;
        let comp = CompPass::new(&gl, comp_body)?;
        let blur = BlurPass::new(&gl, tex_w, tex_h)?;
        let waves = WaveRenderer::new(&gl)?;
        let shapes = ShapeRenderer::new(&gl)?;
        let motion = MotionRenderer::new(&gl)?;
        let output = OutputPass::new(&gl, fxaa)?;
        let textures = Textures::new(&gl);

        let rand_preset = [fastrand::f32(), fastrand::f32(), fastrand::f32(), fastrand::f32()];

        Ok(Self {
            gl,
            tex_w,
            tex_h,
            canvas_w: canvas_w as i32,
            canvas_h: canvas_h as i32,
            fb_main,
            fb_output,
            ping_idx: 0,
            warp,
            comp,
            blur,
            waves,
            shapes,
            motion,
            output,
            textures,
            rand_preset,
            last_sample_uvs: Vec::new(),
        })
    }

    pub fn load_preset(&mut self, preset: &Preset) -> Result<(), String> {
        self.warp.update_program(&self.gl, &preset.warp_glsl)?;
        self.comp.update_program(&self.gl, &preset.comp_glsl)?;
        self.rand_preset = [fastrand::f32(), fastrand::f32(), fastrand::f32(), fastrand::f32()];
        Ok(())
    }

    pub fn resize(&mut self, canvas_w: u32, canvas_h: u32, texture_ratio: f32) -> Result<(), String> {
        self.canvas_w = canvas_w as i32;
        self.canvas_h = canvas_h as i32;
        let tex_w = ((canvas_w as f32 * texture_ratio) as i32).max(1);
        let tex_h = ((canvas_h as f32 * texture_ratio) as i32).max(1);
        if tex_w == self.tex_w && tex_h == self.tex_h {
            return Ok(());
        }
        self.tex_w = tex_w;
        self.tex_h = tex_h;
        self.fb_main[0] = FrameBuffer::new(&self.gl, tex_w, tex_h)?;
        self.fb_main[1] = FrameBuffer::new(&self.gl, tex_w, tex_h)?;
        self.fb_output = FrameBuffer::new(&self.gl, tex_w, tex_h)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_frame(
        &mut self,
        preset: &Preset,
        audio: &AudioLevels,
        eel_env: &mut EelEnv,
        wave_envs: &mut [EelEnv],
        shape_envs: &mut [EelEnv],
        regs: &mut [f64; 100],
        time: f64,
        fps: f64,
        frame: u64,
        dt: f32,
        time_domain: &[f32],
        freq: &[f32],
    ) {
        let gl = &self.gl;
        let q = eel_env.get_q_vals();
        let rand_frame = [fastrand::f32(), fastrand::f32(), fastrand::f32(), fastrand::f32()];
        let bv = &preset.base_vals;
        let tw = self.tex_w as f32;
        let th = self.tex_h as f32;
        let t = time as f32;

        let prev = self.ping_idx;
        let curr = 1 - prev;

        // Upload warp uniforms and render warp pass
        self.warp.upload_uniforms(
            gl, t, fps as f32, frame as u32, bv, audio, &q,
            tw, th, self.rand_preset, rand_frame,
        );
        self.warp.render(
            gl, bv, dt,
            &self.fb_main[prev].tex,
            &self.fb_main[curr].fb,
            self.tex_w, self.tex_h,
        );

        // Blur
        self.blur.render(gl, &self.fb_main[curr].tex);

        // Bind blur textures for comp pass
        // Motion vectors
        let (_, sample_uvs, _, _) = self.warp.compute_mesh(bv, 0.0);
        self.last_sample_uvs = sample_uvs.clone();
        self.motion.render(gl, bv, MESH_W, MESH_H, &sample_uvs);

        // Shapes
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&self.fb_main[curr].fb));
        self.shapes.render(
            gl, &preset.shapes, shape_envs, regs,
            time, fps, frame,
        );

        // Waves
        self.waves.render(
            gl, &preset.waves, bv, time_domain, freq,
            wave_envs, regs, time, fps, frame,
        );
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);

        // Comp pass
        self.comp.upload_uniforms(
            gl, t, fps as f32, frame as u32, bv, audio, &q,
            tw, th, self.rand_preset, rand_frame,
        );
        // Bind blur levels to comp shader
        let comp_prog = &self.comp;
        gl.use_program(None); // comp uploads inside its own method
        self.bind_blur_to_comp(gl, comp_prog);
        self.comp.render(
            gl,
            &self.fb_main[curr].tex,
            &self.fb_output.fb,
            self.tex_w, self.tex_h,
        );

        // Output to canvas
        self.output.render(gl, &self.fb_output.tex, self.canvas_w, self.canvas_h);

        self.ping_idx = curr;
    }

    fn bind_blur_to_comp(&self, gl: &GL, comp: &CompPass) {
        // This requires access to the program — workaround: use pub prog
        // Textures for blur1/2/3 are bound here
        let prog = &comp.prog;
        gl.use_program(Some(prog));

        gl.active_texture(GL::TEXTURE1);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.blur.levels[0].tex));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_blur1") {
            gl.uniform1i(Some(&loc), 1);
        }
        gl.active_texture(GL::TEXTURE2);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.blur.levels[1].tex));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_blur2") {
            gl.uniform1i(Some(&loc), 2);
        }
        gl.active_texture(GL::TEXTURE3);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.blur.levels[2].tex));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_blur3") {
            gl.uniform1i(Some(&loc), 3);
        }

        // Noise textures
        gl.active_texture(GL::TEXTURE4);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.textures.noise_lq));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_noise_lq") {
            gl.uniform1i(Some(&loc), 4);
        }
        gl.active_texture(GL::TEXTURE5);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.textures.noise_mq));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_noise_mq") {
            gl.uniform1i(Some(&loc), 5);
        }
        gl.active_texture(GL::TEXTURE6);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.textures.noise_hq));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_noise_hq") {
            gl.uniform1i(Some(&loc), 6);
        }
        gl.active_texture(GL::TEXTURE7);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.textures.noise_lq_lite));
        if let Some(loc) = gl.get_uniform_location(prog, "sampler_noise_lq_lite") {
            gl.uniform1i(Some(&loc), 7);
        }
    }
}
