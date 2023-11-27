#![no_std]
#![no_main]

/**** low-level imports *****/
use panic_halt as _;
use cortex_m_rt::entry;
use core::fmt::Write as _;

use embedded_hal::blocking::spi::Write;



// Display-related imports
use ssd1331::{DisplayRotation, Ssd1331};
use embedded_graphics::{
    geometry::Point,
    mono_font::{
        ascii::{FONT_6X10, FONT_9X18},
        MonoTextStyleBuilder,
    },
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};

use fugit::RateExtU32;

// Feather-specific imports
use adafruit_feather_rp2040::hal as hal;
use adafruit_feather_rp2040::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        pac,
        pac::interrupt,
        watchdog::Watchdog,
        Sio,
        gpio::FunctionSpi,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};

/** external device imports */
// use fugit::RateExtU32;

// // IMU stuff
// use lis3dh::Lis3dh;
// use accelerometer::{RawAccelerometer, Tracker};

// USB stuff
use usb_device::class_prelude::*;
mod usb_manager;
use usb_manager::UsbManager;

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


// -------------------------------- main function ----------------- //
#[entry]
fn main() -> ! {
    // Get peripherals
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Init the watchdog timer, to pass into clock init
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    
    // Init the clocks
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).ok().unwrap();
    
    // Init Single-cycle IO for GPIO control
    let sio = Sio::new(pac.SIO);
    
    // Init pins to default state
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Delay timer for controlling loop iteration rate
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());


    // Set up USB driver
    let usb = unsafe {
        USB_BUS = Some(UsbBusAllocator::new(hal::usb::UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        )));

        USB_MANAGER = Some(UsbManager::new(USB_BUS.as_ref().unwrap()));

        // Enable interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        USB_MANAGER.as_mut().unwrap()
    };

    /* --------------------- Set up OLED SPI ---------------------*/

    // reference: https://www.reddit.com/r/rust/comments/ugbuvz/anyone_have_a_working_example_of_spi_on_raspberry/
    let _sclk = pins.sclk.into_mode::<FunctionSpi>();
    let _miso = pins.miso.into_mode::<FunctionSpi>();
    let _mosi = pins.mosi.into_mode::<FunctionSpi>();
    let cs  = pins.d9.into_push_pull_output();

    let spi = hal::Spi::<_, _, 8>::new(pac.SPI0);

    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        2_500_000u32.Hz(),
        &embedded_hal::spi::MODE_3,
    );

    let mut rst = pins.d11.into_push_pull_output();
    let dc = pins.d10.into_push_pull_output();
    

    // Ref: https://github.com/jamwaffles/ssd1331/blob/master/examples/text.rs
    let mut display = Ssd1331::new(spi, dc, DisplayRotation::Rotate0);

    display.reset(&mut rst, &mut delay).unwrap();
    display.init().unwrap();
    display.flush().unwrap();

    let white_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::WHITE)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), white_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

        

    // // Set up IMU I2C
    // // reference: https://github.com/rp-rs/rp-hal/blob/main/rp2040-hal/examples/i2c.rs
    // let i2c = hal::I2C::i2c1(
    //     pac.I2C1,
    //     pins.sda.into_mode(),
    //     pins.scl.into_mode(),
    //     RateExtU32::kHz(400),
    //     &mut pac.RESETS,
    //     &clocks.system_clock,
    // );
    //
    // // Set up IMU
    // let mut lis3dh = Lis3dh::new_i2c(i2c, lis3dh::SlaveAddr::Default).unwrap();
    //
    // // Set up IMU tracker
    // // source: https://github.com/BenBergman/lis3dh-rs/blob/master/examples/cpx.rs
    // let mut tracker = Tracker::new(3700.0);
    // 
    // Begin looping
    
    loop {
        // Update accelerometer reading
        // Source: https://github.com/BenBergman/lis3dh-rs/blob/master/examples/cpx.rs
        // let accel = lis3dh.accel_raw().unwrap();
        // let orientation = tracker.update(accel);
        // write!(usb, "Orientation: {:?}\r\n", orientation).unwrap();

        write!(usb, "Test!\r\n").unwrap();


        // End with a newline
        write!(usb, "\r\n").unwrap();

        // Delay before next iteration
        delay.delay_ms(100 as u32);
    }

}
