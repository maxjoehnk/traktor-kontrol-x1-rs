use traktor_kontrol_x1::list_devices;

fn main() {
    let devices = list_devices().unwrap();

    println!("{devices:?}");
}
