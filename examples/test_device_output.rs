use std::time::Duration;
use traktor_kontrol_x1::{Button, list_devices};

fn main() {
    let mut devices = list_devices().unwrap();
    let device = devices.first_mut().unwrap();

    let button = Button::Hotcue;

    let mut writer = device.writer();
    writer.set_led(button, 127);
    writer.write().unwrap();

    std::thread::sleep(Duration::from_secs(1));

    let mut writer = device.writer();
    writer.set_led(button, 64);
    writer.write().unwrap();

    std::thread::sleep(Duration::from_secs(1));

    let mut writer = device.writer();
    writer.set_led(button, 0);
    writer.write().unwrap();
}
