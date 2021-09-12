mod pwm;

use std::sync::{Arc, Mutex};

use dbus::blocking::Connection;
use dbus_crossroads::{Crossroads, IfaceBuilder, IfaceToken};
use pwm::{Controller, Pwm};
use tracing::{debug, info};

use crate::pwm::PwmDummy;

const DBUS_SERVICE_NAME: &'static str = "com.kevinbader.pwmd";
const DBUS_GREETER_INTERFACE_NAME: &'static str = "com.kevinbader.pwmd.Greeter";
const DBUS_PWM_INTERFACE_NAME: &'static str = "com.kevinbader.pwmd.pwm";

type AppState<T> = Arc<Mutex<InnerAppState<T>>>;
struct InnerAppState<T: Pwm> {
    called_count: u32,
    pwm: T,
}

pub fn register_on_dbus() -> anyhow::Result<()> {
    // Connect to DBUS and register service:
    let dbus_client = Connection::new_session()?;
    dbus_client.request_name(DBUS_SERVICE_NAME, false, false, true)?;
    info!("Now known to DBUS as {}", DBUS_SERVICE_NAME);

    // Crossroads as a framework takes care of setting up introspection etc.;
    // it makes writing the handler function below easier.
    let mut cr = Crossroads::new();

    // We define a global application state and expose it through multiple interface on DBUS.
    let pwm = PwmDummy::new();
    let state = Arc::new(Mutex::new(InnerAppState {
        called_count: 0,
        pwm,
    }));
    let greeter_iface = add_greeter_dbus_interface(&mut cr);
    let pwm_iface = add_pwm_dbus_interface(&mut cr);
    // Offer both interfaces on the same path:
    cr.insert("/", &[greeter_iface, pwm_iface], state);
    info!("Exposed DBUS interfaces at path \"/\"");

    // Serve clients forever.
    info!("Ready to accept DBUS method calls.");
    cr.serve(&dbus_client)?;
    unreachable!()

    // // Make a "proxy object" that contains the destination and path of our method call.
    // let proxy = Proxy::new("org.freedesktop.DBus", "/", Duration::from_secs(5), conn);

    // // Call the method and await a response. See the argument guide for details about
    // // how to send and receive arguments to the method.
    // let (names,): (Vec<String>,) = proxy
    //     .method_call("org.freedesktop.DBus", "ListNames", ())
    //     .await?;

    // // Print all the names.
    // for name in names {
    //     println!("{}", name);
    // }

    // std::thread::sleep(Duration::from_secs(5));

    // Ok(())
}

fn add_greeter_dbus_interface<T: Pwm + 'static>(
    cr: &mut Crossroads,
) -> IfaceToken<Arc<Mutex<InnerAppState<T>>>> {
    cr.register(
        DBUS_GREETER_INTERFACE_NAME,
        |builder: &mut IfaceBuilder<AppState<T>>| {
            builder.method(
                "Hello",
                ("name",),
                ("reply",),
                |_ctx, state: &mut AppState<T>, (name,): (String,)| {
                    debug!("Incoming hello call from {}!", name);
                    let mut state = state.lock().unwrap();
                    state.called_count += 1;
                    let reply = format!(
                        "Hello {}! This API has been used {} times.",
                        name, state.called_count
                    );
                    Ok((reply,))
                },
            );
        },
    )

    // cr.insert(DBUS_GREETER_PATH, &[dbus_interface], state);
    // info!(
    //     path = DBUS_GREETER_PATH,
    //     interface = DBUS_GREETER_INTERFACE_NAME,
    //     "Exposed"
    // );
}

fn add_pwm_dbus_interface<T: Pwm + 'static>(
    cr: &mut Crossroads,
) -> IfaceToken<Arc<Mutex<InnerAppState<T>>>> {
    cr.register(
        DBUS_PWM_INTERFACE_NAME,
        |builder: &mut IfaceBuilder<AppState<T>>| {
            builder.method(
                "query",
                (),
                ("reply",),
                |_ctx, state: &mut AppState<T>, _args: ()| {
                    let state = state.lock().unwrap();
                    let reply = format!("PWM QUERY: {:?}", state.pwm.query());
                    Ok((reply,))
                },
            );
            builder.method(
                "export",
                ("controller",),
                ("channels",),
                |_ctx, state: &mut AppState<T>, (controller,): (&Controller,)| {
                    let state = state.lock().unwrap();
                    debug!("would export {:?}", controller);
                    let channels = state.pwm.export(controller);
                    Ok((channels,))
                },
            );
        },
    )
}
