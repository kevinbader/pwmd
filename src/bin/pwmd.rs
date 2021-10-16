use pwmd::Opt;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    set_up_tracing();
    let opts = Opt::from_args();
    pwmd::register_on_dbus(opts)
}

fn set_up_tracing() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed to initialize logging");
}
