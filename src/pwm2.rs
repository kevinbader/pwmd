use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PwmError {
    #[error("{0:?} not found")]
    ControllerNotFound(Controller),
    #[error("{0:?}/{1:?} not found")]
    ControllerChannelNotFound(Controller, Channel),
    #[error("pwm controller not 'exported'")]
    NotExported,
    #[error("failed to read from sysfs")]
    SysfsUnreadable,
    #[error("failed to write to sysfs")]
    SysfsUnwritable(#[from] std::io::Error),
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

    pub fn export(&mut self, controller: Controller) -> Result<(), PwmError> {
        // Exporting an already exported controller is a no-op, so we don't need
        // to check whether the controller is already exported.
        let path = self
            .sysfs_root
            .join(format!("pwmchip{}/export", controller.0));
        if !path.exists() {
            return Err(PwmError::ControllerNotFound(controller.clone()));
        }

        fs::write(&path, "1").map_err(|e| PwmError::SysfsUnwritable(e))
    }

    pub fn unexport(&mut self, controller: Controller) -> Result<(), PwmError> {
        // Un-exporting an already un-exported controller is a no-op, so we
        // don't need to check whether the controller is actually exported.
        let path = self
            .sysfs_root
            .join(format!("pwmchip{}/unexport", controller.0));
        if !path.exists() {
            return Err(PwmError::ControllerNotFound(controller.clone()));
        }

        fs::write(&path, "1").map_err(|e| PwmError::SysfsUnwritable(e))
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
