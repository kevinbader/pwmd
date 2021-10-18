use std::{convert::TryInto, sync::Arc, time::Duration};

use tokio::sync::Notify;
use tracing::{debug, info, instrument, warn};
use zbus::{dbus_interface, fdo, names::WellKnownName, Connection, ConnectionBuilder};

use crate::args::{Args, Bus};
use crate::pwm::{Channel, Controller, Polarity, Pwm};

/// Expose DBUS interface and block on handling connections.
pub async fn listen(args: Args, on_ready: impl FnOnce()) -> anyhow::Result<()> {
    debug!(?args);
    let pwm = match args.sysfs_root {
        Some(sysfs_root) => Pwm::with_sysfs_root(sysfs_root),
        None => Pwm::new(),
    };
    debug!(?pwm);
    let pwm_api = PwmApi {
        pwm,
        done: Arc::new(Notify::new()),
    };
    let done = pwm_api.done.clone();

    let connection: Connection = match args.bus {
        Bus::Session => ConnectionBuilder::session()?.build().await?,
        Bus::System => ConnectionBuilder::system()?.build().await?,
    };
    connection
        .object_server_mut()
        .await
        .at("/com/kevinbader/pwmd/pwm1", pwm_api)?;
    let name: WellKnownName = args
        .dbus_service_name
        .as_str()
        .try_into()
        .expect("invalid dbus name");
    connection.request_name(name).await?;

    on_ready();

    done.notified().await;

    Ok(())
}

#[derive(Debug)]
struct PwmApi {
    pwm: Pwm,
    done: Arc<Notify>,
}

#[dbus_interface(name = "com.kevinbader.pwmd.pwm1")]
impl PwmApi {
    #[instrument]
    async fn quit(&mut self) {
        info!("quit");
        self.done.notify_one();
    }

    #[instrument]
    async fn npwm(&self, controller: u32) -> fdo::Result<u32> {
        let controller = Controller(controller);
        self.pwm.npwm(&controller).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    #[instrument]
    async fn is_exported(&self, controller: u32) -> fdo::Result<bool> {
        let controller = Controller(controller);
        self.pwm.is_exported(&controller).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    #[instrument]
    async fn export(&mut self, controller: u32) -> fdo::Result<()> {
        let controller = Controller(controller);
        self.pwm.export(controller).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    #[instrument]
    async fn unexport(&mut self, controller: u32) -> fdo::Result<()> {
        let controller = Controller(controller);
        self.pwm.unexport(controller).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    async fn is_enabled(&self, controller: u32, channel: u32) -> fdo::Result<bool> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        self.pwm.is_enabled(&controller, &channel).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    #[instrument]
    async fn enable(&mut self, controller: u32, channel: u32) -> fdo::Result<()> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        self.pwm.enable(controller, channel).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    #[instrument]
    async fn disable(&mut self, controller: u32, channel: u32) -> fdo::Result<()> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        self.pwm.disable(controller, channel).map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::Failed(e.to_string())
        })
    }

    #[instrument]
    async fn set_period_ns(
        &mut self,
        controller: u32,
        channel: u32,
        period: u64,
    ) -> fdo::Result<()> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let period = Duration::from_nanos(period);
        self.pwm
            .set_period(controller, channel, period)
            .map_err(|e| {
                warn!("{:?}", e);
                fdo::Error::Failed(e.to_string())
            })
    }

    #[instrument]
    async fn set_duty_cycle_ns(
        &mut self,
        controller: u32,
        channel: u32,
        duty_cycle: u64,
    ) -> fdo::Result<()> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let duty_cycle = Duration::from_nanos(duty_cycle);
        self.pwm
            .set_duty_cycle(controller, channel, duty_cycle)
            .map_err(|e| {
                warn!("{:?}", e);
                fdo::Error::Failed(e.to_string())
            })
    }

    #[instrument]
    async fn set_polarity(
        &mut self,
        controller: u32,
        channel: u32,
        polarity: String,
    ) -> fdo::Result<()> {
        let controller = Controller(controller);
        let channel = Channel(channel);
        let polarity = polarity.parse::<Polarity>().map_err(|e| {
            warn!("{:?}", e);
            fdo::Error::InvalidArgs(e.to_string())
        })?;
        self.pwm
            .set_polarity(controller, channel, polarity)
            .map_err(|e| {
                warn!("{:?}", e);
                fdo::Error::Failed(e.to_string())
            })
    }
}
