use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlVertexArrayObject};

use crate::preset::{BaseVals, WavePreset, WaveVals};
use crate::eel::{EelEnv, parse, Ast};
use super::gl::create_program;

const WAVE_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
    gl_PointSize = 3.0;
}
"#;

const WAVE_FRAG: &str = r#"#version 300 es
precision mediump float;
uniform vec4 u_color;
out vec4 fragColor;
void main() {
    fragColor = u_color;
}
"#;

pub struct WaveRenderer {
    prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
}

impl WaveRenderer {
    pub fn new(gl: &GL) -> Result<Self, String> {
        let prog = create_program(gl, WAVE_VERT, WAVE_FRAG)?;
        let vao = gl.create_vertex_array().ok_or("create_vertex_array")?;
        let vbo = gl.create_buffer().ok_or("create_buffer")?;
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        gl.bind_vertex_array(None);
        Ok(Self { prog, vao, vbo })
    }

    pub fn render(
        &self,
        gl: &GL,
        waves: &[WavePreset],
        bv: &BaseVals,
        time_domain: &[f32],
        freq: &[f32],
        envs: &mut [EelEnv],
        regs: &mut [f64; 100],
        time: f64,
        fps: f64,
        frame: u64,
    ) {
        gl.use_program(Some(&self.prog));

        for (idx, wave_preset) in waves.iter().enumerate() {
            let wv_base = &wave_preset.base_vals;
            if wv_base.enabled < 0.5 {
                continue;
            }

            // Run wave frame EEL
            let env = &mut envs[idx];
            env.set_time(time, fps, frame);
            // copy base vals into env so EEL can override them
            env.set("r", wv_base.r as f64);
            env.set("g", wv_base.g as f64);
            env.set("b", wv_base.b as f64);
            env.set("a", wv_base.a as f64);
            env.set("samples", wv_base.samples as f64);
            env.set("sep", wv_base.sep as f64);
            env.set("scaling", wv_base.scaling as f64);
            env.set("spectrum", wv_base.spectrum as f64);
            env.set("smoothing", wv_base.smoothing as f64);
            env.set("usedots", wv_base.usedots as f64);
            env.set("additive", wv_base.additive as f64);
            env.set("thick", wv_base.thick as f64);

            if !wave_preset.frame_eqs_eel.is_empty() {
                let ast = parse(&wave_preset.frame_eqs_eel);
                env.run(&ast, regs);
            }

            // Read back potentially-overridden values
            let r = env.get("r") as f32;
            let g = env.get("g") as f32;
            let b = env.get("b") as f32;
            let a = env.get("a") as f32;
            let samples = (env.get("samples") as usize).min(512).max(2);
            let spectrum = env.get("spectrum") > 0.5;
            let usedots = env.get("usedots") > 0.5;
            let smoothing = env.get("smoothing") as f32;
            let scaling = env.get("scaling") as f32;
            let sep = env.get("sep") as f32;

            let src = if spectrum { freq } else { time_domain };
            let src_len = src.len();
            if src_len < 2 {
                continue;
            }

            let mut pts: Vec<f32> = Vec::with_capacity(samples * 2);
            for i in 0..samples {
                let t = i as f32 / (samples - 1) as f32;
                let si = ((t * src_len as f32) as usize).min(src_len - 1);
                let s = if spectrum {
                    // linear-magnitude spectrum, src is already linear
                    let half = src_len / 2;
                    if si < half { src[si] } else { 0.0 }
                } else {
                    src[si]
                };

                // Apply global wave mode
                let (px, py) = wave_xy(bv, t, s * scaling * bv.wave_scale, sep);
                pts.push(px * 2.0 - 1.0);
                pts.push(py * 2.0 - 1.0);
            }

            // Optional smoothing pass
            if smoothing > 0.0 {
                let sm = smoothing;
                for i in 1..samples {
                    pts[i * 2] = pts[(i - 1) * 2] * sm + pts[i * 2] * (1.0 - sm);
                    pts[i * 2 + 1] = pts[(i - 1) * 2 + 1] * sm + pts[i * 2 + 1] * (1.0 - sm);
                }
            }

            gl.bind_vertex_array(Some(&self.vao));
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
            unsafe {
                let view = js_sys::Float32Array::view(&pts);
                gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }

            if let Some(loc) = gl.get_uniform_location(&self.prog, "u_color") {
                gl.uniform4f(Some(&loc), r, g, b, a);
            }

            let additive = env.get("additive") > 0.5;
            if additive {
                gl.blend_func(GL::SRC_ALPHA, GL::ONE);
            } else {
                gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
            }
            gl.enable(GL::BLEND);

            let prim = if usedots { GL::POINTS } else { GL::LINE_STRIP };
            gl.draw_arrays(prim, 0, samples as i32);

            gl.disable(GL::BLEND);
            gl.bind_vertex_array(None);
        }
    }
}

/// Map waveform sample to screen position based on global wave_mode.
fn wave_xy(bv: &BaseVals, t: f32, s: f32, sep: f32) -> (f32, f32) {
    use std::f32::consts::TAU;
    let mode = bv.wave_mode as u32;
    let wx = bv.wave_x;
    let wy = bv.wave_y;
    match mode {
        1 => (t, wy + s),
        2 => (wx + s, t),
        3 => {
            // x-y oscilloscope: needs stereo; approximate with mono
            (wx + s, wy + s)
        }
        4 => {
            let angle = t * TAU;
            let r = 0.3 + s * 0.2;
            (0.5 + angle.cos() * r, 0.5 + angle.sin() * r)
        }
        _ => {
            // mode 0: center blob
            let angle = t * TAU;
            (wx + angle.cos() * (0.3 + s * 0.1), wy + angle.sin() * (0.3 + s * 0.1))
        }
    }
}
