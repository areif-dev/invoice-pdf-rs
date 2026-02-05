use minijinja::context;

use crate::invoice::Invoice;

pub fn setup_template_env() -> Result<minijinja::Environment<'static>, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    Ok(env)
}

pub fn render_template(
    env: &minijinja::Environment<'static>,
    invoice: Invoice,
) -> Result<String, minijinja::Error> {
    let template = env.get_template("base.html")?;
    let pages: Vec<_> = invoice.line_items().chunks(21).collect();
    template.render(context! {
        pages => pages,
        invoice => invoice
    })
}
