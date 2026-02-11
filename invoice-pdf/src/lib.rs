//! Utilities for generating professional invoices as PDFs
//!
//! This crate provides functions to render invoice templates and produce PDF files
//! using a connection to headless chrome(ium). It exposes the invoice model types and a high-level
//! function to generate a PDF from an [`Invoice`].
//!
//! # Example
//!
//! ```rust
//! use invoice_pdf::{Invoice, InvoiceBuilder, generate_pdf, PartyBuilder, AddressBuilder};
//!
//! let inv = InvoiceBuilder::default()
//!     .id("1")
//!     .logo("./logo.png")
//!     .receiver(
//!         PartyBuilder::default()
//!             .name("A")
//!             .address(
//!                 AddressBuilder::default()
//!                 .line1("1 street st")
//!                 .city("city")
//!                 .province_code("PR")
//!                 .postal_code("Post")
//!                 .build().unwrap()
//!             )
//!             .build().unwrap())
//!     .sender(
//!         PartyBuilder::default()
//!             .name("B")
//!             .address(
//!                 AddressBuilder::default()
//!                 .line1("1 street st")
//!                 .city("city")
//!                 .province_code("PR")
//!                 .postal_code("Post")
//!                 .build().unwrap()
//!             )
//!         .build().unwrap())
//!     .build().unwrap();
//! generate_pdf(&inv);
//! ```

pub mod error;
pub mod invoice;
pub mod template_env;

use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use base64::{Engine, engine::general_purpose};
pub use error::Error;
pub use invoice::{
    Address, AddressBuilder, AddressBuilderError, Invoice, InvoiceBuilder, InvoiceBuilderError,
    LineItem, LineItemBuilder, LineItemBuilderError, Party, PartyBuilder, PartyBuilderError,
};

use error::AddContext;
use fantoccini::{
    Client, ClientBuilder,
    wd::{PrintConfigurationBuilder, PrintMargins, PrintSize},
};
use serde_json::Map;

use crate::template_env::{render_template, setup_template_env};

/// Starts ChromeDriver as a child process on port 4444
///
/// # Returns
/// - [`Child`] if ChromeDriver successfully starts and the port is available
///
/// # Errors
/// - [`crate::Error`] if the chromedriver binary is not in the path, or if port 4444 is not
/// available, or if the chromedriver process fails to start for any other reason
pub fn start_chromedriver() -> Result<Child, crate::Error> {
    if is_port_in_use(4444) {
        return Err(
            crate::Error::from("Port 4444 is already in use".to_string())
                .add_context("starting chromedriver"),
        );
    }

    let mut child = Command::new("chromedriver")
        .arg("--port=4444")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    for _ in 0..100 {
        if is_port_in_use(4444) {
            return Ok(child);
        }

        if child
            .try_wait()
            .map_err(crate::Error::from)
            .add_context("starting chromedriver")?
            .is_some()
        {
            return Err(
                crate::Error::from(String::from("Chromedriver has stopped unexpectedly"))
                    .add_context("starting chromedriver"),
            )?;
        }

        thread::sleep(Duration::from_millis(10));
    }

    // Double-check port is now in use
    if !is_port_in_use(4444u16) {
        // Kill the child process if it didn't bind to the port
        child.kill()?;
        return Err(
            crate::Error::from(String::from("Chromedriver failed to bind to port 4444"))
                .add_context("starting chromedriver"),
        )?;
    }

    Ok(child)
}

/// Check if a given port is currently in use
///
/// # Arguments
/// - `port` The port number to check
///
/// # Returns
/// - `true` if the TCP port is currently on use on the localhost
/// - `false` if the TCP port is not being used on localhost
fn is_port_in_use(port: u16) -> bool {
    TcpListener::bind(format!("localhost:{port}")).is_err()
}

async fn connect_to_client() -> Result<Client, fantoccini::error::NewSessionError> {
    let mut caps = Map::new();
    caps.insert(
        "goog:chromeOptions".to_string(),
        serde_json::json!({
            "args": ["--headless"]
        }),
    );
    ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await
}

/// Generate a PDF byte array from [`Invoice`]
///
/// This function renders an HTML template from the provided [`Invoice`],
/// navigates a headless browser to the rendered HTML, prints the page as a PDF, and returns the
/// resulting byte array
///
/// # Arguments
///
/// - `invoice`: Reference to the [`Invoice`] to render and print.
///
/// # Returns
///
/// - The byte array representing the PDF if successful
///
/// # Errors
///
/// Returns `Err(crate::Error)` if any step fails:
/// - connecting to the headless browser [`Client`]
/// - setting up the templating environment
/// - rendering the HTML template
/// - navigating the browser to the generated data URL
/// - configuring the print job or printing to PDF
///
/// # Example
///
/// ```rust
/// use invoice_pdf::{Invoice, InvoiceBuilder, PartyBuilder, AddressBuilder, generate_pdf};
///
/// let inv = InvoiceBuilder::default()
///     .id("1")
///     .logo("./logo.png")
///     .receiver(
///         PartyBuilder::default()
///             .name("A")
///             .address(
///                 AddressBuilder::default()
///                 .line1("1 street st")
///                 .city("city")
///                 .province_code("PR")
///                 .postal_code("Post")
///                 .build().unwrap()
///             )
///             .build().unwrap())
///     .sender(
///         PartyBuilder::default()
///             .name("B")
///             .address(
///                 AddressBuilder::default()
///                 .line1("1 street st")
///                 .city("city")
///                 .province_code("PR")
///                 .postal_code("Post")
///                 .build().unwrap()
///             )
///         .build().unwrap())
///     .build().unwrap();
/// generate_pdf(&inv);
/// ```
pub async fn generate_pdf(invoice: &Invoice) -> Result<Vec<u8>, crate::Error> {
    let client = connect_to_client()
        .await
        .map_err(crate::Error::from)
        .add_context("connecting to client")
        .add_context("generating pdf")?;
    let template_env = setup_template_env()
        .map_err(crate::Error::from)
        .add_context("setting up templating environment")
        .add_context("generating pdf")?;
    let render = render_template(&template_env, invoice)
        .map_err(crate::Error::from)
        .add_context("rendering html template")
        .add_context("generating pdf")?;
    let encoded = general_purpose::STANDARD.encode(render.as_bytes());
    let data_url = format!("data:text/html;base64,{encoded}");
    client
        .goto(&data_url)
        .await
        .map_err(crate::Error::from)
        .add_context("navigating to address")
        .add_context("printing pdf")?;
    Ok(client
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
        .add_context("printing pdf")?)
}

#[cfg(test)]
mod tests {
    use std::{process::Command, thread::sleep, time::Duration};

    use bigdecimal::BigDecimal;

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
            .add_line(
                LineItemBuilder::default()
                    .sku("test")
                    .quantity(2)
                    .price(10)
                    .title("this is a test")
                    .build()
                    .unwrap(),
            )
            .add_line(
                LineItemBuilder::default()
                    .sku("test")
                    .quantity(1)
                    .price(10)
                    .title("this is a test")
                    .build()
                    .unwrap(),
            )
            .paid(BigDecimal::from(1))
            .build()
            .unwrap();
        let v = generate_pdf(&inv).await.unwrap();
        std::fs::write("test.pdf", &v).unwrap();
        assert!(v.len() > 0);
        c.kill().unwrap();
    }
}
