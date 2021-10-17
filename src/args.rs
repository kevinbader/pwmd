use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "pwmd", about = "Exposes PWM chips to DBUS.")]
pub struct Args {
    /// For testing: path to the sysfs pwm class directory.
    #[structopt(long, parse(from_os_str), env)]
    pub sysfs_root: Option<PathBuf>,
}
