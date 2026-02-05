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
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct LineItem {
    sku: String,
    title: String,
    quantity: i32,

    #[serde(serialize_with = "serialize_bigdecimal")]
    price: BigDecimal,
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
#[builder(setter(strip_option, into), pattern = "owned")]
pub struct Invoice {
    id: String,
    #[serde(serialize_with = "serialize_datetime")]
    #[builder(default = Local::now().into())]
    created_datetime: DateTime<FixedOffset>,
    #[serde(serialize_with = "serialize_datetime")]
    #[builder(default = (Local::now() + Duration::days(30)).into())]
    net_due_datetime: DateTime<FixedOffset>,
    receiver: Party,
    sender: Party,
    logo: Option<PathBuf>,
    #[builder(default = Vec::new())]
    line_items: Vec<LineItem>,
    #[serde(serialize_with = "serialize_bigdecimal")]
    #[builder(default = BigDecimal::from(0))]
    paid: BigDecimal,
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
}
