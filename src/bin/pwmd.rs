use pwmd::Args;
use structopt::StructOpt;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pwmd::setup_logging().unwrap();
    let opts = Args::from_args();
    pwmd::dbus::listen(opts, || {
        info!("Ready.");
    })
    .await?;
    Ok(())
}
