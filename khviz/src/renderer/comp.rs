use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlVertexArrayObject};

use crate::preset::BaseVals;
use super::gl::{build_preset_frag, create_program, FULLSCREEN_VERT};

pub struct CompPass {
    pub prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
}

const FALLBACK_COMP_BODY: &str = r#"
    ret = texture(sampler_main, uv).rgb;
    ret = pow(max(ret, vec3(0.0)), vec3(1.0 / max(gammaadj, 0.01)));
"#;

// Full-screen triangle positions
static QUAD: [f32; 6] = [
    -1.0, -1.0,
     3.0, -1.0,
    -1.0,  3.0,
];

impl CompPass {
    pub fn new(gl: &GL, body: &str) -> Result<Self, String> {
        let frag_body = if body.trim().is_empty() { FALLBACK_COMP_BODY } else { body };
        // gammaadj needs to be a uniform in the body context; add it
        let frag_with_gamma = format!(
            "uniform float gammaadj;\n{}",
            build_preset_frag(frag_body)
        );
        let prog = create_program(gl, FULLSCREEN_VERT, &frag_with_gamma)?;

        let vao = gl.create_vertex_array().ok_or("create_vertex_array")?;
        let vbo = gl.create_buffer().ok_or("create_buffer")?;

        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let view = js_sys::Float32Array::view(&QUAD);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
        }
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        gl.bind_vertex_array(None);

        Ok(Self { prog, vao, vbo })
    }

    pub fn update_program(&mut self, gl: &GL, body: &str) -> Result<(), String> {
        let frag_body = if body.trim().is_empty() { FALLBACK_COMP_BODY } else { body };
        let frag_with_gamma = format!(
            "uniform float gammaadj;\n{}",
            build_preset_frag(frag_body)
        );
        let prog = create_program(gl, FULLSCREEN_VERT, &frag_with_gamma)?;
        gl.delete_program(Some(&self.prog));
        self.prog = prog;
        Ok(())
    }

    pub fn render(
        &self,
        gl: &GL,
        src_tex: &web_sys::WebGlTexture,
        dst_fb: &web_sys::WebGlFramebuffer,
        w: i32,
        h: i32,
    ) {
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(dst_fb));
        gl.viewport(0, 0, w, h);
        gl.use_program(Some(&self.prog));

        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(src_tex));
        if let Some(loc) = gl.get_uniform_location(&self.prog, "sampler_main") {
            gl.uniform1i(Some(&loc), 0);
        }
        if let Some(loc) = gl.get_uniform_location(&self.prog, "u_color") {
            gl.uniform4f(Some(&loc), 1.0, 1.0, 1.0, 1.0);
        }

        gl.bind_vertex_array(Some(&self.vao));
        gl.draw_arrays(GL::TRIANGLES, 0, 3);
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
        u1f!("gammaadj", bv.gammaadj);
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
        let ql = ["_qa","_qb","_qc","_qd","_qe","_qf","_qg","_qh"];
        for (i, name) in ql.iter().enumerate() {
            let o = i * 4;
            u4f!(name, q[o], q[o+1], q[o+2], q[o+3]);
        }
        u4f!("rand_preset", rand_preset[0], rand_preset[1], rand_preset[2], rand_preset[3]);
        u4f!("rand_frame", rand_frame[0], rand_frame[1], rand_frame[2], rand_frame[3]);
    }
}
