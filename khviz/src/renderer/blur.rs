use web_sys::{WebGl2RenderingContext as GL, WebGlFramebuffer, WebGlProgram, WebGlTexture, WebGlVertexArrayObject, WebGlBuffer};

use super::gl::{create_fbo, create_program, create_texture_rgba8};

const BLUR_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
out vec2 uv;
void main() {
    uv = a_pos * 0.5 + 0.5;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const BLUR_FRAG_H: &str = r#"#version 300 es
precision mediump float;
in vec2 uv;
out vec4 fragColor;
uniform sampler2D src;
uniform vec2 texel;
void main() {
    vec3 c = texture(src, uv).rgb * 0.2270270270;
    c += texture(src, uv + vec2(texel.x, 0.0) * 1.3846153846).rgb * 0.3162162162;
    c += texture(src, uv - vec2(texel.x, 0.0) * 1.3846153846).rgb * 0.3162162162;
    c += texture(src, uv + vec2(texel.x, 0.0) * 3.2307692308).rgb * 0.0702702703;
    c += texture(src, uv - vec2(texel.x, 0.0) * 3.2307692308).rgb * 0.0702702703;
    fragColor = vec4(c, 1.0);
}
"#;

const BLUR_FRAG_V: &str = r#"#version 300 es
precision mediump float;
in vec2 uv;
out vec4 fragColor;
uniform sampler2D src;
uniform vec2 texel;
void main() {
    vec3 c = texture(src, uv).rgb * 0.2270270270;
    c += texture(src, uv + vec2(0.0, texel.y) * 1.3846153846).rgb * 0.3162162162;
    c += texture(src, uv - vec2(0.0, texel.y) * 1.3846153846).rgb * 0.3162162162;
    c += texture(src, uv + vec2(0.0, texel.y) * 3.2307692308).rgb * 0.0702702703;
    c += texture(src, uv - vec2(0.0, texel.y) * 3.2307692308).rgb * 0.0702702703;
    fragColor = vec4(c, 1.0);
}
"#;

static QUAD: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

pub struct BlurLevel {
    pub tex: WebGlTexture,
    pub fb: WebGlFramebuffer,
    w: i32,
    h: i32,
}

pub struct BlurPass {
    prog_h: WebGlProgram,
    prog_v: WebGlProgram,
    vao: WebGlVertexArrayObject,
    _vbo: WebGlBuffer,
    // intermediate ping-pong texture for H pass
    ping_tex: WebGlTexture,
    ping_fb: WebGlFramebuffer,
    pub levels: [BlurLevel; 3],
}

impl BlurPass {
    pub fn new(gl: &GL, base_w: i32, base_h: i32) -> Result<Self, String> {
        let prog_h = create_program(gl, BLUR_VERT, BLUR_FRAG_H)?;
        let prog_v = create_program(gl, BLUR_VERT, BLUR_FRAG_V)?;

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

        let w0 = (base_w / 2).max(1);
        let h0 = (base_h / 2).max(1);
        let w1 = (base_w / 4).max(1);
        let h1 = (base_h / 4).max(1);
        let w2 = (base_w / 8).max(1);
        let h2 = (base_h / 8).max(1);

        let ping_tex = create_texture_rgba8(gl, w0, h0)?;
        let ping_fb = create_fbo(gl, &ping_tex)?;

        let t0 = create_texture_rgba8(gl, w0, h0)?;
        let f0 = create_fbo(gl, &t0)?;
        let t1 = create_texture_rgba8(gl, w1, h1)?;
        let f1 = create_fbo(gl, &t1)?;
        let t2 = create_texture_rgba8(gl, w2, h2)?;
        let f2 = create_fbo(gl, &t2)?;

        Ok(Self {
            prog_h,
            prog_v,
            vao,
            _vbo: vbo,
            ping_tex,
            ping_fb,
            levels: [
                BlurLevel { tex: t0, fb: f0, w: w0, h: h0 },
                BlurLevel { tex: t1, fb: f1, w: w1, h: h1 },
                BlurLevel { tex: t2, fb: f2, w: w2, h: h2 },
            ],
        })
    }

    fn draw_quad(&self, gl: &GL) {
        gl.bind_vertex_array(Some(&self.vao));
        gl.draw_arrays(GL::TRIANGLES, 0, 3);
        gl.bind_vertex_array(None);
    }

    fn run_pass(
        &self,
        gl: &GL,
        prog: &WebGlProgram,
        src: &WebGlTexture,
        dst_fb: &WebGlFramebuffer,
        w: i32,
        h: i32,
        tx: f32,
        ty: f32,
    ) {
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(dst_fb));
        gl.viewport(0, 0, w, h);
        gl.use_program(Some(prog));
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(src));
        if let Some(loc) = gl.get_uniform_location(prog, "src") {
            gl.uniform1i(Some(&loc), 0);
        }
        if let Some(loc) = gl.get_uniform_location(prog, "texel") {
            gl.uniform2f(Some(&loc), tx, ty);
        }
        self.draw_quad(gl);
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
    }

    /// Downsample main texture into 3 blur levels.
    pub fn render(&self, gl: &GL, src: &WebGlTexture) {
        let src_tex = src;
        for i in 0..3 {
            let (dw, dh) = (self.levels[i].w, self.levels[i].h);
            let tx = 1.0 / dw as f32;
            let ty = 1.0 / dh as f32;

            // H pass src → ping
            self.run_pass(
                gl,
                &self.prog_h,
                if i == 0 { src_tex } else { &self.levels[i - 1].tex },
                &self.ping_fb,
                dw, dh, tx, ty,
            );
            // V pass ping → level[i]
            self.run_pass(
                gl,
                &self.prog_v,
                &self.ping_tex,
                &self.levels[i].fb,
                dw, dh, tx, ty,
            );
        }
    }
}
