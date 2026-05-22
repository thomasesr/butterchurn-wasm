use std::f32::consts::TAU;
use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlVertexArrayObject};

use crate::preset::ShapePreset;
use crate::eel::{EelEnv, parse};
use super::gl::create_program;

const SHAPE_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec4 a_color;
out vec4 vColor;
void main() {
    vColor = a_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const SHAPE_FRAG: &str = r#"#version 300 es
precision mediump float;
in vec4 vColor;
out vec4 fragColor;
void main() {
    fragColor = vColor;
}
"#;

pub struct ShapeRenderer {
    prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
}

impl ShapeRenderer {
    pub fn new(gl: &GL) -> Result<Self, String> {
        let prog = create_program(gl, SHAPE_VERT, SHAPE_FRAG)?;
        let vao = gl.create_vertex_array().ok_or("create_vertex_array")?;
        let vbo = gl.create_buffer().ok_or("create_buffer")?;

        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.enable_vertex_attrib_array(0);
        // pos: 2 floats at offset 0
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        // color: 4 floats at offset 8
        gl.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, 24, 8);
        gl.bind_vertex_array(None);

        Ok(Self { prog, vao, vbo })
    }

    pub fn render(
        &self,
        gl: &GL,
        shapes: &[ShapePreset],
        envs: &mut [EelEnv],
        regs: &mut [f64; 100],
        time: f64,
        fps: f64,
        frame: u64,
    ) {
        gl.use_program(Some(&self.prog));

        for (idx, shape_preset) in shapes.iter().enumerate() {
            let sv = &shape_preset.base_vals;
            if sv.enabled < 0.5 {
                continue;
            }

            let env = &mut envs[idx];
            env.set_time(time, fps, frame);
            env.set("x", sv.x as f64);
            env.set("y", sv.y as f64);
            env.set("r", sv.r as f64);
            env.set("g", sv.g as f64);
            env.set("b", sv.b as f64);
            env.set("a", sv.a as f64);
            env.set("r2", sv.r2 as f64);
            env.set("g2", sv.g2 as f64);
            env.set("b2", sv.b2 as f64);
            env.set("a2", sv.a2 as f64);
            env.set("rad", sv.rad as f64);
            env.set("ang", sv.ang as f64);
            env.set("sides", sv.sides as f64);
            env.set("additive", sv.additive as f64);

            if !shape_preset.frame_eqs_eel.is_empty() {
                let ast = parse(&shape_preset.frame_eqs_eel);
                env.run(&ast, regs);
            }

            let x = env.get("x") as f32;
            let y = env.get("y") as f32;
            let r = env.get("r") as f32;
            let g = env.get("g") as f32;
            let b = env.get("b") as f32;
            let a = env.get("a") as f32;
            let r2 = env.get("r2") as f32;
            let g2 = env.get("g2") as f32;
            let b2 = env.get("b2") as f32;
            let a2 = env.get("a2") as f32;
            let rad = env.get("rad") as f32;
            let ang = env.get("ang") as f32;
            let sides = (env.get("sides") as usize).clamp(3, 100);
            let additive = env.get("additive") > 0.5;

            // Triangle fan: center + N rim verts
            // Interleaved: [px, py, cr, cg, cb, ca] per vertex (6 f32 = 24 bytes)
            let cx = x * 2.0 - 1.0;
            let cy = -(y * 2.0 - 1.0); // flip Y for GL

            let mut verts: Vec<f32> = Vec::with_capacity((sides + 2) * 6);
            // center vertex — outer color
            verts.extend_from_slice(&[cx, cy, r, g, b, a]);

            for i in 0..=sides {
                let t = i as f32 / sides as f32;
                let angle = t * TAU + ang;
                let px = cx + angle.cos() * rad * 2.0;
                let py = cy + angle.sin() * rad * 2.0;
                // rim interpolates between outer (r,g,b,a) and inner (r2,g2,b2,a2)
                let blend = t;
                verts.extend_from_slice(&[
                    px, py,
                    r * (1.0 - blend) + r2 * blend,
                    g * (1.0 - blend) + g2 * blend,
                    b * (1.0 - blend) + b2 * blend,
                    a * (1.0 - blend) + a2 * blend,
                ]);
            }

            gl.bind_vertex_array(Some(&self.vao));
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
            unsafe {
                let view = js_sys::Float32Array::view(&verts);
                gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
            }

            if additive {
                gl.blend_func(GL::SRC_ALPHA, GL::ONE);
            } else {
                gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
            }
            gl.enable(GL::BLEND);
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, (sides + 2) as i32);
            gl.disable(GL::BLEND);
            gl.bind_vertex_array(None);
        }
    }
}
