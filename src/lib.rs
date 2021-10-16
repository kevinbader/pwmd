// mod pwm;
mod pwm2;

use std::{convert::TryInto, path::PathBuf};

use enumflags2::BitFlags;
use pwm2::Pwm;
use tracing::{debug, error, info, instrument};
use zbus::dbus_interface;

const DBUS_SERVICE_NAME: &'static str = "com.kevinbader.pwmd";

use crate::pwm2::Controller;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "pwmd", about = "Exposes PWM chips to DBUS.")]
pub struct Opt {
    /// For testing: path to the sysfs pwm class directory.
    #[structopt(long, parse(from_os_str), env)]
    sysfs_root: Option<PathBuf>,
}

// use zbus_macros::DBusError;
// #[derive(DBusError, Debug)]
// #[dbus_error(prefix = "com.kevinbader.pwmd")]
// enum MyError {
//     ZBus(zbus::Error),
//     PwmError(String),
// }

/// Low-level interface to the PWM controllers as exposed by Linux through sysfs.
// #[derive(Debug)]
// struct PwmApi<T>
// where
//     T: Pwm,
// {
//     pwm: T,
// }

// impl<T> PwmApi<T>
// where
//     T: Pwm,
// {
//     fn new(pwm: T) -> Self {
//         Self { pwm }
//     }
// }

// #[dbus_interface(name = "com.kevinbader.pwmd.pwm")]
// impl<T> PwmApi<T>
// where
//     T: Pwm + core::fmt::Debug + 'static,
// {
//     fn controllers(&mut self) -> Vec<ControllerId> {
//         self.pwm.controllers()
//     }

//     fn channels(&mut self, controller: ControllerId) -> Result<Vec<ChannelId>, Error> {
//         self.pwm
//             .channels(controller)
//             .map_err(|e| Error::MyError(e.into()))
//     }

//     fn export(&mut self, controller: ControllerId) -> Result<(), Error> {
//         self.pwm
//             .export(controller)
//             .map_err(|e| Error::MyError(e.into()))
//     }

//     fn unexport(&mut self, controller: ControllerId) -> Result<(), Error> {
//         self.pwm
//             .unexport(controller)
//             .map_err(|e| Error::MyError(e.into()))
//     }

//     fn set_period(
//         &mut self,
//         controller: ControllerId,
//         channel: ChannelId,
//         period: u64,
//     ) -> Result<(), Error> {
//         self.pwm
//             .set_period(controller, channel, period)
//             .map_err(|e| Error::MyError(e.into()))
//     }

//     fn set_duty_cycle(
//         &mut self,
//         controller: ControllerId,
//         channel: ChannelId,
//         duty_cycle: u64,
//     ) -> Result<(), Error> {
//         self.pwm
//             .set_duty_cycle(controller, channel, duty_cycle)
//             .map_err(|e| Error::MyError(e.into()))
//     }

//     fn enable(&mut self, controller: ControllerId, channel: ChannelId) -> Result<(), Error> {
//         self.pwm
//             .enable(controller, channel)
//             .map_err(|e| Error::MyError(e.into()))
//     }

//     fn disable(&mut self, controller: ControllerId, channel: ChannelId) -> Result<(), Error> {
//         self.pwm
//             .disable(controller, channel)
//             .map_err(|e| Error::MyError(e.into()))
//     }
// }

// /// Abstraction for controlling LEDs using a PWM controller.
// struct LedApi<T>
// where
//     T: Pwm,
// {
//     pwm: T,
// }

// impl<T> LedApi<T>
// where
//     T: Pwm,
// {
//     fn new(pwm: T) -> Self {
//         Self { pwm }
//     }
// }

// #[dbus_interface(name = "com.kevinbader.pwmd.led")]
// impl<T> LedApi<T>
// where
//     T: Pwm + 'static,
// {
//     fn bar(&mut self) -> zbus::fdo::Result<String> {
//         Ok(format!("this is baaaaaaaaaaaaarta!"))
//         // Err(zbus::fdo::Error::Failed("oh noes haha".to_string()))
//     }
// }

#[derive(Debug)]
struct PwmApi {
    pwm: Pwm,
}

#[dbus_interface(name = "com.kevinbader.pwmd.pwm")]
impl PwmApi {
    #[instrument]
    pub fn export(&mut self, controller: u32) -> zbus::Result<String> {
        let controller = Controller(controller);
        let res = match self.pwm.export(controller) {
            Ok(()) => "OK".to_string(),
            Err(e) => e.to_string(),
        };
        debug!("{:?}", res);
        Ok(res)
    }

    #[instrument]
    pub fn unexport(&mut self, controller: u32) -> zbus::Result<String> {
        let controller = Controller(controller);
        let res = match self.pwm.unexport(controller) {
            Ok(()) => "OK".to_string(),
            Err(e) => e.to_string(),
        };
        debug!("{:?}", res);
        Ok(res)
    }
}

pub fn register_on_dbus(opts: Opt) -> anyhow::Result<()> {
    let pwm = match opts.sysfs_root {
        Some(sysfs_root) => Pwm::with_sysfs_root(sysfs_root),
        None => Pwm::new(),
    };
    debug!("{:?}", pwm);
    let pwm_api = PwmApi { pwm };
    // let pwm_api = PwmApi::new(PwmDummy::new());
    // let led_api = LedApi::new(PwmDummy::new());

    // Connect to DBUS and register service:
    let connection = zbus::Connection::new_session()?;
    zbus::fdo::DBusProxy::new(&connection)?.request_name(DBUS_SERVICE_NAME, BitFlags::empty())?;

    let mut object_server = zbus::ObjectServer::new(&connection);
    object_server.at(&"/".try_into()?, pwm_api)?;
    // object_server.at(&"/led".try_into()?, led_api)?;

    // Serve clients forever.
    info!(
        service = DBUS_SERVICE_NAME,
        path = "/",
        "Listening on DBUS."
    );
    loop {
        match object_server.try_handle_next() {
            Ok(Some(msg)) => debug!("received {:?}", msg),
            Ok(None) => {}
            Err(error) => error!("{}", error),
        }
    }
}
