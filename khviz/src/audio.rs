pub struct AudioLevels {
    val: [f32; 3],
    att: [f32; 3],
    avg: [f32; 3],
    long_avg: [f32; 3],
    frame: u64,
}

impl Default for AudioLevels {
    fn default() -> Self {
        Self {
            val: [1.0; 3],
            att: [1.0; 3],
            avg: [0.0; 3],
            long_avg: [0.001; 3],
            frame: 0,
        }
    }
}

impl AudioLevels {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, freq: &[f32], sample_rate: f32, fft_size: usize, fps: f32) {
        let num_bins = freq.len();
        if num_bins == 0 {
            return;
        }
        let bucket_hz = sample_rate / fft_size as f32;

        let bass_low = ((20.0_f32 / bucket_hz).round() as usize)
            .saturating_sub(1)
            .min(num_bins - 1);
        let bass_high = ((320.0_f32 / bucket_hz).round() as usize)
            .saturating_sub(1)
            .min(num_bins - 1);
        let mid_high = ((2800.0_f32 / bucket_hz).round() as usize)
            .saturating_sub(1)
            .min(num_bins - 1);
        let treb_high = ((11025.0_f32 / bucket_hz).round() as usize)
            .saturating_sub(1)
            .min(num_bins - 1);

        let bands = [
            (bass_low, bass_high),
            (bass_high, mid_high),
            (mid_high, treb_high),
        ];

        let fps_clamped = fps.clamp(15.0, 144.0);
        let pow_factor = 30.0 / fps_clamped;

        for i in 0..3 {
            let (lo, hi) = bands[i];
            let imm = if lo < hi && hi < num_bins {
                freq[lo..=hi].iter().sum::<f32>()
            } else {
                0.0
            };

            let rate = if imm > self.avg[i] { 0.2_f32 } else { 0.5_f32 };
            let rate = rate.powf(pow_factor);
            self.avg[i] = self.avg[i] * rate + imm * (1.0 - rate);

            let long_rate = if self.frame < 50 { 0.9_f32 } else { 0.992_f32 };
            let long_rate = long_rate.powf(pow_factor);
            self.long_avg[i] = self.long_avg[i] * long_rate + imm * (1.0 - long_rate);

            if self.long_avg[i] < 0.001 {
                self.val[i] = 1.0;
                self.att[i] = 1.0;
            } else {
                self.val[i] = imm / self.long_avg[i];
                self.att[i] = self.avg[i] / self.long_avg[i];
            }
        }
        self.frame += 1;
    }

    pub fn bass(&self) -> f32 { self.val[0] }
    pub fn mid(&self) -> f32 { self.val[1] }
    pub fn treb(&self) -> f32 { self.val[2] }
    pub fn bass_att(&self) -> f32 { self.att[0] }
    pub fn mid_att(&self) -> f32 { self.att[1] }
    pub fn treb_att(&self) -> f32 { self.att[2] }
    pub fn vol(&self) -> f32 { (self.val[0] + self.val[1] + self.val[2]) / 3.0 }
    pub fn vol_att(&self) -> f32 { (self.att[0] + self.att[1] + self.att[2]) / 3.0 }
}
