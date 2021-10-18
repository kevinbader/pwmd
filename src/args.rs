use std::path::PathBuf;

use structopt::{clap::arg_enum, StructOpt};

arg_enum! {
    #[derive(Debug)]
    pub enum Bus {
        System,
        Session
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "pwmd", about = "Exposes PWM chips to DBUS.")]
pub struct Args {
    /// Connect to session/user or system-wide message bus.
    #[structopt(short, long, env, possible_values=&Bus::variants(), case_insensitive=true, default_value = "system")]
    pub bus: Bus,

    /// DBUS service name.
    #[structopt(long, env, default_value = "com.kevinbader.pwmd")]
    pub dbus_service_name: String,

    /// For testing: path to the sysfs pwm class directory.
    #[structopt(long, parse(from_os_str), env)]
    pub sysfs_root: Option<PathBuf>,
}
