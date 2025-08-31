#[cfg(feature = "tui")]
mod keymap;
#[cfg(feature = "tui")]
mod theme;
#[cfg(feature = "tui")]
mod screens;
#[cfg(feature = "tui")]
mod app;
#[cfg(feature = "voice")]
mod audio;

#[cfg(feature = "tui")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}

#[cfg(not(feature = "tui"))]
fn main() {
    println!("ui-tui built without 'tui' feature. Run: cargo run -p ui-tui --features tui,http");
}
