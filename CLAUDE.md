# khviz — Rust/WASM Milkdrop Visualizer

## Goal

Implement a Rust → WASM drop-in replacement for `butterchurn` v3 beta that:

- Renders all 504 presets from `butterchurn-presets@3.0.0-beta.4` identically
- Runs entirely in WASM (WebGL2 via `web-sys`)
- Exposes a JS API compatible with butterchurn's surface API
- Includes an EEL interpreter for `frame_eqs_eel` / `init_eqs_eel` (CPU-side)
- Uses pre-compiled GLSL from preset JSON for `warp` / `comp` shader passes (no EEL→GLSL compiler needed for v3 presets — all 504 ship pre-compiled)
- Compiles with `wasm-pack --target web`

## Crate Layout

```
khviz/
├── Cargo.toml
├── src/
│   ├── lib.rs             — wasm-bindgen entry, public API
│   ├── visualizer.rs      — main Visualizer struct, render loop
│   ├── preset.rs          — JSON parsing, default merging
│   ├── eel/
│   │   ├── mod.rs
│   │   ├── lexer.rs       — EEL tokenizer
│   │   ├── parser.rs      — EEL → AST
│   │   └── eval.rs        — AST interpreter, EelEnv
│   ├── audio.rs           — AudioLevels: bass/mid/treb, beat detection
│   ├── renderer/
│   │   ├── mod.rs
│   │   ├── gl.rs          — WebGL2 context wrapper helpers
│   │   ├── warp.rs        — WarpShader pass
│   │   ├── comp.rs        — CompShader pass
│   │   ├── blur.rs        — Blur passes (3 levels)
│   │   ├── waves.rs       — Custom wave renderer
│   │   ├── shapes.rs      — Custom shape renderer
│   │   ├── motion.rs      — Motion vector renderer
│   │   └── output.rs      — Final output / gamma pass
│   ├── textures.rs        — Noise textures, image textures
│   └── utils.rs           — Math helpers, smoothstep, etc.
```

## Dependencies (`Cargo.toml`)

```toml
[package]
name = "khviz"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
  "Window", "Document", "HtmlCanvasElement",
  "WebGl2RenderingContext", "WebGlProgram", "WebGlShader",
  "WebGlBuffer", "WebGlVertexArrayObject", "WebGlTexture",
  "WebGlFramebuffer", "WebGlRenderbuffer", "WebGlUniformLocation",
  "ImageData", "OffscreenCanvas",
] }
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
serde_json = "1"
fastrand = "2"

[profile.release]
opt-level = 3
lto = true
```

## Public WASM API (`src/lib.rs`)

```rust
#[wasm_bindgen]
pub struct Visualizer { /* ... */ }

#[wasm_bindgen]
impl Visualizer {
    /// canvas: HtmlCanvasElement
    /// opts: { width, height, meshWidth?, meshHeight?,
    ///         pixelRatio?, textureRatio?, outputFXAA? }
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &HtmlCanvasElement, opts: &JsValue) -> Result<Visualizer, JsValue>;

    /// preset: raw JSON object (JsValue) from butterchurn-presets
    /// blend_time: seconds to cross-blend from previous preset (0 = instant)
    pub fn load_preset(&mut self, preset: &JsValue, blend_time: f32) -> Result<(), JsValue>;

    /// image_data: the imageData export from butterchurn-presets
    /// { "name": { "width": N, "height": N, "data": Uint8Array } }
    pub fn load_extra_images(&mut self, image_data: &JsValue) -> Result<(), JsValue>;

    /// Connect an AudioNode gain output to the visualizer.
    /// Butterchurn compat: call before render().
    /// In WASM this stores an AnalyserNode ref and reads data internally.
    pub fn connect_audio(&mut self, gain_node: &JsValue);

    /// Alternative: JS pushes raw audio arrays each frame.
    /// time_domain: Float32Array from analyserNode.getFloatTimeDomainData()
    /// frequency:   Float32Array from analyserNode.getFloatFrequencyData()
    ///              (must be linear, not dB — convert with 10^(db/20) before passing)
    pub fn set_audio_data(&mut self, time_domain: &js_sys::Float32Array, frequency: &js_sys::Float32Array);

    /// Render one frame. Call from requestAnimationFrame.
    pub fn render(&mut self);

    /// Update canvas size. Call from ResizeObserver.
    pub fn set_renderer_size(&mut self, width: u32, height: u32);

    /// Release WebGL resources.
    pub fn destroy(self);
}
```

### JS Usage (drop-in for butterchurn)

```javascript
import init, { Visualizer } from './khviz/khviz.js';
import presets from 'butterchurn-presets/all';
import imageData from 'butterchurn-presets/imageData';

await init(); // load WASM binary once

const viz = new Visualizer(canvasEl, { width: 1280, height: 720 });
viz.loadExtraImages(imageData);
viz.connectAudio(gainNode); // OR call setAudioData() each frame

const preset = presets['presetName'];
viz.loadPreset(preset, 2.5);

function frame() {
  viz.render();
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
```

---

## Preset JSON Format

Source: `butterchurn-presets@3.0.0-beta.4`, `presets/converted/*.json`

```json
{
  "version": 2,
  "baseVals": { ... },
  "shapes": [
    { "baseVals": { ... }, "init_eqs_eel": "...", "frame_eqs_eel": "..." },
    ...
  ],
  "waves": [
    { "baseVals": { ... }, "init_eqs_eel": "...", "frame_eqs_eel": "..." },
    ...
  ],
  "init_eqs_eel": "...",
  "frame_eqs_eel": "...",
  "pixel_eqs_eel": "...",
  "warp": " shader_body { ... }",
  "comp": " shader_body { ... }"
}
```

**Important:** All 504 presets ship pre-compiled `warp` and `comp` GLSL. Use these directly; do not compile `pixel_eqs_eel` to GLSL. The EEL interpreter is only needed for `init_eqs_eel`, `frame_eqs_eel`, and the wave/shape equation variants.

### `baseVals` defaults (merge preset over these before use)

```rust
pub struct BaseVals {
    pub gammaadj: f32,        // 1.25
    pub decay: f32,           // 0.9
    pub zoom: f32,            // 1.0
    pub zoomexp: f32,         // 1.0
    pub rot: f32,             // 0.0
    pub warp: f32,            // 0.01
    pub warpscale: f32,       // 1.0
    pub warpanimspeed: f32,   // 1.0
    pub cx: f32,              // 0.5
    pub cy: f32,              // 0.5
    pub dx: f32,              // 0.0
    pub dy: f32,              // 0.0
    pub sx: f32,              // 1.0
    pub sy: f32,              // 1.0
    pub wave_mode: f32,       // 0.0
    pub wave_a: f32,          // 1.0
    pub wave_r: f32,          // 0.5
    pub wave_g: f32,          // 0.5
    pub wave_b: f32,          // 0.5
    pub wave_x: f32,          // 0.5
    pub wave_y: f32,          // 0.5
    pub wave_scale: f32,      // 1.0
    pub wave_smoothing: f32,  // 0.75
    pub wave_mystery: f32,    // -0.2
    pub wave_dots: f32,       // 0.0
    pub wave_brighten: f32,   // 0.0
    pub additivewave: f32,    // 0.0
    pub modwavealphabyvolume: f32, // 0.0
    pub modwavealphastart: f32,   // 0.75
    pub modwavealphaend: f32,     // 0.95
    pub brighten: f32,        // 0.0
    pub darken: f32,          // 0.0
    pub solarize: f32,        // 0.0
    pub invert: f32,          // 0.0
    pub darken_center: f32,   // 0.0
    pub red_blue: f32,        // 0.0
    pub fshader: f32,         // 0.0
    pub echo_zoom: f32,       // 1.0
    pub echo_alpha: f32,      // 0.0
    pub echo_orient: f32,     // 0.0
    pub wrap: f32,            // 0.0
    pub ob_size: f32,         // 0.0
    pub ob_r: f32,            // 0.5
    pub ob_g: f32,            // 0.5
    pub ob_b: f32,            // 0.5
    pub ob_a: f32,            // 0.0
    pub ib_size: f32,         // 0.0
    pub ib_r: f32,            // 0.5
    pub ib_g: f32,            // 0.5
    pub ib_b: f32,            // 0.5
    pub ib_a: f32,            // 0.0
    pub mv_x: f32,            // 12.0
    pub mv_y: f32,            // 9.0
    pub mv_dx: f32,           // 0.0
    pub mv_dy: f32,           // 0.0
    pub mv_l: f32,            // 0.0
    pub mv_r: f32,            // 0.5
    pub mv_g: f32,            // 0.5
    pub mv_b: f32,            // 0.5
    pub mv_a: f32,            // 0.0
    pub bmotionvectorson: f32,// 0.0
    pub rating: f32,          // 5.0
}
```

---

## EEL Language Spec (`src/eel/`)

EEL (Extensible Expression Language) is Milkdrop's scripting language for per-frame and per-init equations. It is NOT needed for pixel shaders (those are pre-compiled to GLSL).

### Syntax

- Statements separated by `;` or newlines
- `//` single-line comments
- `/* */` block comments
- No type system — all values are `f64`, truthy if `abs(x) > 1e-6`
- Assignment: `x = expr`
- Compound assign: `x += expr`, `x -= expr`, `x *= expr`, `x /= expr`, `x %= expr`, `x ^= expr`

### Operators (precedence low→high)

| Operator | Meaning |
|----------|---------|
| `,` | sequence (evaluates both, returns right) |
| `=` `+=` `-=` etc. | assignment |
| `\|` | bitwise OR |
| `&` | bitwise AND |
| `<` `>` | comparison (return 1.0/0.0) |
| `+` `-` | addition/subtraction |
| `*` `/` `%` | multiply/divide/modulo |
| `^` | power (`x^y` = `x.powf(y)`) |
| unary `-` `!` | negate, logical NOT |

### Variables

- **Local**: any identifier. Persists between calls on the same `EelEnv`.
- **Global registers**: `reg00` … `reg99` — shared across all EEL environments for a preset frame (presets can communicate via these).
- **Q variables**: `q1` … `q32` — passed to GPU shaders as uniforms `_qa` through `_qh` (vec4 packing).
- **T variables**: `t1` … `t8` — wave/shape-local scratch.
- **Audio inputs** (read-only in EEL, set by AudioLevels before each frame):
  `bass`, `mid`, `treb`, `vol`, `bass_att`, `mid_att`, `treb_att`, `vol_att`
- **System inputs** (read-only):
  `time` (seconds since start), `fps`, `frame` (frame count)

### Built-in Functions

```
sin(x)          cos(x)          tan(x)
asin(x)         acos(x)         atan(x)         atan2(y,x)
sqrt(x)         sqr(x)          pow(x,y)
log(x)          log10(x)        exp(x)
abs(x)          sign(x)         
floor(x)        ceil(x)         int(x)          frac(x)
min(x,y)        max(x,y)        clamp(x,lo,hi)
invsqrt(x)
rand(x)         — random int in [0, floor(x))
band(x,y)       — bitwise AND of int(x) & int(y), returned as float
bor(x,y)        — bitwise OR
bnot(x)         — bitwise NOT
if(cond,a,b)    — cond != 0 ? a : b  (lazy evaluation not required)
equal(x,y)      — abs(x-y) < 1e-6 ? 1 : 0
above(x,y)      — x > y ? 1 : 0
below(x,y)      — x < y ? 1 : 0
assign(x,y)     — x = y (alias for = operator)
exec2(a,b)      — evaluate a then b, return b
exec3(a,b,c)    — evaluate a, b, c, return c
loop(count, body) — execute body floor(count) times (max 1024)
megabuf(i)      — preset-local float buffer, index i (0–999999)
gmegabuf(i)     — global float buffer, same index space
```

### EelEnv Struct

```rust
pub struct EelEnv {
    pub vars: HashMap<String, f64>,   // local + q/t/reg variables
    pub megabuf: Vec<f64>,            // 1_000_000 floats, zero-initialized
}

impl EelEnv {
    pub fn set_audio(&mut self, bass: f64, mid: f64, treb: f64, vol: f64,
                     bass_att: f64, mid_att: f64, treb_att: f64, vol_att: f64);
    pub fn set_time(&mut self, time: f64, fps: f64, frame: u64);
    pub fn run(&mut self, ast: &Ast) -> f64;
    pub fn get(&self, name: &str) -> f64;        // returns 0.0 if not set
    pub fn set(&mut self, name: &str, val: f64);
    /// Copy q1–q32 values out for GPU uniform upload
    pub fn get_q_vals(&self) -> [f32; 32];
}
```

Global registers `reg00`–`reg99` live outside `EelEnv` (pass a `&mut [f64; 100]` reference so all envs in a frame share them).

---

## Audio Levels (`src/audio.rs`)

```rust
pub struct AudioLevels {
    val: [f32; 3],       // [bass, mid, treb] normalized
    att: [f32; 3],       // smoothed version of val
    avg: [f32; 3],       // short-term average
    long_avg: [f32; 3],  // long-term average (normalization reference)
}
```

### Frequency Bands

Given `sample_rate` (typically 44100) and `fft_size` (power of 2, typically 2048):

```
bucket_hz = sample_rate / fft_size

bass_low  = clamp(round(20   / bucket_hz) - 1, 0, num_bins-1)
bass_high = clamp(round(320  / bucket_hz) - 1, 0, num_bins-1)
mid_high  = clamp(round(2800 / bucket_hz) - 1, 0, num_bins-1)
treb_high = clamp(round(11025/ bucket_hz) - 1, 0, num_bins-1)

bands: [bass_low..bass_high], [bass_high..mid_high], [mid_high..treb_high]
```

### Beat Detection Algorithm (per frame)

```rust
for i in 0..3 {
    imm[i] = freq[bands[i]].iter().sum::<f32>();

    let rate = if imm[i] > avg[i] { 0.2f32 } else { 0.5f32 };
    let rate = rate.powf(30.0 / fps.clamp(15.0, 144.0));
    avg[i] = avg[i] * rate + imm[i] * (1.0 - rate);

    let long_rate = if frame < 50 { 0.9f32 } else { 0.992f32 };
    let long_rate = long_rate.powf(30.0 / fps.clamp(15.0, 144.0));
    long_avg[i] = long_avg[i] * long_rate + imm[i] * (1.0 - long_rate);

    if long_avg[i] < 0.001 {
        val[i] = 1.0;
        att[i] = 1.0;
    } else {
        val[i] = imm[i] / long_avg[i];
        att[i] = avg[i] / long_avg[i];
    }
}
// bass = val[0], mid = val[1], treb = val[2]
// bass_att = att[0], etc.
// vol = (bass + mid + treb) / 3.0
```

---

## Rendering Pipeline

### Framebuffers

```
fb_main[2]   — ping-pong 32-bit RGBA textures at render resolution
fb_output    — final output before gamma/FXAA
fb_blur[3]   — blur pyramid (half/quarter/eighth resolution)
```

### Per-Frame Sequence

```
1. Update AudioLevels (bass/mid/treb/vol)
2. Run init_eqs_eel (first frame only, or on preset load)
3. Run frame_eqs_eel → updates q1–q32, zoom/rot/warp/etc.
4. Compute warp mesh UVs (cpu-side, 48×36 grid)
5. WarpShader pass: sample fb_main[prev] through warp mesh → fb_main[curr]
6. Blur passes: downsample fb_main[curr] → fb_blur[0,1,2]
7. MotionVectors pass (if mv_a > 0 and bmotionvectorson)
8. CustomShapes pass (up to 4 shapes)
9. CustomWaves pass (up to 8 waves)
10. InnerBorder / OuterBorder pass (if ob_size/ib_size > 0)
11. CompShader pass: fb_main[curr] → fb_output (post-process, gamma)
12. OutputShader: fb_output → canvas (optional FXAA)
13. swap ping-pong index
```

### Warp Mesh (CPU)

Grid: `mesh_w × mesh_h` quads (default 48×36). For each vertex `(gx, gy)`:

```
u = gx / (mesh_w - 1)       // 0..1
v = gy / (mesh_h - 1)

// center-relative
ux = u - 0.5
uy = v - 0.5

// apply zoom (zoomexp controls falloff from center)
r = sqrt(ux^2 + uy^2)
zoom_factor = zoom * (r^zoomexp / r)  // handle r=0 separately

// apply rotation
cos_rot = cos(-rot * 2π)
sin_rot = sin(-rot * 2π)
ux2 = ux * cos_rot - uy * sin_rot
uy2 = ux * sin_rot + uy * cos_rot

// apply scale + translation
sample_u = 0.5 + (ux2 / zoom_factor) / sx - dx
sample_v = 0.5 + (uy2 / zoom_factor) / sy - dy

// warp displacement (uses animated noise)
sample_u += warp * sin(warp_time1 * u + warp_time3 * v + ...)
sample_v += warp * cos(warp_time2 * u + warp_time4 * v + ...)
```

The warp time offsets are updated each frame:
```
warp_time += dt * warpanimspeed * 0.5 * warpscale
warp_time1 = warp_time * 1.413 + 3.900
warp_time2 = warp_time * 1.731 + 7.800  (etc., 4 offsets)
```

Each vertex also gets a `vColor` = `vec4(decay)` for the warp pass to multiply alpha.

### GLSL Shader Wrapper

The preset's `warp` and `comp` fields contain a `shader_body { ... }` block. Wrap it in this template to produce a complete fragment shader:

```glsl
#version 300 es
precision highp float;
precision highp int;
precision mediump sampler2D;

in vec2 uv;         // current pixel UV [0,1]
in vec2 uv_orig;    // same as uv in warp pass; used for rad/ang
in vec4 vColor;     // per-vertex color (warp pass: decay; comp pass: white)

out vec4 fragColor;

// Samplers
uniform sampler2D sampler_main;
uniform sampler2D sampler_fw_main;   // flipped-warp copy
uniform sampler2D sampler_fc_main;   // flipped-comp copy
uniform sampler2D sampler_pw_main;   // prev-warp copy
uniform sampler2D sampler_pc_main;   // prev-comp copy
uniform sampler2D sampler_blur1;
uniform sampler2D sampler_blur2;
uniform sampler2D sampler_blur3;
uniform sampler2D sampler_noise_lq;
uniform sampler2D sampler_noise_lq_lite;
uniform sampler2D sampler_noise_mq;
uniform sampler2D sampler_noise_hq;
uniform sampler3D sampler_noisevol_lq;
uniform sampler3D sampler_noisevol_hq;

// Time / frame
uniform float time;
uniform float frame;
uniform float fps;
uniform float decay;

// Resolution
uniform vec2  resolution;
uniform vec4  aspect;          // (aspectx, aspecty, 1/aspectx, 1/aspecty)
uniform vec4  texsize;         // (w, h, 1/w, 1/h)
uniform vec4  texsize_noise_lq;
uniform vec4  texsize_noise_mq;
uniform vec4  texsize_noise_hq;
uniform vec4  texsize_noise_lq_lite;

// Audio
uniform float bass;     uniform float bass_att;
uniform float mid;      uniform float mid_att;
uniform float treb;     uniform float treb_att;
uniform float vol;      uniform float vol_att;

// Q variables (q1–q32 packed into 8 vec4s)
uniform vec4 _qa;  uniform vec4 _qb;  uniform vec4 _qc;  uniform vec4 _qd;
uniform vec4 _qe;  uniform vec4 _qf;  uniform vec4 _qg;  uniform vec4 _qh;
#define q1  _qa.x
#define q2  _qa.y
#define q3  _qa.z
#define q4  _qa.w
#define q5  _qb.x
// ... through q32 _qh.w

// Animated rotation/warp constants
uniform vec4 slow_roam_cos;  uniform vec4 roam_cos;
uniform vec4 slow_roam_sin;  uniform vec4 roam_sin;

// Blur range/scale
uniform float blur1_min; uniform float blur1_max;
uniform float blur2_min; uniform float blur2_max;
uniform float blur3_min; uniform float blur3_max;
uniform float scale1; uniform float bias1;
uniform float scale2; uniform float bias2;
uniform float scale3; uniform float bias3;

// Random per-frame and per-preset
uniform vec4 rand_frame;
uniform vec4 rand_preset;

const float PI = 3.141592653589793;

// <<< preset shader_body content is inserted here >>>
// Extract content between outermost { } of shader_body { ... } block

void main(void) {
  vec3 ret = vec3(0.0);
  float rad = length(uv_orig - vec2(0.5));
  float ang = atan(uv_orig.x - 0.5, uv_orig.y - 0.5);

  // preset body executes here, sets ret
  // (shader_body content is inlined, not called as function)

  fragColor = vec4(ret, 1.0) * vColor;
}
```

**Extraction**: Find `shader_body` string in preset field, then take content between first `{` and matching last `}`. Insert inline before `void main`, replacing the body comment above.

---

## Uniform Upload (per frame)

After running `frame_eqs_eel`, upload these to both warp and comp shader programs:

```rust
// Audio
gl.uniform1f(loc_bass, audio.bass);
gl.uniform1f(loc_mid, audio.mid);
gl.uniform1f(loc_treb, audio.treb);
gl.uniform1f(loc_vol, audio.vol);
// ...and *_att variants

// Time
gl.uniform1f(loc_time, elapsed_secs);
gl.uniform1f(loc_fps, fps);
gl.uniform1f(loc_frame, frame_count as f32);
gl.uniform1f(loc_decay, base_vals.decay);

// Resolution
gl.uniform2f(loc_resolution, tex_w, tex_h);
gl.uniform4f(loc_aspect, aspectx, aspecty, 1/aspectx, 1/aspecty);
gl.uniform4f(loc_texsize, tex_w, tex_h, 1/tex_w, 1/tex_h);

// Q variables (from EelEnv.get_q_vals())
let q = eel_env.get_q_vals(); // [f32; 32]
gl.uniform4fv(loc_qa, &q[0..4]);
gl.uniform4fv(loc_qb, &q[4..8]);
// ... through _qh

// Animated sin/cos for warp displacement
gl.uniform4f(loc_slow_roam_cos, ...);
gl.uniform4f(loc_roam_cos, ...);
// see butterchurn source for exact formula

// Blur range (derived from q1–q3 by convention, or preset-specific)
// ...

// Random values
gl.uniform4f(loc_rand_frame, fastrand::f32(), ...);
// rand_preset set once at preset load
```

---

## Wave Rendering (`src/renderer/waves.rs`)

Up to 8 waves per preset. Each wave has `baseVals` and optional `frame_eqs_eel`.

### Wave baseVals

```rust
pub struct WaveVals {
    pub enabled: f32,    // 0 or 1
    pub r: f32,          // color red
    pub g: f32,
    pub b: f32,
    pub a: f32,          // alpha
    pub samples: f32,    // number of sample points (max 512)
    pub sep: f32,        // L/R separation
    pub scaling: f32,    // amplitude scale
    pub smoothing: f32,  // 0..1
    pub usedots: f32,    // render dots instead of lines
    pub additive: f32,   // additive blend
    pub spectrum: f32,   // 0=waveform, 1=spectrum
    pub thick: f32,      // line thickness multiplier
}
```

### Wave Modes (global baseVals.wave_mode)

| mode | shape |
|------|-------|
| 0 | center blob |
| 1 | horizontal line |
| 2 | vertical line |
| 3 | x-y oscilloscope |
| 4 | circle (angle=time, radius=waveform) |
| 5 | star burst |
| 6 | smooth/noodle |
| 7 | double horizontal |

### Per-Wave EEL

Run `wave.frame_eqs_eel` into `wave_env` each frame. Variables `r`, `g`, `b`, `a`, `samples`, `scaling`, `spectrum`, `smoothing`, `usedots`, `additive`, `thick`, `sep` are writable from EEL.

---

## Shape Rendering (`src/renderer/shapes.rs`)

Up to 4 shapes per preset. Rendered as triangle-fan circles/polygons.

### Shape baseVals

```rust
pub struct ShapeVals {
    pub enabled: f32,
    pub r: f32, pub g: f32, pub b: f32, pub a: f32,
    pub r2: f32, pub g2: f32, pub b2: f32, pub a2: f32,  // inner color
    pub x: f32, pub y: f32,    // center position [0,1]
    pub rad: f32,              // radius
    pub ang: f32,              // rotation angle (radians)
    pub tex_ang: f32,
    pub tex_zoom: f32,
    pub sides: f32,            // number of sides (3–100)
    pub additive: f32,
    pub textured: f32,         // 0=solid, 1=textured from sampler_main
    pub thickoutline: f32,
}
```

---

## Motion Vectors (`src/renderer/motion.rs`)

Rendered when `base_vals.bmotionvectorson != 0.0` and `base_vals.mv_a > 0.0`.

Grid: `mv_x × mv_y` arrows showing warp displacement direction. Draw as line segments from each grid point to where the warp mesh maps it. Color: `(mv_r, mv_g, mv_b, mv_a)`.

---

## Noise Textures (`src/textures.rs`)

Generate procedurally at startup — no file loading needed:

| Name | Size | Format |
|------|------|--------|
| `noise_lq` | 256×256 | RGBA8, animated (update 1/30s) |
| `noise_lq_lite` | 32×32 | RGBA8, animated |
| `noise_mq` | 256×256 | RGBA8, static |
| `noise_hq` | 256×256 | RGBA8, static |
| `noisevol_lq` | 32×32×32 | 3D RGBA8, static |
| `noisevol_hq` | 32×32×32 | 3D RGBA8, static |

Use `fastrand` to fill. Noise LQ is periodically re-randomized for animation.

## Image Textures (`src/textures.rs`)

`load_extra_images(image_data: JsValue)` receives the JS object from `butterchurn-presets/imageData`:

```js
{ "name": { "width": N, "height": N, "data": Uint8Array /* RGBA */ } }
```

Upload each as a `sampler2D` named `sampler_<name>`. Shaders that reference `sampler_<name>` will sample from it. Ignore missing textures gracefully (bind a 1×1 white texture as fallback).

---

## Preset Blending

When `load_preset(new_preset, blend_time)` is called with `blend_time > 0`:

1. Keep current framebuffer state as `fb_blend_from`
2. Start rendering new preset into `fb_main`
3. Each frame during blend: output = `mix(fb_blend_from, fb_main, t/blend_time)`
4. After `blend_time` seconds: discard `fb_blend_from`, render new preset normally

---

## Build & Integration

### Build

```bash
# Install wasm-pack once
cargo install wasm-pack

# Build release WASM (from khviz/ repo root)
wasm-pack build --target web --out-dir pkg

# Output: pkg/khviz.js (ES module), pkg/khviz_bg.wasm
```

### Vite Integration (in KaraokeHub frontend)

Install as a local dependency or submodule:
```json
{ "khviz": "file:../khviz/pkg" }
```

In `vite.config.ts`, add to `optimizeDeps.exclude` to prevent Vite from trying to bundle the WASM:
```ts
optimizeDeps: { exclude: ['khviz'] }
```

Import pattern:
```ts
import init, { Visualizer } from 'khviz';
const wasmReady = init();   // call once, cache promise
```

---

## References

- Butterchurn v3 source: `butterchurn@3.0.0-beta.4` — `dist/butterchurn.js` (12890 lines, readable)
- Preset format: `butterchurn-presets@3.0.0-beta.4` — `presets/converted/*.json` (504 files)
- Original Milkdrop shader spec: MilkDrop 2 source (Geiss, Nullsoft)
- EEL language: same as Winamp AVS Expression language
- `web-sys` WebGL2 docs: https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGl2RenderingContext.html
