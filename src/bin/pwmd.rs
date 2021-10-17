use pwmd::Args;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    pwmd::setup_logging().unwrap();
    let opts = Args::from_args();
    pwmd::dbus::listen(opts, || {})
}
