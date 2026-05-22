use serde::Deserialize;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BaseVals {
    pub gammaadj: f32,
    pub decay: f32,
    pub zoom: f32,
    pub zoomexp: f32,
    pub rot: f32,
    pub warp: f32,
    pub warpscale: f32,
    pub warpanimspeed: f32,
    pub cx: f32,
    pub cy: f32,
    pub dx: f32,
    pub dy: f32,
    pub sx: f32,
    pub sy: f32,
    pub wave_mode: f32,
    pub wave_a: f32,
    pub wave_r: f32,
    pub wave_g: f32,
    pub wave_b: f32,
    pub wave_x: f32,
    pub wave_y: f32,
    pub wave_scale: f32,
    pub wave_smoothing: f32,
    pub wave_mystery: f32,
    pub wave_dots: f32,
    pub wave_brighten: f32,
    pub additivewave: f32,
    pub modwavealphabyvolume: f32,
    pub modwavealphastart: f32,
    pub modwavealphaend: f32,
    pub brighten: f32,
    pub darken: f32,
    pub solarize: f32,
    pub invert: f32,
    pub darken_center: f32,
    pub red_blue: f32,
    pub fshader: f32,
    pub echo_zoom: f32,
    pub echo_alpha: f32,
    pub echo_orient: f32,
    pub wrap: f32,
    pub ob_size: f32,
    pub ob_r: f32,
    pub ob_g: f32,
    pub ob_b: f32,
    pub ob_a: f32,
    pub ib_size: f32,
    pub ib_r: f32,
    pub ib_g: f32,
    pub ib_b: f32,
    pub ib_a: f32,
    pub mv_x: f32,
    pub mv_y: f32,
    pub mv_dx: f32,
    pub mv_dy: f32,
    pub mv_l: f32,
    pub mv_r: f32,
    pub mv_g: f32,
    pub mv_b: f32,
    pub mv_a: f32,
    pub bmotionvectorson: f32,
    pub rating: f32,
}

impl Default for BaseVals {
    fn default() -> Self {
        Self {
            gammaadj: 1.25,
            decay: 0.9,
            zoom: 1.0,
            zoomexp: 1.0,
            rot: 0.0,
            warp: 0.01,
            warpscale: 1.0,
            warpanimspeed: 1.0,
            cx: 0.5,
            cy: 0.5,
            dx: 0.0,
            dy: 0.0,
            sx: 1.0,
            sy: 1.0,
            wave_mode: 0.0,
            wave_a: 1.0,
            wave_r: 0.5,
            wave_g: 0.5,
            wave_b: 0.5,
            wave_x: 0.5,
            wave_y: 0.5,
            wave_scale: 1.0,
            wave_smoothing: 0.75,
            wave_mystery: -0.2,
            wave_dots: 0.0,
            wave_brighten: 0.0,
            additivewave: 0.0,
            modwavealphabyvolume: 0.0,
            modwavealphastart: 0.75,
            modwavealphaend: 0.95,
            brighten: 0.0,
            darken: 0.0,
            solarize: 0.0,
            invert: 0.0,
            darken_center: 0.0,
            red_blue: 0.0,
            fshader: 0.0,
            echo_zoom: 1.0,
            echo_alpha: 0.0,
            echo_orient: 0.0,
            wrap: 0.0,
            ob_size: 0.0,
            ob_r: 0.5,
            ob_g: 0.5,
            ob_b: 0.5,
            ob_a: 0.0,
            ib_size: 0.0,
            ib_r: 0.5,
            ib_g: 0.5,
            ib_b: 0.5,
            ib_a: 0.0,
            mv_x: 12.0,
            mv_y: 9.0,
            mv_dx: 0.0,
            mv_dy: 0.0,
            mv_l: 0.0,
            mv_r: 0.5,
            mv_g: 0.5,
            mv_b: 0.5,
            mv_a: 0.0,
            bmotionvectorson: 0.0,
            rating: 5.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WaveVals {
    pub enabled: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub samples: f32,
    pub sep: f32,
    pub scaling: f32,
    pub smoothing: f32,
    pub usedots: f32,
    pub additive: f32,
    pub spectrum: f32,
    pub thick: f32,
}

impl Default for WaveVals {
    fn default() -> Self {
        Self {
            enabled: 0.0,
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
            samples: 512.0,
            sep: 0.0,
            scaling: 1.0,
            smoothing: 0.5,
            usedots: 0.0,
            additive: 0.0,
            spectrum: 0.0,
            thick: 1.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ShapeVals {
    pub enabled: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub r2: f32,
    pub g2: f32,
    pub b2: f32,
    pub a2: f32,
    pub x: f32,
    pub y: f32,
    pub rad: f32,
    pub ang: f32,
    pub tex_ang: f32,
    pub tex_zoom: f32,
    pub sides: f32,
    pub additive: f32,
    pub textured: f32,
    pub thickoutline: f32,
}

impl Default for ShapeVals {
    fn default() -> Self {
        Self {
            enabled: 0.0,
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
            r2: 0.0,
            g2: 1.0,
            b2: 0.0,
            a2: 1.0,
            x: 0.5,
            y: 0.5,
            rad: 0.1,
            ang: 0.0,
            tex_ang: 0.0,
            tex_zoom: 1.0,
            sides: 4.0,
            additive: 0.0,
            textured: 0.0,
            thickoutline: 0.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WavePreset {
    #[serde(rename = "baseVals", default)]
    pub base_vals: WaveVals,
    #[serde(rename = "init_eqs_eel", default)]
    pub init_eqs_eel: String,
    #[serde(rename = "frame_eqs_eel", default)]
    pub frame_eqs_eel: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ShapePreset {
    #[serde(rename = "baseVals", default)]
    pub base_vals: ShapeVals,
    #[serde(rename = "init_eqs_eel", default)]
    pub init_eqs_eel: String,
    #[serde(rename = "frame_eqs_eel", default)]
    pub frame_eqs_eel: String,
}

#[derive(Debug, Clone)]
pub struct Preset {
    pub base_vals: BaseVals,
    pub waves: Vec<WavePreset>,
    pub shapes: Vec<ShapePreset>,
    pub init_eqs_eel: String,
    pub frame_eqs_eel: String,
    pub pixel_eqs_eel: String,
    pub warp_glsl: String,
    pub comp_glsl: String,
}

#[derive(Deserialize)]
struct PresetRaw {
    #[serde(rename = "baseVals", default)]
    base_vals: serde_json::Value,
    #[serde(default)]
    waves: Vec<WavePreset>,
    #[serde(default)]
    shapes: Vec<ShapePreset>,
    #[serde(rename = "init_eqs_eel", default)]
    init_eqs_eel: String,
    #[serde(rename = "frame_eqs_eel", default)]
    frame_eqs_eel: String,
    #[serde(rename = "pixel_eqs_eel", default)]
    pixel_eqs_eel: String,
    #[serde(rename = "warp", default)]
    warp: String,
    #[serde(rename = "comp", default)]
    comp: String,
}

/// Extract the body of `shader_body { ... }` from a preset shader field.
pub fn extract_shader_body(src: &str) -> String {
    if let Some(start) = src.find("shader_body") {
        let after = &src[start + "shader_body".len()..];
        if let Some(rel_open) = after.find('{') {
            let chars: Vec<char> = after[rel_open + 1..].chars().collect();
            let mut depth = 1usize;
            let mut end = chars.len();
            for (i, &c) in chars.iter().enumerate() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            return chars[..end].iter().collect();
        }
    }
    String::new()
}

impl Preset {
    pub fn from_js_value(val: &JsValue) -> Result<Self, String> {
        let json_str = js_sys::JSON::stringify(val)
            .map_err(|_| "stringify failed".to_string())?
            .as_string()
            .ok_or("stringify result not string")?;
        Self::from_json_str(&json_str)
    }

    pub fn from_json_str(s: &str) -> Result<Self, String> {
        let raw: PresetRaw = serde_json::from_str(s).map_err(|e| e.to_string())?;

        let base_vals: BaseVals = if raw.base_vals.is_object() {
            serde_json::from_value(raw.base_vals).unwrap_or_default()
        } else {
            BaseVals::default()
        };

        Ok(Preset {
            base_vals,
            waves: raw.waves,
            shapes: raw.shapes,
            init_eqs_eel: raw.init_eqs_eel,
            frame_eqs_eel: raw.frame_eqs_eel,
            pixel_eqs_eel: raw.pixel_eqs_eel,
            warp_glsl: extract_shader_body(&raw.warp),
            comp_glsl: extract_shader_body(&raw.comp),
        })
    }
}
