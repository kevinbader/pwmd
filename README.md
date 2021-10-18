# pwmd

[![Crates.io](https://img.shields.io/crates/v/pwmd?style=flat-square)](https://crates.io/crates/pwmd)
[![docs.rs](https://img.shields.io/docsrs/pwmd?style=flat-square)](https://docs.rs/pwmd/)

pwmd exposes the Linux' sysfs PWM interface to DBUS.

## Why

PWM controllers are often used for LEDs. Playing around with LEDs is fun and it's also super easy to control them, thanks to a simple sysfs based API the Linux kernel exposes. Only drawback: your hacky script needs root privileges to use it.

With pwmd you can use DBUS to control LEDs without root privileges. Under the hood, pwmd uses the sysfs API - it exposes this part of sysfs to user-space via DBUS, without running the risk of scripts causing mayhem to other parts of sysfs.

## Getting started

```bash
$ cargo install --git https://github.com/kevinbader/pwmd
$ sudo pwmd
```

pwmd then exposes its API on DBUS. For example, you could export the first PWM controller using `dbus-send`:

```bash
SERVICE="com.kevinbader.pwmd"
OBJECT_PATH="/com/kevinbader/pwmd/pwm1"
INTERFACE="com.kevinbader.pwmd.pwm1"
METHOD="Export"
dbus-send --system \
  --type=method_call --print-reply \
  --dest=$SERVICE \
  $OBJECT_PATH \
  ${INTERFACE}.${METHOD} uint32:0
```

## TODOs

- [ ] CONTRIBUTORS file
- [ ] GitHub Actions pipeline setup
- [ ] systemd file
- [ ] describe how to control logging output
- [ ] high-level API specifically for LEDs
