use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use zvariant::{derive::Type, Type};

pub type ControllerId = u32;
pub type ChannelId = u32;

pub trait Pwm {
    // fn query(&self) -> Vec<Controller>;
    // fn export(&mut self, controller: &Controller) -> Vec<Channel>;
    // fn unexport(&mut self, controller: &Controller);
    // fn set_period(&mut self, channel: &Channel, period: u64);
    // fn set_duty_cycle(&mut self, channel: &Channel, duty_cycle: u64);
    // fn enable(&mut self, channel: &Channel);
    // fn disable(&mut self, channel: &Channel);

    fn controllers(&mut self) -> Vec<ControllerId>;
    fn channels(&mut self, controller: ControllerId) -> Result<Vec<ChannelId>, &'static str>;

    fn export(&mut self, controller: ControllerId) -> Result<(), &'static str>;
    fn unexport(&mut self, controller: ControllerId) -> Result<(), &'static str>;

    fn set_period(
        &mut self,
        controller: ControllerId,
        channel: ChannelId,
        period: u64,
    ) -> Result<(), &'static str>;
    fn set_duty_cycle(
        &mut self,
        controller: ControllerId,
        channel: ChannelId,
        duty_cycle: u64,
    ) -> Result<(), &'static str>;

    fn enable(&mut self, controller: ControllerId, channel: ChannelId) -> Result<(), &'static str>;
    fn disable(&mut self, controller: ControllerId, channel: ChannelId)
        -> Result<(), &'static str>;
}

// #[derive(Debug, Serialize, Deserialize, Type)]
// pub struct Controller {
//     /// Id of the controller (a.k.a. base).
//     pub id: u32,
//     /// Number of channels this controller supports (a.k.a. npwm).
//     pub n_channels: u32,
// }

// #[derive(Debug, Serialize, Deserialize, Type)]
// pub struct Channel {
//     // ID of the controller this channel belongs to.
//     controller_id: u32,
//     /// The total period of the PWM signal (read/write) in nanoseconds.
//     /// Value is the sum of the active and inactive time of the PWM.
//     period: u64,
//     /// The active time of the PWM signal (read/write) in nanoseconds.
//     /// Must be less than the period.
//     duty_cycle: u64,
// }

// impl Channel {
//     fn new(controller_id: u32) -> Self {
//         let frequency_hz = 1000;
//         let period = 10_u64.pow(9) / frequency_hz;
//         let duty_cycle = period / 2;
//         Self {
//             controller_id,
//             period,
//             duty_cycle,
//         }
//     }
// }

#[derive(Debug, Default)]
pub struct PwmDummy {
    exported: bool,
    enabled: bool,
    period: u64,
    duty_cycle: u64,
}
impl PwmDummy {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Pwm for PwmDummy {
    #[instrument]
    fn controllers(&mut self) -> Vec<ControllerId> {
        debug!("controllers");
        vec![0]
    }

    #[instrument]
    fn channels(&mut self, controller: ControllerId) -> Result<Vec<ChannelId>, &'static str> {
        debug!("channels");
        match controller {
            0 => Ok(vec![0, 1]),
            _ => Err("unknown controller"),
        }
    }

    #[instrument]
    fn export(&mut self, controller: ControllerId) -> Result<(), &'static str> {
        debug!("export");
        match (controller, self.exported) {
            (0, false) => {
                self.exported = true;
                Ok(())
            }
            (0, true) => Err("controller already exported"),
            _ => Err("unknown controller"),
        }
    }

    #[instrument]
    fn unexport(&mut self, controller: ControllerId) -> Result<(), &'static str> {
        debug!("unexport");
        match (controller, self.exported) {
            (0, true) => {
                self.exported = false;
                Ok(())
            }
            (0, false) => Err("controller not exported"),
            _ => Err("unknown controller"),
        }
    }

    #[instrument]
    fn set_period(
        &mut self,
        controller: ControllerId,
        channel: ChannelId,
        period: u64,
    ) -> Result<(), &'static str> {
        debug!("set_period");
        match controller {
            0 => {
                self.period = period;
                Ok(())
            }
            _ => Err("unknown controller"),
        }
    }

    #[instrument]
    fn set_duty_cycle(
        &mut self,
        controller: ControllerId,
        channel: ChannelId,
        duty_cycle: u64,
    ) -> Result<(), &'static str> {
        debug!("set_duty_cycle");
        match controller {
            0 if duty_cycle > self.period => Err("duty cycle greater than period"),
            0 => {
                self.duty_cycle = duty_cycle;
                Ok(())
            }
            _ => Err("unknown controller"),
        }
    }

    #[instrument]
    fn enable(&mut self, controller: ControllerId, channel: ChannelId) -> Result<(), &'static str> {
        debug!("enable");
        match (controller, self.enabled) {
            (0, false) => {
                self.enabled = true;
                Ok(())
            }
            // Enabling an enabled PWM is a no-op.
            (0, true) => Ok(()),
            _ => Err("unknown controller"),
        }
    }

    #[instrument]
    fn disable(
        &mut self,
        controller: ControllerId,
        channel: ChannelId,
    ) -> Result<(), &'static str> {
        debug!("disable");
        match (controller, self.enabled) {
            (0, true) => {
                self.enabled = false;
                Ok(())
            }
            // Disabling a disabled PWM is a no-op.
            (0, false) => Ok(()),
            _ => Err("unknown controller"),
        }
    }
}
