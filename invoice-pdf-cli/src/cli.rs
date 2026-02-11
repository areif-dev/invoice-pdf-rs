use std::path::PathBuf;

use clap::Parser;
use invoice_pdf::Invoice;

#[derive(Debug, Parser)]
pub struct Cli {
    /// Path to the JSON file with invoice data to print
    #[arg(short, long)]
    data: Option<PathBuf>,

    /// Path to the directory where PDF outputs should be saved
    #[arg(short, long)]
    out: Option<PathBuf>,
}

impl Cli {
    pub fn get_data(&self) -> Result<Vec<Invoice>, invoice_pdf::Error> {
        Ok(Vec::new())
    }
}
