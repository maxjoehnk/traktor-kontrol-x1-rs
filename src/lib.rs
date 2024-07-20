use thiserror::Error;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::time::Duration;
use rusb::{Context, Device, DeviceHandle, UsbContext};

const VENDOR_ID: u16 = 0x17cc;
const PRODUCT_ID: u16 = 0x2305;

const LED_MAX: u8 = 0x7F;

const USB_INTERFACE: u8 = 0x00;

const WRITE_ENDPOINT: u8 = 0x01;
const WRITE_CONFIRM_ENDPOINT: u8 = 0x81;

const READ_ENDPOINT: u8 = 0x84;

const READ_TIMEOUT: Duration = Duration::from_millis(50);
const WRITE_TIMEOUT: Duration = Duration::from_millis(50);

type Result<T> = std::result::Result<T, X1Error>;

pub fn list_devices() -> Result<Vec<TraktorX1>> {
    let context = Context::new()?;

    let devices = context.devices()?
        .iter()
        .filter(|device| {
            let Ok(descriptor) = device.device_descriptor() else {
                return false;
            };
            descriptor.vendor_id() == VENDOR_ID && descriptor.product_id() == PRODUCT_ID
        })
        .map(TraktorX1::new)
        .collect::<Result<_>>()?;

    Ok(devices)
}

#[derive(Debug)]
pub struct TraktorX1 {
    handle: DeviceHandle<Context>
}

impl TraktorX1 {
    fn new(device: Device<Context>) -> Result<Self> {
        let handle = device.open()?;
        if handle.kernel_driver_active(USB_INTERFACE)? {
            handle.detach_kernel_driver(USB_INTERFACE)?;
        }
        handle.claim_interface(USB_INTERFACE)?;

        Ok(Self { handle })
    }

    pub fn writer(&mut self) -> LedWriter {
        LedWriter::new(&mut self.handle)
    }

    pub fn write_leds<'a>(&mut self, leds: impl Iterator<Item = (&'a Button, &'a u8)>) -> Result<()> {
        let mut writer = self.writer();
        for (button, on) in leds {
            writer.set_led(*button, *on);
        }
        writer.write()
    }

    pub fn read_state(&self) -> Result<X1State> {
        let mut buffer = [0u8; 24];
        self.handle.read_bulk(READ_ENDPOINT, &mut buffer, READ_TIMEOUT)
            .map_err(|err| match err {
                rusb::Error::Timeout => X1Error::Timeout,
                err => X1Error::Libusb(err),
            })?;

        Ok(X1State::new(buffer))
    }
}

fn hex2bin(hex: u8) -> [u8; 8] {
    let mut bin = [0u8; 8];
    for i in 0..8 {
        bin[i] = (hex >> i) & 1;
    }
    bin
}

#[derive(Clone, Copy)]
pub struct X1State {
    button_bits: [[u8; 8]; 5],
    knob_bytes: [(u8, u8); 8],
}

impl X1State {
    fn new(buffer: [u8; 24]) -> Self {
        let bin_0 = hex2bin(buffer[1]);
        let bin_1 = hex2bin(buffer[2]);
        let bin_2 = hex2bin(buffer[3]);
        let bin_3 = hex2bin(buffer[4]);
        let bin_4 = hex2bin(buffer[5]);

        let button_bits = [bin_0, bin_1, bin_2, bin_3, bin_4];

        let knob_bytes = [
            (buffer[16], buffer[17]),
            (buffer[20], buffer[21]),
            (buffer[22], buffer[23]),
            (buffer[18], buffer[19]),
            (buffer[12], buffer[13]),
            (buffer[10], buffer[11]),
            (buffer[8], buffer[9]),
            (buffer[14], buffer[15]),
        ];

        Self {
            button_bits,
            knob_bytes,
        }
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        let address = match button {
            Button::Shift => (4, 4),
            Button::Hotcue => (4, 7),
            Button::FX1(FxButton::On) => (3, 4),
            Button::FX1(FxButton::Button1) => (3, 5),
            Button::FX1(FxButton::Button2) => (3, 6),
            Button::FX1(FxButton::Button3) => (3, 7),
            Button::FX2(FxButton::On) => (4, 0),
            Button::FX2(FxButton::Button1) => (4, 1),
            Button::FX2(FxButton::Button2) => (4, 2),
            Button::FX2(FxButton::Button3) => (4, 3),
            Button::DeckA(DeckButton::Browse) => (3, 0),
            Button::DeckA(DeckButton::FX1) => (1, 1),
            Button::DeckA(DeckButton::FX2) => (1, 0),
            Button::DeckA(DeckButton::Loop) => (3, 2),
            Button::DeckA(DeckButton::In) => (2, 4),
            Button::DeckA(DeckButton::Out) => (0, 3),
            Button::DeckA(DeckButton::BeatBackward) => (0, 2),
            Button::DeckA(DeckButton::BeatForward) => (2, 5),
            Button::DeckA(DeckButton::Cue) => (0, 1),
            Button::DeckA(DeckButton::Cup) => (2, 6),
            Button::DeckA(DeckButton::Play) => (0, 0),
            Button::DeckA(DeckButton::Sync) => (2, 7),
            Button::DeckB(DeckButton::Browse) => (3, 1),
            Button::DeckB(DeckButton::FX1) => (4, 6),
            Button::DeckB(DeckButton::FX2) => (4, 5),
            Button::DeckB(DeckButton::Loop) => (3, 3),
            Button::DeckB(DeckButton::In) => (1, 4),
            Button::DeckB(DeckButton::Out) => (2, 3),
            Button::DeckB(DeckButton::BeatBackward) => (2, 2),
            Button::DeckB(DeckButton::BeatForward) => (1, 5),
            Button::DeckB(DeckButton::Cue) => (2, 1),
            Button::DeckB(DeckButton::Cup) => (1, 6),
            Button::DeckB(DeckButton::Play) => (2, 0),
            Button::DeckB(DeckButton::Sync) => (1, 7),
        };

        self.button_bits[address.0][address.1] > 0
    }

    pub fn read_knob(&self, knob: Knob) -> u16 {
        let (c1, c2) = match knob {
            Knob::FX1(FxKnob::DryWet) => &self.knob_bytes[0],
            Knob::FX1(FxKnob::Param1) => &self.knob_bytes[1],
            Knob::FX1(FxKnob::Param2) => &self.knob_bytes[2],
            Knob::FX1(FxKnob::Param3) => &self.knob_bytes[3],
            Knob::FX2(FxKnob::DryWet) => &self.knob_bytes[4],
            Knob::FX2(FxKnob::Param1) => &self.knob_bytes[5],
            Knob::FX2(FxKnob::Param2) => &self.knob_bytes[6],
            Knob::FX2(FxKnob::Param3) => &self.knob_bytes[7],
        };

        u16::from_be_bytes([*c1, *c2])
    }

    pub fn read_encoder(&self, encoder: Encoder) -> EncoderState {
        todo!()
    }
}

impl Debug for X1State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(X1State))
            .field("Shift", &self.is_button_pressed(Button::Shift))
            .field("Hotcue", &self.is_button_pressed(Button::Hotcue))
            .field("FX1 On", &self.is_button_pressed(Button::FX1(FxButton::On)))
            .field("FX1 1", &self.is_button_pressed(Button::FX1(FxButton::Button1)))
            .field("FX1 2", &self.is_button_pressed(Button::FX1(FxButton::Button2)))
            .field("FX1 3", &self.is_button_pressed(Button::FX1(FxButton::Button3)))
            .field("FX2 On", &self.is_button_pressed(Button::FX2(FxButton::On)))
            .field("FX2 1", &self.is_button_pressed(Button::FX2(FxButton::Button1)))
            .field("FX2 2", &self.is_button_pressed(Button::FX2(FxButton::Button2)))
            .field("FX2 3", &self.is_button_pressed(Button::FX2(FxButton::Button3)))
            .field("DeckA Browse", &self.is_button_pressed(Button::DeckA(DeckButton::Browse)))
            .field("DeckA FX1", &self.is_button_pressed(Button::DeckA(DeckButton::FX1)))
            .field("DeckA FX2", &self.is_button_pressed(Button::DeckA(DeckButton::FX2)))
            .field("DeckA Loop", &self.is_button_pressed(Button::DeckA(DeckButton::Loop)))
            .field("DeckA In", &self.is_button_pressed(Button::DeckA(DeckButton::In)))
            .field("DeckA Out", &self.is_button_pressed(Button::DeckA(DeckButton::Out)))
            .field("DeckA BeatMinus", &self.is_button_pressed(Button::DeckA(DeckButton::BeatBackward)))
            .field("DeckA BeatPlus", &self.is_button_pressed(Button::DeckA(DeckButton::BeatForward)))
            .field("DeckA Cue", &self.is_button_pressed(Button::DeckA(DeckButton::Cue)))
            .field("DeckA Cup", &self.is_button_pressed(Button::DeckA(DeckButton::Cup)))
            .field("DeckA Play", &self.is_button_pressed(Button::DeckA(DeckButton::Play)))
            .field("DeckA Sync", &self.is_button_pressed(Button::DeckA(DeckButton::Sync)))
            .field("DeckB Browse", &self.is_button_pressed(Button::DeckB(DeckButton::Browse)))
            .field("DeckB FX1", &self.is_button_pressed(Button::DeckB(DeckButton::FX1)))
            .field("DeckB FX2", &self.is_button_pressed(Button::DeckB(DeckButton::FX2)))
            .field("DeckB Loop", &self.is_button_pressed(Button::DeckB(DeckButton::Loop)))
            .field("DeckB In", &self.is_button_pressed(Button::DeckB(DeckButton::In)))
            .field("DeckB Out", &self.is_button_pressed(Button::DeckB(DeckButton::Out)))
            .field("DeckB BeatMinus", &self.is_button_pressed(Button::DeckB(DeckButton::BeatBackward)))
            .field("DeckB BeatPlus", &self.is_button_pressed(Button::DeckB(DeckButton::BeatForward)))
            .field("DeckB Cue", &self.is_button_pressed(Button::DeckB(DeckButton::Cue)))
            .field("DeckB Cup", &self.is_button_pressed(Button::DeckB(DeckButton::Cup)))
            .field("DeckB Play", &self.is_button_pressed(Button::DeckB(DeckButton::Play)))
            .field("DeckB Sync", &self.is_button_pressed(Button::DeckB(DeckButton::Sync)))
            .field("FX1 Dry/Wet", &self.read_knob(Knob::FX1(FxKnob::DryWet)))
            .field("FX1 Param1", &self.read_knob(Knob::FX1(FxKnob::Param1)))
            .field("FX1 Param2", &self.read_knob(Knob::FX1(FxKnob::Param2)))
            .field("FX1 Param3", &self.read_knob(Knob::FX1(FxKnob::Param3)))
            .field("FX2 Dry/Wet", &self.read_knob(Knob::FX2(FxKnob::DryWet)))
            .field("FX2 Param1", &self.read_knob(Knob::FX2(FxKnob::Param1)))
            .field("FX2 Param2", &self.read_knob(Knob::FX2(FxKnob::Param2)))
            .field("FX2 Param3", &self.read_knob(Knob::FX2(FxKnob::Param3)))
            .finish()

    }
}

pub struct LedWriter<'a> {
    handle: &'a mut DeviceHandle<Context>,
    leds: HashMap<Button, u8>,
}

impl<'a> LedWriter<'a> {
    pub fn new(handle: &'a mut DeviceHandle<Context>) -> Self {
        Self {
            handle,
            leds: HashMap::new(),
        }
    }

    pub fn set_led(&mut self, button: Button, on: u8) -> &mut Self {
        self.leds.insert(button, on);
        self
    }

    pub fn write(mut self) -> Result<()> {
        let mut buffer = [0u8; 32];
        buffer[0] = 0x0c;
        for (button, on) in self.leds {
            let address = match button {
                Button::Shift => 29,
                Button::Hotcue => 30,
                Button::FX1(FxButton::On) => 8,
                Button::FX1(FxButton::Button1) => 7,
                Button::FX1(FxButton::Button2) => 6,
                Button::FX1(FxButton::Button3) => 5,
                Button::FX2(FxButton::On) => 4,
                Button::FX2(FxButton::Button1) => 3,
                Button::FX2(FxButton::Button2) => 2,
                Button::FX2(FxButton::Button3) => 1,
                Button::DeckA(DeckButton::FX1) => 25,
                Button::DeckA(DeckButton::FX2) => 26,
                Button::DeckA(DeckButton::In) => 18,
                Button::DeckA(DeckButton::Out) => 17,
                Button::DeckA(DeckButton::BeatBackward) => 20,
                Button::DeckA(DeckButton::BeatForward) => 19,
                Button::DeckA(DeckButton::Cue) => 22,
                Button::DeckA(DeckButton::Cup) => 21,
                Button::DeckA(DeckButton::Play) => 24,
                Button::DeckA(DeckButton::Sync) => 23,
                Button::DeckB(DeckButton::FX1) => 27,
                Button::DeckB(DeckButton::FX2) => 28,
                Button::DeckB(DeckButton::In) => 16,
                Button::DeckB(DeckButton::Out) => 15,
                Button::DeckB(DeckButton::BeatBackward) => 14,
                Button::DeckB(DeckButton::BeatForward) => 13,
                Button::DeckB(DeckButton::Cue) => 12,
                Button::DeckB(DeckButton::Cup) => 11,
                Button::DeckB(DeckButton::Play) => 10,
                Button::DeckB(DeckButton::Sync) => 9,
                _ => continue,
            };
            buffer[address] = on.min(LED_MAX);
        }
        self.handle.write_bulk(WRITE_ENDPOINT, &buffer, WRITE_TIMEOUT)
            .map_err(|err| match err {
                rusb::Error::Timeout => X1Error::Timeout,
                err => X1Error::Libusb(err),
            })?;

        let mut confirm_buffer = [0u8; 1];
        self.handle.read_bulk(WRITE_CONFIRM_ENDPOINT, &mut confirm_buffer, READ_TIMEOUT)
            .map_err(|err| match err {
                rusb::Error::Timeout => X1Error::Timeout,
                err => X1Error::Libusb(err),
            })?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Knob {
    FX1(FxKnob),
    FX2(FxKnob),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxKnob {
    DryWet,
    Param1,
    Param2,
    Param3
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoder {
    DeckA(DeckEncoder),
    DeckB(DeckEncoder),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckEncoder {
    Browse,
    Loop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncoderState {
    None,
    CW,
    CCW,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Shift,
    Hotcue,
    FX1(FxButton),
    FX2(FxButton),
    DeckA(DeckButton),
    DeckB(DeckButton),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxButton {
    On,
    Button1,
    Button2,
    Button3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckButton {
    Browse,
    FX1,
    FX2,
    Loop,
    In,
    Out,
    BeatBackward,
    BeatForward,
    Cue,
    Cup,
    Play,
    Sync,
}

#[derive(Debug, Error)]
pub enum X1Error {
    #[error("USB Timeout")]
    Timeout,
    #[error("Usb error: {0}")]
    Libusb(#[from] rusb::Error),
}
