pub mod error;
pub mod invoice;
pub mod template_env;

pub use error::Error;
pub use invoice::{
    Address, AddressBuilder, AddressBuilderError, Invoice, InvoiceBuilder, InvoiceBuilderError,
    LineItem, LineItemBuilder, LineItemBuilderError, Party, PartyBuilder, PartyBuilderError,
};

use error::AddContext;
use fantoccini::{
    ClientBuilder,
    wd::{PrintConfigurationBuilder, PrintMargins, PrintSize},
};
use serde_json::Map;
use std::{env::current_dir, fs};

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
    let path = current_dir()
        .map_err(crate::Error::from)
        .add_context("fetching current working dir to find invoice template")
        .add_context("printing pdf")?
        .join("invoice.html");
    let path = path
        .to_str()
        .ok_or(crate::Error::from(
            "fetching current working dir to find invoice template".to_string(),
        ))
        .add_context("printing pdf")?;
    client
        .goto(&format!("file://{path}"))
        .await
        .map_err(crate::Error::from)
        .add_context("navigating to address")
        .add_context("printing pdf")?;
    let bytes = client
        .print(
            PrintConfigurationBuilder::default()
                .margins(PrintMargins {
                    top: 0.5,
                    left: 1.5,
                    right: 1.5,
                    bottom: 0.5,
                })
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
    use std::{process::Command, thread::sleep, time::Duration};

    use super::*;

    #[tokio::test]
    async fn test_generate_pdf() {
        let mut c = Command::new("chromedriver")
            .arg("--port=4444")
            .spawn()
            .unwrap();
        sleep(Duration::from_secs(1));
        let inv = InvoiceBuilder::default()
            .id("test-inv")
            .sender(PartyBuilder::default().name("sender").build().unwrap())
            .receiver(PartyBuilder::default().name("receiver").build().unwrap())
            .build()
            .unwrap();
        let v = generate_pdf(&inv).await.unwrap();
        assert!(v.len() > 0);
        c.kill().unwrap();
    }
}
