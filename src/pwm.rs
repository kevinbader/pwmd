use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::{debug, instrument};

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
}

#[derive(Debug)]
pub enum Access {
    Read(PathBuf),
    Write(PathBuf),
}

#[derive(Debug)]
pub struct Pwm {
    sysfs_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Controller(pub u32);

#[derive(Debug, Clone)]
pub struct Channel(pub u32);

impl Pwm {
    pub fn new() -> Self {
        Self::with_sysfs_root(PathBuf::from("/sys/class/pwm"))
    }

    pub fn with_sysfs_root(sysfs_root: PathBuf) -> Self {
        if !sysfs_root.exists() {
            panic!("sysfs root does not exist: {:?}", sysfs_root);
        }
        Self { sysfs_root }
    }

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

    #[instrument]
    pub fn enable(&mut self, controller: Controller, channel: Channel) -> Result<(), PwmError> {
        self.update_enable_file(controller, channel, "1")
    }

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
