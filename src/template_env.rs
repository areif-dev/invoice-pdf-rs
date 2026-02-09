//! Utilities for setting up and using the HTML template environment.
//!
//! This module provides template filters and helpers used when rendering
//! invoices to HTML. It registers custom filters for formatting RFC3339
//! datetimes and decimal prices, exposes a pre-defined base template, and
//! offers a convenience function to render an Invoice into HTML using the
//! minijinja template environment.
//!
use std::str::FromStr;

use bigdecimal::BigDecimal;
use minijinja::context;

use crate::invoice::Invoice;

/// A custom filter used in the Invoice template. Parses an RFC3339 datetime string and
/// returns only the year-month-day portion. If the input is not an RFC3339 string, then the
/// returned value will be "N/A"
///
/// # Arguments
/// * `raw` - A string slice containing an RFC3339 datetime (e.g. "2024-01-02T15:04:05Z"). If the
/// string is not an RFC3339 string, then the value returned will be "N/A"
///
/// # Returns
/// A string with the date formatted as YYYY-MM-DD. If parsing fails, returns "N/A".
///
/// # Example
/// ```rust
/// use invoice_pdf::template_env;
///
/// let s = "2024-01-02T15:04:05Z";
/// assert_eq!(&template_env::format_ymd(s), "2024-01-02");
/// ```
pub fn format_ymd(raw: &str) -> String {
    let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(raw) else {
        return String::from("N/A");
    };
    datetime.format("%Y-%m-%d").to_string()
}

/// A custom filter to be used in the Invoice template. Format a decimal-like string as a
/// US-style dollar amount with two decimals.
///
/// # Arguments
/// * `raw` - A string slice containing a decimal representation (e.g. "12.5").
///
/// # Returns
/// A string with a leading $ and exactly two fractional digits (e.g. "$12.50").
/// If parsing fails, returns "NaN".
///
/// # Example
/// ```rust
/// use invoice_pdf::template_env;
///
/// let s = "12.5";
/// assert_eq!(&template_env::pretty_price(s), "$12.50");
/// ```
pub fn pretty_price(raw: &str) -> String {
    let Ok(dec) = BigDecimal::from_str(&raw) else {
        return String::from("NaN");
    };
    format!("${:.2}", dec)
}

/// Create and configure a minijinja template environment.
///
/// Registers the [`format_ymd`] and [`pretty_price`] filters and loads the
/// built-in base HTML template.
///
/// # Returns
/// * [`minijinja::Environment`] with the configured environment on success.
///
/// # Errors
/// * [`minijinja::Error`] if loading the built-in template fails.
///
/// # Example
/// ```rust
/// use invoice_pdf::template_env;
///
/// let env = template_env::setup_template_env().expect("setup env");
/// ```
pub fn setup_template_env() -> Result<minijinja::Environment<'static>, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    env.add_filter("format_ymd", format_ymd);
    env.add_filter("pretty_price", pretty_price);
    env.add_template("base.html", BASE)?;
    Ok(env)
}

/// Render the Invoice using the provided [`minijinja`] environment and the
/// embedded base template.
///
/// # Arguments
/// * `env` - A reference to a configured [`minijinja::Environment`].
/// * `invoice` - The [`Invoice`] to render. Ownership is taken because templates
///   may borrow or clone data from the invoice as needed by [`minijinja`].
///
/// # Returns
/// * [`String`] containing the rendered HTML on success.
///
/// # Errors
/// * [`minijinja::Error`] if template retrieval or rendering fails.
///
/// # Example
/// ```rust
/// use invoice_pdf::{template_env, InvoiceBuilder, AddressBuilder, PartyBuilder};
///
/// let env = template_env::setup_template_env().unwrap();
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
/// let html = template_env::render_template(&env, inv).unwrap();
/// ```
pub fn render_template(
    env: &minijinja::Environment<'static>,
    invoice: Invoice,
) -> Result<String, minijinja::Error> {
    let template = env.get_template("base.html")?;
    template.render(context! {
        lines => invoice.line_items(),
        invoice => invoice
    })
}

// The embedded html template used to render the PDF content. This string is directly copied from
// templates/base.html
const BASE: &'static str = r#"<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8">
  <title>Invoice</title>
  <style>
    body {
      font-family: 'Helvetica Neue', Arial, sans-serif;
      font-size: 13px;
      color: #333;
      line-height: 1.5;
      margin: 0;
    }

    .page {
      width: 8.5in;
      height: 11in;
    }

    .header {
      display: flex;
      justify-content: space-between;
    }

    .header-left {
      width: 60%;
    }

    .logo {
      height: 2.3cm;
      display: block;
      margin-bottom: .2cm;
    }

    .address {
      line-height: 1.4;
      height: 3.5cm;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      border-bottom: 1px solid #ddd;
      margin-bottom: 1.5cm;
      top: 9.5cm;
    }

    th,
    td {
      border-top: 1px solid #ddd;
      padding: 6px 4px;
      text-align: left;
      vertical-align: top;
    }

    th {
      background: #f5f5f5;
      font-weight: 600;
    }

    .totals {
      width: 40%;
      float: right;
      margin-bottom: 0.5cm;
    }

    .totals td {
      padding: 4px 0;
    }

    .invoice-meta td {
      line-height: 1.3;
      vertical-align: top;
      padding-top: 0;
      border: none;
    }
  </style>
</head>

<body>
  <section class="page">
    <section class="header">
      <div class="header-left">
        <img class="logo" src="{{ invoice.logo }}" alt="Logo">
        <address class="address">
          <strong>{{ invoice.sender.name }}</strong><br>
          {% if invoice.sender.address %}
          {{ invoice.sender.address.line1 }}<br>
          {% if invoice.sender.address.line2 %}
          {{ invoice.sender.address.line2 }}<br>
          {% endif %}
          {{ invoice.sender.address.city }}, {{ invoice.sender.address.province_code }} {{
          invoice.sender.address.postal_code }}<br>
          {% endif %}
          {% if invoice.sender.phone %}
          {{ invoice.sender.phone }}<br>
          {% endif %}
          {% if invoice.sender.email %}
          {{ invoice.sender.email }}<br>
          {% endif %}
        </address>
        <address class="address">
          <strong>{{ invoice.receiver.name }}</strong><br>
          {% if invoice.receiver.address %}
          {{ invoice.receiver.address.line1 }}<br>
          {% if invoice.receiver.address.line2 %}
          {{ invoice.receiver.address.line2 }}<br>
          {% endif %}
          {{ invoice.receiver.address.city }}, {{ invoice.receiver.address.province_code }} {{
          invoice.receiver.address.postal_code }}<br>
          {% endif %}
          {% if invoice.receiver.phone %}
          {{ invoice.receiver.phone }}<br>
          {% endif %}
          {% if invoice.receiver.email %}
          {{ invoice.receiver.email }}<br>
          {% endif %}
        </address>
      </div>
      <div class="invoice-meta">
        <table style="border: none;">
          <tr>
            <td><strong>Invoice:</strong></td>
            <td>{{ invoice.id }}</td>
          </tr>
          <tr>
            <td><strong>Date:</strong></td>
            <td>{{ invoice.created_datetime | format_ymd }}</td>
          </tr>
          <tr>
            <td><strong>Due Date:</strong></td>
            <td>{{ invoice.net_due_datetime | format_ymd }}</td>
          </tr>
          {% if invoice.acct_id %}
          <tr>
            <td><strong>Account ID:</strong></td>
            <td>{{ invoice.acct_id }}</td>
          </tr>
          {% endif %}
          {% if invoice.purchase_order %}
          <tr>
            <td><strong>Purchase Order:</strong></td>
            <td>{{ invoice.purchase_order }}</td>
          </tr>
          {% endif %}
        </table>
      </div>
    </section>

    <table>
      <thead>
        <tr>
          <th style="width:10%">No.</th>
          <th>Description</th>
          <th style="width:10%; text-align: right;">Qty</th>
          <th style="width:15%; text-align: right;">Unit Price</th>
          <th style="width:15%; text-align: right;">Amount</th>
        </tr>
      </thead>
      <tbody>
        {% for line in lines %}
        <tr>
          <td>{{ line.sku }}</td>
          <td>{{ line.title }}</td>
          <td style="text-align: right;">{{ line.quantity }}</td>
          <td style="text-align: right;">{{ line.price | pretty_price }}</td>
          <td style="text-align: right;">{{ line.total | pretty_price }}</td>
        </tr>
        {% endfor %}
      </tbody>
    </table>
    <table class="totals">
      <tr>
        <td><strong>Total:</strong></td>
        <td style="text-align:right;">{{ invoice.total | pretty_price }}</td>
      </tr>
      <tr>
        <td><strong>Paid:</strong></td>
        <td style="text-align:right;">{{ invoice.paid | pretty_price }}</td>
      </tr>
      <tr>
        <td><strong>Due:</strong></td>
        <td style="text-align:right;">{{ invoice.due | pretty_price }}</td>
      </tr>
    </table>
  </section>
</body>

</html>
"#;
