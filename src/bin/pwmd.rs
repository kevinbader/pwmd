fn main() -> anyhow::Result<()> {
    set_up_tracing();
    pwmd::register_on_dbus()
}

fn set_up_tracing() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed to initialize logging");
}
