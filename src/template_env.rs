use std::str::FromStr;

use bigdecimal::BigDecimal;
use minijinja::context;

use crate::invoice::Invoice;

pub fn format_ymd(raw: &str) -> String {
    let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(raw) else {
        return String::from("N/A");
    };
    datetime.format("%Y-%m-%d").to_string()
}

pub fn pretty_price(raw: &str) -> String {
    let Ok(dec) = BigDecimal::from_str(&raw) else {
        return String::from("NaN");
    };
    format!("${:.2}", dec)
}

pub fn setup_template_env() -> Result<minijinja::Environment<'static>, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    env.add_filter("format_ymd", format_ymd);
    env.add_filter("pretty_price", pretty_price);
    env.set_loader(minijinja::path_loader("templates"));
    Ok(env)
}

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
