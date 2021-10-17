#![doc = include_str!("../README.md")]

/// Global options
pub mod args;
/// DBUS interface
pub mod dbus;
/// Wraps/exposes the Linux Kernel's PWM functionality.
mod pwm;

pub use args::Args;
use color_eyre::Report;
use tracing_subscriber::EnvFilter;

/// Basic tracing/env setup for pwmd.
pub fn setup_logging() -> Result<(), Report> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "full")
    }
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "full")
    }
    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    // let subscriber = tracing_subscriber::FmtSubscriber::builder()
    //     .with_max_level(tracing::Level::TRACE)
    //     .finish();
    // tracing::subscriber::set_global_default(subscriber).expect("failed to initialize logging");

    Ok(())
}
