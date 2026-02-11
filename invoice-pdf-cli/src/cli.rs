use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

use clap::Parser;
use invoice_pdf::{Invoice, error::AddContext};

fn read_until_eof() -> io::Result<String> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

#[derive(Debug, Parser)]
pub struct Cli {
    /// Path to the JSON file with invoice data to print
    #[arg(short, long)]
    pub data: Option<PathBuf>,

    /// Path to the directory where PDF outputs should be saved
    #[arg(short, long)]
    pub out: Option<PathBuf>,
}

impl Cli {
    pub fn get_invoices(&self) -> Result<Vec<Invoice>, invoice_pdf::Error> {
        let raw = match &self.data {
            Some(path) => fs::read_to_string(path)
                .map_err(invoice_pdf::Error::from)
                .add_context(&format!(
                    "reading invoice data from file '{}'",
                    path.to_str().unwrap_or("UNKNOWN")
                ))?,
            None => read_until_eof()
                .map_err(invoice_pdf::Error::from)
                .add_context("reading invoice data from stdin")?,
        };

        Ok(serde_json::from_str(&raw)
            .map_err(|e| invoice_pdf::Error::from(format!("{:?}", e)))
            .add_context("parsing invoice JSON")?)
    }
}
