use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Duration,
};

use pwmd::{dbus::StatusErrorPair, Args};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use temp_dir::TempDir;

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
            dbus_service_name: dbus_service_name2,
            sysfs_root,
        };
        pwmd::dbus::listen(args, || tx.send(()).unwrap()).unwrap();
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = zbus::Connection::new_session()?;
    let destination = Some(dbus_service_name.as_ref());
    let path = "/pwm1";
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

    // export the chip:
    let res = connection.call_method(destination, path, iface, "Export", &(0u32,))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&export_file, "1");

    // fake channel pwm0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let enable_file = touch(channel_dir.join("enable"));
    let period_file = write(channel_dir.join("period"), "100");
    let duty_cycle_file = write(channel_dir.join("duty_cycle"), "70");
    let polarity_file = write(channel_dir.join("polarity"), "normal");

    //
    // Change properties
    //

    let res = connection.call_method(
        destination,
        path,
        iface,
        "SetPeriodNs",
        &(0u32, 0u32, 1000u64),
    )?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&period_file, "1000");

    let res = connection.call_method(
        destination,
        path,
        iface,
        "SetDutyCycleNs",
        &(0u32, 0u32, 700u64),
    )?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&duty_cycle_file, "700");

    let res = connection.call_method(
        destination,
        path,
        iface,
        "SetPolarity",
        &(0u32, 0u32, "inversed"),
    )?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&polarity_file, "inversed");

    //
    // Enable pwmchip0/pwm0:
    //

    let res = connection.call_method(destination, path, iface, "Enable", &(0u32, 0u32))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&enable_file, "1");

    //
    // Disable pwmchip0/pwm0:
    //

    let res = connection.call_method(destination, path, iface, "Disable", &(0u32, 0u32))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&enable_file, "0");

    //
    // Unexport pwmchip0:
    //

    let res = connection.call_method(destination, path, iface, "Unexport", &(0u32,))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(&res, (200, s) if s == ""), "got {:?}", &res);
    check_file(&unexport_file, "1");

    let res = connection.call_method(destination, path, iface, "Quit", &())?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(res, (200, s) if s == ""));

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
            dbus_service_name: dbus_service_name2,
            sysfs_root,
        };
        pwmd::dbus::listen(args, || tx.send(()).unwrap()).unwrap();
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = zbus::Connection::new_session()?;
    let destination = Some(dbus_service_name.as_ref());
    let path = "/pwm1";
    let iface = Some("com.kevinbader.pwmd.pwm1");

    // fake controller pwmchip0:
    let chip_dir = tmpdir.child("pwmchip0");
    fs::create_dir(&chip_dir).unwrap();
    let _ = write(chip_dir.join("npwm"), "1");
    let _ = touch(chip_dir.join("export"));

    // export the chip:
    let res = connection.call_method(destination, path, iface, "Export", &(0u32,))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(res, (200, s) if s == ""));

    // fake channel pwm0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let _ = touch(channel_dir.join("enable"));
    let period_file = write(channel_dir.join("period"), "100");
    let duty_cycle_file = write(channel_dir.join("duty_cycle"), "70");

    // set duty_cycle to the value of period:
    let res = connection.call_method(
        destination,
        path,
        iface,
        "SetDutyCycleNs",
        &(0u32, 0u32, 100u64),
    )?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    let error = error.to_lowercase();
    assert!(
        error.contains("less than"),
        "expected an error about duty cycle not being less than period, but got this: {}",
        error
    );
    assert_eq!(status, 400);
    // still the old value for duty cycle:
    check_file(&duty_cycle_file, "70");

    // now we try setting the period to the same value as duty cycle, which should also fail:
    let res = connection.call_method(
        destination,
        path,
        iface,
        "SetPeriodNs",
        &(0u32, 0u32, 70u64),
    )?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    let error = error.to_lowercase();
    assert!(
        error.contains("less than"),
        "expected an error about duty cycle not being less than period, but got this: {}",
        error
    );
    assert_eq!(status, 400);
    // still the old value for period:
    check_file(&period_file, "100");

    // quit:
    let res = connection.call_method(destination, path, iface, "Quit", &())?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(res, (200, s) if s == ""));

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
            dbus_service_name: dbus_service_name2,
            sysfs_root,
        };
        pwmd::dbus::listen(args, || tx.send(()).unwrap()).unwrap();
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = zbus::Connection::new_session()?;
    let destination = Some(dbus_service_name.as_ref());
    let path = "/pwm1";
    let iface = Some("com.kevinbader.pwmd.pwm1");

    // fake controller pwmchip0:
    let chip_dir = tmpdir.child("pwmchip0");
    fs::create_dir(&chip_dir).unwrap();
    let _ = write(chip_dir.join("npwm"), "1");
    let _ = touch(chip_dir.join("export"));

    // export the chip:
    let res = connection.call_method(destination, path, iface, "Export", &(0u32,))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(res, (200, s) if s == ""));

    // fake channel pwm0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let _ = touch(channel_dir.join("enable"));
    let polarity_file = write(channel_dir.join("polarity"), "normal");

    // enable pwmchip0/pwm0:
    let res = connection.call_method(destination, path, iface, "Enable", &(0u32, 0u32))?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(res, (200, s) if s == ""));

    // set polarity - this should fail:
    let res = connection.call_method(
        destination,
        path,
        iface,
        "SetPolarity",
        &(0u32, 0u32, "inversed"),
    )?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    let error = error.to_lowercase();
    assert!(
        error.contains("enabled") || error.contains("disabled"),
        "expected an error about channel having to be disabled for this to work, but got this: {}",
        error
    );
    assert_eq!(status, 400);
    check_file(&polarity_file, "normal");

    // quit:
    let res = connection.call_method(destination, path, iface, "Quit", &())?;
    let res: StatusErrorPair = res.body().unwrap();
    assert!(matches!(res, (200, s) if s == ""));

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
