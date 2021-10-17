use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Duration,
};

use pwmd::{dbus::StatusErrorPair, Args};
use temp_dir::TempDir;

#[test]
fn happy_flow() -> anyhow::Result<()> {
    // std::env::set_var("RUST_BACKTRACE", "full");
    pwmd::setup().unwrap();

    // fake /sys/class/pwm directory:
    let tmpdir = TempDir::new().unwrap();
    let sysfs_root = Some(tmpdir.path().to_owned());

    let (tx, rx) = channel();
    let dbus_thread = std::thread::spawn(move || {
        let args = Args { sysfs_root };
        pwmd::dbus::listen(args, || tx.send(()).unwrap()).unwrap();
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let connection = zbus::Connection::new_session()?;
    let destination = Some("com.kevinbader.pwmd");
    let path = "/pwm1";
    let iface = Some("com.kevinbader.pwmd.pwm1");

    //
    // Export pwmchip0:
    //

    // fake controller pwmchip0:
    let chip_dir = tmpdir.child("pwmchip0");
    fs::create_dir(&chip_dir).unwrap();
    let npwm_file = chip_dir.join("npwm");
    fs::write(&npwm_file, b"1").unwrap();
    let export_file = touch(chip_dir.join("export"));
    let unexport_file = touch(chip_dir.join("unexport"));

    // export the chip:
    let res = connection.call_method(destination, path, iface, "Export", &(0u32,))?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    assert_eq!(error, "");
    assert_eq!(status, 200);
    check_file(&export_file, "1");

    //
    // Enable pwmchip0/pwm0:
    //

    // Fake controller pwmchip0:
    let channel_dir = chip_dir.join("pwm0");
    fs::create_dir(&channel_dir).unwrap();
    let enable_file = touch(channel_dir.join("enable"));
    let res = connection.call_method(destination, path, iface, "Enable", &(0u32, 0u32))?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    assert_eq!(error, "");
    assert_eq!(status, 200);
    check_file(&enable_file, "1");

    //
    // Disable pwmchip0/pwm0:
    //

    let res = connection.call_method(destination, path, iface, "Disable", &(0u32, 0u32))?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    assert_eq!(error, "");
    assert_eq!(status, 200);
    check_file(&enable_file, "0");

    //
    // Unexport pwmchip0:
    //

    let res = connection.call_method(destination, path, iface, "Unexport", &(0u32,))?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    assert_eq!(error, "");
    assert_eq!(status, 200);
    check_file(&unexport_file, "1");

    let res = connection.call_method(destination, path, iface, "Quit", &())?;
    let (status, error): StatusErrorPair = res.body().unwrap();
    assert_eq!(status, 200);
    assert!(error.is_empty());

    dbus_thread.join().unwrap();
    Ok(())
}

fn touch(path: PathBuf) -> PathBuf {
    fs::write(&path, b"").unwrap();
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
