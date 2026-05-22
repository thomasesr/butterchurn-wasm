use std::f32::consts::TAU;

use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlVertexArrayObject};

use crate::preset::BaseVals;
use super::gl::{build_preset_frag, create_program, FULLSCREEN_VERT};

pub struct WarpPass {
    prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo_pos: WebGlBuffer,
    vbo_uv: WebGlBuffer,
    vbo_color: WebGlBuffer,
    ibo: WebGlBuffer,
    mesh_w: usize,
    mesh_h: usize,
    warp_time: f32,
}

const WARP_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec4 a_color;
out vec2 uv;
out vec2 uv_orig;
out vec4 vColor;
void main() {
    uv = a_uv;
    uv_orig = a_pos * 0.5 + 0.5;
    vColor = a_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const FALLBACK_BODY: &str = r#"
    ret = texture(sampler_main, uv).rgb * decay;
"#;

impl WarpPass {
    pub fn new(gl: &GL, mesh_w: usize, mesh_h: usize, body: &str) -> Result<Self, String> {
        let frag_body = if body.trim().is_empty() { FALLBACK_BODY } else { body };
        let frag_src = build_preset_frag(frag_body);
        let prog = create_program(gl, WARP_VERT, &frag_src)?;

        let vao = gl
            .create_vertex_array()
            .ok_or("create_vertex_array failed")?;
        let vbo_pos = gl.create_buffer().ok_or("create_buffer failed")?;
        let vbo_uv = gl.create_buffer().ok_or("create_buffer failed")?;
        let vbo_color = gl.create_buffer().ok_or("create_buffer failed")?;
        let ibo = gl.create_buffer().ok_or("create_buffer failed")?;

        Ok(Self {
            prog,
            vao,
            vbo_pos,
            vbo_uv,
            vbo_color,
            ibo,
            mesh_w,
            mesh_h,
            warp_time: 0.0,
        })
    }

    pub fn update_program(&mut self, gl: &GL, body: &str) -> Result<(), String> {
        let frag_body = if body.trim().is_empty() { FALLBACK_BODY } else { body };
        let frag_src = build_preset_frag(frag_body);
        let prog = create_program(gl, WARP_VERT, &frag_src)?;
        gl.delete_program(Some(&self.prog));
        self.prog = prog;
        Ok(())
    }

    pub fn compute_mesh(
        &mut self,
        bv: &BaseVals,
        dt: f32,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>) {
        let mw = self.mesh_w;
        let mh = self.mesh_h;

        self.warp_time += dt * bv.warpanimspeed * 0.5 * bv.warpscale;
        let wt = self.warp_time;
        let wt1 = wt * 1.413 + 3.900;
        let wt2 = wt * 1.731 + 7.800;
        let wt3 = wt * 1.234 + 15.600;
        let wt4 = wt * 1.512 + 22.200;

        let n_verts = mw * mh;
        let mut pos = Vec::with_capacity(n_verts * 2);
        let mut sample_uv = Vec::with_capacity(n_verts * 2);
        let mut color = Vec::with_capacity(n_verts * 4);

        let cos_rot = (-bv.rot * TAU).cos();
        let sin_rot = (-bv.rot * TAU).sin();

        for gy in 0..mh {
            for gx in 0..mw {
                let u = gx as f32 / (mw - 1) as f32;
                let v = gy as f32 / (mh - 1) as f32;

                let clip_x = u * 2.0 - 1.0;
                let clip_y = v * 2.0 - 1.0;
                pos.push(clip_x);
                pos.push(clip_y);

                let ux = u - 0.5;
                let uy = v - 0.5;

                let r = (ux * ux + uy * uy).sqrt();
                let zoom_factor = if r < 1e-6 {
                    bv.zoom
                } else {
                    bv.zoom * r.powf(bv.zoomexp - 1.0)
                };

                let ux2 = (ux * cos_rot - uy * sin_rot) / zoom_factor / bv.sx;
                let uy2 = (ux * sin_rot + uy * cos_rot) / zoom_factor / bv.sy;

                let mut su = 0.5 + ux2 - bv.dx;
                let mut sv = 0.5 + uy2 - bv.dy;

                su += bv.warp * (wt1 * u + wt3 * v).sin();
                sv += bv.warp * (wt2 * u + wt4 * v).cos();

                sample_uv.push(su);
                sample_uv.push(sv);

                let d = bv.decay;
                color.push(d);
                color.push(d);
                color.push(d);
                color.push(d);
            }
        }

        let mut indices = Vec::with_capacity((mw - 1) * (mh - 1) * 6);
        for gy in 0..mh - 1 {
            for gx in 0..mw - 1 {
                let tl = (gy * mw + gx) as u32;
                let tr = tl + 1;
                let bl = tl + mw as u32;
                let br = bl + 1;
                indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
            }
        }

        (pos, sample_uv, color, indices)
    }

    pub fn render(
        &mut self,
        gl: &GL,
        bv: &BaseVals,
        dt: f32,
        src_tex: &web_sys::WebGlTexture,
        dst_fb: &web_sys::WebGlFramebuffer,
        w: i32,
        h: i32,
    ) {
        let (pos, uv_data, color, indices) = self.compute_mesh(bv, dt);

        gl.bind_vertex_array(Some(&self.vao));

        // position buffer
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo_pos));
        unsafe {
            let view = js_sys::Float32Array::view(&pos);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);

        // uv buffer
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo_uv));
        unsafe {
            let view = js_sys::Float32Array::view(&uv_data);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 0, 0);

        // color (decay) per-vertex
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo_color));
        unsafe {
            let view = js_sys::Float32Array::view(&color);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, 0, 0);

        // index buffer
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.ibo));
        unsafe {
            let view = js_sys::Uint32Array::view(&indices);
            gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }

        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(dst_fb));
        gl.viewport(0, 0, w, h);
        gl.use_program(Some(&self.prog));

        // bind sampler_main
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(src_tex));
        if let Some(loc) = gl.get_uniform_location(&self.prog, "sampler_main") {
            gl.uniform1i(Some(&loc), 0);
        }

        gl.draw_elements_with_i32(GL::TRIANGLES, indices.len() as i32, GL::UNSIGNED_INT, 0);

        gl.bind_vertex_array(None);
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
    }

    pub fn upload_uniforms(
        &self,
        gl: &GL,
        time: f32,
        fps: f32,
        frame: u32,
        bv: &BaseVals,
        audio: &crate::audio::AudioLevels,
        q: &[f32; 32],
        tex_w: f32,
        tex_h: f32,
        rand_preset: [f32; 4],
        rand_frame: [f32; 4],
    ) {
        let p = &self.prog;
        macro_rules! u1f {
            ($name:expr, $val:expr) => {
                if let Some(loc) = gl.get_uniform_location(p, $name) {
                    gl.uniform1f(Some(&loc), $val);
                }
            };
        }
        macro_rules! u4f {
            ($name:expr, $a:expr, $b:expr, $c:expr, $d:expr) => {
                if let Some(loc) = gl.get_uniform_location(p, $name) {
                    gl.uniform4f(Some(&loc), $a, $b, $c, $d);
                }
            };
        }
        gl.use_program(Some(p));
        u1f!("time", time);
        u1f!("fps", fps);
        u1f!("frame", frame as f32);
        u1f!("decay", bv.decay);
        u1f!("bass", audio.bass());
        u1f!("mid", audio.mid());
        u1f!("treb", audio.treb());
        u1f!("vol", audio.vol());
        u1f!("bass_att", audio.bass_att());
        u1f!("mid_att", audio.mid_att());
        u1f!("treb_att", audio.treb_att());
        u1f!("vol_att", audio.vol_att());
        if let Some(loc) = gl.get_uniform_location(p, "resolution") {
            gl.uniform2f(Some(&loc), tex_w, tex_h);
        }
        let ax = if tex_w > tex_h { tex_w / tex_h } else { 1.0 };
        let ay = if tex_h > tex_w { tex_h / tex_w } else { 1.0 };
        u4f!("aspect", ax, ay, 1.0 / ax, 1.0 / ay);
        u4f!("texsize", tex_w, tex_h, 1.0 / tex_w, 1.0 / tex_h);
        // Q uniforms
        let ql = ["_qa","_qb","_qc","_qd","_qe","_qf","_qg","_qh"];
        for (i, name) in ql.iter().enumerate() {
            let o = i * 4;
            u4f!(name, q[o], q[o+1], q[o+2], q[o+3]);
        }
        u4f!("rand_preset", rand_preset[0], rand_preset[1], rand_preset[2], rand_preset[3]);
        u4f!("rand_frame", rand_frame[0], rand_frame[1], rand_frame[2], rand_frame[3]);
    }
}
