// src/audio_buffer.rs

#[derive(Debug)]
pub struct AudioBuffer<'a> {
    pub channels: usize,
    pub frames: usize,
    pub data: &'a mut [f32], // interleaved: ch0..chN, frame by frame
}

impl<'a> AudioBuffer<'a> {
    #[inline]
    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }

    #[inline]
    pub fn channel(&self, ch: usize) -> &[f32] {
        let start = ch * self.frames;
        &self.data[start..start + self.frames]
    }

    #[inline]
    pub fn channel_mut(&mut self, ch: usize) -> &mut [f32] {
        let start = ch * self.frames;
        &mut self.data[start..start + self.frames]
    }
}
