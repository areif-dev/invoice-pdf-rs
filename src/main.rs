use std::error::Error;

use invoice_pdf::{AddressBuilder, InvoiceBuilder, PartyBuilder, generate_pdf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let inv = InvoiceBuilder::default()
        .id("1")
        .logo("./logo.png")
        .receiver(
            PartyBuilder::default()
                .name("A")
                .address(
                    AddressBuilder::default()
                        .line1("1 street st")
                        .city("city")
                        .province_code("PR")
                        .postal_code("Post")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .sender(
            PartyBuilder::default()
                .name("B")
                .address(
                    AddressBuilder::default()
                        .line1("1 street st")
                        .city("city")
                        .province_code("PR")
                        .postal_code("Post")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    generate_pdf(&inv).await.unwrap();
    Ok(())
}
