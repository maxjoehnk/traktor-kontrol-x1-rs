use traktor_kontrol_x1::{Button, DeckButton, FxButton, list_devices};

fn main() -> anyhow::Result<()> {
    let mut devices = list_devices()?;
    let device = devices.first_mut().unwrap();

    let buttons = vec![
        Button::Shift,
        Button::Hotcue,
        Button::FX1(FxButton::On),
        Button::FX1(FxButton::Button1),
        Button::FX1(FxButton::Button2),
        Button::FX1(FxButton::Button3),
        Button::FX2(FxButton::On),
        Button::FX2(FxButton::Button1),
        Button::FX2(FxButton::Button2),
        Button::FX2(FxButton::Button3),
        Button::DeckA(DeckButton::FX1),
        Button::DeckA(DeckButton::FX2),
        Button::DeckA(DeckButton::In),
        Button::DeckA(DeckButton::Out),
        Button::DeckA(DeckButton::BeatBackward),
        Button::DeckA(DeckButton::BeatForward),
        Button::DeckA(DeckButton::Cue),
        Button::DeckA(DeckButton::Cup),
        Button::DeckA(DeckButton::Play),
        Button::DeckA(DeckButton::Sync),
        Button::DeckB(DeckButton::FX1),
        Button::DeckB(DeckButton::FX2),
        Button::DeckB(DeckButton::In),
        Button::DeckB(DeckButton::Out),
        Button::DeckB(DeckButton::BeatBackward),
        Button::DeckB(DeckButton::BeatForward),
        Button::DeckB(DeckButton::Cue),
        Button::DeckB(DeckButton::Cup),
        Button::DeckB(DeckButton::Play),
        Button::DeckB(DeckButton::Sync),
    ];

    loop {
        let state = device.read_state()?;
        let mut writer = device.writer();
        for button in &buttons {
            writer.set_led(*button, if state.is_button_pressed(*button) { 127 } else { 10 });
        }
        writer.write()?;

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
