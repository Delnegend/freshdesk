use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::Path;
use tracing::info;

use crate::client::FreshdeskClient;
use crate::error::{FreshdeskError, Result};

/// Represents a calendar quarter (e.g. 2026Q3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quarter {
    pub year: i32,
    pub quarter: u8,
}

impl Quarter {
    /// Parse string like "2026Q3", "2026-Q3", "2026q3".
    pub fn parse(s: &str) -> Result<Self> {
        let trimmed = s.trim().to_uppercase();
        let cleaned = trimmed.replace('-', "");

        let parts: Vec<&str> = cleaned.split('Q').collect();
        if parts.len() != 2 {
            return Err(FreshdeskError::Configuration(format!(
                "Invalid quarter format: '{}'. Expected format like '2026Q3'",
                s
            )));
        }

        let year: i32 = parts[0].parse().map_err(|_| {
            FreshdeskError::Configuration(format!("Invalid year in quarter '{}'", s))
        })?;

        let quarter: u8 = parts[1].parse().map_err(|_| {
            FreshdeskError::Configuration(format!("Invalid quarter number in '{}'", s))
        })?;

        if !(1..=4).contains(&quarter) {
            return Err(FreshdeskError::Configuration(format!(
                "Quarter must be between 1 and 4, got {}",
                quarter
            )));
        }

        Ok(Self { year, quarter })
    }

    /// Date bounds for Freshdesk API query.
    /// Returns (start_day_before, end_day_after) for:
    /// `created_at:>'<start_day_before>' AND created_at:<'<end_day_after>'`.
    pub fn search_date_bounds(&self) -> (&'static str, &'static str, String, String) {
        match self.quarter {
            1 => (
                "01-01",
                "03-31",
                format!("{}-12-31", self.year - 1),
                format!("{}-04-01", self.year),
            ),
            2 => (
                "04-01",
                "06-30",
                format!("{}-03-31", self.year),
                format!("{}-07-01", self.year),
            ),
            3 => (
                "07-01",
                "09-30",
                format!("{}-06-30", self.year),
                format!("{}-10-01", self.year),
            ),
            4 => (
                "10-01",
                "12-31",
                format!("{}-09-30", self.year),
                format!("{}-01-01", self.year + 1),
            ),
            _ => unreachable!(),
        }
    }

    pub fn to_string_code(&self) -> String {
        format!("{}Q{}", self.year, self.quarter)
    }
}

/// Parses a comma-separated product list from an environment variable into a
/// lowercased set for case-insensitive lookup. Returns `None` when the variable
/// is unset, empty, or contains only separators.
pub fn product_set_from_env(key: &str) -> Option<HashSet<String>> {
    let raw = std::env::var(key).ok()?;
    let set: HashSet<String> = raw
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

/// Returns true if the product is in the default "In Charge" set.
///
/// The set is supplied via the `FD_DEFAULT_IN_CHARGE_PRODUCTS` environment
/// variable as comma-separated product names. No products are pre-configured
/// in the code, so this repository ships tenant-agnostic defaults.
pub fn is_default_in_charge(product: &str) -> bool {
    match product_set_from_env("FD_DEFAULT_IN_CHARGE_PRODUCTS") {
        Some(set) => set.contains(&product.trim().to_lowercase()),
        None => false,
    }
}

/// Statistics for a single product in a quarter.
#[derive(Debug, Clone, Default)]
pub struct ProductQuarterReport {
    pub product: String,
    pub total_tickets: u64,
    pub total_overdue: u64,
    pub overdue_ticket_ids: Vec<u64>,
}

impl ProductQuarterReport {
    pub fn overdue_ids_string(&self) -> String {
        self.overdue_ticket_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Aggregated report data for a quarter.
#[derive(Debug, Clone)]
pub struct QuarterlyReportData {
    pub quarter: Quarter,
    pub products: Vec<ProductQuarterReport>,
}

impl QuarterlyReportData {
    /// Collects report data from Freshdesk for the given quarter.
    pub async fn collect(client: &FreshdeskClient, quarter: Quarter) -> Result<Self> {
        let (_start_label, _end_label, start_day_before, end_day_after) =
            quarter.search_date_bounds();

        info!(
            "Collecting quarterly report data for {} ({} to {})...",
            quarter.to_string_code(),
            start_day_before,
            end_day_after
        );

        // 1. Get all configured product components
        let mut components = client.get_components_choices().await?;
        if components.is_empty() {
            components = vec!["Uncategorized".to_string()];
        }

        // 2. Fetch all overdue tickets in this quarter to group by component and collect IDs
        info!(
            "Fetching overdue tickets for quarter {}...",
            quarter.to_string_code()
        );
        let mut overdue_by_product: HashMap<String, Vec<u64>> = HashMap::new();

        // A ticket counts as overdue only when it breached *both* its TTR SLA
        // and its L3 escalation SLA. Lucene cannot compare two fields
        // (`cf__l3_time_actual:>cf__l3_time_allowed` is rejected with 400), so
        // the query matches the upstream `cf__l3_violated` automation field,
        // which encodes exactly `_L3 Time Actual > _L3 Time Allowed`.
        // `tests/live_tests.rs` guards that equivalence.
        for page in 1..=10 {
            let overdue_query = format!(
                "created_at:>'{}' AND created_at:<'{}' \
                 AND (cf_ttr_overdue:'Yes' AND cf__l3_violated:'Yes')",
                start_day_before, end_day_after
            );

            let search_query = crate::query::SearchTicketsQuery::new(overdue_query).page(page);
            let search_res = client.search_tickets(&search_query).await?;

            if search_res.results.is_empty() {
                break;
            }

            let total = search_res.total;
            for t in search_res.results {
                let comp = t
                    .primary_component()
                    .unwrap_or_else(|| "Uncategorized".to_string());
                overdue_by_product.entry(comp).or_default().push(t.id);
            }

            if (page * 30) as usize >= total {
                break;
            }
        }

        // 3. Fetch total ticket counts for each component concurrently (batch size 4)
        info!(
            "Fetching total ticket counts for {} products...",
            components.len()
        );
        let mut total_by_product: HashMap<String, u64> = HashMap::new();

        let batch_size = 4;
        for chunk in components.chunks(batch_size) {
            let mut tasks = Vec::new();
            for comp in chunk {
                let comp_name = comp.clone();
                let q_str = format!(
                    "created_at:>'{}' AND created_at:<'{}' AND cf__components:'{}'",
                    start_day_before, end_day_after, comp_name
                );
                let q = crate::query::SearchTicketsQuery::new(q_str);
                tasks.push(async move {
                    let res = client.search_tickets(&q).await;
                    (comp_name, res.map(|r| r.total as u64).unwrap_or(0))
                });
            }

            let results = futures_util_join(tasks).await;
            for (comp, total) in results {
                total_by_product.insert(comp, total);
            }
        }

        // Include any components found in overdue that weren't in choices list
        for comp in overdue_by_product.keys() {
            if !components.contains(comp) {
                components.push(comp.clone());
            }
        }

        // Assemble product reports
        let mut product_reports: Vec<ProductQuarterReport> = components
            .into_iter()
            .map(|comp| {
                let total = total_by_product.get(&comp).copied().unwrap_or(0);
                let overdue_ids = overdue_by_product.get(&comp).cloned().unwrap_or_default();
                let overdue_count = overdue_ids.len() as u64;

                ProductQuarterReport {
                    product: comp,
                    total_tickets: total.max(overdue_count),
                    total_overdue: overdue_count,
                    overdue_ticket_ids: overdue_ids,
                }
            })
            .collect();

        // Sort: products with tickets first (descending by total tickets), then alphabetically
        product_reports.sort_by(|a, b| {
            b.total_tickets
                .cmp(&a.total_tickets)
                .then_with(|| a.product.cmp(&b.product))
        });

        Ok(Self {
            quarter,
            products: product_reports,
        })
    }

    /// Generates the Excel file using `rust_xlsxwriter` according to the requested layout.
    pub fn write_excel(&self, path: &Path) -> Result<()> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(format!("{} Report", self.quarter.to_string_code()))
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        // Formatting styles
        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x1F4E78)) // Dark blue
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
            .set_background_color(Color::RGB(0xD9E1F2)) // Light blue accent
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Medium);

        let rate_value_format = Format::new()
            .set_bold()
            .set_font_size(12)
            .set_background_color(Color::RGB(0xFFF2CC)) // Light yellow accent
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Medium)
            .set_num_format("0.00%");
        let percent_format = Format::new()
            .set_align(FormatAlign::Right)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Thin)
            .set_num_format("0.00%");

        // 1. Write headers (Row 0, 0-indexed)
        worksheet
            .write_with_format(0, 0, "In Charge", &header_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .write_with_format(0, 1, "Product", &header_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .write_with_format(0, 2, "Total Tickets", &header_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .write_with_format(0, 3, "Total Overdue Ticket", &header_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .write_with_format(0, 4, "Overdue Rate", &header_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .write_with_format(0, 5, "Overdue Tickets", &header_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        // 2. Write data rows
        let num_products = self.products.len();
        for (i, p) in self.products.iter().enumerate() {
            let row = (i + 1) as u32;
            let excel_row = row + 1; // 1-indexed for Excel formulas

            // In Charge: Checkbox
            // Defaults to TRUE only for the specified products, FALSE for all others
            let checked = is_default_in_charge(&p.product);
            worksheet
                .insert_checkbox(row, 0, checked)
                .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

            // Product name
            worksheet
                .write_with_format(row, 1, &p.product, &data_format)
                .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

            // Total Tickets
            worksheet
                .write_with_format(row, 2, p.total_tickets, &number_format)
                .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

            // Total Overdue Ticket
            worksheet
                .write_with_format(row, 3, p.total_overdue, &number_format)
                .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

            // Overdue Rate formula for this product row: =IF(C{row}>0, D{row}/C{row}, 0)
            let row_rate_formula = format!("=IF(C{excel_row}>0, D{excel_row}/C{excel_row}, 0)");
            worksheet
                .write_formula_with_format(row, 4, row_rate_formula.as_str(), &percent_format)
                .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

            // Overdue Tickets (list of ticket IDs separated by ", ")
            worksheet
                .write_with_format(row, 5, p.overdue_ids_string(), &data_format)
                .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        }

        // 3. Right-side Overdue Rate calculation
        // Placed in Column H (7) and Column I (8)
        let last_row = if num_products > 0 {
            num_products as u32 + 1
        } else {
            2
        };

        // Header and formula
        worksheet
            .write_with_format(0, 7, "Overdue Rate", &rate_label_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        // Formula: =IF(SUMIF(A2:A{last}, TRUE, C2:C{last})>0, SUMIF(A2:A{last}, TRUE, D2:D{last})/SUMIF(A2:A{last}, TRUE, C2:C{last}), 0)
        let formula = format!(
            "=IF(SUMIF(A2:A{last_row}, TRUE, C2:C{last_row})>0, SUMIF(A2:A{last_row}, TRUE, D2:D{last_row})/SUMIF(A2:A{last_row}, TRUE, C2:C{last_row}), 0)"
        );

        worksheet
            .write_formula_with_format(0, 8, formula.as_str(), &rate_value_format)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        // Explanatory breakdown rows underneath
        let label_fmt = Format::new()
            .set_align(FormatAlign::Left)
            .set_border(FormatBorder::Thin);
        let val_fmt = Format::new()
            .set_align(FormatAlign::Right)
            .set_border(FormatBorder::Thin);

        worksheet
            .write_with_format(2, 7, "Checked Total Tickets", &label_fmt)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        let formula_total = format!("=SUMIF(A2:A{last_row}, TRUE, C2:C{last_row})");
        worksheet
            .write_formula_with_format(2, 8, formula_total.as_str(), &val_fmt)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        worksheet
            .write_with_format(3, 7, "Checked Overdue Tickets", &label_fmt)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        let formula_overdue = format!("=SUMIF(A2:A{last_row}, TRUE, D2:D{last_row})");
        worksheet
            .write_formula_with_format(3, 8, formula_overdue.as_str(), &val_fmt)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        worksheet
            .write_with_format(4, 7, "Checked Products Count", &label_fmt)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        let formula_count = format!("=COUNTIF(A2:A{last_row}, TRUE)");
        worksheet
            .write_formula_with_format(4, 8, formula_count.as_str(), &val_fmt)
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        // 4. Set column widths for polished presentation
        worksheet
            .set_column_width(0, 12) // In Charge
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(1, 24) // Product
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(2, 16) // Total Tickets
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(3, 22) // Total Overdue Ticket
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(4, 16) // Overdue Rate
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(5, 45) // Overdue Tickets
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(6, 4) // spacer column
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(7, 24) // Summary label
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;
        worksheet
            .set_column_width(8, 16) // Summary value
            .map_err(|e| FreshdeskError::Configuration(e.to_string()))?;

        // Save workbook
        workbook.save(path).map_err(|e| {
            FreshdeskError::Configuration(format!("Failed to save Excel file: {}", e))
        })?;

        info!(
            "Successfully generated quarterly report Excel file at {:?}",
            path
        );
        Ok(())
    }
}

async fn futures_util_join<F, T>(futures: Vec<F>) -> Vec<T>
where
    F: Future<Output = T>,
{
    let mut outputs = Vec::new();
    for f in futures {
        outputs.push(f.await);
    }
    outputs
}
