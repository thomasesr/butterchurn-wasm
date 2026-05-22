use web_sys::{WebGl2RenderingContext as GL, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture};

pub fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(shader_type).ok_or("create_shader failed")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        gl.delete_shader(Some(&shader));
        Err(format!("Shader compile error: {}", log))
    }
}

pub fn link_program(
    gl: &GL,
    vert: &WebGlShader,
    frag: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let prog = gl.create_program().ok_or("create_program failed")?;
    gl.attach_shader(&prog, vert);
    gl.attach_shader(&prog, frag);
    gl.link_program(&prog);
    if gl
        .get_program_parameter(&prog, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(prog)
    } else {
        let log = gl.get_program_info_log(&prog).unwrap_or_default();
        gl.delete_program(Some(&prog));
        Err(format!("Program link error: {}", log))
    }
}

pub fn create_program(gl: &GL, vert_src: &str, frag_src: &str) -> Result<WebGlProgram, String> {
    let vert = compile_shader(gl, GL::VERTEX_SHADER, vert_src)?;
    let frag = compile_shader(gl, GL::FRAGMENT_SHADER, frag_src)?;
    let prog = link_program(gl, &vert, &frag)?;
    gl.delete_shader(Some(&vert));
    gl.delete_shader(Some(&frag));
    Ok(prog)
}

/// Create an RGBA32F texture at given size with CLAMP_TO_EDGE / LINEAR.
pub fn create_texture_f32(gl: &GL, w: i32, h: i32) -> Result<WebGlTexture, String> {
    let tex = gl.create_texture().ok_or("create_texture failed")?;
    gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D,
        0,
        GL::RGBA32F as i32,
        w,
        h,
        0,
        GL::RGBA,
        GL::FLOAT,
        None,
    )
    .map_err(|e| format!("tex_image_2d: {:?}", e))?;
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
    gl.bind_texture(GL::TEXTURE_2D, None);
    Ok(tex)
}

/// Create an RGBA8 texture at given size.
pub fn create_texture_rgba8(gl: &GL, w: i32, h: i32) -> Result<WebGlTexture, String> {
    let tex = gl.create_texture().ok_or("create_texture failed")?;
    gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D,
        0,
        GL::RGBA as i32,
        w,
        h,
        0,
        GL::RGBA,
        GL::UNSIGNED_BYTE,
        None,
    )
    .map_err(|e| format!("tex_image_2d: {:?}", e))?;
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::REPEAT as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::REPEAT as i32);
    gl.bind_texture(GL::TEXTURE_2D, None);
    Ok(tex)
}

/// Create a framebuffer backed by a texture.
pub fn create_fbo(gl: &GL, tex: &WebGlTexture) -> Result<WebGlFramebuffer, String> {
    let fb = gl.create_framebuffer().ok_or("create_framebuffer failed")?;
    gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&fb));
    gl.framebuffer_texture_2d(
        GL::FRAMEBUFFER,
        GL::COLOR_ATTACHMENT0,
        GL::TEXTURE_2D,
        Some(tex),
        0,
    );
    let status = gl.check_framebuffer_status(GL::FRAMEBUFFER);
    gl.bind_framebuffer(GL::FRAMEBUFFER, None);
    if status != GL::FRAMEBUFFER_COMPLETE {
        return Err(format!("Framebuffer incomplete: {:#x}", status));
    }
    Ok(fb)
}

/// Full-screen quad vertex shader (outputs uv / uv_orig).
pub const FULLSCREEN_VERT: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
out vec2 uv;
out vec2 uv_orig;
out vec4 vColor;
uniform vec4 u_color;
void main() {
    uv = a_pos * 0.5 + 0.5;
    uv_orig = uv;
    vColor = u_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

/// Fragment shader header shared by warp and comp passes.
pub const FRAG_HEADER: &str = r#"#version 300 es
precision highp float;
precision highp int;
precision mediump sampler2D;

in vec2 uv;
in vec2 uv_orig;
in vec4 vColor;
out vec4 fragColor;

uniform sampler2D sampler_main;
uniform sampler2D sampler_fw_main;
uniform sampler2D sampler_fc_main;
uniform sampler2D sampler_pw_main;
uniform sampler2D sampler_pc_main;
uniform sampler2D sampler_blur1;
uniform sampler2D sampler_blur2;
uniform sampler2D sampler_blur3;
uniform sampler2D sampler_noise_lq;
uniform sampler2D sampler_noise_lq_lite;
uniform sampler2D sampler_noise_mq;
uniform sampler2D sampler_noise_hq;
uniform sampler3D sampler_noisevol_lq;
uniform sampler3D sampler_noisevol_hq;

uniform float time;
uniform float frame;
uniform float fps;
uniform float decay;

uniform vec2  resolution;
uniform vec4  aspect;
uniform vec4  texsize;
uniform vec4  texsize_noise_lq;
uniform vec4  texsize_noise_mq;
uniform vec4  texsize_noise_hq;
uniform vec4  texsize_noise_lq_lite;

uniform float bass;     uniform float bass_att;
uniform float mid;      uniform float mid_att;
uniform float treb;     uniform float treb_att;
uniform float vol;      uniform float vol_att;

uniform vec4 _qa; uniform vec4 _qb; uniform vec4 _qc; uniform vec4 _qd;
uniform vec4 _qe; uniform vec4 _qf; uniform vec4 _qg; uniform vec4 _qh;
#define q1  _qa.x
#define q2  _qa.y
#define q3  _qa.z
#define q4  _qa.w
#define q5  _qb.x
#define q6  _qb.y
#define q7  _qb.z
#define q8  _qb.w
#define q9  _qc.x
#define q10 _qc.y
#define q11 _qc.z
#define q12 _qc.w
#define q13 _qd.x
#define q14 _qd.y
#define q15 _qd.z
#define q16 _qd.w
#define q17 _qe.x
#define q18 _qe.y
#define q19 _qe.z
#define q20 _qe.w
#define q21 _qf.x
#define q22 _qf.y
#define q23 _qf.z
#define q24 _qf.w
#define q25 _qg.x
#define q26 _qg.y
#define q27 _qg.z
#define q28 _qg.w
#define q29 _qh.x
#define q30 _qh.y
#define q31 _qh.z
#define q32 _qh.w

uniform vec4 slow_roam_cos; uniform vec4 roam_cos;
uniform vec4 slow_roam_sin; uniform vec4 roam_sin;

uniform float blur1_min; uniform float blur1_max;
uniform float blur2_min; uniform float blur2_max;
uniform float blur3_min; uniform float blur3_max;
uniform float scale1; uniform float bias1;
uniform float scale2; uniform float bias2;
uniform float scale3; uniform float bias3;

uniform vec4 rand_frame;
uniform vec4 rand_preset;

const float PI = 3.141592653589793;
"#;

/// Build a complete fragment shader by inlining `body` (content of shader_body{}) into void main.
pub fn build_preset_frag(body: &str) -> String {
    format!(
        r#"{header}
void main(void) {{
  vec3 ret = vec3(0.0);
  float rad = length(uv_orig - vec2(0.5));
  float ang = atan(uv_orig.x - 0.5, uv_orig.y - 0.5);
  {body}
  fragColor = vec4(ret, 1.0) * vColor;
}}
"#,
        header = FRAG_HEADER,
        body = body
    )
}

/// Passthrough fragment shader (identity, for fallback / output pass).
pub const PASSTHROUGH_FRAG: &str = r#"#version 300 es
precision mediump float;
in vec2 uv;
out vec4 fragColor;
uniform sampler2D sampler_main;
void main() {
    fragColor = texture(sampler_main, uv);
}
"#;
