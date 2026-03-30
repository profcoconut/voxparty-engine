use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::time::Instant;
use serde::Serialize;
use serde_json::json;

/// Suffix appended to all telemetry session filenames.
const SESSION_DIR: &str = "telemetry";
/// Flush to disk every N lines.
const BUFFER_FLUSH_THRESHOLD: usize = 64;

pub struct TelemetryLogger {
    file: Option<BufWriter<File>>,
    buffer: Vec<String>,
    frame_count: u64,
    start: Instant,
}

impl TelemetryLogger {
    /// Create a new logger. No session is active until `session_start` is called.
    pub fn new() -> Self {
        Self {
            file: None,
            buffer: Vec::with_capacity(BUFFER_FLUSH_THRESHOLD * 2),
            frame_count: 0,
            start: Instant::now(),
        }
    }

    /// Milliseconds elapsed since this logger was created.
    fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Start a new telemetry session.
    /// Creates `telemetry/` directory and opens `telemetry/session_<session_id>.jsonl`.
    /// If `session_id` is None, uses unix timestamp. For tests, pass a fixed string like "0".
    /// Writes a JSON header line describing the session.
    pub fn session_start(&mut self, episode_id: &str, session_id: Option<&str>) {
        // Create telemetry directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(SESSION_DIR) {
            eprintln!("[TELEMETRY] Failed to create {} directory: {}", SESSION_DIR, e);
            return;
        }

        let id = match session_id {
            Some(s) => format!("session_{}", s),
            None => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                format!("session_{}", ts)
            }
        };

        let path = format!("{}/{}.jsonl", SESSION_DIR, id);

        let file = match OpenOptions::new()
            .create(true)
            .write(true)
            .append(false)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[TELEMETRY] Failed to open {}: {}", path, e);
                return;
            }
        };

        self.file = Some(BufWriter::new(file));
        self.frame_count = 0;

        // Write header line
        let header = json!({
            "version": 1,
            "episode": episode_id,
            "players": 2
        });
        if let Err(e) = self.write_line(&serde_json::to_string(&header).unwrap()) {
            eprintln!("[TELEMETRY] Failed to write session header: {}", e);
        }
    }

    /// Emit a telemetry event. Buffered — flushes every 64 lines or on `flush()`/`drop`.
    pub fn log_event(&mut self, event: &str, payload: impl Serialize) {
        self.frame_count += 1;
        let line = match serde_json::to_string(&json!({
            "ts_ms": self.elapsed_ms(),
            "frame": self.frame_count,
            "event": event,
            "payload": payload,
        })) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[TELEMETRY] Failed to serialize event '{}': {}", event, e);
                return;
            }
        };
        self.buffer.push(line);
        if self.buffer.len() >= BUFFER_FLUSH_THRESHOLD {
            self.flush();
        }
    }

    fn write_line(&mut self, line: &str) -> Result<(), std::io::Error> {
        if let Some(ref mut f) = self.file {
            f.write_all(line.as_bytes())?;
            f.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Flush buffered lines to disk.
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        if let Some(ref mut f) = self.file {
            for line in self.buffer.drain(..) {
                if let Err(e) = f.write_all(line.as_bytes()) {
                    eprintln!("[TELEMETRY] Write error: {}", e);
                }
                if let Err(e) = f.write_all(b"\n") {
                    eprintln!("[TELEMETRY] Write error: {}", e);
                }
            }
            if let Err(e) = f.flush() {
                eprintln!("[TELEMETRY] Flush error: {}", e);
            }
        } else {
            self.buffer.clear();
        }
    }

    /// Close the session cleanly. Flushes any remaining buffered lines.
    pub fn session_end(&mut self) {
        self.flush();
        if let Some(ref mut f) = self.file {
            if let Err(e) = f.flush() {
                eprintln!("[TELEMETRY] Final flush error: {}", e);
            }
        }
        self.file = None;
    }
}

impl Default for TelemetryLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TelemetryLogger {
    fn drop(&mut self) {
        self.session_end();
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use serde_json::json;

    // Use nanosecond timestamp for unique session IDs in tests
    fn temp_logger() -> (TelemetryLogger, PathBuf) {
        let mut logger = TelemetryLogger::new();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos().to_string())
            .unwrap_or_else(|_| "0".to_string());
        logger.session_start("test_episode", Some(&unique_id));
        let session_path = Path::new(SESSION_DIR).join(format!("session_{}.jsonl", unique_id));
        (logger, session_path)
    }

    #[test]
    fn test_session_creates_file_and_header() {
        let (logger, path) = temp_logger();
        drop(logger);

        assert!(path.exists(), "Session file should exist: {:?}", path);
        let content = fs::read_to_string(&path).unwrap();
        let first_line: serde_json::Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(first_line["version"], 1);
        assert_eq!(first_line["episode"], "test_episode");
        assert_eq!(first_line["players"], 2);
    }

    #[test]
    fn test_log_event_writes_json_line() {
        let (mut logger, path) = temp_logger();
        logger.log_event("test_event", json!({"key": "value"}));
        logger.session_end();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        // Header + 1 event = 2 lines
        assert_eq!(lines.len(), 2);
        let event_line: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(event_line["event"], "test_event");
        assert_eq!(event_line["payload"]["key"], "value");
        assert!(event_line["frame"].as_u64().is_some());
        assert!(event_line["ts_ms"].as_u64().is_some());
    }

    #[test]
    fn test_flush_on_64_lines() {
        let (mut logger, path) = temp_logger();
        // Log exactly 64 events
        for i in 0..64 {
            logger.log_event("item", json!({"n": i}));
        }
        // File should already have header + 64 lines (flushed at 64)
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 65); // 1 header + 64 events
    }

    #[test]
    fn test_flush_on_drop() {
        let (mut logger, path) = temp_logger();
        // Log a few events without manual flush
        logger.log_event("a", json!({}));
        logger.log_event("b", json!({}));
        // Drop without explicit session_end — drop should flush
        drop(logger);

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3); // 1 header + 2 events
    }
}