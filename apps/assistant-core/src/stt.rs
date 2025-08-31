use anyhow::Result;

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

pub async fn transcribe_local_pcm16(pcm: &[i16], sr: u32, endpoint: Option<&str>) -> Result<String> {
    let url = endpoint.map(|s| s.to_string()).unwrap_or_else(|| std::env::var("LOCAL_STT_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:7071/v1/stt/transcribe".into()));
    let client = reqwest::Client::new();
    // Body is raw PCM16 little-endian, mono
    let mut bytes = Vec::with_capacity(pcm.len()*2);
    for s in pcm { bytes.extend_from_slice(&s.to_le_bytes()); }
    let resp = client.post(&url).header("content-type","application/octet-stream").header("x-sample-rate", sr.to_string()).body(bytes).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        anyhow::bail!(format!("local stt http {}: {}", status, txt));
    }
    let v: serde_json::Value = resp.json().await?;
    let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok(text)
}
