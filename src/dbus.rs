use std::{cell::RefCell, convert::TryInto, rc::Rc, time::Duration};

use enumflags2::BitFlags;
use tracing::{debug, error, info, instrument};
use zbus::dbus_interface;

use crate::{
    pwm::{Channel, Controller, Polarity, Pwm, PwmError},
    Args,
};

/// Expose DBUS interface and block on handling connections.
pub fn listen(args: Args, on_ready: impl FnOnce()) -> anyhow::Result<()> {
    debug!(?args);
    let pwm = match args.sysfs_root {
        Some(sysfs_root) => Pwm::with_sysfs_root(sysfs_root),
        None => Pwm::new(),
    };
    debug!(?pwm);
    let quit = Rc::new(RefCell::new(false));
    let pwm_api = PwmApi {
        pwm,
        quit: quit.clone(),
    };
    // let pwm_api = PwmApi::new(PwmDummy::new());
    // let led_api = LedApi::new(PwmDummy::new());

    // Connect to DBUS and register service:
    let connection = zbus::Connection::new_session()?;
    zbus::fdo::DBusProxy::new(&connection)?
        .request_name(&args.dbus_service_name, BitFlags::empty())?;

    let mut object_server = zbus::ObjectServer::new(&connection);
    object_server.at(&"/pwm1".try_into()?, pwm_api)?;
    // object_server.at(&"/led".try_into()?, led_api)?;

    on_ready();

    // Serve clients forever.
    info!("Listening on DBUS.");
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

/// All methods return a code-error-pair. An empty string means no error.
pub type StatusErrorPair = (StatusCode, String);
/// HTTP inspired status code that indicate success, client error and server error.
pub type StatusCode = u16;

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

    #[instrument]
    pub fn set_period_ns(
        &mut self,
        controller: u32,
        channel: u32,
        period: u64,
    ) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let period = Duration::from_nanos(period);
        let res = match self.pwm.set_period(controller, channel, period) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::DutyCycleNotLessThanPeriod) => (400, e.to_string()),
            Err(e @ PwmError::ControllerNotFound(_)) | Err(e @ PwmError::ChannelNotFound(_, _)) => {
                (404, e.to_string())
            }
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }

    #[instrument]
    pub fn set_duty_cycle_ns(
        &mut self,
        controller: u32,
        channel: u32,
        duty_cycle: u64,
    ) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let duty_cycle = Duration::from_nanos(duty_cycle);
        let res = match self.pwm.set_duty_cycle(controller, channel, duty_cycle) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::DutyCycleNotLessThanPeriod) => (400, e.to_string()),
            Err(e @ PwmError::ControllerNotFound(_)) | Err(e @ PwmError::ChannelNotFound(_, _)) => {
                (404, e.to_string())
            }
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }

    #[instrument]
    pub fn set_polarity(
        &mut self,
        controller: u32,
        channel: u32,
        polarity: String,
    ) -> zbus::Result<StatusErrorPair> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let polarity = match polarity.parse::<Polarity>() {
            Ok(p) => p,
            Err(e @ PwmError::InvalidPolarity) => {
                let res = (400, e.to_string());
                debug!("{:?}", res);
                return Ok(res);
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        };
        let res = match self.pwm.set_polarity(controller, channel, polarity) {
            Ok(()) => (200, "".to_owned()),
            Err(e @ PwmError::IllegalChangeWhileEnabled("polarity")) => (400, e.to_string()),
            Err(e @ PwmError::ControllerNotFound(_)) | Err(e @ PwmError::ChannelNotFound(_, _)) => {
                (404, e.to_string())
            }
            Err(e) => (500, e.to_string()),
        };
        debug!("{:?}", res);
        Ok(res)
    }
}
