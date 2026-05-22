#![allow(dead_code, unused_imports, unused_variables)]
mod audio;
mod eel;
mod preset;
mod renderer;
mod textures;
mod utils;
mod visualizer;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::preset::Preset;
use crate::visualizer::Visualizer as VizInner;

#[wasm_bindgen]
pub struct Visualizer {
    inner: VizInner,
}

#[wasm_bindgen]
impl Visualizer {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &HtmlCanvasElement, opts: &JsValue) -> Result<Visualizer, JsValue> {
        console_error_panic_hook();

        let width = opts_u32(opts, "width").unwrap_or(canvas.width());
        let height = opts_u32(opts, "height").unwrap_or(canvas.height());
        let mesh_width = opts_u32(opts, "meshWidth").unwrap_or(48) as usize;
        let mesh_height = opts_u32(opts, "meshHeight").unwrap_or(36) as usize;
        let pixel_ratio = opts_f32(opts, "pixelRatio").unwrap_or(1.0);
        let texture_ratio = opts_f32(opts, "textureRatio").unwrap_or(pixel_ratio);
        let fxaa = opts_bool(opts, "outputFXAA").unwrap_or(false);

        canvas.set_width(width);
        canvas.set_height(height);

        let gl = canvas
            .get_context("webgl2")
            .map_err(|e| JsValue::from(format!("getContext error: {:?}", e)))?
            .ok_or_else(|| JsValue::from("WebGL2 not available"))?
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|_| JsValue::from("cast to WebGl2RenderingContext failed"))?;

        let inner = VizInner::new(gl, width, height, mesh_width, mesh_height, texture_ratio, fxaa)
            .map_err(|e| JsValue::from(e))?;

        Ok(Visualizer { inner })
    }

    pub fn load_preset(&mut self, preset: &JsValue, blend_time: f32) -> Result<(), JsValue> {
        let p = Preset::from_js_value(preset).map_err(|e| JsValue::from(e))?;
        self.inner.load_preset(p, blend_time);
        Ok(())
    }

    pub fn load_extra_images(&mut self, image_data: &JsValue) -> Result<(), JsValue> {
        self.inner
            .load_extra_images(image_data)
            .map_err(|e| JsValue::from(e))
    }

    /// JS calls this with an AudioNode — stored as opaque JsValue.
    /// Actual audio routing is done via setAudioData each frame.
    pub fn connect_audio(&mut self, _gain_node: &JsValue) {
        // Audio routing happens in JS; use set_audio_data each frame.
    }

    pub fn set_audio_data(
        &mut self,
        time_domain: &js_sys::Float32Array,
        frequency: &js_sys::Float32Array,
    ) {
        let td = time_domain.to_vec();
        let fr = frequency.to_vec();
        self.inner.set_audio_data(&td, &fr);
    }

    /// Render one frame. `now` is performance.now() in milliseconds.
    pub fn render(&mut self, now: f64) {
        self.inner.render(now);
    }

    pub fn set_renderer_size(&mut self, width: u32, height: u32) {
        self.inner.set_renderer_size(width, height);
    }

    pub fn destroy(self) {
        // Drop releases all GL resources via Rust's RAII.
    }
}

fn opts_u32(opts: &JsValue, key: &str) -> Option<u32> {
    js_sys::Reflect::get(opts, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as u32)
}

fn opts_f32(opts: &JsValue, key: &str) -> Option<f32> {
    js_sys::Reflect::get(opts, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
}

fn opts_bool(opts: &JsValue, key: &str) -> Option<bool> {
    js_sys::Reflect::get(opts, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_bool())
}

fn console_error_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
