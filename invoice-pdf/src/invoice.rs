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
use gtin::Gtin;
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

fn deserialize_scale3<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let d = BigDecimal::from_str(&s).map_err(serde::de::Error::custom)?;
    Ok(scale3_from_bigdecimal(&d))
}

fn deserialize_scale2<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let d = BigDecimal::from_str(&s).map_err(serde::de::Error::custom)?;
    Ok(scale2_from_bigdecimal(&d))
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)
}

fn scale3_from_bigdecimal(bd: &BigDecimal) -> BigDecimal {
    bd.with_scale_round(3, bigdecimal::RoundingMode::Up)
}

fn scale2_from_bigdecimal(bd: &BigDecimal) -> BigDecimal {
    bd.with_scale_round(2, bigdecimal::RoundingMode::HalfEven)
}

/// A single invoice line item encoding information such as stock keeping unit, title, quantity,
/// and unit price.
#[derive(Debug, Builder, Serialize, Deserialize)]
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct LineItem {
    sku: String,
    title: String,
    quantity: i32,
    #[builder(setter(into), default)]
    gtin: Option<Gtin>,
    #[serde(
        serialize_with = "serialize_bigdecimal",
        deserialize_with = "deserialize_scale3"
    )]
    #[builder(setter(custom))]
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
        deserialize_with = "deserialize_scale2"
    )]
    #[builder(default = BigDecimal::from(0), setter(custom))]
    paid: BigDecimal,
    #[builder(default)]
    acct_id: Option<String>,
    #[builder(default)]
    purchase_order: Option<String>,
}

impl LineItemBuilder {
    pub fn price(self, p: impl Into<BigDecimal>) -> Self {
        let p: BigDecimal = p.into();
        let price = scale3_from_bigdecimal(&p);
        Self {
            price: Some(price),
            ..self
        }
    }
}

impl LineItem {
    /// Return the unit price for this line item.
    pub fn price(&self) -> BigDecimal {
        self.price.clone()
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
        (&self.price * self.quantity).with_scale_round(2, bigdecimal::RoundingMode::HalfEven)
    }

    /// Return this line item's barcode/upc/gtin, if it exists
    pub fn gtin(&self) -> Option<Gtin> {
        self.gtin
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
        let line_item_total: BigDecimal = self.line_items.iter().map(LineItem::total).sum();
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
        self.line_items.iter().map(LineItem::total).sum()
    }

    /// Return a copy of this [`Invoice`]'s id
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    /// Return a reference to the invoice's line items.
    pub fn line_items(&self) -> &Vec<LineItem> {
        &self.line_items
    }

    /// Get the date and time when this invoice was created
    pub fn created_datetime(&self) -> &DateTime<FixedOffset> {
        &self.created_datetime
    }

    /// Get the date and time when full payment is due
    pub fn net_due_datetime(&self) -> &DateTime<FixedOffset> {
        &self.net_due_datetime
    }

    /// Get the information for the receiver of the invoice
    pub fn receiver(&self) -> &Party {
        &self.receiver
    }

    /// Get the information for the sender of the invoice
    pub fn sender(&self) -> &Party {
        &self.sender
    }

    /// Get the path to the logo, if one exists
    pub fn logo(&self) -> &Option<PathBuf> {
        &self.logo
    }

    /// Get the amount paid on the invoice as a [`BigDecimal`]
    pub fn paid(&self) -> BigDecimal {
        self.paid.clone()
    }

    /// Get the receiver's account id, if one exists
    pub fn acct_id(&self) -> &Option<String> {
        &self.acct_id
    }

    /// Get the receiver's purchase order, if one exists
    pub fn purchase_order(&self) -> &Option<String> {
        &self.purchase_order
    }
}

impl Party {
    /// Get this [`Party`]'s name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the phone number, if one exists
    pub fn phone(&self) -> &Option<String> {
        &self.phone
    }

    /// Get the email address, if one exists
    pub fn email(&self) -> &Option<String> {
        &self.email
    }

    /// Get the address, if one exists
    pub fn address(&self) -> &Option<Address> {
        &self.address
    }
}

impl Address {
    /// Get the primary line of the address. This will likely be a house number and street name. Eg
    /// 1600 Pennsylvania Ave
    pub fn line1(&self) -> &str {
        &self.line1
    }

    /// Get the optional secondary address line. This is usually an apartment or po box
    pub fn line2(&self) -> &Option<String> {
        &self.line2
    }

    /// Get the name of the city where the [`Party`] lives
    pub fn city(&self) -> &str {
        &self.city
    }

    /// Get the province or state code. Eg Pennsylvania should be "PA"
    pub fn province_code(&self) -> &str {
        &self.province_code
    }

    /// Get the postal or zip code
    pub fn postal_code(&self) -> &str {
        &self.postal_code
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

    pub fn paid(self, p: impl Into<BigDecimal>) -> Self {
        let p: BigDecimal = p.into();
        let price = scale2_from_bigdecimal(&p);
        Self {
            paid: Some(price),
            ..self
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

    fn make_line_item(qty: i32, price: &str) -> LineItem {
        LineItemBuilder::default()
            .sku("test")
            .title("test")
            .quantity(qty)
            .price(price.parse::<BigDecimal>().unwrap())
            .build()
            .unwrap()
    }

    // Helper to create a minimal valid Party using the builder
    fn make_party(name: &str) -> Party {
        PartyBuilder::default().name(name).build().unwrap()
    }

    #[test]
    fn test_deserialize_price() {
        #[derive(Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "super::deserialize_scale3")]
            bd: BigDecimal,
        }

        let val = serde_json::json!({"bd": "12.121"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "12.121");

        let val = serde_json::json!({"bd": "12.129"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "12.129");

        let val = serde_json::json!({"bd": "12.1299"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "12.130");

        let val = serde_json::json!({"bd": "12.1291"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "12.130");

        let val = serde_json::json!({"bd": "12.1295"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "12.130");
    }

    #[test]
    fn test_deserialize_scale2() {
        #[derive(Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "super::deserialize_scale2")]
            bd: BigDecimal,
        }

        let val = serde_json::json!({"bd": "12.50"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "12.50");
        let val = serde_json::json!({"bd": "reee"});
        let x = serde_json::from_value::<Wrap>(val);
        assert!(x.is_err());

        let val = serde_json::json!({"bd": "1.995"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "2.00");

        let val = serde_json::json!({"bd": "-1.995"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "-2.00");

        let val = serde_json::json!({"bd": "5.001"});
        let w: Wrap = serde_json::from_value(val).unwrap();
        assert_eq!(&w.bd.to_string(), "5.00");
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
            .gtin(Gtin::new("082657543338").unwrap())
            .title("Gadget")
            .quantity(2)
            .price(price.clone())
            .build()
            .unwrap();

        // Check accessors
        assert_eq!(item.quantity(), 2);
        assert_eq!(&item.title(), "Gadget");
        assert_eq!(&item.sku(), "ABC123");
        assert_eq!(item.price(), price);
        assert_eq!(&item.gtin().unwrap().to_string(), "00082657543338");
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

        // Missing UPC, which is not required, so it should work
        let _ = InvoiceBuilder::default()
            .id("1")
            .receiver(make_party("R"))
            .sender(make_party("S"))
            .logo(PathBuf::from("./logo.png"))
            .build()
            .unwrap();
    }

    #[test]
    fn test_invoice_math() {
        let sender = make_party("sender");
        let receiver = make_party("receiver");
        let line_items = vec![
            make_line_item(1, "9.123"),
            make_line_item(168, "9.1231"),
            make_line_item(22, "10"),
            make_line_item(3, "10.001"),
            make_line_item(3, "-18.4441"),
        ];
        let expected = vec![
            ("9.123", "9.12"),
            ("9.124", "1532.83"),
            ("10.000", "220.00"),
            ("10.001", "30.00"),
            ("-18.445", "-55.34"),
        ];
        let expected: Vec<_> = expected
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        let actual: Vec<_> = line_items
            .iter()
            .map(|l| (l.price().to_string(), l.total().to_string()))
            .collect();
        assert_eq!(expected, actual);
        let invoice = InvoiceBuilder::default()
            .line_items(line_items)
            .sender(sender)
            .receiver(receiver)
            .id("1")
            .paid("16.999".parse::<BigDecimal>().unwrap())
            .build()
            .unwrap();
        assert_eq!(&invoice.paid().to_string(), "17.00");
        assert_eq!(&invoice.total().to_string(), "1736.61");
        assert_eq!(&invoice.net_due().to_string(), "1719.61");
    }
}
