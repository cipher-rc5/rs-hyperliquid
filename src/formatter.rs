/// Trade data formatting and output display utilities.
use std::io::{self, Write};

use chrono::{DateTime, Local};
use serde::{Serialize, Serializer};

use crate::types::Trade;

// ANSI color codes
pub struct Colors;

impl Colors {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const DIM: &'static str = "\x1b[2m";

    // Colors
    pub const RED: &'static str = "\x1b[31m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const MAGENTA: &'static str = "\x1b[35m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const WHITE: &'static str = "\x1b[37m";
    pub const GRAY: &'static str = "\x1b[90m";

    // Bright colors
    pub const BRIGHT_RED: &'static str = "\x1b[91m";
    pub const BRIGHT_GREEN: &'static str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &'static str = "\x1b[93m";
    pub const BRIGHT_BLUE: &'static str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &'static str = "\x1b[95m";
    pub const BRIGHT_CYAN: &'static str = "\x1b[96m";
    pub const BRIGHT_WHITE: &'static str = "\x1b[97m";
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Csv,
    Json,
    Minimal,
}

impl From<&str> for OutputFormat {
    fn from(value: &str) -> Self {
        match value {
            _ if value.eq_ignore_ascii_case("csv") => Self::Csv,
            _ if value.eq_ignore_ascii_case("json") => Self::Json,
            _ if value.eq_ignore_ascii_case("minimal") => Self::Minimal,
            _ => Self::Table,
        }
    }
}

#[derive(Serialize)]
struct JsonTrade<'a> {
    #[serde(rename = "#")]
    count: u64,
    coin: &'a str,
    hash: &'a str,
    #[serde(serialize_with = "serialize_local_time")]
    local_time: DateTime<Local>,
    price: f64,
    side: &'static str,
    size: f64,
    trade_id: i64,
    unix_timestamp: i64,
    value: f64,
}

fn serialize_local_time<S: Serializer>(
    time: &DateTime<Local>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&time.format("%Y-%m-%d %H:%M:%S"))
}

pub struct TradeFormatter {
    format: OutputFormat,
    colored: bool,
    verbose: bool,
    quiet: bool,
    price_only: bool,
    csv_export: bool,
    trade_count: u64,
    json_buffer: Vec<u8>,
}

impl TradeFormatter {
    pub fn new(
        format: OutputFormat,
        colored: bool,
        verbose: bool,
        quiet: bool,
        price_only: bool,
        csv_export: bool,
    ) -> Self {
        Self {
            format,
            colored,
            verbose,
            quiet,
            price_only,
            csv_export,
            trade_count: 0,
            json_buffer: Vec::new(),
        }
    }

    pub fn print_header(&self) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Table => self.print_table_header(),
            OutputFormat::Csv => println!("#,side,price,size,value,local_time,unix_timestamp"),
            OutputFormat::Json | OutputFormat::Minimal => {}
        }
    }

    pub fn print_trade(&mut self, trade: &Trade) {
        self.trade_count += 1;
        if self.price_only {
            println!(
                "{}{:.2}{}",
                self.side_color(trade),
                trade.px,
                self.color(Colors::RESET)
            );
            return;
        }
        match self.format {
            OutputFormat::Table => self.print_table_row(trade),
            OutputFormat::Csv => self
                .write_csv_row(&mut io::stdout().lock(), trade)
                .expect("failed to print CSV trade"),
            OutputFormat::Json => {
                self.json_buffer.clear();
                Self::write_json_row(&mut self.json_buffer, trade, self.trade_count)
                    .expect("failed to encode JSON trade");
                io::stdout()
                    .lock()
                    .write_all(&self.json_buffer)
                    .expect("failed to print JSON trade");
            }
            OutputFormat::Minimal => self.print_minimal_row(trade),
        }
        if self.csv_export {
            self.write_csv_row(&mut io::stderr().lock(), trade)
                .expect("failed to export CSV trade");
        }
        if self.verbose {
            self.print_verbose_trade_details(trade);
        }
    }

    pub fn trade_count(&self) -> u64 {
        self.trade_count
    }

    fn color(&self, color: &'static str) -> &'static str {
        if self.colored { color } else { "" }
    }

    fn side_color(&self, trade: &Trade) -> &'static str {
        self.color(if trade.is_buy() {
            Colors::BRIGHT_GREEN
        } else {
            Colors::BRIGHT_RED
        })
    }

    fn print_table_header(&self) {
        let bold = self.color(Colors::BOLD);
        let gray = self.color(Colors::GRAY);
        let reset = self.color(Colors::RESET);
        println!(
            "{bold}{gray}┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐{reset}"
        );
        println!(
            "{bold}{gray}│{reset} {:<7} {gray}│{reset} {:<4} {gray}│{reset} {:<11} {gray}│{reset} {:<11} {gray}│{reset} {:<11} {gray}│{reset} {:<19} {gray}│{reset}",
            "#", "SIDE", "PRICE", "SIZE", "VALUE", "TIME"
        );
        println!(
            "{bold}{gray}├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤{reset}"
        );
    }

    fn print_table_row(&self, trade: &Trade) {
        let side_color = self.side_color(trade);
        let reset = self.color(Colors::RESET);
        let gray = self.color(Colors::GRAY);
        println!(
            "{gray}│{reset} {:<7} {gray}│{reset} {side_color}{:<4}{reset} {gray}│{reset} {:<11.2} {gray}│{reset} {:<11.6} {gray}│{reset} {:<11.2} {gray}│{reset} {:<19} {gray}│{reset}",
            self.trade_count,
            trade.side_formatted(),
            trade.px,
            trade.sz,
            trade.value(),
            trade.datetime_local().format("%H:%M:%S")
        );
    }

    fn write_csv_row(&self, writer: &mut impl Write, trade: &Trade) -> io::Result<()> {
        writeln!(
            writer,
            "{},{},{:.2},{:.6},{:.2},{},{}",
            self.trade_count,
            trade.side_formatted(),
            trade.px,
            trade.sz,
            trade.value(),
            trade.datetime_local().format("%Y-%m-%d %H:%M:%S"),
            trade.time
        )
    }

    fn write_json_row(writer: &mut impl Write, trade: &Trade, count: u64) -> io::Result<()> {
        serde_json::to_writer(
            &mut *writer,
            &JsonTrade {
                count,
                coin: &trade.coin,
                hash: &trade.hash,
                local_time: trade.datetime_local(),
                price: trade.px,
                side: trade.side_formatted(),
                size: trade.sz,
                trade_id: trade.tid,
                unix_timestamp: trade.time,
                value: trade.value(),
            },
        )?;
        writeln!(writer)
    }

    fn print_minimal_row(&self, trade: &Trade) {
        let side_symbol = if trade.is_buy() { "↗" } else { "↘" };
        println!(
            "{} {}{}{} {:<8.2} {:<8.6} {}",
            trade.datetime_local().format("%H:%M:%S"),
            self.side_color(trade),
            side_symbol,
            self.color(Colors::RESET),
            trade.px,
            trade.sz,
            trade.coin
        );
    }

    fn print_verbose_trade_details(&self, trade: &Trade) {
        if matches!(self.format, OutputFormat::Table | OutputFormat::Minimal) {
            match trade.buyer_seller() {
                (Some(buyer), Some(seller)) => {
                    println!("  users: buyer={} seller={}", buyer, seller)
                }
                (Some(buyer), None) => println!("  users: buyer={}", buyer),
                _ => {}
            }
        }
    }

    pub fn print_status(&self, status: &str, message: &str) {
        if self.quiet && status != "ERROR" {
            return;
        }
        let symbol = match status {
            "CONNECTING" => "*",
            "CONNECTED" => "+",
            "LISTENING" => "~",
            "ERROR" => "!",
            _ => "-",
        };
        println!("[{status}] {symbol} {message}");
    }

    pub fn print_summary(&self, total_trades: u64, duration_secs: u64) {
        if self.quiet {
            return;
        }
        let rate = if duration_secs > 0 {
            total_trades as f64 / duration_secs as f64
        } else {
            0.0
        };
        println!(
            "\n{}{}Summary: {} trades in {}s ({:.2} trades/sec){}",
            self.color(Colors::BOLD),
            self.color(Colors::BRIGHT_CYAN),
            total_trades,
            duration_secs,
            rate,
            self.color(Colors::RESET)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_and_csv_preserve_the_output_contract() -> Result<(), Box<dyn std::error::Error>> {
        let trade = Trade {
            coin: "B\"TC\\\n".into(),
            side: "buy".into(),
            px: 100000.5,
            sz: 0.25,
            time: 1760000000000,
            hash: "0x\\\"1234".into(),
            tid: 42,
            users: Vec::new(),
        };
        let mut formatter =
            TradeFormatter::new(OutputFormat::Json, false, false, true, false, false);
        formatter.trade_count = 7;
        let mut output = Vec::new();
        TradeFormatter::write_json_row(&mut output, &trade, formatter.trade_count())?;
        let expected = serde_json::json!({
            "#": 7, "coin": trade.coin, "hash": trade.hash,
            "local_time": trade.datetime_local().format("%Y-%m-%d %H:%M:%S").to_string(),
            "price": 100000.5, "side": "BUY", "size": 0.25, "trade_id": 42,
            "unix_timestamp": 1760000000000_i64, "value": 25000.125,
        });
        assert_eq!(String::from_utf8(output)?, format!("{expected}\n"));
        let mut csv = Vec::new();
        formatter.write_csv_row(&mut csv, &trade)?;
        assert_eq!(
            String::from_utf8(csv)?,
            format!(
                "7,BUY,100000.50,0.250000,25000.12,{},1760000000000\n",
                trade.datetime_local().format("%Y-%m-%d %H:%M:%S")
            )
        );
        Ok(())
    }

    #[test]
    fn json_preserves_null_for_non_finite_numbers() -> Result<(), Box<dyn std::error::Error>> {
        for price in [f64::NAN, f64::INFINITY] {
            let trade = Trade {
                px: price,
                sz: 1.0,
                ..Trade::default()
            };
            let mut output = Vec::new();
            TradeFormatter::write_json_row(&mut output, &trade, 0)?;
            let value: serde_json::Value = serde_json::from_slice(&output)?;
            assert!(value["price"].is_null());
            assert!(value["value"].is_null());
        }
        Ok(())
    }
}
