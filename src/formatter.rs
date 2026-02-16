/// file: src/formatter.rs
/// description: Trade data formatting and output display utilities for various formats
/// reference: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket
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
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "csv" => OutputFormat::Csv,
            "json" => OutputFormat::Json,
            "minimal" => OutputFormat::Minimal,
            _ => OutputFormat::Table,
        }
    }
}

pub struct TradeFormatter {
    format: OutputFormat,
    colored: bool,
    verbose: bool,
    quiet: bool,
    price_only: bool,
    csv_export: bool,
    trade_count: u64,
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
        }
    }

    pub fn print_header(&self) {
        if self.quiet {
            return;
        }

        match self.format {
            OutputFormat::Table => self.print_table_header(),
            OutputFormat::Csv => self.print_csv_header(),
            OutputFormat::Json => {}    // JSON doesn't need headers
            OutputFormat::Minimal => {} // Minimal doesn't need headers
        }
    }

    pub fn print_trade(&mut self, trade: &Trade) {
        self.trade_count += 1;

        if self.price_only {
            self.print_price_only(trade);
            return;
        }

        match self.format {
            OutputFormat::Table => self.print_table_row(trade),
            OutputFormat::Csv => self.print_csv_row(trade),
            OutputFormat::Json => self.print_json_row(trade),
            OutputFormat::Minimal => self.print_minimal_row(trade),
        }

        // Export to CSV on stderr if enabled
        if self.csv_export {
            self.export_csv_to_stderr(trade);
        }

        if self.verbose {
            self.print_verbose_trade_details(trade);
        }
    }

    pub fn trade_count(&self) -> u64 {
        self.trade_count
    }

    fn print_table_header(&self) {
        if !self.quiet {
            let header = if self.colored {
                format!(
                    "{}{}┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐{}",
                    Colors::BOLD,
                    Colors::GRAY,
                    Colors::RESET
                )
            } else {
                "┌─────────┬──────┬─────────────┬─────────────┬─────────────┬─────────────────────┐"
                    .to_string()
            };
            println!("{}", header);

            let labels = if self.colored {
                format!(
                    "{}{}│{} {:<7} {}│{} {:<4} {}│{} {:<11} {}│{} {:<11} {}│{} {:<11} {}│{} {:<19} {}│{}",
                    Colors::BOLD,
                    Colors::GRAY,
                    Colors::RESET,
                    "#",
                    Colors::GRAY,
                    Colors::RESET,
                    "SIDE",
                    Colors::GRAY,
                    Colors::RESET,
                    "PRICE",
                    Colors::GRAY,
                    Colors::RESET,
                    "SIZE",
                    Colors::GRAY,
                    Colors::RESET,
                    "VALUE",
                    Colors::GRAY,
                    Colors::RESET,
                    "TIME",
                    Colors::GRAY,
                    Colors::RESET,
                )
            } else {
                format!(
                    "│ {:<7} │ {:<4} │ {:<11} │ {:<11} │ {:<11} │ {:<19} │",
                    "#", "SIDE", "PRICE", "SIZE", "VALUE", "TIME"
                )
            };
            println!("{}", labels);

            let separator = if self.colored {
                format!(
                    "{}{}├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤{}",
                    Colors::BOLD,
                    Colors::GRAY,
                    Colors::RESET
                )
            } else {
                "├─────────┼──────┼─────────────┼─────────────┼─────────────┼─────────────────────┤"
                    .to_string()
            };
            println!("{}", separator);
        }
    }

    fn print_csv_header(&self) {
        if !self.quiet {
            println!("#,side,price,size,value,local_time,unix_timestamp");
        }
    }

    fn print_table_row(&self, trade: &Trade) {
        let side_color = if self.colored {
            if trade.is_buy() {
                Colors::BRIGHT_GREEN
            } else {
                Colors::BRIGHT_RED
            }
        } else {
            ""
        };

        let reset = if self.colored { Colors::RESET } else { "" };
        let gray = if self.colored { Colors::GRAY } else { "" };

        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        println!(
            "{}│{} {:<7} {}│{} {}{:<4}{} {}│{} {:<11.2} {}│{} {:<11.6} {}│{} {:<11.2} {}│{} {:<19} {}│{}",
            gray,
            reset,
            self.trade_count,
            gray,
            reset,
            side_color,
            side_text,
            reset,
            gray,
            reset,
            price,
            gray,
            reset,
            size,
            gray,
            reset,
            value,
            gray,
            reset,
            local_time.format("%H:%M:%S"),
            gray,
            reset
        );
    }

    fn print_csv_row(&self, trade: &Trade) {
        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        println!(
            "{},{},{:.2},{:.6},{:.2},{},{}",
            self.trade_count,
            side_text,
            price,
            size,
            value,
            local_time.format("%Y-%m-%d %H:%M:%S"),
            trade.time
        );
    }

    fn print_json_row(&self, trade: &Trade) {
        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        let json_obj = serde_json::json!({
            "#": self.trade_count,
            "coin": trade.coin,
            "side": side_text,
            "price": price,
            "size": size,
            "value": value,
            "local_time": local_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            "unix_timestamp": trade.time,
            "trade_id": trade.tid,
            "hash": trade.hash
        });

        println!("{}", serde_json::to_string(&json_obj).unwrap_or_default());
    }

    fn print_minimal_row(&self, trade: &Trade) {
        let side_symbol = if trade.is_buy() { "↗" } else { "↘" };
        let side_color = if self.colored {
            if trade.is_buy() {
                Colors::BRIGHT_GREEN
            } else {
                Colors::BRIGHT_RED
            }
        } else {
            ""
        };
        let reset = if self.colored { Colors::RESET } else { "" };

        let price = trade.px;
        let size = trade.sz;
        let local_time = trade.datetime_local();

        println!(
            "{} {}{}{} {:<8.2} {:<8.6} {}",
            local_time.format("%H:%M:%S"),
            side_color,
            side_symbol,
            reset,
            price,
            size,
            trade.coin
        );
    }

    fn print_price_only(&self, trade: &Trade) {
        let price = trade.px;
        let side_color = if self.colored {
            if trade.is_buy() {
                Colors::BRIGHT_GREEN
            } else {
                Colors::BRIGHT_RED
            }
        } else {
            ""
        };
        let reset = if self.colored { Colors::RESET } else { "" };

        println!("{}{:.2}{}", side_color, price, reset);
    }

    fn export_csv_to_stderr(&self, trade: &Trade) {
        let side_text = if trade.is_buy() { "BUY" } else { "SELL" };
        let local_time = trade.datetime_local();

        let price = trade.px;
        let size = trade.sz;
        let value = price * size;

        eprintln!(
            "{},{},{:.2},{:.6},{:.2},{},{}",
            self.trade_count,
            side_text,
            price,
            size,
            value,
            local_time.format("%Y-%m-%d %H:%M:%S"),
            trade.time
        );
    }

    fn print_verbose_trade_details(&self, trade: &Trade) {
        match self.format {
            OutputFormat::Table | OutputFormat::Minimal => {
                let (buyer, seller) = trade.buyer_seller();
                match (buyer, seller) {
                    (Some(buyer), Some(seller)) => {
                        println!("  users: buyer={} seller={}", buyer, seller);
                    }
                    (Some(buyer), None) => {
                        println!("  users: buyer={}", buyer);
                    }
                    _ => {}
                }
            }
            OutputFormat::Csv | OutputFormat::Json => {}
        }
    }

    pub fn print_status(&self, status: &str, message: &str) {
        if self.quiet && status != "ERROR" {
            return;
        }

        let symbol = if self.colored {
            match status {
                "CONNECTING" => (Colors::BRIGHT_YELLOW, "*"),
                "CONNECTED" => (Colors::BRIGHT_GREEN, "+"),
                "LISTENING" => (Colors::BRIGHT_BLUE, "~"),
                "ERROR" => (Colors::BRIGHT_RED, "!"),
                _ => (Colors::WHITE, "-"),
            }
        } else {
            (
                "",
                match status {
                    "CONNECTING" => "*",
                    "CONNECTED" => "+",
                    "LISTENING" => "~",
                    "ERROR" => "!",
                    _ => "-",
                },
            )
        };

        let _reset = if self.colored { Colors::RESET } else { "" };

        println!("[{}] {} {}", status, symbol.1, message);
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

        println!();
        if self.colored {
            println!(
                "{}{}Summary: {} trades in {}s ({:.2} trades/sec){}",
                Colors::BOLD,
                Colors::BRIGHT_CYAN,
                total_trades,
                duration_secs,
                rate,
                Colors::RESET
            );
        } else {
            println!(
                "Summary: {} trades in {}s ({:.2} trades/sec)",
                total_trades, duration_secs, rate
            );
        }
    }
}
