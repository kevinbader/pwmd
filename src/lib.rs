pub mod args;
pub mod dbus;
mod pwm;

pub use args::Args;
use color_eyre::Report;
use tracing_subscriber::EnvFilter;

pub fn setup() -> Result<(), Report> {
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
