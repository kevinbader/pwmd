use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use thiserror::Error;
use tracing::{debug, instrument};

/// Everything that can go wrong.
#[derive(Error, Debug)]
pub enum PwmError {
    #[error("{0:?} not found")]
    ControllerNotFound(Controller),
    #[error("{0:?}/{1:?} not found")]
    ChannelNotFound(Controller, Channel),
    #[error("{0:?} not exported")]
    NotExported(Controller),
    #[error("failed to {0:?}: {1}")]
    Sysfs(Access, #[source] std::io::Error),
    #[error("duty cycle value must be less than the period value")]
    DutyCycleNotLessThanPeriod,
    #[error("legal polarity values: 'normal', 'inversed'")]
    InvalidPolarity,
    #[error("{0} cannot be changed while channel is enabled")]
    IllegalChangeWhileEnabled(&'static str),
    #[error("expected boolean value, got {0:?}")]
    NotBoolean(String),
    #[error("expected a duration in nanoseconds, got {0:?}: {1}")]
    NotADuration(String, #[source] std::num::ParseIntError),
}

/// Used in PwmError to format sysfs related errors.
#[derive(Debug)]
pub enum Access {
    Read(PathBuf),
    Write(PathBuf),
}

/// Exposes PWM functionality.
///
/// Since the Linux kernel exposes PWM controllers and their settings through
/// sysfs, PWM operations are just file reads and writes. To allow testing with
/// a real file system but outside of sysfs, the `sysfs_root` property may be
/// used to "offset" those operations to an alternative directory.
///
/// Documentation on Linux PWM sysfs:
/// <https://www.kernel.org/doc/html/latest/driver-api/pwm.html>
#[derive(Debug)]
pub struct Pwm {
    sysfs_root: PathBuf,
}

/// A PWM controller (a.k.a. PWM chip) is identified by a non-negative number.
#[derive(Debug, Clone)]
pub struct Controller(pub u32);

/// PWM controllers expose channels, which are also identified by non-negative numbers.
#[derive(Debug, Clone)]
pub struct Channel(pub u32);

type Result<T> = std::result::Result<T, PwmError>;

impl Pwm {
    /// Initialize PWM.
    pub fn new() -> Self {
        Self::with_sysfs_root(PathBuf::from("/sys/class/pwm"))
    }

    /// Initialize PWM with an alternative sysfs directory, for testing.
    pub fn with_sysfs_root(sysfs_root: PathBuf) -> Self {
        if !sysfs_root.exists() {
            panic!("sysfs root does not exist: {:?}", sysfs_root);
        }
        Self { sysfs_root }
    }

    /// Returns the number of channels for the given controller.
    #[instrument]
    pub fn npwm(&self, controller: &Controller) -> Result<u32> {
        self.controller_file(controller, "npwm")
            .and_then(|path| read(&path))
            .map(|s| {
                s.trim()
                    .parse::<u32>()
                    .expect("npwm expected to contain the number of channels")
            })
    }

    /// Returns whether a controller's channels are ready to be used.
    #[instrument]
    pub fn is_exported(&self, controller: &Controller) -> Result<bool> {
        // A controller is exported if the channel subdirectories are there.
        // Since a controller without any channel doesn't make sense, it's
        // enough to check for the existance of the first channel's enable file.
        match self.channel_dir(controller, &Channel(0)) {
            Ok(_) => Ok(true),
            Err(PwmError::NotExported(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Export a PWM controller, which enables access to its channels.
    #[instrument]
    pub fn export(&mut self, controller: Controller) -> Result<()> {
        self.controller_file(&controller, "export")
            .and_then(|path| write(&path, "1"))
    }

    /// Unexport a PWM controller, which disables access to its channels.
    #[instrument]
    pub fn unexport(&mut self, controller: Controller) -> Result<()> {
        self.controller_file(&controller, "unexport")
            .and_then(|path| write(&path, "1"))
    }

    /// Returns whether a controller's channel is enabled.
    #[instrument]
    pub fn is_enabled(&self, controller: &Controller, channel: &Channel) -> Result<bool> {
        self.channel_file(controller, channel, "enable")
            .and_then(|path| read(&path))
            .and_then(parse_bool)
    }

    /// Enable a channel.
    #[instrument]
    pub fn enable(&mut self, controller: Controller, channel: Channel) -> Result<()> {
        self.channel_file(&controller, &channel, "enable")
            .and_then(|path| write(&path, "1"))
    }

    /// Disable a channel.
    #[instrument]
    pub fn disable(&mut self, controller: Controller, channel: Channel) -> Result<()> {
        self.channel_file(&controller, &channel, "enable")
            .and_then(|path| write(&path, "0"))
    }

    /// The total period of the PWM signal (read/write). Value is in nanoseconds
    /// and is the sum of the active and inactive time of the PWM.
    #[instrument]
    pub fn set_period(
        &mut self,
        controller: Controller,
        channel: Channel,
        period: Duration,
    ) -> Result<()> {
        let duty_cycle = self
            .channel_file(&controller, &channel, "duty_cycle")
            .and_then(|path| read(&path))
            .and_then(parse_duration)?;

        if duty_cycle >= period {
            return Err(PwmError::DutyCycleNotLessThanPeriod);
        }

        self.channel_file(&controller, &channel, "period")
            .and_then(|path| write(&path, &period.as_nanos().to_string()))
    }

    /// The active time of the PWM signal (read/write). Value is in nanoseconds
    /// and must be less than the period.
    #[instrument]
    pub fn set_duty_cycle(
        &mut self,
        controller: Controller,
        channel: Channel,
        duty_cycle: Duration,
    ) -> Result<()> {
        let period = self
            .channel_file(&controller, &channel, "period")
            .and_then(|path| read(&path))
            .and_then(parse_duration)?;

        if duty_cycle >= period {
            return Err(PwmError::DutyCycleNotLessThanPeriod);
        }

        self.channel_file(&controller, &channel, "duty_cycle")
            .and_then(|path| write(&path, &duty_cycle.as_nanos().to_string()))
    }

    /// Changes the polarity of the PWM signal (read/write). Writes to this
    /// property only work if the PWM chip supports changing the polarity. The
    /// polarity can only be changed if the PWM is not enabled. Value is the
    /// string “normal” or “inversed”.
    #[instrument]
    pub fn set_polarity(
        &mut self,
        controller: Controller,
        channel: Channel,
        polarity: Polarity,
    ) -> Result<()> {
        // setting polarity is only allowed if channel is disabled:
        if self.is_enabled(&controller, &channel)? {
            return Err(PwmError::IllegalChangeWhileEnabled("polarity"));
        }

        self.channel_file(&controller, &channel, "polarity")
            .and_then(|path| write(&path, &polarity.to_string()))
    }

    fn controller_dir(&self, controller: &Controller) -> Result<PathBuf> {
        let path = self.sysfs_root.join(format!("pwmchip{}", controller.0));
        if path.is_dir() {
            Ok(path)
        } else {
            Err(PwmError::ControllerNotFound(controller.clone()))
        }
    }

    fn controller_file(&self, controller: &Controller, fname: &str) -> Result<PathBuf> {
        let path = self
            .sysfs_root
            .join(format!("pwmchip{}/{}", controller.0, fname));
        if path.is_file() {
            Ok(path)
        } else {
            Err(PwmError::ControllerNotFound(controller.clone()))
        }
    }

    fn channel_dir(&self, controller: &Controller, channel: &Channel) -> Result<PathBuf> {
        let n_pwm = self.npwm(controller)?;
        if channel.0 >= n_pwm {
            return Err(PwmError::ChannelNotFound(
                controller.clone(),
                channel.clone(),
            ));
        }

        let path = self
            .controller_dir(controller)
            .map(|controller| controller.join(format!("pwm{}", channel.0)))?;
        if path.is_dir() {
            Ok(path)
        } else {
            Err(PwmError::NotExported(controller.clone()))
        }
    }

    fn channel_file(
        &self,
        controller: &Controller,
        channel: &Channel,
        fname: &str,
    ) -> Result<PathBuf> {
        let path = self
            .channel_dir(controller, channel)
            .map(|channel| channel.join(fname))?;
        if path.is_file() {
            Ok(path)
        } else {
            Err(PwmError::NotExported(controller.clone()))
        }
    }
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| PwmError::Sysfs(Access::Read(path.to_owned()), e))
}

fn write(path: &Path, contents: &str) -> Result<()> {
    debug!("writing to {:?}", path);
    fs::write(path, contents).map_err(|e| PwmError::Sysfs(Access::Write(path.to_owned()), e))
}

fn parse_bool(s: String) -> Result<bool> {
    // sysfs compatible according to http://lkml.iu.edu/hypermail/linux/kernel/1103.2/02488.html
    match s.trim_end().to_lowercase().as_ref() {
        "1" | "y" | "yes" | "true" => Ok(true),
        "0" | "n" | "no" | "false" | "" => Ok(false),
        _ => Err(PwmError::NotBoolean(s)),
    }
}

fn parse_duration(s: String) -> Result<Duration> {
    s.trim_end()
        .parse::<u64>()
        .map_err(|e| PwmError::NotADuration(s, e))
        .map(Duration::from_nanos)
}

#[derive(Debug)]
pub enum Polarity {
    Normal,
    Inversed,
}

impl Display for Polarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Polarity::*;
        match *self {
            Normal => write!(f, "normal"),
            Inversed => write!(f, "inversed"),
        }
    }
}

impl FromStr for Polarity {
    type Err = PwmError;

    fn from_str(s: &str) -> Result<Self> {
        use Polarity::*;
        match s {
            "normal" => Ok(Normal),
            "inversed" => Ok(Inversed),
            _ => Err(PwmError::InvalidPolarity),
        }
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use temp_dir::TempDir;

    #[test]
    fn fail_if_controller_not_found() {
        let tmp = TempDir::new().unwrap();
        let mut pwm = Pwm::with_sysfs_root(tmp.path().to_owned());

        assert!(matches!(
            pwm.export(Controller(4)),
            Err(PwmError::ControllerNotFound(Controller(4)))
        ));
        assert!(matches!(
            pwm.unexport(Controller(4)),
            Err(PwmError::ControllerNotFound(Controller(4)))
        ));
    }

    #[test]
    fn export_and_unexport_a_controller() {
        let tmp = TempDir::new().unwrap();
        let chip = tmp.child("pwmchip0");
        fs::create_dir(&chip).unwrap();
        let export = touch(chip.join("export"));
        let unexport = touch(chip.join("unexport"));
        let mut pwm = Pwm::with_sysfs_root(tmp.path().to_owned());

        pwm.export(Controller(0)).unwrap();
        assert_eq!(fs::read_to_string(&export).unwrap(), "1");

        pwm.unexport(Controller(0)).unwrap();
        assert_eq!(fs::read_to_string(&unexport).unwrap(), "1");
    }

    fn touch(path: PathBuf) -> PathBuf {
        fs::write(&path, b"").unwrap();
        path
    }
}
