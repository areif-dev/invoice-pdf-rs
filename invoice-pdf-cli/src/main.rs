use std::{io::Write, process::Child};

use clap::Parser;
use invoice_pdf::{Invoice, error::AddContext, generate_pdf, start_chromedriver};

use crate::cli::Cli;

mod cli;

fn kill_chrome(chrome_process: &mut Child) -> Result<(), invoice_pdf::Error> {
    chrome_process
        .kill()
        .map_err(invoice_pdf::Error::from)
        .add_context("killing chromedriver process from cli")?;
    Ok(())
}

async fn write_invoice_pdf(invoice: &Invoice, cli: &Cli) -> Result<(), invoice_pdf::Error> {
    let data = generate_pdf(invoice)
        .await
        .add_context("generating pdf data from invoice")
        .add_context(&format!("invoice id: {}", invoice.id()))?;
    match &cli.out {
        Some(out) => {
            let path = out.join(format!("{}.pdf", invoice.id()));
            if std::fs::write(&path, data).is_err() {
                eprintln!(
                    "Failed to write invoice '{}' to '{}'",
                    invoice.id(),
                    &path.to_string_lossy()
                );
                write_invoice_pdf_to_stdout(invoice).await
            } else {
                Ok(())
            }
        }
        None => write_invoice_pdf_to_stdout(invoice).await,
    }
}

async fn write_invoice_pdf_to_stdout(invoice: &Invoice) -> Result<(), invoice_pdf::Error> {
    let mut buf = generate_pdf(invoice)
        .await
        .add_context("generating invoice pdf")
        .add_context("printing to stdout")?;
    std::io::stdout()
        .write_all(&mut buf)
        .map_err(invoice_pdf::Error::from)
        .add_context("writing invoice pdf to stdout")?;
    std::io::stdout()
        .flush()
        .map_err(invoice_pdf::Error::from)
        .add_context("flushing stdout")
        .add_context("printing to stdout")?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), invoice_pdf::Error> {
    let mut chrome_process = start_chromedriver().add_context("starting chromedriver in cli")?;
    let cli = Cli::parse();
    let invoices = cli
        .get_invoices()
        .or_else(|e| {
            kill_chrome(&mut chrome_process)?;
            Err(e)
        })
        .add_context("deserializing invoices from cli")?;
    for invoice in invoices {
        write_invoice_pdf(&invoice, &cli).await?;
    }
    kill_chrome(&mut chrome_process)?;
    Ok(())
}
