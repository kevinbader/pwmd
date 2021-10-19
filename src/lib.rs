#![doc = include_str!("../README.md")]

/// Global options
pub mod args;
/// DBUS interface
pub mod dbus;
/// Wraps/exposes the Linux Kernel's PWM functionality.
mod pwm;

pub use args::Args;
use tracing_subscriber::EnvFilter;

/// Basic tracing/env setup for pwmd.
pub fn setup_logging() {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
