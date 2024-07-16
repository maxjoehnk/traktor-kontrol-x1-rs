use traktor_kontrol_x1::{Button, list_devices};

fn main() {
    let mut devices = list_devices().unwrap();
    let device = devices.first_mut().unwrap();

    loop {
        let state = device.read_state().unwrap();

        println!("{state:#?}");

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
