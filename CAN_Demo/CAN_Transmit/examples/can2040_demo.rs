//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use defmt_rtt as _;
use embedded_can::nb::Can;
use embedded_can::{Frame, StandardId};
use embedded_hal::digital::InputPin;
use embedded_hal::digital::StatefulOutputPin;
// use panic_probe as _;
use panic_halt as _;
use rp2040_hal::clocks::init_clocks_and_plls;
use rp2040_hal::gpio::Pins;
use rp2040_hal::{entry, pac, Sio, Watchdog};
use rp_pico::XOSC_CRYSTAL_FREQ;
// const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

use can2040::global_allocator::init_allocator;
use can2040::{Can2040, CanFrame};

const CONFIG_CANBUS_FREQUENCY: u32 = 10_000;
const CONFIG_RP2040_CANBUS_GPIO_RX: u32 = 8;
const CONFIG_RP2040_CANBUS_GPIO_TX: u32 = 7;

// Second-stage bootloader ------------------------------------------------------------------------
#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

#[entry]
fn main() -> ! {
    init_allocator();
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let _clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

    let mut led_pin = pins.gpio13.into_push_pull_output();
    let mut leak_pin = pins.gpio24.into_pull_up_input();

    let mut can_bus = can2040::initialize_cbus(
        &mut core,
        CONFIG_CANBUS_FREQUENCY,
        CONFIG_RP2040_CANBUS_GPIO_RX,
        CONFIG_RP2040_CANBUS_GPIO_TX,
    );

    let mut count = 0u64;
    // let mut packet_num = 0u64;
    loop {
        count += 1;

        if count % 1_000_000 == 13 {
            // packet_num += 1;
            let leak = leak_pin.is_low().unwrap() as u64;
            info!("leak: {:?}", leak);

            let f = create_frame(5, leak);
            match <Can2040 as embedded_can::blocking::Can>::transmit(&mut can_bus, &f) {
                Ok(_) => {}
                Err(err) => {
                    error!("Transmit error: {}", err);
                }
            }
            info!("Transmitted package: {:?}", f);
            led_pin.toggle().unwrap();
        }
        match can_bus.receive() {
            Ok(f) => {
                info!("Received packet: {:?}", f);
            }
            Err(nb::Error::Other(err)) => {
                error!("Errors in reading CAN frame, {:?}", err);
            }
            _ => (), // ignore
        }
    }
}

pub fn create_frame(cob_id: u16, data: u64) -> CanFrame {
    CanFrame::new(
        StandardId::new(cob_id).expect("error in create standard id"),
        &[1, 2, 3, (data & 0xff) as u8],
    )
    .expect("error in create_frame")
}
