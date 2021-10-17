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

    /// Export a PWM controller, which enables access to its channels.
    #[instrument]
    pub fn export(&mut self, controller: Controller) -> Result<(), PwmError> {
        // Exporting an already exported controller is a no-op, so we don't need
        // to check whether the controller is already exported.
        let path = self
            .sysfs_root
            .join(format!("pwmchip{}/export", controller.0));
        if !path.exists() {
            return Err(PwmError::ControllerNotFound(controller.clone()));
        }

        debug!("writing to {:?}", &path);
        fs::write(&path, "1").map_err(|e| PwmError::Sysfs(Access::Write(path), e))
    }

    /// Unexport a PWM controller, which disables access to its channels.
    #[instrument]
    pub fn unexport(&mut self, controller: Controller) -> Result<(), PwmError> {
        // Un-exporting an already un-exported controller is a no-op, so we
        // don't need to check whether the controller is actually exported.
        let path = self
            .sysfs_root
            .join(format!("pwmchip{}/unexport", controller.0));
        if !path.exists() {
            return Err(PwmError::ControllerNotFound(controller.clone()));
        }

        fs::write(&path, "1").map_err(|e| PwmError::Sysfs(Access::Write(path), e))
    }

    /// Enable a channel.
    #[instrument]
    pub fn enable(&mut self, controller: Controller, channel: Channel) -> Result<(), PwmError> {
        self.update_enable_file(controller, channel, "1")
    }

    /// Disable a channel.
    #[instrument]
    pub fn disable(&mut self, controller: Controller, channel: Channel) -> Result<(), PwmError> {
        self.update_enable_file(controller, channel, "0")
    }

    fn update_enable_file(
        &mut self,
        controller: Controller,
        channel: Channel,
        value: &str,
    ) -> Result<(), PwmError> {
        // Enabling/disabling an already enabled/disabled channel is a no-op, so
        // we don't need to check whether the channel is already
        // enabled/disabled.

        let chip_dir = self.sysfs_root.join(format!("pwmchip{}", controller.0));
        if !chip_dir.exists() {
            return Err(PwmError::ControllerNotFound(controller));
        }

        let n_pwm = read_npwm_file(&chip_dir)?;
        if channel.0 >= n_pwm {
            return Err(PwmError::ChannelNotFound(controller, channel));
        }

        let enable_file = chip_dir.join(format!("pwm{}/enable", channel.0));
        if !enable_file.exists() {
            return Err(PwmError::NotExported(controller));
        }

        debug!("writing to {:?}", &enable_file);
        fs::write(&enable_file, value).map_err(|e| PwmError::Sysfs(Access::Write(enable_file), e))
    }

    /// The total period of the PWM signal (read/write). Value is in nanoseconds
    /// and is the sum of the active and inactive time of the PWM.
    #[instrument]
    pub fn set_period(
        &mut self,
        controller: Controller,
        channel: Channel,
        period: Duration,
    ) -> Result<(), PwmError> {
        let chip_dir = self.sysfs_root.join(format!("pwmchip{}", controller.0));
        if !chip_dir.exists() {
            return Err(PwmError::ControllerNotFound(controller));
        }

        let n_pwm = read_npwm_file(&chip_dir)?;
        if channel.0 >= n_pwm {
            return Err(PwmError::ChannelNotFound(controller, channel));
        }

        let period_file = chip_dir.join(format!("pwm{}/period", channel.0));
        let duty_cycle_file = chip_dir.join(format!("pwm{}/duty_cycle", channel.0));
        if !period_file.exists() || !duty_cycle_file.exists() {
            return Err(PwmError::NotExported(controller));
        }

        let duty_cycle = fs::read_to_string(&duty_cycle_file)
            .map_err(|e| PwmError::Sysfs(Access::Read(duty_cycle_file), e))?
            .parse::<u64>()
            .map(|ns| Duration::from_nanos(ns))
            .expect("duty cycle file expected to contain a number");

        if duty_cycle >= period {
            return Err(PwmError::DutyCycleNotLessThanPeriod);
        }

        debug!("writing to {:?}", &period_file);
        fs::write(&period_file, period.as_nanos().to_string())
            .map_err(|e| PwmError::Sysfs(Access::Write(period_file), e))
    }

    /// The active time of the PWM signal (read/write). Value is in nanoseconds
    /// and must be less than the period.
    #[instrument]
    pub fn set_duty_cycle(
        &mut self,
        controller: Controller,
        channel: Channel,
        duty_cycle: Duration,
    ) -> Result<(), PwmError> {
        let chip_dir = self.sysfs_root.join(format!("pwmchip{}", controller.0));
        if !chip_dir.exists() {
            return Err(PwmError::ControllerNotFound(controller));
        }

        let n_pwm = read_npwm_file(&chip_dir)?;
        if channel.0 >= n_pwm {
            return Err(PwmError::ChannelNotFound(controller, channel));
        }

        let period_file = chip_dir.join(format!("pwm{}/period", channel.0));
        let duty_cycle_file = chip_dir.join(format!("pwm{}/duty_cycle", channel.0));
        if !period_file.exists() || !duty_cycle_file.exists() {
            return Err(PwmError::NotExported(controller));
        }

        let period = fs::read_to_string(&period_file)
            .map_err(|e| PwmError::Sysfs(Access::Read(period_file), e))?
            .parse::<u64>()
            .map(|ns| Duration::from_nanos(ns))
            .expect("period file expected to contain a number");

        if duty_cycle >= period {
            return Err(PwmError::DutyCycleNotLessThanPeriod);
        }

        debug!("writing to {:?}", &duty_cycle_file);
        fs::write(&duty_cycle_file, duty_cycle.as_nanos().to_string())
            .map_err(|e| PwmError::Sysfs(Access::Write(duty_cycle_file), e))
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
    ) -> Result<(), PwmError> {
        let chip_dir = self.sysfs_root.join(format!("pwmchip{}", controller.0));
        if !chip_dir.exists() {
            return Err(PwmError::ControllerNotFound(controller));
        }

        let n_pwm = read_npwm_file(&chip_dir)?;
        if channel.0 >= n_pwm {
            return Err(PwmError::ChannelNotFound(controller, channel));
        }

        let polarity_file = chip_dir.join(format!("pwm{}/polarity", channel.0));
        let enable_file = chip_dir.join(format!("pwm{}/enable", channel.0));
        if !polarity_file.exists() || !enable_file.exists() {
            return Err(PwmError::NotExported(controller));
        }

        // setting polarity is only allowed if channel is disabled:
        let is_enabled = fs::read_to_string(&enable_file)
            .map_err(|e| PwmError::Sysfs(Access::Read(enable_file), e))
            .and_then(parse_bool)?;

        if is_enabled {
            return Err(PwmError::IllegalChangeWhileEnabled("polarity"));
        }

        debug!("writing to {:?}", &polarity_file);
        fs::write(&polarity_file, polarity.to_string())
            .map_err(|e| PwmError::Sysfs(Access::Write(polarity_file), e))
    }
}

fn read_npwm_file(chip_dir: &Path) -> Result<u32, PwmError> {
    let npwm_file = chip_dir.join("npwm");
    match fs::read_to_string(&npwm_file) {
        Ok(s) => {
            let num = s
                .parse::<u32>()
                .expect("npwm expected to contain the number of channels");
            Ok(num)
        }
        Err(e) => return Err(PwmError::Sysfs(Access::Read(npwm_file), e)),
    }
}

fn parse_bool(s: String) -> Result<bool, PwmError> {
    // sysfs compatible according to http://lkml.iu.edu/hypermail/linux/kernel/1103.2/02488.html
    match s.to_lowercase().as_ref() {
        "1" | "y" | "yes" | "true" => Ok(true),
        "0" | "n" | "no" | "false" | "" => Ok(false),
        _ => Err(PwmError::NotBoolean(s)),
    }
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
