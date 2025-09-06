use crate::types::{AllMids, Bbo, Book, Candle, Trade};

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
    _verbose: bool,
    quiet: bool,
    price_only: bool,
    csv_export: bool,
    trade_count: u64,
}

impl TradeFormatter {
    pub fn new(
        format: OutputFormat,
        colored: bool,
        _verbose: bool,
        quiet: bool,
        price_only: bool,
        csv_export: bool,
    ) -> Self {
        Self {
            format,
            colored,
            _verbose,
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
                    "COUNT",
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
                    "COUNT", "SIDE", "PRICE", "SIZE", "VALUE", "TIME"
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
            println!("count,side,price,size,value,local_time,unix_timestamp");
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

        let price = trade.price().unwrap_or(0.0);
        let size = trade.size().unwrap_or(0.0);
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

        let price = trade.price().unwrap_or(0.0);
        let size = trade.size().unwrap_or(0.0);
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

        let price = trade.price().unwrap_or(0.0);
        let size = trade.size().unwrap_or(0.0);
        let value = price * size;

        let json_obj = serde_json::json!({
            "count": self.trade_count,
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

        let price = trade.price().unwrap_or(0.0);
        let size = trade.size().unwrap_or(0.0);
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
        let price = trade.price().unwrap_or(0.0);
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

        let price = trade.price().unwrap_or(0.0);
        let size = trade.size().unwrap_or(0.0);
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

    pub fn print_status(&self, status: &str, message: &str) {
        if self.quiet && status != "ERROR" {
            return;
        }

        let symbol = if self.colored {
            match status {
                "CONNECTING" => (Colors::BRIGHT_YELLOW, "⚡"),
                "CONNECTED" => (Colors::BRIGHT_GREEN, "✓"),
                "LISTENING" => (Colors::BRIGHT_BLUE, "📡"),
                "ERROR" => (Colors::BRIGHT_RED, "❌"),
                _ => (Colors::WHITE, "•"),
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

pub struct BookFormatter;

impl BookFormatter {
    pub fn format_book(&self, book: &Book) -> String {
        let _local_time = chrono::DateTime::from_timestamp_millis(book.time)
            .unwrap_or_else(chrono::Utc::now)
            .with_timezone(&chrono::Local);

        let mut output = format!(
            "{}{}[ORDER BOOK]{} {} {} | Unix: {}{}{}\n",
            Colors::BOLD,
            Colors::BRIGHT_BLUE,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            book.coin,
            Colors::RESET,
            Colors::DIM,
            book.time
        );

        // Format asks (descending order)
        output.push_str(&format!(
            "{}{}ASKS{}\n",
            Colors::BOLD,
            Colors::BRIGHT_RED,
            Colors::RESET
        ));
        for ask in book.levels.1.iter().take(10) {
            if let (Ok(price), Ok(size)) = (ask.px.parse::<f64>(), ask.sz.parse::<f64>()) {
                output.push_str(&format!(
                    "  {}{:>12.2}{} | {}{:>10.6}{} | Orders: {}{}{}\n",
                    Colors::RED,
                    price,
                    Colors::RESET,
                    Colors::BRIGHT_WHITE,
                    size,
                    Colors::RESET,
                    Colors::GRAY,
                    ask.n,
                    Colors::RESET
                ));
            }
        }

        output.push_str(&format!(
            "{}--- SPREAD ---{}\n",
            Colors::GRAY,
            Colors::RESET
        ));

        // Format bids (ascending order)
        output.push_str(&format!(
            "{}{}BIDS{}\n",
            Colors::BOLD,
            Colors::BRIGHT_GREEN,
            Colors::RESET
        ));
        for bid in book.levels.0.iter().take(10) {
            if let (Ok(price), Ok(size)) = (bid.px.parse::<f64>(), bid.sz.parse::<f64>()) {
                output.push_str(&format!(
                    "  {}{:>12.2}{} | {}{:>10.6}{} | Orders: {}{}{}\n",
                    Colors::GREEN,
                    price,
                    Colors::RESET,
                    Colors::BRIGHT_WHITE,
                    size,
                    Colors::RESET,
                    Colors::GRAY,
                    bid.n,
                    Colors::RESET
                ));
            }
        }

        output
    }
}

pub struct BboFormatter;

impl BboFormatter {
    pub fn format_bbo(&self, bbo: &Bbo) -> String {
        let _local_time = chrono::DateTime::from_timestamp_millis(bbo.time)
            .unwrap_or_else(chrono::Utc::now)
            .with_timezone(&chrono::Local);

        let mut output = format!(
            "{}{}[BBO]{} {} {} | Unix: {}{}{}\n",
            Colors::BOLD,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            bbo.coin,
            Colors::RESET,
            Colors::DIM,
            bbo.time
        );

        if let Some(ref ask) = bbo.bbo.1
            && let (Ok(price), Ok(size)) = (ask.px.parse::<f64>(), ask.sz.parse::<f64>())
        {
            output.push_str(&format!(
                "  Ask: {}{:>12.2}{} | Size: {}{:>10.6}{}\n",
                Colors::RED,
                price,
                Colors::RESET,
                Colors::BRIGHT_WHITE,
                size,
                Colors::RESET
            ));
        }

        if let Some(ref bid) = bbo.bbo.0
            && let (Ok(price), Ok(size)) = (bid.px.parse::<f64>(), bid.sz.parse::<f64>())
        {
            output.push_str(&format!(
                "  Bid: {}{:>12.2}{} | Size: {}{:>10.6}{}\n",
                Colors::GREEN,
                price,
                Colors::RESET,
                Colors::BRIGHT_WHITE,
                size,
                Colors::RESET
            ));
        }

        // Calculate spread if both bid and ask exist
        if let (Some(bid), Some(ask)) = (&bbo.bbo.0, &bbo.bbo.1)
            && let (Ok(bid_price), Ok(ask_price)) = (bid.px.parse::<f64>(), ask.px.parse::<f64>())
        {
            let spread = ask_price - bid_price;
            let spread_pct = (spread / ask_price) * 100.0;
            output.push_str(&format!(
                "  Spread: {}{:.2}{} ({}{:.4}%{})\n",
                Colors::YELLOW,
                spread,
                Colors::RESET,
                Colors::YELLOW,
                spread_pct,
                Colors::RESET
            ));
        }

        output
    }
}

pub struct CandleFormatter;

impl CandleFormatter {
    pub fn format_candle(&self, candle: &Candle) -> String {
        let open_local = candle.open_time_local();
        let close_local = candle.close_time_local();

        let change = candle.c - candle.o;
        let change_pct = (change / candle.o) * 100.0;
        let change_color = if change >= 0.0 {
            Colors::GREEN
        } else {
            Colors::RED
        };
        let change_sign = if change >= 0.0 { "+" } else { "" };

        format!(
            "{}{}[CANDLE]{} {} {} | Interval: {}{}{} | {}{} - {}{}\n\
            Open: {}{:>12.2}{} | High: {}{:>12.2}{} | Low: {}{:>12.2}{} | Close: {}{:>12.2}{}\n\
            Change: {}{}{:.2}{} ({}{:.2}%{}) | Volume: {}{:.6}{} | Trades: {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            candle.s,
            Colors::RESET,
            Colors::CYAN,
            candle.i,
            Colors::RESET,
            Colors::GRAY,
            open_local.format("%H:%M:%S"),
            close_local.format("%H:%M:%S"),
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.o,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.h,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.l,
            Colors::RESET,
            Colors::BRIGHT_WHITE,
            candle.c,
            Colors::RESET,
            change_color,
            change_sign,
            change,
            Colors::RESET,
            change_color,
            change_pct,
            Colors::RESET,
            Colors::BRIGHT_CYAN,
            candle.v,
            Colors::RESET,
            Colors::YELLOW,
            candle.n
        )
    }
}

pub struct AllMidsFormatter;

impl AllMidsFormatter {
    pub fn format_all_mids(&self, all_mids: &AllMids) -> String {
        let mut output = format!(
            "{}{}[ALL MIDS]{} {} symbols\n",
            Colors::BOLD,
            Colors::BRIGHT_YELLOW,
            Colors::RESET,
            all_mids.mids.len()
        );

        let mut sorted_mids: Vec<_> = all_mids.mids.iter().collect();
        sorted_mids.sort_by(|a, b| a.0.cmp(b.0));

        for (coin, price) in sorted_mids.iter() {
            if let Ok(price_val) = price.parse::<f64>() {
                output.push_str(&format!(
                    "  {}{:>8}{}: {}{:>12.2}{}\n",
                    Colors::BRIGHT_YELLOW,
                    coin,
                    Colors::RESET,
                    Colors::BRIGHT_WHITE,
                    price_val,
                    Colors::RESET
                ));
            }
        }

        output
    }
}
