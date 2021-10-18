use std::{
    convert::TryInto,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Duration,
};

use pwmd::{args::Bus, Args};
use rand::Rng;
use temp_dir::TempDir;
use zbus::{blocking::Connection, names::BusName};

#[test]
fn happy_flow() -> anyhow::Result<()> {
    // std::env::set_var("RUST_BACKTRACE", "full");
    pwmd::setup_logging().unwrap();

    // fake /sys/class/pwm directory:
    let tmpdir = TempDir::new().unwrap();
    let dbus_service_name = random_dbus_service_name();
    let dbus_service_name2 = dbus_service_name.clone();
    let sysfs_root = Some(tmpdir.path().to_owned());

    let (tx, rx) = channel();
    let dbus_thread = std::thread::spawn(move || {
        let args = Args {
            bus: Bus::Session,
            dbus_service_name: dbus_service_name2,
            sysfs_root,
        };
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            pwmd::dbus::listen(args, || tx.send(()).unwrap())
                .await
                .unwrap();
        });
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = Connection::session()?;
    let destination: BusName<'_> = dbus_service_name.as_str().try_into().unwrap();
    let destination = Some(&destination);
    let path = "/com/kevinbader/pwmd/pwm1";
    let iface = Some("com.kevinbader.pwmd.pwm1");

    //
    // Export pwmchip0:
    //

    // fake controller pwmchip0:
    let chip_dir = tmpdir.child("pwmchip0");
    fs::create_dir(&chip_dir).unwrap();
    let _npwm_file = write(chip_dir.join("npwm"), "1");
    let export_file = touch(chip_dir.join("export"));
    let unexport_file = touch(chip_dir.join("unexport"));

    // channel count is 1:
    assert_eq!(
        1,
        connection
            .call_method(destination, path, iface, "Npwm", &(0u32,))
            .and_then(|res| res.body::<u32>())?
    );

    // should be unexported before:
    assert!(!connection
        .call_method(destination, path, iface, "IsExported", &(0u32,))
        .and_then(|res| res.body::<bool>())?);

    // export the chip:
    let _ = connection.call_method(destination, path, iface, "Export", &(0u32,))?;
    check_file(&export_file, "1");

    // fake channel pwm0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let enable_file = touch(channel_dir.join("enable"));
    let period_file = write(channel_dir.join("period"), "100");
    let duty_cycle_file = write(channel_dir.join("duty_cycle"), "70");
    let polarity_file = write(channel_dir.join("polarity"), "normal");

    // should be exported afterwards:
    assert!(connection
        .call_method(destination, path, iface, "IsExported", &(0u32,))
        .and_then(|res| res.body::<bool>())?);

    //
    // Change properties
    //

    let _ = connection.call_method(
        destination,
        path,
        iface,
        "SetPeriodNs",
        &(0u32, 0u32, 1000u64),
    )?;
    check_file(&period_file, "1000");

    let _ = connection.call_method(
        destination,
        path,
        iface,
        "SetDutyCycleNs",
        &(0u32, 0u32, 700u64),
    )?;
    check_file(&duty_cycle_file, "700");

    let _ = connection.call_method(
        destination,
        path,
        iface,
        "SetPolarity",
        &(0u32, 0u32, "inversed"),
    )?;
    check_file(&polarity_file, "inversed");

    //
    // Enable pwmchip0/pwm0:
    //

    // should be disabled before:
    assert!(!connection
        .call_method(destination, path, iface, "IsEnabled", &(0u32, 0u32))
        .and_then(|res| res.body::<bool>())?);
    let _ = connection.call_method(destination, path, iface, "Enable", &(0u32, 0u32))?;
    // should be enabled afterwards:
    check_file(&enable_file, "1");
    assert!(connection
        .call_method(destination, path, iface, "IsEnabled", &(0u32, 0u32))
        .and_then(|res| res.body::<bool>())?);

    //
    // Disable pwmchip0/pwm0:
    //

    let _ = connection.call_method(destination, path, iface, "Disable", &(0u32, 0u32))?;
    check_file(&enable_file, "0");

    //
    // Unexport pwmchip0:
    //

    let _ = connection.call_method(destination, path, iface, "Unexport", &(0u32,))?;
    check_file(&unexport_file, "1");

    let _ = connection.call_method(destination, path, iface, "Quit", &())?;

    dbus_thread.join().unwrap();
    Ok(())
}

#[test]
fn test_duty_cycle_cannot_be_larger_than_period() -> anyhow::Result<()> {
    // fake /sys/class/pwm directory:
    let tmpdir = TempDir::new().unwrap();
    let dbus_service_name = random_dbus_service_name();
    let dbus_service_name2 = dbus_service_name.clone();
    let sysfs_root = Some(tmpdir.path().to_owned());

    let (tx, rx) = channel();
    let dbus_thread = std::thread::spawn(move || {
        let args = Args {
            bus: Bus::Session,
            dbus_service_name: dbus_service_name2,
            sysfs_root,
        };
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            pwmd::dbus::listen(args, || tx.send(()).unwrap())
                .await
                .unwrap();
        });
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = Connection::session()?;
    let destination: BusName<'_> = dbus_service_name.as_str().try_into().unwrap();
    let destination = Some(&destination);
    let path = "/com/kevinbader/pwmd/pwm1";
    let iface = Some("com.kevinbader.pwmd.pwm1");

    // fake controller pwmchip0:
    let chip_dir = tmpdir.child("pwmchip0");
    fs::create_dir(&chip_dir).unwrap();
    let _ = write(chip_dir.join("npwm"), "1");
    let _ = touch(chip_dir.join("export"));

    // export the chip:
    let _ = connection.call_method(destination, path, iface, "Export", &(0u32,))?;

    // fake channel pwm0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let _ = touch(channel_dir.join("enable"));
    let period_file = write(channel_dir.join("period"), "100");
    let duty_cycle_file = write(channel_dir.join("duty_cycle"), "70");

    // setting duty_cycle to the value of period should fail:
    let error = match connection.call_method(
        destination,
        path,
        iface,
        "SetDutyCycleNs",
        &(0u32, 0u32, 100u64),
    ) {
        Err(zbus::Error::MethodError(_, Some(error), _)) => error.to_lowercase(),
        x => panic!("expected MethodError, got: {:?}", x),
    };
    assert!(
        error.contains("less than"),
        "expected an error about duty cycle not being less than period, but got this: {}",
        error
    );
    // still the old value for duty cycle:
    check_file(&duty_cycle_file, "70");

    // now we try setting the period to the same value as duty cycle, which should also fail:
    let error = match connection.call_method(
        destination,
        path,
        iface,
        "SetPeriodNs",
        &(0u32, 0u32, 70u64),
    ) {
        Err(zbus::Error::MethodError(_, Some(error), _)) => error.to_lowercase(),
        x => panic!("expected MethodError, got: {:?}", x),
    };
    assert!(
        error.contains("less than"),
        "expected an error about duty cycle not being less than period, but got this: {}",
        error
    );
    // still the old value for period:
    check_file(&period_file, "100");

    // quit:
    let _ = connection.call_method(destination, path, iface, "Quit", &())?;
    dbus_thread.join().unwrap();
    Ok(())
}

#[test]
fn test_polarity_cannot_be_changed_if_channel_is_enabled() -> anyhow::Result<()> {
    // fake /sys/class/pwm directory:
    let tmpdir = TempDir::new().unwrap();
    let dbus_service_name = random_dbus_service_name();
    let dbus_service_name2 = dbus_service_name.clone();
    let sysfs_root = Some(tmpdir.path().to_owned());

    let (tx, rx) = channel();
    let dbus_thread = std::thread::spawn(move || {
        let args = Args {
            bus: Bus::Session,
            dbus_service_name: dbus_service_name2,
            sysfs_root,
        };
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            pwmd::dbus::listen(args, || tx.send(()).unwrap())
                .await
                .unwrap();
        });
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = Connection::session()?;
    let destination: BusName<'_> = dbus_service_name.as_str().try_into().unwrap();
    let destination = Some(&destination);
    let path = "/com/kevinbader/pwmd/pwm1";
    let iface = Some("com.kevinbader.pwmd.pwm1");

    // fake controller pwmchip0:
    let chip_dir = tmpdir.child("pwmchip0");
    fs::create_dir(&chip_dir).unwrap();
    let _ = write(chip_dir.join("npwm"), "1");
    let _ = touch(chip_dir.join("export"));

    // export the chip:
    let _ = connection.call_method(destination, path, iface, "Export", &(0u32,))?;

    // fake channel pwm0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let _ = touch(channel_dir.join("enable"));
    let polarity_file = write(channel_dir.join("polarity"), "normal");

    // enable pwmchip0/pwm0:
    let _ = connection.call_method(destination, path, iface, "Enable", &(0u32, 0u32))?;

    // set polarity - this should fail:
    let error = match connection.call_method(
        destination,
        path,
        iface,
        "SetPolarity",
        &(0u32, 0u32, "inversed"),
    ) {
        Err(zbus::Error::MethodError(_, Some(error), _)) => error.to_lowercase(),
        x => panic!("expected MethodError, got: {:?}", x),
    };
    assert!(
        error.contains("enabled") || error.contains("disabled"),
        "expected an error about channel having to be disabled for this to work, but got this: {}",
        error
    );
    check_file(&polarity_file, "normal");

    // quit:
    let _ = connection.call_method(destination, path, iface, "Quit", &())?;
    dbus_thread.join().unwrap();
    Ok(())
}

fn touch(path: PathBuf) -> PathBuf {
    fs::write(&path, b"").unwrap();
    path
}

fn write(path: PathBuf, content: &str) -> PathBuf {
    fs::write(&path, content).unwrap();
    path
}

fn check_file(path: &Path, expected: &str) {
    let actual = fs::read_to_string(path).unwrap();
    assert_eq!(
        actual, expected,
        "unexpected contents of {:?}: {:?}",
        path, actual
    );
}

fn random_dbus_service_name() -> String {
    let base = "com.kevinbader.pwmd.X";
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let unique: String = (0..20)
        .map(|_| {
            let i = rng.gen_range(0..CHARSET.len());
            CHARSET[i] as char
        })
        .collect();
    format!("{}{}", base, unique)
}
