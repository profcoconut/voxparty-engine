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
                // NOTE: do NOT detach — sink must live for audio to finish playing
            }
        }
    }

    /// Start looping background music.
    pub fn play_music(&mut self, path: &str) {
        if let Ok(file) = File::open(path) {
            if let Ok(decoder) = rodio::Decoder::new(file) {
                let sink = match Sink::try_new(&self._stream_handle) {
                    Ok(sink) => sink,
                    Err(e) => {
                        log::warn!("Failed to create audio sink for music: {}", e);
                        return;
                    }
                };
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

    /// Play a simple synthesized sine wave tone as a placeholder for real music.
    /// This allows the game to produce audio output without requiring audio files.
    pub fn play_tone(&mut self, freq: f32, duration_secs: f32) {
        use rodio::Source;
        if self.music_sink.is_none() {
            // Create a new sink for music if we don't have one
            if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                self.music_sink = Some(sink);
            } else {
                return;
            }
        }
        let source = rodio::source::SineWave::new(freq)
            .take_duration(std::time::Duration::from_secs_f32(duration_secs))
            .fade_in(std::time::Duration::from_millis(100));
        if let Some(ref sink) = self.music_sink {
            sink.append(source);
        }
    }

    /// Play a simple synthesized melody as placeholder background music.
    /// Plays a short ascending then descending melody loop.
    pub fn play_music_stub(&mut self) {
        use rodio::Source;
        if self.music_sink.is_none() {
            if let Ok(sink) = Sink::try_new(&self._stream_handle) {
                self.music_sink = Some(sink);
            } else {
                return;
            }
        }

        // Create a simple melody using repeated tones
        // C5-E5-G5-E5-C5-E5-G5-B5 (ascending, then higher)
        let notes = vec![
            (523.25, 0.15),  // C5
            (659.25, 0.15),  // E5
            (783.99, 0.15),  // G5
            (659.25, 0.15),  // E5
            (523.25, 0.15),  // C5
            (587.33, 0.15),  // D5
            (698.46, 0.15),  // F5
            (880.00, 0.25),  // A5
        ];

        let sources: Vec<_> = notes.iter().map(|(freq, dur)| {
            rodio::source::SineWave::new(*freq)
                .take_duration(std::time::Duration::from_secs_f32(*dur))
                .fade_in(std::time::Duration::from_millis(30))
                .take_duration(std::time::Duration::from_secs_f32((*dur - 0.03).max(0.0)))
        }).collect();

        if let Some(ref sink) = self.music_sink {
            for source in sources {
                sink.append(source);
            }
            // Repeat the melody a few times for continuous music effect
            sink.append(rodio::source::SineWave::new(0.0) // silent gap
                .take_duration(std::time::Duration::from_millis(500)));
        }
    }
}

impl Default for AudioManager {
    fn default() -> Self { Self::new() }
}
