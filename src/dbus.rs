use std::{cell::RefCell, convert::TryInto, rc::Rc};

use enumflags2::BitFlags;
use tracing::{debug, error, info, instrument};
use zbus::dbus_interface;

const DBUS_SERVICE_NAME: &'static str = "com.kevinbader.pwmd";

use crate::{
    pwm::{Channel, Controller, Pwm, PwmError},
    Args,
};

pub fn listen(args: Args, on_ready: impl FnOnce() -> ()) -> anyhow::Result<()> {
    let pwm = match args.sysfs_root {
        Some(sysfs_root) => Pwm::with_sysfs_root(sysfs_root),
        None => Pwm::new(),
    };
    debug!("{:?}", pwm);
    let quit = Rc::new(RefCell::new(false));
    let pwm_api = PwmApi {
        pwm,
        quit: quit.clone(),
    };
    // let pwm_api = PwmApi::new(PwmDummy::new());
    // let led_api = LedApi::new(PwmDummy::new());

    // Connect to DBUS and register service:
    let connection = zbus::Connection::new_session()?;
    zbus::fdo::DBusProxy::new(&connection)?.request_name(DBUS_SERVICE_NAME, BitFlags::empty())?;

    let mut object_server = zbus::ObjectServer::new(&connection);
    object_server.at(&"/pwm1".try_into()?, pwm_api)?;
    // object_server.at(&"/led".try_into()?, led_api)?;

    on_ready();

    // Serve clients forever.
    info!(service = DBUS_SERVICE_NAME, "Listening on DBUS.");
    loop {
        match object_server.try_handle_next() {
            Ok(Some(msg)) => debug!("received {:?}", msg),
            Ok(None) => {}
            Err(error) => error!("{}", error),
        }

        if *quit.borrow() {
            break;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct PwmApi {
    pwm: Pwm,
    quit: Rc<RefCell<bool>>,
}

pub type StatusCode = u16;
pub type StatusErrorPair = (StatusCode, String);

#[dbus_interface(name = "com.kevinbader.pwmd.pwm1")]
impl PwmApi {
    #[instrument]
    pub fn quit(&mut self) -> zbus::Result<StatusErrorPair> {
        info!("quit");
        *self.quit.borrow_mut() = true;
        Ok((200, "".into()))
    }

    #[instrument]
    pub fn export(&mut self, controller: u32) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let res = match self.pwm.export(controller) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::ControllerNotFound(_)) => (404, e.to_string()),
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }

    #[instrument]
    pub fn unexport(&mut self, controller: u32) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let res = match self.pwm.unexport(controller) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::ControllerNotFound(_)) => (404, e.to_string()),
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }

    #[instrument]
    pub fn enable(&mut self, controller: u32, channel: u32) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let res = match self.pwm.enable(controller, channel) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::ControllerNotFound(_)) | Err(e @ PwmError::ChannelNotFound(_, _)) => {
                (404, e.to_string())
            }
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }

    #[instrument]
    pub fn disable(&mut self, controller: u32, channel: u32) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let res = match self.pwm.disable(controller, channel) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::ControllerNotFound(_)) | Err(e @ PwmError::ChannelNotFound(_, _)) => {
                (404, e.to_string())
            }
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }
}
