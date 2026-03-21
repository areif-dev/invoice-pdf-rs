//! Utilities for setting up and using the HTML template environment.
//!
//! This module provides template filters and helpers used when rendering
//! invoices to HTML. It registers custom filters for formatting RFC3339
//! datetimes and decimal prices, and offers a convenience function to
//! render an Invoice into HTML using the askama template engine.

use askama::Template;
use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset};

use crate::invoice::Invoice;

/// Define the filters module for Askama.
/// Askama automatically looks for a `filters` module in the same scope as the template.
mod filters {
    use super::*;

    pub fn format_ymd_helper(dt: &DateTime<FixedOffset>) -> String {
        dt.format("%Y-%m-%d").to_string()
    }

    pub fn pretty_price_helper(b: BigDecimal) -> String {
        format!("${:.2}", b)
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
    pub fn pretty_price(b: BigDecimal, _env: &dyn askama::Values) -> askama::Result<String> {
        Ok(pretty_price_helper(b))
    }
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct InvoiceTemplate<'a> {
    pub invoice: &'a Invoice,
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
        assert_eq!(filters::pretty_price_helper(b), "$19.99");
    }

    #[test]
    fn test_render_template() {
        let inv = InvoiceBuilder::default()
            .id("test id")
            .sender(PartyBuilder::default().name("sender").build().unwrap())
            .receiver(PartyBuilder::default().name("receiver").build().unwrap())
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
        assert!(render.contains("<strong>sender</strong>"));
        assert!(render.contains("<strong>receiver</strong>"));
        assert!(render.contains("<td>test</td>"));
        assert!(render.contains("<td>this is a test</td>"));
        assert!(render.contains(r#"<td style="text-align: right;">$20.00</td>"#));
        assert!(render.contains(r#"<td style="text-align:right;">$30.00</td>"#));
        assert!(render.contains(r#"<td style="text-align:right;">$29.00</td>"#));
    }
}
