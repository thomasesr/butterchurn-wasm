use std::collections::HashMap;
use web_sys::{WebGl2RenderingContext as GL, WebGlTexture};

pub struct Textures {
    pub noise_lq: WebGlTexture,
    pub noise_lq_lite: WebGlTexture,
    pub noise_mq: WebGlTexture,
    pub noise_hq: WebGlTexture,
    pub noise_vol_lq: WebGlTexture,
    pub noise_vol_hq: WebGlTexture,
    pub extra: HashMap<String, WebGlTexture>,
    pub fallback: WebGlTexture,
    noise_lq_timer: f64,
}

fn rand_rgba(size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size * 4];
    for i in 0..size {
        buf[i * 4] = fastrand::u8(..);
        buf[i * 4 + 1] = fastrand::u8(..);
        buf[i * 4 + 2] = fastrand::u8(..);
        buf[i * 4 + 3] = 255;
    }
    buf
}

fn upload_2d(gl: &GL, tex: &WebGlTexture, w: i32, h: i32, data: &[u8]) {
    gl.bind_texture(GL::TEXTURE_2D, Some(tex));
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        GL::TEXTURE_2D, 0, GL::RGBA as i32, w, h, 0, GL::RGBA, GL::UNSIGNED_BYTE, Some(data),
    )
    .ok();
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::REPEAT as i32);
    gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::REPEAT as i32);
    gl.bind_texture(GL::TEXTURE_2D, None);
}

fn make_tex_2d(gl: &GL, w: i32, h: i32) -> WebGlTexture {
    let tex = gl.create_texture().expect("create_texture");
    let data = rand_rgba((w * h) as usize);
    upload_2d(gl, &tex, w, h, &data);
    tex
}

fn make_tex_3d(gl: &GL, size: i32) -> WebGlTexture {
    let tex = gl.create_texture().expect("create_texture");
    let n = (size * size * size) as usize;
    let data = rand_rgba(n);
    gl.bind_texture(GL::TEXTURE_3D, Some(&tex));
    gl.tex_image_3d_with_opt_u8_array(
        GL::TEXTURE_3D, 0, GL::RGBA as i32, size, size, size, 0, GL::RGBA, GL::UNSIGNED_BYTE,
        Some(&data),
    )
    .ok();
    gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
    gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_WRAP_S, GL::REPEAT as i32);
    gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_WRAP_T, GL::REPEAT as i32);
    gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_WRAP_R, GL::REPEAT as i32);
    gl.bind_texture(GL::TEXTURE_3D, None);
    tex
}

fn make_fallback(gl: &GL) -> WebGlTexture {
    let tex = gl.create_texture().expect("create_texture");
    let data = [255u8, 255, 255, 255];
    upload_2d(gl, &tex, 1, 1, &data);
    tex
}

impl Textures {
    pub fn new(gl: &GL) -> Self {
        Self {
            noise_lq: make_tex_2d(gl, 256, 256),
            noise_lq_lite: make_tex_2d(gl, 32, 32),
            noise_mq: make_tex_2d(gl, 256, 256),
            noise_hq: make_tex_2d(gl, 256, 256),
            noise_vol_lq: make_tex_3d(gl, 32),
            noise_vol_hq: make_tex_3d(gl, 32),
            extra: HashMap::new(),
            fallback: make_fallback(gl),
            noise_lq_timer: 0.0,
        }
    }

    pub fn update_animated(&mut self, gl: &GL, dt: f64) {
        self.noise_lq_timer += dt;
        if self.noise_lq_timer >= 1.0 / 30.0 {
            self.noise_lq_timer = 0.0;
            let data = rand_rgba(256 * 256);
            upload_2d(gl, &self.noise_lq, 256, 256, &data);
            let data_lite = rand_rgba(32 * 32);
            upload_2d(gl, &self.noise_lq_lite, 32, 32, &data_lite);
        }
    }

    /// Load extra images from `{ name: { width, height, data: Uint8Array } }`.
    pub fn load_extra_images(
        &mut self,
        gl: &GL,
        image_data: &wasm_bindgen::JsValue,
    ) -> Result<(), String> {
        use js_sys::{Object, Reflect, Uint8Array};
        let obj = Object::from(image_data.clone());
        let keys = Object::keys(&obj);
        for i in 0..keys.length() {
            let name_val = keys.get(i);
            let name = name_val.as_string().unwrap_or_default();
            let entry = Reflect::get(&obj, &name_val).map_err(|_| "reflect failed")?;
            let width = Reflect::get(&entry, &"width".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as i32;
            let height = Reflect::get(&entry, &"height".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as i32;
            let data_val = Reflect::get(&entry, &"data".into())
                .map_err(|_| "reflect data")?;
            let bytes = Uint8Array::new(&data_val).to_vec();

            let tex = gl.create_texture().ok_or("create_texture")?;
            upload_2d(gl, &tex, width, height, &bytes);
            self.extra.insert(name, tex);
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> &WebGlTexture {
        self.extra.get(name).unwrap_or(&self.fallback)
    }
}
