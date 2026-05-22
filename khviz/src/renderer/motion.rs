use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlVertexArrayObject};

use crate::preset::BaseVals;
use super::gl::create_program;

const MV_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const MV_FRAG: &str = r#"#version 300 es
precision mediump float;
uniform vec4 u_color;
out vec4 fragColor;
void main() {
    fragColor = u_color;
}
"#;

pub struct MotionRenderer {
    prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
}

impl MotionRenderer {
    pub fn new(gl: &GL) -> Result<Self, String> {
        let prog = create_program(gl, MV_VERT, MV_FRAG)?;
        let vao = gl.create_vertex_array().ok_or("create_vertex_array")?;
        let vbo = gl.create_buffer().ok_or("create_buffer")?;
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        gl.bind_vertex_array(None);
        Ok(Self { prog, vao, vbo })
    }

    /// Render motion vectors using the warp mesh sample UVs to show displacement.
    /// `mesh_uvs` is the flat array [su, sv, su, sv, ...] from WarpPass::compute_mesh.
    pub fn render(
        &self,
        gl: &GL,
        bv: &BaseVals,
        mesh_w: usize,
        mesh_h: usize,
        sample_uvs: &[f32],
    ) {
        if bv.bmotionvectorson < 0.5 || bv.mv_a < 1e-4 {
            return;
        }

        let nx = (bv.mv_x as usize).max(1).min(mesh_w);
        let ny = (bv.mv_y as usize).max(1).min(mesh_h);

        let mut lines: Vec<f32> = Vec::with_capacity(nx * ny * 4);

        for gy in 0..ny {
            for gx in 0..nx {
                // Map grid to mesh vertex index
                let mx = gx * (mesh_w - 1) / nx.max(1);
                let my = gy * (mesh_h - 1) / ny.max(1);
                let vi = my * mesh_w + mx;

                let src_u = gx as f32 / (nx - 1).max(1) as f32;
                let src_v = gy as f32 / (ny - 1).max(1) as f32;

                let dst_u = if vi * 2 + 1 < sample_uvs.len() {
                    sample_uvs[vi * 2]
                } else {
                    src_u
                };
                let dst_v = if vi * 2 + 1 < sample_uvs.len() {
                    sample_uvs[vi * 2 + 1]
                } else {
                    src_v
                };

                // convert UV [0,1] to clip [-1,1]
                lines.push(src_u * 2.0 - 1.0);
                lines.push(src_v * 2.0 - 1.0);
                lines.push(dst_u * 2.0 - 1.0);
                lines.push(dst_v * 2.0 - 1.0);
            }
        }

        gl.bind_vertex_array(Some(&self.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        unsafe {
            let view = js_sys::Float32Array::view(&lines);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }

        gl.use_program(Some(&self.prog));
        if let Some(loc) = gl.get_uniform_location(&self.prog, "u_color") {
            gl.uniform4f(Some(&loc), bv.mv_r, bv.mv_g, bv.mv_b, bv.mv_a);
        }

        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
        gl.draw_arrays(GL::LINES, 0, (lines.len() / 2) as i32);
        gl.disable(GL::BLEND);
        gl.bind_vertex_array(None);
    }
}
