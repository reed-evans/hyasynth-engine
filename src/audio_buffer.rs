// src/audio_buffer.rs

#[derive(Debug)]
pub struct AudioBuffer<'a> {
    pub channels: usize,
    pub frames: usize,
    pub data: &'a mut [f32], // interleaved: ch0..chN, frame by frame
}

impl<'a> AudioBuffer<'a> {
    /// Create a new AudioBuffer wrapping existing data.
    #[inline]
    pub fn new(data: &'a mut [f32], channels: usize) -> Self {
        let frames = data.len() / channels;
        Self {
            channels,
            frames,
            data,
        }
    }

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

    /// Get direct access to the interleaved sample data.
    #[inline]
    pub fn samples(&self) -> &[f32] {
        self.data
    }

    /// Get mutable access to the interleaved sample data.
    #[inline]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        self.data
    }
}
