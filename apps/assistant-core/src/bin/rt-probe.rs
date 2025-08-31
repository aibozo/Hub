use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let base = std::env::var("FOREMAN_CORE").unwrap_or_else(|_| "http://127.0.0.1:6061".into());
    if args.len() < 2 {
        eprintln!("usage: rt-probe [status|start|stop] [endpoint]");
        std::process::exit(2);
    }
    let cmd = args[1].as_str();
    match cmd {
        "status" => {
            let url = format!("{}/api/realtime/status", base);
            let v: serde_json::Value = reqwest::get(&url).await?.json().await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        "start" => {
            let url = format!("{}/api/realtime/start", base);
            let endpoint = args.get(2).cloned();
            let body = json!({
                "model": std::env::var("OPENAI_REALTIME_MODEL").ok().unwrap_or_else(|| "gpt-realtime".into()),
                "voice": "alloy",
                "audio": {"in_sr": 16000, "out_format": "g711_ulaw"},
                "endpoint": endpoint,
            });
            let resp = reqwest::Client::new().post(url).json(&body).send().await?;
            println!("http {}", resp.status());
            if !resp.status().is_success() {
                let v: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({"error":"(no body)"}));
                println!("{}", serde_json::to_string_pretty(&v)?);
            }
        }
        "stop" => {
            let url = format!("{}/api/realtime/stop", base);
            let resp = reqwest::Client::new().post(url).send().await?;
            println!("http {}", resp.status());
        }
        _ => {
            eprintln!("unknown cmd: {}", cmd);
            std::process::exit(2);
        }
    }
    Ok(())
}

