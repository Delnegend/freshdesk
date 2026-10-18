//! Quarterly-style Excel report for an arbitrary inclusive date range.
//!
//! Mirrors `QuarterlyReportData::collect` + `write_excel` from `src/report.rs`
//! (same column layout, same TTR+L3 overdue rule, same `FD_DEFAULT_IN_CHARGE_PRODUCTS`
//! checkboxes), but takes `--from` / `--to` dates instead of a `2026Qn` quarter.
//!
//! Example:
//! ```sh
//! cargo run --example range_report -- --from 2026-09-30 --to 2026-10-14
//! cargo run --example range_report -- --from 2026-07-30 --to 2026-10-14 --output range.xlsx
//! ```

use chrono::{NaiveDate, TimeDelta};
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use freshdesk::FreshdeskClient;
use freshdesk::models::Ticket;
use freshdesk::query::SearchTicketsQuery;
use freshdesk::report::{is_in_product_set, product_set_from_env};

#[derive(Debug, Clone)]
struct ProductRow {
    product: String,
    total_tickets: u64,
    total_overdue: u64,
    overdue_ticket_ids: Vec<u64>,
}

impl ProductRow {
    fn overdue_ids_string(&self) -> String {
        self.overdue_ticket_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let mut from: Option<NaiveDate> = None;
    let mut to: Option<NaiveDate> = None;
    let mut output: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => from = Some(parse_date(args.next())?),
            "--to" => to = Some(parse_date(args.next())?),
            "--output" => output = Some(args.next().ok_or("--output needs a value")?),
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    let (from, to) = match (from, to) {
        (Some(f), Some(t)) => (f, t),
        _ => return Err("--from and --to are both required (YYYY-MM-DD)".into()),
    };
    if to < from {
        return Err("--to must not be before --from".into());
    }

    // Same convention as Quarter::search_date_bounds: exclusive Lucene bounds
    // one day outside the inclusive range.
    let day_before = from
        .checked_sub_signed(TimeDelta::days(1))
        .ok_or("date underflow")?
        .format("%Y-%m-%d")
        .to_string();
    let day_after = to
        .checked_add_signed(TimeDelta::days(1))
        .ok_or("date overflow")?
        .format("%Y-%m-%d")
        .to_string();

    let range_code = format!("{}_to_{}", from.format("%Y-%m-%d"), to.format("%Y-%m-%d"));
    let output_path = output.unwrap_or_else(|| format!("{range_code}_report.xlsx"));

    println!("Collecting report data {from} .. {to} (inclusive)...");

    let client = FreshdeskClient::from_env().await?;

    // 1. Product catalog from the `_Components` ticket field choices.
    let mut components = client.get_components_choices().await?;
    if components.is_empty() {
        components = vec!["Uncategorized".to_string()];
    }
    println!("{} products in the component catalog", components.len());

    // 2. Overdue tickets: both TTR and L3 breaches, grouped by primary component.
    //    Same server-side query as the quarterly report (Lucene cannot compare
    //    two custom fields, so `cf__l3_violated` stands in for
    //    `_L3 Time Actual > _L3 Time Allowed`).
    let overdue_query = format!(
        "created_at:>'{day_before}' AND created_at:<'{day_after}' AND (cf_ttr_overdue:'Yes' AND cf__l3_violated:'Yes')"
    );
    let overdue: Vec<Ticket> = client
        .search_all_tickets_paginated(&SearchTicketsQuery::new(overdue_query), Some(10))
        .await?;
    println!("{} overdue tickets in range", overdue.len());

    let mut overdue_by_product: HashMap<String, Vec<u64>> = HashMap::new();
    for t in overdue {
        let comp = t
            .primary_component()
            .unwrap_or_else(|| "Uncategorized".to_string());
        overdue_by_product.entry(comp).or_default().push(t.id);
    }

    // 3. Total tickets per component (search `total` only; one request each).
    let mut total_by_product: HashMap<String, u64> = HashMap::new();
    for comp in &components {
        let q_str = format!(
            "created_at:>'{day_before}' AND created_at:<'{day_after}' AND cf__components:'{comp}'"
        );
        let res = client
            .search_tickets(&SearchTicketsQuery::new(q_str))
            .await?;
        total_by_product.insert(comp.clone(), res.total as u64);
    }
    println!("Fetched totals for {} products", total_by_product.len());

    // Components seen only on overdue tickets still deserve a row.
    for comp in overdue_by_product.keys() {
        if !components.contains(comp) {
            components.push(comp.clone());
        }
    }

    let mut products: Vec<ProductRow> = components
        .into_iter()
        .map(|comp| {
            let total = total_by_product.get(&comp).copied().unwrap_or(0);
            let ids = overdue_by_product.get(&comp).cloned().unwrap_or_default();
            ProductRow {
                product: comp,
                total_tickets: total.max(ids.len() as u64),
                total_overdue: ids.len() as u64,
                overdue_ticket_ids: ids,
            }
        })
        .collect();
    products.sort_by(|a, b| {
        b.total_tickets
            .cmp(&a.total_tickets)
            .then_with(|| a.product.cmp(&b.product))
    });

    write_excel(&products, &range_code, Path::new(&output_path))?;

    let grand_total: u64 = products.iter().map(|p| p.total_tickets).sum();
    let grand_overdue: u64 = products.iter().map(|p| p.total_overdue).sum();
    println!(
        "Report written: {output_path} ({} products)",
        products.len()
    );
    println!("Range totals: {grand_total} tickets, {grand_overdue} overdue");

    Ok(())
}

fn parse_date(value: Option<String>) -> Result<NaiveDate, Box<dyn std::error::Error>> {
    let raw = value.ok_or("missing date value")?;
    NaiveDate::parse_from_str(&raw, "%Y-%m-%d")
        .map_err(|e| format!("invalid date '{raw}': {e}").into())
}

/// Same layout as `QuarterlyReportData::write_excel`, with a range-based sheet name.
fn write_excel(
    products: &[ProductRow],
    range_code: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(format!("{range_code} Report"))?;

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x1F4E78))
        .set_font_color(Color::White)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let data_format = Format::new()
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let number_format = Format::new()
        .set_align(FormatAlign::Right)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let rate_label_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xD9E1F2))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Medium);
    let rate_value_format = Format::new()
        .set_bold()
        .set_font_size(12)
        .set_background_color(Color::RGB(0xFFF2CC))
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Medium)
        .set_num_format("0.00%");
    let percent_format = Format::new()
        .set_align(FormatAlign::Right)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_num_format("0.00%");

    worksheet.write_with_format(0, 0, "In Charge", &header_format)?;
    worksheet.write_with_format(0, 1, "Product", &header_format)?;
    worksheet.write_with_format(0, 2, "Total Tickets", &header_format)?;
    worksheet.write_with_format(0, 3, "Total Overdue Ticket", &header_format)?;
    worksheet.write_with_format(0, 4, "Overdue Rate", &header_format)?;
    worksheet.write_with_format(0, 5, "Overdue Tickets", &header_format)?;

    let in_charge: Option<HashSet<String>> = product_set_from_env("FD_DEFAULT_IN_CHARGE_PRODUCTS");
    let num_products = products.len();
    for (i, p) in products.iter().enumerate() {
        let row = (i + 1) as u32;
        let excel_row = row + 1;

        let checked = in_charge
            .as_ref()
            .is_some_and(|set| is_in_product_set(&p.product, set));
        worksheet.insert_checkbox(row, 0, checked)?;
        worksheet.write_with_format(row, 1, &p.product, &data_format)?;
        worksheet.write_with_format(row, 2, p.total_tickets, &number_format)?;
        worksheet.write_with_format(row, 3, p.total_overdue, &number_format)?;
        let row_rate_formula = format!("=IF(C{excel_row}>0, D{excel_row}/C{excel_row}, 0)");
        worksheet.write_formula_with_format(row, 4, row_rate_formula.as_str(), &percent_format)?;
        worksheet.write_with_format(row, 5, p.overdue_ids_string(), &data_format)?;
    }

    let last_row = if num_products > 0 {
        num_products as u32 + 1
    } else {
        2
    };

    worksheet.write_with_format(0, 7, "Overdue Rate", &rate_label_format)?;
    let formula = format!(
        "=IF(SUMIF(A2:A{last_row}, TRUE, C2:C{last_row})>0, SUMIF(A2:A{last_row}, TRUE, D2:D{last_row})/SUMIF(A2:A{last_row}, TRUE, C2:C{last_row}), 0)"
    );
    worksheet.write_formula_with_format(0, 8, formula.as_str(), &rate_value_format)?;

    let label_fmt = Format::new()
        .set_align(FormatAlign::Left)
        .set_border(FormatBorder::Thin);
    let val_fmt = Format::new()
        .set_align(FormatAlign::Right)
        .set_border(FormatBorder::Thin);

    worksheet.write_with_format(2, 7, "Checked Total Tickets", &label_fmt)?;
    worksheet.write_formula_with_format(
        2,
        8,
        format!("=SUMIF(A2:A{last_row}, TRUE, C2:C{last_row})").as_str(),
        &val_fmt,
    )?;

    worksheet.write_with_format(3, 7, "Checked Overdue Tickets", &label_fmt)?;
    worksheet.write_formula_with_format(
        3,
        8,
        format!("=SUMIF(A2:A{last_row}, TRUE, D2:D{last_row})").as_str(),
        &val_fmt,
    )?;

    worksheet.write_with_format(4, 7, "Checked Products Count", &label_fmt)?;
    worksheet.write_formula_with_format(
        4,
        8,
        format!("=COUNTIF(A2:A{last_row}, TRUE)").as_str(),
        &val_fmt,
    )?;

    worksheet.set_column_width(0, 12)?;
    worksheet.set_column_width(1, 24)?;
    worksheet.set_column_width(2, 16)?;
    worksheet.set_column_width(3, 22)?;
    worksheet.set_column_width(4, 16)?;
    worksheet.set_column_width(5, 45)?;
    worksheet.set_column_width(6, 4)?;
    worksheet.set_column_width(7, 24)?;
    worksheet.set_column_width(8, 16)?;

    workbook.save(path)?;
    Ok(())
}
