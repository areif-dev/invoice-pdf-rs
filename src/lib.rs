pub mod error;

pub use error::Error;

use error::AddContext;
use fantoccini::{
    ClientBuilder,
    wd::{PrintConfigurationBuilder, PrintSize},
};
use serde_json::Map;
use std::{
    ffi::OsStr,
    fs,
    process::{Child, Command},
    thread::sleep,
    time::Duration,
};

pub fn start_driver(driver_path: impl AsRef<OsStr>) -> Result<Child, Error> {
    Ok(Command::new(&driver_path)
        .args(["--port=4444"])
        .spawn()
        .map_err(crate::Error::from)
        .add_context("Attempting to start webdriver")?)
}

pub async fn print() -> Result<(), crate::Error> {
    let mut caps = Map::new();
    caps.insert(
        "goog:chromeOptions".to_string(),
        serde_json::json!({
            "args": ["--headless"]
        }),
    );
    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await
        .map_err(crate::Error::from)
        .add_context("connecting to client")
        .add_context("printing pdf")?;
    client
        .goto("https://ajreifsnyder.com")
        .await
        .map_err(crate::Error::from)
        .add_context("navigating to address")
        .add_context("printing pdf")?;
    sleep(Duration::from_secs(1));
    let bytes = client
        .print(
            PrintConfigurationBuilder::default()
                .size(PrintSize::US_LETTER)
                .build()
                .map_err(crate::Error::from)
                .add_context("configuring printer")
                .add_context("printing pdf")?,
        )
        .await
        .map_err(crate::Error::from)
        .add_context("printing pdf")?;
    fs::write("test.pdf", &bytes)
        .map_err(crate::Error::from)
        .add_context("saving file")
        .add_context("printing pdf")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
