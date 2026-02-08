use std::path::PathBuf;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, FixedOffset, Local};
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
    pub fn price(&self) -> &BigDecimal {
        &self.price
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn title(&self) -> String {
        self.title.to_string()
    }

    pub fn sku(&self) -> String {
        self.sku.to_string()
    }

    pub fn total(&self) -> &BigDecimal {
        &self.total
    }
}

impl Invoice {
    pub fn net_due(&self) -> BigDecimal {
        let line_item_total: BigDecimal =
            self.line_items.iter().map(|l| l.quantity * &l.price).sum();
        line_item_total - &self.paid
    }

    pub fn line_items(&self) -> &Vec<LineItem> {
        &self.line_items
    }
}

impl InvoiceBuilder {
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
