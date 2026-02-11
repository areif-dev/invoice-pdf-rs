//! Invoice domain types and serialization helpers.
//!
//! This module defines the structures used to represent invoices, parties,
//! addresses, and line items. It also provides custom serde serializers for
//! types that need to be represented as strings in JSON ([`BigDecimal`] and
//! [`DateTime`]). Builders are derived for constructing instances,
//! with some custom build logic for computing totals and due amounts.

use std::{path::PathBuf, str::FromStr};

use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset, Local};
use derive_builder::Builder;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn serialize_bigdecimal<S>(value: &BigDecimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn serialize_datetime<S>(value: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_rfc3339())
}

fn deserialize_bigdecimal<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    BigDecimal::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)
}

/// A single invoice line item encoding information such as stock keeping unit, title, quantity,
/// and unit price.
#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct LineItem {
    sku: String,
    title: String,
    quantity: i32,
    #[serde(
        serialize_with = "serialize_bigdecimal",
        deserialize_with = "deserialize_bigdecimal"
    )]
    price: BigDecimal,
}

/// A party involved in the invoice (sender or receiver)
#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct Party {
    name: String,
    #[builder(default)]
    phone: Option<String>,
    #[builder(default)]
    email: Option<String>,
    #[builder(default)]
    address: Option<Address>,
}

/// A postal address
#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct Address {
    line1: String,
    #[builder(default)]
    line2: Option<String>,
    city: String,
    province_code: String,
    postal_code: String,
}

/// Invoice top level model
#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct Invoice {
    id: String,
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    #[builder(default = Local::now().into())]
    created_datetime: DateTime<FixedOffset>,
    #[serde(
        serialize_with = "serialize_datetime",
        deserialize_with = "deserialize_datetime"
    )]
    #[builder(default = Local::now().into())]
    net_due_datetime: DateTime<FixedOffset>,
    receiver: Party,
    sender: Party,
    #[builder(default)]
    logo: Option<PathBuf>,
    #[builder(default = Vec::new())]
    line_items: Vec<LineItem>,
    #[serde(
        serialize_with = "serialize_bigdecimal",
        deserialize_with = "deserialize_bigdecimal"
    )]
    #[builder(default = BigDecimal::from(0))]
    paid: BigDecimal,
    #[builder(default)]
    acct_id: Option<String>,
    #[builder(default)]
    purchase_order: Option<String>,
}

impl LineItem {
    /// Return the unit price for this line item.
    pub fn price(&self) -> &BigDecimal {
        &self.price
    }

    /// Return the quantity for this line item.
    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    /// Return the title for this line item.
    pub fn title(&self) -> String {
        self.title.to_string()
    }

    /// Return the stock keeping unit for this line item.
    pub fn sku(&self) -> String {
        self.sku.to_string()
    }

    /// Return the computed total for this line item equal to `quantity * price`
    pub fn total(&self) -> BigDecimal {
        &self.price * self.quantity
    }
}

impl Invoice {
    /// Compute net amount due as `sum(line_items) - paid`.
    ///
    /// # Returns
    /// A [`BigDecimal`] representing the remaining amount owed.
    ///
    /// # Example
    /// ```rust
    /// use bigdecimal::BigDecimal;
    /// use invoice_pdf::{InvoiceBuilder, PartyBuilder, AddressBuilder};
    ///
    /// let inv = InvoiceBuilder::default()
    ///     .id("1")
    ///     .logo("./logo.png")
    ///     .receiver(
    ///         PartyBuilder::default()
    ///             .name("A")
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
    /// assert_eq!(inv.net_due(), BigDecimal::from(0));
    /// ```
    pub fn net_due(&self) -> BigDecimal {
        let line_item_total: BigDecimal =
            self.line_items.iter().map(|l| l.quantity * &l.price).sum();
        line_item_total - &self.paid
    }

    /// Compute the invoice total as `sum(line_items)`
    ///
    /// # Returns
    /// A [`BigDecimal`] representing the total value of the invoice without taking any payments
    /// into account
    ///
    /// # Example
    /// ```rust
    /// use std::str::FromStr;
    ///
    /// use bigdecimal::BigDecimal;
    /// use invoice_pdf::{InvoiceBuilder, PartyBuilder, AddressBuilder, LineItemBuilder};
    ///
    /// let inv = InvoiceBuilder::default()
    ///     .id("1")
    ///     .receiver(
    ///         PartyBuilder::default()
    ///             .name("A")
    ///             .build().unwrap())
    ///     .sender(
    ///         PartyBuilder::default()
    ///             .name("B")
    ///             .build().unwrap())
    ///     .add_line(
    ///         LineItemBuilder::default()
    ///             .sku("test")
    ///             .title("test")
    ///             .quantity(1)
    ///             .price(BigDecimal::from_str("10.99").unwrap())
    ///             .build().unwrap()
    ///     )
    ///     .paid(BigDecimal::from(5))
    ///     .build().unwrap();
    /// assert_eq!(inv.total(), BigDecimal::from_str("10.99").unwrap());
    ///
    /// ```
    pub fn total(&self) -> BigDecimal {
        self.line_items.iter().map(|l| l.quantity * &l.price).sum()
    }

    /// Return a reference to the invoice's line items.
    pub fn line_items(&self) -> &Vec<LineItem> {
        &self.line_items
    }
}

impl InvoiceBuilder {
    /// Add a [`LineItem`] to the builder's internal list.
    ///
    /// # Arguments
    /// * `line` - The [`LineItem`] to append.
    ///
    /// # Returns
    /// The updated [`InvoiceBuilder`].
    ///
    /// # Example
    /// ```rust
    /// use invoice_pdf::{InvoiceBuilder, LineItemBuilder};
    /// use bigdecimal::BigDecimal;
    /// use std::str::FromStr;
    ///
    /// let line_item = LineItemBuilder::default()
    ///     .sku("TEST")
    ///     .title("This is a test")
    ///     .quantity(1)
    ///     .price(BigDecimal::from_str("12.99").unwrap())
    ///     .build().unwrap();
    /// let builder = InvoiceBuilder::default().add_line(line_item);
    /// ```
    pub fn add_line(self, line: LineItem) -> Self {
        match self.line_items {
            Some(mut l) => {
                l.push(line);
                Self {
                    line_items: Some(l),
                    ..self
                }
            }
            None => Self {
                line_items: Some(vec![line]),
                ..self
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, FixedOffset, TimeZone};
    use std::path::PathBuf;
    use std::str::FromStr;

    // Helper to create a minimal valid Address using the builder
    fn make_address() -> Address {
        AddressBuilder::default()
            .line1("1 street st")
            .city("City")
            .province_code("PR")
            .postal_code("POST")
            .build()
            .unwrap()
    }

    // Helper to create a minimal valid Party using the builder
    fn make_party(name: &str) -> Party {
        PartyBuilder::default().name(name).build().unwrap()
    }

    #[test]
    fn test_deserialize_bigdecimal() {
        #[derive(Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "super::deserialize_bigdecimal")]
            bd: BigDecimal,
        }

        let val = serde_json::json!({"bd": "12.50"});
        let _: Wrap = serde_json::from_value(val).unwrap();
        let val = serde_json::json!({"bd": "reee"});
        let x = serde_json::from_value::<Wrap>(val);
        assert!(x.is_err())
    }

    #[test]
    fn test_deserialize_datetime() {
        #[derive(Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "super::deserialize_datetime")]
            date: DateTime<FixedOffset>,
        }

        let val = serde_json::json!({"date": "2026-02-10T12:00:00+00:00"});
        let _: Wrap = serde_json::from_value(val).unwrap();
        let val = serde_json::json!({"bd": "reee"});
        let x = serde_json::from_value::<Wrap>(val);
        assert!(x.is_err())
    }

    #[test]
    fn test_serialize_bigdecimal() {
        #[derive(Serialize)]
        struct Wrap<'a> {
            #[serde(serialize_with = "super::serialize_bigdecimal")]
            v: &'a bigdecimal::BigDecimal,
        }

        let bd = bigdecimal::BigDecimal::from_str("12.34").unwrap();
        let j = serde_json::to_value(Wrap { v: &bd }).unwrap();
        assert_eq!(j.get("v").and_then(|v| v.as_str()), Some("12.34"));
    }

    #[test]
    fn test_serialize_datetime() {
        #[derive(Serialize)]
        struct Wrap {
            #[serde(serialize_with = "super::serialize_datetime")]
            dt: chrono::DateTime<chrono::FixedOffset>,
        }

        let dt = chrono::Utc
            .with_ymd_and_hms(2026, 02, 09, 12, 00, 00)
            .unwrap()
            .into();
        let j = serde_json::to_value(Wrap { dt }).unwrap();
        let s = j.get("dt").and_then(|v| v.as_str()).unwrap();
        assert_eq!("2026-02-09T12:00:00+00:00", s);
    }

    #[test]
    fn line_item_builder_success_and_accessors() {
        let price = BigDecimal::from_str("9.50").unwrap();
        let item = LineItemBuilder::default()
            .sku("ABC123")
            .title("Gadget")
            .quantity(2)
            .price(price.clone())
            .build()
            .unwrap();

        // Check accessors
        assert_eq!(item.quantity(), 2);
        assert_eq!(&item.title(), "Gadget");
        assert_eq!(&item.sku(), "ABC123");
        assert_eq!(item.price(), &price);
        assert_eq!(item.total(), BigDecimal::from(19));
    }

    #[test]
    fn line_item_builder_missing_required_fields_fails() {
        // Missing price
        let _ = LineItemBuilder::default()
            .sku("X")
            .title("Y")
            .quantity(1)
            .build()
            .unwrap_err();

        // Missing quantity
        let _ = LineItemBuilder::default()
            .sku("X")
            .title("Y")
            .price(BigDecimal::from(1))
            .build()
            .unwrap_err();

        // Missing sku
        let _ = LineItemBuilder::default()
            .title("Y")
            .quantity(1)
            .price(BigDecimal::from(1))
            .build()
            .unwrap_err();

        // Missing title
        let _ = LineItemBuilder::default()
            .sku("X")
            .quantity(1)
            .price(BigDecimal::from(1))
            .build()
            .unwrap_err();
    }

    #[test]
    fn minimal_party_builder_success() {
        // Party with optional phone/email omitted
        let party = PartyBuilder::default().name("Alice").build().unwrap();
        assert_eq!(party.name, "Alice".to_string());
        assert!(party.phone.is_none());
        assert!(party.email.is_none());
        assert!(party.address.is_none());
    }

    #[test]
    fn party_builder_missing_required_name_fails() {
        let _ = PartyBuilder::default().build().unwrap_err();
    }

    #[test]
    fn address_builder_missing_required_fields_fails() {
        // Missing line1
        let _ = AddressBuilder::default()
            .city("C")
            .province_code("P")
            .postal_code("Z")
            .build()
            .unwrap_err();

        // Missing city
        let _ = AddressBuilder::default()
            .line1("L")
            .province_code("P")
            .postal_code("Z")
            .build()
            .unwrap_err();

        // Missing province_code
        let _ = AddressBuilder::default()
            .line1("L")
            .city("C")
            .postal_code("Z")
            .build()
            .unwrap_err();

        // Missing postal_code
        let _ = AddressBuilder::default()
            .line1("L")
            .city("C")
            .province_code("P")
            .build()
            .unwrap_err();
    }

    #[test]
    fn invoice_builder_success_and_computations() {
        // create two line items
        let item1 = LineItemBuilder::default()
            .sku("A")
            .title("Item A")
            .quantity(1)
            .price(BigDecimal::from_str("10.00").unwrap())
            .build()
            .unwrap();

        let item2 = LineItemBuilder::default()
            .sku("B")
            .title("Item B")
            .quantity(3)
            .price(BigDecimal::from_str("2.50").unwrap())
            .build()
            .unwrap();

        // create invoice with some paid amount and logo path
        let paid = BigDecimal::from_str("5.00").unwrap();
        let logo = PathBuf::from("./logo.png");

        let inv = InvoiceBuilder::default()
            .id("inv-1")
            // intentionally omit created_datetime and net_due_datetime to use defaults
            .receiver(make_party("Receiver"))
            .sender(make_party("Sender"))
            .logo(logo.clone())
            .add_line(item1)
            .add_line(item2)
            .paid(paid.clone())
            .build()
            .unwrap();

        // total = 1*10.00 + 3*2.50 = 10.00 + 7.50 = 17.50
        let expected_total = BigDecimal::from_str("17.50").unwrap();
        assert_eq!(inv.total(), expected_total);

        // due = total - paid = 12.50
        let expected_due = &expected_total - &paid;
        assert_eq!(inv.net_due(), expected_due);

        // net_due() should compute same value
        assert_eq!(inv.net_due(), expected_due);

        // created_datetime and net_due_datetime should be present and set to today in the local
        // timezone by default
        let expected_date = chrono::Local::now();
        let created = inv.created_datetime;
        let due = inv.net_due_datetime;
        assert_eq!(expected_date.date_naive(), created.date_naive());
        assert_eq!(expected_date.date_naive(), due.date_naive());

        // line_items accessor
        assert_eq!(inv.line_items().len(), 2);
    }

    #[test]
    fn invoice_builder_missing_required_fields_fails() {
        // missing id
        let _ = InvoiceBuilder::default()
            .receiver(make_party("R"))
            .sender(make_party("S"))
            .logo(PathBuf::from("./logo.png"))
            .build()
            .unwrap_err();

        // missing receiver
        let _ = InvoiceBuilder::default()
            .id("1")
            .sender(make_party("S"))
            .logo(PathBuf::from("./logo.png"))
            .build()
            .unwrap_err();

        // missing sender
        let _ = InvoiceBuilder::default()
            .id("1")
            .receiver(make_party("R"))
            .logo(PathBuf::from("./logo.png"))
            .build()
            .unwrap_err();
    }
}
