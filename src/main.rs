use std::error::Error;

use invoice_pdf_rs::{error::AddContext, start_driver};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut driver_process = start_driver("./chromedriver-win64/chromedriver.exe").unwrap();
    invoice_pdf_rs::print().await.unwrap();
    driver_process
        .kill()
        .map_err(invoice_pdf_rs::Error::from)
        .add_context("printing finished. killing webdriver")?;
    Ok(())
}
