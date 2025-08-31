use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use std::io::Write as _;

fn encode_wav_pcm16(pcm: &[i16], sr: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    let byte_rate = sr * 2; // mono, 16-bit
    let block_align = 2u16;
    let subchunk2_size = (pcm.len() * 2) as u32;
    let chunk_size = 36 + subchunk2_size;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&chunk_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM subchunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    out.extend_from_slice(&1u16.to_le_bytes()); // channels
    out.extend_from_slice(&sr.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&subchunk2_size.to_le_bytes());
    for s in pcm { out.extend_from_slice(&s.to_le_bytes()); }
    out
}

pub async fn transcribe_openai_pcm16(pcm: &[i16], sr: u32) -> Result<String> {
    let key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
    let model = std::env::var("OPENAI_STT_MODEL").unwrap_or_else(|_| "whisper-1".into());
    let wav = encode_wav_pcm16(pcm, sr);
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let form = reqwest::multipart::Form::new()
        .text("model", model)
        .part("file", part);
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        anyhow::bail!(format!("openai stt http {}: {}", status, txt));
    }
    let v: serde_json::Value = resp.json().await?;
    let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok(text)
}

/// Transcribe using a local whisper.cpp binary. Requires env:
///  - WHISPER_CPP_BIN: path to whisper.cpp binary (e.g., ./whisper.cpp/main)
///  - WHISPER_CPP_MODEL: path to ggml model (e.g., ./models/ggml-base.en.bin)
/// Optional:
///  - WHISPER_LANG (default: en)
pub fn transcribe_whisper_cpp(pcm: &[i16], sr: u32) -> Result<String> {
    let bin = std::env::var("WHISPER_CPP_BIN").map_err(|_| anyhow::anyhow!("WHISPER_CPP_BIN not set"))?;
    let model = std::env::var("WHISPER_CPP_MODEL").map_err(|_| anyhow::anyhow!("WHISPER_CPP_MODEL not set"))?;
    let lang = std::env::var("WHISPER_LANG").unwrap_or_else(|_| "en".into());
    // Write temporary WAV under storage/tmp
    let tmp_dir = {
        let d = PathBuf::from("storage/tmp");
        let _ = std::fs::create_dir_all(&d);
        d
    };
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
    let wav_path = tmp_dir.join(format!("wake-{}.wav", ts));
    let out_prefix = tmp_dir.join(format!("wake-{}-out", ts));
    let mut f = std::fs::File::create(&wav_path)?;
    let wav = encode_wav_pcm16(pcm, sr);
    f.write_all(&wav)?;
    drop(f);

    // Call whisper.cpp: write pure text to <out_prefix>.txt
    // Flags: -l <lang>, -otxt to output .txt, -of <prefix>, -q for quiet, -bs 0 to disable benchmark prints (if supported)
    let status = Command::new(&bin)
        .args(["-m", &model, "-f", &wav_path.to_string_lossy(), "-l", &lang, "-otxt", "-of", &out_prefix.to_string_lossy(), "-q"]) 
        .status();
    match status {
        Ok(st) if st.success() => {
            let txt_path = PathBuf::from(format!("{}{}.txt", out_prefix.to_string_lossy(), ""));
            let text = std::fs::read_to_string(&txt_path).unwrap_or_default();
            // Clean up small temp files (best-effort)
            let _ = std::fs::remove_file(&wav_path);
            let _ = std::fs::remove_file(&txt_path);
            Ok(text.trim().to_string())
        }
        Ok(st) => Err(anyhow::anyhow!(format!("whisper.cpp exited with code {}", st.code().unwrap_or(-1)))),
        Err(e) => Err(anyhow::anyhow!(format!("spawn whisper.cpp: {}", e))),
    }
}
