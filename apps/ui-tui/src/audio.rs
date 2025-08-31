#![cfg(feature = "voice")]
use std::io::Write;

pub struct VoiceRecorder {
    child: std::process::Child,
    path: std::path::PathBuf,
}

impl VoiceRecorder {
    pub fn start() -> anyhow::Result<Self> {
        // Ensure storage/tmp exists
        let tmp_dir = std::path::Path::new("storage/tmp");
        std::fs::create_dir_all(tmp_dir)?;
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let path = tmp_dir.join(format!("voice-{}.wav", ts));

        // Prefer ffmpeg for graceful stop by sending 'q' to stdin
        let mut use_ffmpeg = false;
        if std::process::Command::new("ffmpeg").arg("-version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok() {
            use_ffmpeg = true;
        }

        let child = if use_ffmpeg {
            // Linux ALSA default device; mono, 16 kHz
            std::process::Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-f", "alsa", "-i", "default", "-ac", "1", "-ar", "16000", "-y"])
                .arg(&path)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("failed to start ffmpeg: {}", e))?
        } else {
            // Fallback to arecord; mono, 16 kHz, 16-bit LE, WAV
            std::process::Command::new("arecord")
                .args(["-q", "-f", "S16_LE", "-r", "16000", "-c", "1", "-t", "wav"])
                .arg(&path)
                .stdin(std::process::Stdio::null())
                .spawn()
                .map_err(|e| anyhow::anyhow!("failed to start arecord: {}", e))?
        };

        Ok(Self { child, path })
    }

    pub fn stop_and_into_wav(mut self) -> anyhow::Result<Vec<u8>> {
        // Try graceful stop if ffmpeg (send 'q') else kill
        if let Some(mut stdin) = self.child.stdin.take() {
            let _ = stdin.write_all(b"q\n");
        } else {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();

        // Read file and remove it
        let bytes = std::fs::read(&self.path)?;
        let _ = std::fs::remove_file(&self.path);
        Ok(bytes)
    }
}
