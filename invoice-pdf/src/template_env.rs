//! Utilities for setting up and using the HTML template environment.
//!
//! This module provides template filters and helpers used when rendering
//! invoices to HTML. It registers custom filters for formatting RFC3339
//! datetimes and decimal prices, and offers a convenience function to
//! render an Invoice into HTML using the askama template engine.

use std::io::Cursor;

use askama::Template;
use base64::{Engine, engine::general_purpose};
use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset};
use image::Luma;
use qrcode::QrCode;

use crate::invoice::Invoice;

/// Define the filters module for Askama.
/// Askama automatically looks for a `filters` module in the same scope as the template.
mod filters {
    use super::*;

    pub fn format_ymd_helper(dt: &DateTime<FixedOffset>) -> String {
        dt.format("%Y-%m-%d").to_string()
    }

    pub fn pretty_price_helper(b: BigDecimal, max_fractional_digits: usize) -> String {
        format!("${:.max_fractional_digits$}", b)
    }

    /// Format a datetime as YYYY-MM-DD.
    #[askama::filter_fn]
    pub fn format_ymd(
        dt: &DateTime<FixedOffset>,
        _env: &dyn askama::Values,
    ) -> askama::Result<String> {
        Ok(format_ymd_helper(dt))
    }

    /// Format a BigDecimal as a US-style dollar amount.
    #[askama::filter_fn]
    pub fn pretty_price(
        b: BigDecimal,
        _env: &dyn askama::Values,
        max_fractional_digits: usize,
    ) -> askama::Result<String> {
        Ok(pretty_price_helper(b, max_fractional_digits))
    }
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct InvoiceTemplate<'a> {
    pub invoice: &'a Invoice,
}

impl<'a> InvoiceTemplate<'a> {
    /// Returns the logo as a base64 encoded data URI if it exists.
    pub fn logo_data_uri(&self) -> Option<String> {
        let path = self.invoice.logo().as_ref()?;
        let data = std::fs::read(path).ok()?;
        let encoded = general_purpose::STANDARD.encode(&data);
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();

        let mime_type = match extension.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "tiff" | "tif" => "image/tiff",
            _ => "image/png",
        };

        Some(format!("data:{};base64,{}", mime_type, encoded))
    }

    /// Creates a data url containing a base64 encoded qrcode image that will take the user to a
    /// payment portal if one is configured for the invoice.
    ///
    /// If there is no payment portal or if the qrcode image or base64 encoding fails, then return
    /// `None`
    pub fn payment_qrcode_data_uri(&self) -> Option<String> {
        let payment_url = self.invoice.payment_url().as_ref()?.as_bytes();
        let qrcode = QrCode::new(payment_url).ok()?;
        let image = qrcode.render::<Luma<u8>>().max_dimensions(100, 100).build();
        let mut buf = Cursor::new(Vec::new());
        let dynimg = image::DynamicImage::ImageLuma8(image);
        dynimg.write_to(&mut buf, image::ImageFormat::Png).ok()?;
        let image_bytes = buf.into_inner();
        let encoded = general_purpose::STANDARD.encode(&image_bytes);
        Some(format!("data:image/png;base64,{}", encoded))
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use std::str::FromStr;

    use crate::{InvoiceBuilder, LineItemBuilder, PartyBuilder};

    use super::*;

    #[test]
    fn test_format_ymd() {
        let dt = chrono::Utc
            .with_ymd_and_hms(2026, 02, 09, 12, 00, 00)
            .unwrap()
            .into();
        assert_eq!(filters::format_ymd_helper(&dt), "2026-02-09");
    }

    #[test]
    fn test_pretty_price() {
        let b = BigDecimal::from_str("19.99").unwrap();
        assert_eq!(filters::pretty_price_helper(b, 2), "$19.99");
    }

    #[test]
    fn test_render_template() {
        let inv = InvoiceBuilder::default()
            .id("test id")
            .sender(PartyBuilder::default().name("sender").build().unwrap())
            .bill_to(PartyBuilder::default().name("bill_to").build().unwrap())
            .add_line(
                LineItemBuilder::default()
                    .sku("test")
                    .quantity(2)
                    .price(BigDecimal::from(10))
                    .title("this is a test")
                    .build()
                    .unwrap(),
            )
            .add_line(
                LineItemBuilder::default()
                    .sku("test")
                    .quantity(1)
                    .price(BigDecimal::from(10))
                    .title("this is a test")
                    .build()
                    .unwrap(),
            )
            .paid(BigDecimal::from(1))
            .build()
            .unwrap();
        let render = InvoiceTemplate { invoice: &inv }.render().unwrap();
        assert!(render.starts_with("<!DOCTYPE html>"));
        assert!(render.contains("<td>test id</td>"));
        assert!(render.contains("sender"));
        assert!(render.contains("bill_to"));
        assert!(render.contains("<td>test</td>"));
        assert!(render.contains("<td>this is a test</td>"));
        assert!(render.contains(r#"<td style="text-align: right;">$20.00</td>"#));
        assert!(render.contains(r#"<td style="text-align:right;">$30.00</td>"#));
        assert!(render.contains(r#"<td style="text-align:right;">$29.00</td>"#));
    }
}
