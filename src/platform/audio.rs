use rodio::{Sink, Source, OutputStream, OutputStreamHandle};
use std::collections::HashMap;
use std::fs::File;

/// Simple audio manager using rodio.
pub struct AudioManager {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    music_sink: Option<Sink>,
    /// Maps sound name → WAV/MP3 file contents
    sfx_data: HashMap<String, Vec<u8>>,
}

impl AudioManager {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default()
            .expect("Failed to open audio output");
        Self {
            _stream: stream,
            _stream_handle: stream_handle,
            music_sink: None,
            sfx_data: HashMap::new(),
        }
    }

    /// Load a sound effect file into memory.
    pub fn load_sfx(&mut self, name: &str, path: &str) {
        if let Ok(data) = std::fs::read(path) {
            self.sfx_data.insert(name.to_string(), data);
        }
    }

    /// Play a sound effect by name.
    pub fn play_sfx(&self, name: &str) {
        let data = match self.sfx_data.get(name) {
            Some(d) => d,
            None => return,
        };
        let cursor = std::io::Cursor::new(data.clone());
        if let Ok(decoder) = rodio::Decoder::new(cursor) {
            if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                sink.append(decoder);
                sink.detach();
            }
        }
    }

    /// Start looping background music.
    pub fn play_music(&mut self, path: &str) {
        if let Ok(file) = File::open(path) {
            if let Ok(decoder) = rodio::Decoder::new(file) {
                let sink = Sink::try_new(&self._stream_handle).unwrap();
                sink.append(decoder.repeat_infinite());
                self.music_sink = Some(sink);
            }
        }
    }

    /// Stop background music.
    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }
}

impl Default for AudioManager {
    fn default() -> Self { Self::new() }
}
