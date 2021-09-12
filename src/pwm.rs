use std::time::Duration;

use tracing::instrument;

pub trait Pwm: Sync + Send {
    fn query(&self) -> Vec<Controller>;
    fn export(&self, controller: &Controller) -> Vec<Channel>;
    fn unexport(&self, controller: &Controller);
    fn set_period(&self, channel: &Channel, period: u64);
    fn set_duty_cycle(&self, channel: &Channel, duty_cycle: u64);
    fn enable(&self, channel: &Channel);
    fn disable(&self, channel: &Channel);
}

#[derive(Debug)]
pub struct Controller {
    /// Id of the controller (a.k.a. base).
    id: u32,
    /// Number of channels this controller supports (a.k.a. npwm).
    n_channels: u32,
}

#[derive(Debug)]
pub struct Channel {
    // ID of the controller this channel belongs to.
    controller_id: u32,
    /// The total period of the PWM signal (read/write).
    /// Value is the sum of the active and inactive time of the PWM.
    /// (sysfs expects this in nanoseconds.)
    period: Duration,
    /// The active time of the PWM signal (read/write).
    /// Must be less than the period.
    /// (sysfs expects this in nanoseconds.)
    duty_cycle: Duration,
}

impl Channel {
    fn new(controller_id: u32) -> Self {
        let frequency_hz = 1000;
        let period = Duration::from_secs(1 / frequency_hz);
        let duty_cycle = Duration::from_secs((1 / frequency_hz) / 2);
        Self {
            controller_id,
            period,
            duty_cycle,
        }
    }
}

#[derive(Debug)]
pub struct PwmDummy;
impl PwmDummy {
    pub fn new() -> Self {
        Self {}
    }
}
impl Pwm for PwmDummy {
    #[instrument]
    fn query(&self) -> Vec<Controller> {
        vec![Controller {
            id: 0,
            n_channels: 1,
        }]
    }

    #[instrument]
    fn export(&self, controller: &Controller) -> Vec<Channel> {
        let channel = Channel::new(controller.id);
        vec![channel]
    }

    #[instrument]
    fn unexport(&self, _controller: &Controller) {}

    #[instrument]
    fn set_period(&self, _channel: &Channel, period: u64) {}

    #[instrument]
    fn set_duty_cycle(&self, _channel: &Channel, duty_cycle: u64) {}

    #[instrument]
    fn enable(&self, _channel: &Channel) {}

    #[instrument]
    fn disable(&self, _channel: &Channel) {}
}
