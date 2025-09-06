// file: src/ui.rs
// description: ui presentation layer that handles events from the client

use crate::{
    events::{ClientEvent, EventReceiver},
    formatter::{Colors, OutputFormat, TradeFormatter},
};
use tracing::{debug, info, warn};

pub struct UIController {
    event_receiver: EventReceiver,
    trade_formatter: TradeFormatter,
    quiet_mode: bool,
    header_printed: bool,
}

impl UIController {
    pub fn new(
        event_receiver: EventReceiver,
        format: OutputFormat,
        colored: bool,
        verbose: bool,
        quiet: bool,
        price_only: bool,
        csv_export: bool,
    ) -> Self {
        Self {
            event_receiver,
            trade_formatter: TradeFormatter::new(
                format, colored, verbose, quiet, price_only, csv_export,
            ),
            quiet_mode: quiet,
            header_printed: false, // Initialize as false
        }
    }

    pub async fn run(&mut self) {
        self.print_startup_banner();
        while let Some(event) = self.event_receiver.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn handle_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Starting => {
                info!("Client starting...");
            }
            ClientEvent::Connecting { url } => {
                self.print_connection_status("CONNECTING", &url);
            }
            ClientEvent::Connected { connection_id } => {
                self.print_connection_status("CONNECTED", &format!("ID: {}", connection_id));
            }
            ClientEvent::SubscriptionSent { message } => {
                self.print_subscription_info(&message);
            }
            ClientEvent::SubscriptionConfirmed { sub_type, coin } => {
                self.print_subscription_confirmed(&sub_type, &coin);
                // Print the table header here, after connection is fully established
                if !self.header_printed {
                    self.trade_formatter.print_header();
                    self.header_printed = true;
                }
            }
            ClientEvent::TradeReceived(trade) => {
                // Ensure header is printed before any trades (fallback safety)
                if !self.header_printed {
                    self.trade_formatter.print_header();
                    self.header_printed = true;
                }
                self.trade_formatter.print_trade(&trade);
            }
            ClientEvent::MessageReceived { raw_message } => {
                debug!("Received message: {}", raw_message);
            }
            ClientEvent::ConnectionFailed(error) => {
                self.print_error("CONNECTION FAILED", &error);
            }
            ClientEvent::Reconnecting {
                attempt,
                delay_secs,
            } => {
                self.print_reconnect_info(delay_secs, attempt);
            }
            ClientEvent::HealthCheckFailed { reason } => {
                warn!("Health check failed: {}", reason);
            }
            ClientEvent::Disconnected => {
                self.print_connection_status("DISCONNECTED", "Connection closed");
            }
            ClientEvent::Stopping => {
                self.print_connection_status("STOPPING", "Client shutting down");
            }
        }
    }

    fn print_startup_banner(&self) {
        if self.quiet_mode {
            return;
        }

        println!();
        println!(
            "{}{}╔══════════════════════════════════════════════════════════════════════════════╗{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}║                         HYPERLIQUID WEBSOCKET CLIENT                        ║{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}╠══════════════════════════════════════════════════════════════════════════════╣{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}║{} Version: {}{:<8}{} │ Type: {}{:<10}{} │ Status: {}INITIALIZING{}{}║{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET,
            Colors::BRIGHT_GREEN,
            env!("CARGO_PKG_VERSION"),
            Colors::RESET,
            Colors::BRIGHT_YELLOW,
            "TRADES",
            Colors::RESET,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!(
            "{}{}╚══════════════════════════════════════════════════════════════════════════════╝{}",
            Colors::BOLD,
            Colors::BRIGHT_CYAN,
            Colors::RESET
        );
        println!();
    }

    fn print_connection_status(&self, status: &str, message: &str) {
        if self.quiet_mode && status != "ERROR" {
            return;
        }

        let (color, symbol) = match status {
            "CONNECTING" => (Colors::BRIGHT_YELLOW, "*"),
            "CONNECTED" => (Colors::BRIGHT_GREEN, "+"),
            "LISTENING" => (Colors::BRIGHT_BLUE, "~"),
            "DISCONNECTED" => (Colors::BRIGHT_RED, "X"),
            "STOPPING" => (Colors::BRIGHT_MAGENTA, "!"),
            _ => (Colors::WHITE, "-"),
        };

        println!(
            "{}{}[{}]{} {} {}{}{}",
            Colors::BOLD,
            color,
            status,
            Colors::RESET,
            symbol,
            Colors::WHITE,
            message,
            Colors::RESET
        );
    }

    fn print_subscription_info(&self, message: &str) {
        if self.quiet_mode {
            return;
        }

        println!(
            "{}{}[SUBSCRIBING]{} > {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_MAGENTA,
            Colors::RESET,
            Colors::DIM,
            message,
            Colors::RESET
        );
    }

    fn print_subscription_confirmed(&self, sub_type: &str, coin: &str) {
        if self.quiet_mode {
            return;
        }

        println!(
            "{}{}[SUBSCRIPTION OK]{} + {} subscription active for {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_GREEN,
            Colors::RESET,
            sub_type,
            Colors::BRIGHT_YELLOW,
            coin,
            Colors::RESET
        );
        println!();
    }

    fn print_error(&self, error_type: &str, message: &str) {
        println!(
            "{}{}[{}]{} ! {}{}{}",
            Colors::BOLD,
            Colors::BRIGHT_RED,
            error_type,
            Colors::RESET,
            Colors::RED,
            message,
            Colors::RESET
        );
    }

    fn print_reconnect_info(&self, delay_secs: u64, attempt: u32) {
        println!(
            "{}{}[RECONNECTING]{} > Attempt {} in {}s...",
            Colors::BOLD,
            Colors::BRIGHT_YELLOW,
            Colors::RESET,
            attempt,
            delay_secs
        );
    }
}
