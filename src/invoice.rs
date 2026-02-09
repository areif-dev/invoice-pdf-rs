//! Invoice domain types and serialization helpers.
//!
//! This module defines the structures used to represent invoices, parties,
//! addresses, and line items. It also provides custom serde serializers for
//! types that need to be represented as strings in JSON ([`BigDecimal`] and
//! [`DateTime`]). Builders are derived for constructing instances,
//! with some custom build logic for computing totals and due amounts.

use std::path::PathBuf;

use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset, Local};
use derive_builder::Builder;
use serde::{Serialize, Serializer};

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

/// A single invoice line item encoding information such as stock keeping unit, title, quantity,
/// and unit price.
#[derive(Debug, Builder, Serialize)]
#[builder(setter(strip_option, into), build_fn(skip), pattern = "owned")]
pub struct LineItem {
    sku: String,
    title: String,
    quantity: i32,
    #[serde(serialize_with = "serialize_bigdecimal")]
    price: BigDecimal,
    #[builder(setter(skip))]
    #[serde(serialize_with = "serialize_bigdecimal")]
    total: BigDecimal,
}

/// A party involved in the invoice (sender or receiver)
#[derive(Debug, Builder, Serialize)]
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct Party {
    name: String,
    #[builder(default)]
    phone: Option<String>,
    #[builder(default)]
    email: Option<String>,
    address: Option<Address>,
}

/// A postal address
#[derive(Debug, Builder, Serialize)]
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
#[derive(Debug, Builder, Serialize)]
#[builder(setter(strip_option, into), pattern = "owned", build_fn(skip))]
pub struct Invoice {
    id: String,
    #[serde(serialize_with = "serialize_datetime")]
    created_datetime: DateTime<FixedOffset>,
    #[serde(serialize_with = "serialize_datetime")]
    net_due_datetime: DateTime<FixedOffset>,
    receiver: Party,
    sender: Party,
    logo: Option<PathBuf>,
    line_items: Vec<LineItem>,
    #[serde(serialize_with = "serialize_bigdecimal")]
    paid: BigDecimal,
    #[serde(serialize_with = "serialize_bigdecimal")]
    #[builder(setter(skip))]
    total: BigDecimal,
    #[serde(serialize_with = "serialize_bigdecimal")]
    #[builder(setter(skip))]
    due: BigDecimal,
    acct_id: Option<String>,
    purchase_order: Option<String>,
}

impl LineItemBuilder {
    /// Validate builder fields and compute the `total` for the line item.
    ///
    /// # Returns
    /// [`LineItem`] on success with `total = quantity * price`.
    ///
    /// # Errors
    /// [`LineItemBuilderError::UninitializedField`] if required fields are missing
    ///
    /// # Example
    /// ```rust
    /// use bigdecimal::BigDecimal;
    /// use invoice_pdf::LineItemBuilder;
    ///
    /// let item = LineItemBuilder::default()
    ///     .sku("ABC123")
    ///     .title("Gadget")
    ///     .quantity(2)
    ///     .price(BigDecimal::from(9))
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(item.quantity(), 2);
    /// ```
    pub fn build(self) -> Result<LineItem, LineItemBuilderError> {
        let quantity = self
            .quantity
            .ok_or(LineItemBuilderError::UninitializedField("quantity"))?;
        let price = self
            .price
            .ok_or(LineItemBuilderError::UninitializedField("price"))?;
        let sku = self
            .sku
            .ok_or(LineItemBuilderError::UninitializedField("sku"))?;
        let title = self
            .title
            .ok_or(LineItemBuilderError::UninitializedField("title"))?;

        Ok(LineItem {
            total: quantity * &price,
            sku,
            title,
            quantity,
            price,
        })
    }
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
    pub fn total(&self) -> &BigDecimal {
        &self.total
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
    /// assert_eq!(inv.net_due(), BigDecimal::from(0));
    /// ```
    pub fn net_due(&self) -> BigDecimal {
        let line_item_total: BigDecimal =
            self.line_items.iter().map(|l| l.quantity * &l.price).sum();
        line_item_total - &self.paid
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

    /// Finalize the builder into an [`Invoice`], computing `total` and `due`.
    ///
    /// Missing optional fields are filled with reasonable defaults:
    /// * `created_datetime` and `net_due_datetime` default to [`Local::now`].
    /// * `line_items` defaults to empty vector.
    /// * `paid` defaults to zero.
    ///
    /// # Returns
    /// [`Invoice`] on success.
    ///
    /// # Errors
    /// [`InvoiceBuilderError::UninitializedField`] if required fields are missing.
    ///
    /// # Example
    /// ```rust
    /// use invoice_pdf::{InvoiceBuilder, PartyBuilder, AddressBuilder};
    ///
    /// // A simple invoice with only required fields filled
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
    /// ```
    pub fn build(self) -> Result<Invoice, InvoiceBuilderError> {
        let id = self
            .id
            .ok_or(InvoiceBuilderError::UninitializedField("id"))?;
        let created_datetime = self.created_datetime.unwrap_or(Local::now().into());
        let net_due_datetime = self.net_due_datetime.unwrap_or(Local::now().into());
        let receiver = self
            .receiver
            .ok_or(InvoiceBuilderError::UninitializedField("receiver"))?;
        let sender = self
            .sender
            .ok_or(InvoiceBuilderError::UninitializedField("sender"))?;
        let logo = self
            .logo
            .ok_or(InvoiceBuilderError::UninitializedField("logo"))?;
        let line_items = self.line_items.unwrap_or(Vec::new());
        let paid = self.paid.unwrap_or(BigDecimal::from(0));
        let acct_id = self.acct_id.unwrap_or(None);
        let purchase_order = self.purchase_order.unwrap_or(None);

        let total: BigDecimal = line_items.iter().map(LineItem::total).sum();
        let due = &total - &paid;
        Ok(Invoice {
            id,
            created_datetime,
            net_due_datetime,
            receiver,
            sender,
            logo,
            line_items,
            paid,
            due,
            total,
            acct_id,
            purchase_order,
        })
    }
}
