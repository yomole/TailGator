#![no_std]
#![no_main]

/**** low-level imports *****/
use core::fmt::Write;
use core::panic::PanicInfo;
// use panic_halt as _;
// use cortex_m::prelude::*;
// use cortex_m_rt::entry;
use embedded_hal::{
    digital::v2::{OutputPin},
};


// CAN BUS --------------------------------------------------------------------
extern crate alloc;

use embedded_can::nb::Can;
use embedded_can::{Frame, StandardId};
// use embedded_hal::digital::StatefulOutputPin;
// use panic_probe as _;
// use panic_halt as _;
use rp2040_hal as hal;
use rp2040_hal::clocks::init_clocks_and_plls;
use rp2040_hal::gpio::Pins;
use rp2040_hal::{entry, pac, pac::interrupt, Sio, Watchdog};
use rp_pico::XOSC_CRYSTAL_FREQ;
// const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

use can2040::global_allocator::init_allocator;
use can2040::CanFrame;

const CONFIG_CANBUS_FREQUENCY: u32 = 10_000;
const CONFIG_RP2040_CANBUS_GPIO_RX: u32 = 8;
const CONFIG_RP2040_CANBUS_GPIO_TX: u32 = 7;
// ----------------------------------------------------------------------------

// USB Device support
use usb_device::class_prelude::*;
// USB Communications Class Device support
mod usb_manager;
use usb_manager::UsbManager;
// Global USB objects & interrupt
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
static mut USB_MANAGER: Option<UsbManager> = None;
#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    match USB_MANAGER.as_mut() {
        Some(manager) => manager.interrupt(),
        None => (),
    };
}
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    if let Some(usb) = unsafe { USB_MANAGER.as_mut() } {
        writeln!(usb, "{}", panic_info).ok();
    }
    loop {}
}

#[entry]
fn main() -> ! {
    init_allocator();

    // Grab the singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
    // Init the watchdog timer, to pass into the clock init
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).ok().unwrap();
    
    // Setup USB
    let usb = unsafe {
        USB_BUS = Some(UsbBusAllocator::new(hal::usb::UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        )));
        USB_MANAGER = Some(UsbManager::new(USB_BUS.as_ref().unwrap()));
        // Enable the USB interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        USB_MANAGER.as_mut().unwrap()
    };

    // initialize the Single Cycle IO
    let sio = Sio::new(pac.SIO);
    // initialize the pins to default state
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    use rp2040_hal::Clock;
    // let mut timer = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let mut led_pin = pins.gpio13.into_push_pull_output();



    let mut can_bus = can2040::initialize_cbus(
        &mut core,
        CONFIG_CANBUS_FREQUENCY,
        CONFIG_RP2040_CANBUS_GPIO_RX,
        CONFIG_RP2040_CANBUS_GPIO_TX,
    );


    let mut count = 0u64;
    let mut packet_num = 0u64;

    /*
    Loop Section
    */
    let delay: u32 = 500;   // loop delay in ms
    let mut n: u32 = 0;
    loop {
        if n % 1_000_000 == 13 {
            // write!(usb, "starting loop number {:?}\r\n", n).unwrap();
            led_pin.set_low().unwrap();
            // Await CAN packet
            match can_bus.receive() {
                Ok(f) => {
                    // info!("Received packet: {:?}", f.data()[3]);
                    write!(usb, "Received data: {:?}\r\n", f.data()[3]);
                }
                // Err(nb::Error::Other(err)) => {
                //     // error!("Errors in reading CAN frame, {:?}", err);
                // }
                _ => () // ignore
            }

            // timer.delay_ms(delay as u32);
            led_pin.set_high().unwrap();
            // timer.delay_ms(delay as u32);
            
        }
        n = n + 1;
    }

}

pub fn create_frame(cob_id: u16, data: u64) -> CanFrame {
    CanFrame::new(
        StandardId::new(cob_id).expect("error in create standard id"),
        &[1, 2, 3, (data & 0xff) as u8],
    )
    .expect("error in create_frame")
}