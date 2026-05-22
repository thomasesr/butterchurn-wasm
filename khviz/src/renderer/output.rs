use web_sys::{WebGl2RenderingContext as GL, WebGlProgram, WebGlBuffer, WebGlVertexArrayObject};

use super::gl::create_program;

const OUTPUT_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
out vec2 uv;
void main() {
    uv = a_pos * 0.5 + 0.5;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const OUTPUT_FRAG_PLAIN: &str = r#"#version 300 es
precision mediump float;
in vec2 uv;
out vec4 fragColor;
uniform sampler2D sampler_main;
void main() {
    fragColor = texture(sampler_main, uv);
}
"#;

// FXAA pass adapted from Timothy Lottes' FXAA 3.11
const OUTPUT_FRAG_FXAA: &str = r#"#version 300 es
precision mediump float;
in vec2 uv;
out vec4 fragColor;
uniform sampler2D sampler_main;
uniform vec2 rcp_frame;

#define FXAA_SPAN_MAX   8.0
#define FXAA_REDUCE_MUL (1.0/8.0)
#define FXAA_REDUCE_MIN (1.0/128.0)

void main() {
    vec2 rcpFrame = rcp_frame;
    vec3 rgbNW = texture(sampler_main, uv + vec2(-1.0, -1.0) * rcpFrame).rgb;
    vec3 rgbNE = texture(sampler_main, uv + vec2( 1.0, -1.0) * rcpFrame).rgb;
    vec3 rgbSW = texture(sampler_main, uv + vec2(-1.0,  1.0) * rcpFrame).rgb;
    vec3 rgbSE = texture(sampler_main, uv + vec2( 1.0,  1.0) * rcpFrame).rgb;
    vec3 rgbM  = texture(sampler_main, uv).rgb;

    vec3 luma = vec3(0.299, 0.587, 0.114);
    float lumaNW = dot(rgbNW, luma);
    float lumaNE = dot(rgbNE, luma);
    float lumaSW = dot(rgbSW, luma);
    float lumaSE = dot(rgbSE, luma);
    float lumaM  = dot(rgbM,  luma);

    float lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    float lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    vec2 dir;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y =  ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    float dirReduce = max(
        (lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * FXAA_REDUCE_MUL),
        FXAA_REDUCE_MIN);
    float rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);
    dir = min(vec2(FXAA_SPAN_MAX), max(vec2(-FXAA_SPAN_MAX),
              dir * rcpDirMin)) * rcpFrame;

    vec3 rgbA = 0.5 * (
        texture(sampler_main, uv + dir * (1.0/3.0 - 0.5)).rgb +
        texture(sampler_main, uv + dir * (2.0/3.0 - 0.5)).rgb);
    vec3 rgbB = rgbA * 0.5 + 0.25 * (
        texture(sampler_main, uv + dir * -0.5).rgb +
        texture(sampler_main, uv + dir *  0.5).rgb);
    float lumaB = dot(rgbB, luma);
    if (lumaB < lumaMin || lumaB > lumaMax) {
        fragColor = vec4(rgbA, 1.0);
    } else {
        fragColor = vec4(rgbB, 1.0);
    }
}
"#;

static QUAD: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

pub struct OutputPass {
    prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    _vbo: WebGlBuffer,
    fxaa: bool,
}

impl OutputPass {
    pub fn new(gl: &GL, fxaa: bool) -> Result<Self, String> {
        let frag = if fxaa { OUTPUT_FRAG_FXAA } else { OUTPUT_FRAG_PLAIN };
        let prog = create_program(gl, OUTPUT_VERT, frag)?;

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

        Ok(Self { prog, vao, _vbo: vbo, fxaa })
    }

    /// Blit `src_tex` to the default framebuffer (canvas).
    pub fn render(
        &self,
        gl: &GL,
        src_tex: &web_sys::WebGlTexture,
        canvas_w: i32,
        canvas_h: i32,
    ) {
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);
        gl.viewport(0, 0, canvas_w, canvas_h);
        gl.use_program(Some(&self.prog));

        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(src_tex));
        if let Some(loc) = gl.get_uniform_location(&self.prog, "sampler_main") {
            gl.uniform1i(Some(&loc), 0);
        }
        if self.fxaa {
            if let Some(loc) = gl.get_uniform_location(&self.prog, "rcp_frame") {
                gl.uniform2f(Some(&loc), 1.0 / canvas_w as f32, 1.0 / canvas_h as f32);
            }
        }

        gl.bind_vertex_array(Some(&self.vao));
        gl.draw_arrays(GL::TRIANGLES, 0, 3);
        gl.bind_vertex_array(None);
    }
}
