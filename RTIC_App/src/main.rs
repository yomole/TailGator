#![no_std]
#![no_main]

use rtic::app;
use panic_halt as _;

mod usb_manager;

#[app(device = adafruit_feather_rp2040::pac, peripherals = true, dispatchers = [RTC_IRQ])]
mod app {

    use core::fmt::Write;

    // Board specific imports
    use adafruit_feather_rp2040::hal as hal;
    use adafruit_feather_rp2040::{
        Pins,
        Scl,        // __ these are type aliases for the configured pins
        Sda,        //
        XOSC_CRYSTAL_FREQ,
    };
    use hal::{
        clocks::init_clocks_and_plls,
        Clock,
        watchdog::Watchdog,
        Sio,
        gpio::FunctionI2C,
        i2c::I2C,
        pio::PIOExt,
    };
    use embedded_hal::digital::v2::{
        StatefulOutputPin,
        OutputPin,
    };

    // Imports for the OLED display
    use sh1107::{
        prelude::*,
        mode::GraphicsMode,
        interface::i2c::I2cInterface,
        Builder
    };
    // A type alias to make things easier to read
    type OledDisplay = GraphicsMode<
        I2cInterface<hal::i2c::I2C<hal::pac::I2C1, (Sda, Scl)>>
    >;

    // Imports for IMU
    use fugit::RateExtU32;
    use lis3dh::Lis3dh;
    use accelerometer::{
        RawAccelerometer,
        Tracker,
        Orientation::{
            LandscapeUp, LandscapeDown, PortraitUp, PortraitDown
        }
    };

    // IMU type alias
    type Lis3dhIMU = Lis3dh<
        lis3dh::Lis3dhI2C<I2C<hal::pac::I2C1,
        (   hal::gpio::Pin<hal::gpio::bank0::Gpio2, hal::gpio::Function<hal::gpio::I2C>>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio3, hal::gpio::Function<hal::gpio::I2C>>)
    >>>;

    // Imports for the NeoPixels
    use ws2812_pio::Ws2812Direct;
    use smart_leds::{RGB8, SmartLedsWrite};
    type PioNeopixel = Ws2812Direct<
        hal::pac::PIO0,
        hal::pio::SM0,
        hal::gpio::pin::bank0::Gpio7,
    >;

    /**************************************************************************
    DATA STRUCTURE setup
    **************************************************************************/
    use usb_device::class_prelude::*;
    use crate::usb_manager::UsbManager;
    #[shared]
    struct DataCommon {
        usb_manager: UsbManager,
    }

    #[local]
    struct DataLocal {
        led: hal::gpio::Pin<hal::gpio::pin::bank0::Gpio13, hal::gpio::ReadableOutput>,
        display: OledDisplay,
        pixel: (u8, u8),
        dirs: (bool, bool),
        // imu:  Lis3dhIMU,
        // imu_tracker: Tracker,
        // neopixel: PioNeopixel,
        // neopixel_idx: u8,
    }

    use systick_monotonic::ExtU64;
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    /**************************************************************************
    INIT ROUTINE
    **************************************************************************/
    #[init(local = [usb_bus: Option<usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>> = None])]
    fn init(cx: init::Context) -> (DataCommon, DataLocal, init::Monotonics) {
        
        let mut resets = cx.device.RESETS;
        let mut watchdog = Watchdog::new(cx.device.WATCHDOG);
        let clocks = init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            cx.device.XOSC,
            cx.device.CLOCKS,
            cx.device.PLL_SYS,
            cx.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        /**********************************************************************
        Setup the USB driver and USB Manager for serial port printing
        **********************************************************************/
        let usb_bus: &'static _ =
            cx.local.usb_bus.insert(UsbBusAllocator::new(hal::usb::UsbBus::new(
                cx.device.USBCTRL_REGS,
                cx.device.USBCTRL_DPRAM,
                clocks.usb_clock,
                true,
                &mut resets,
            )));
        let mut usb_manager = UsbManager::new(usb_bus);

        /**********************************************************************
        Setup the GPIO and led pin 
        **********************************************************************/
        // initialize the Single Cycle IO
        let sio = Sio::new(cx.device.SIO);
        // initialize the pins to default state
        let pins = Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );
        let led_pin = pins.d13.into_readable_output();
        // let (x, y) = (2, 1.2);
        /**********************************************************************
        Setup the PIO NeoPixel driver
        **********************************************************************/
        // // let timer = hal::timer::Timer::new(cx.device.TIMER, &mut resets);
        // let (mut pio, sm0, _, _, _) = cx.device.PIO0.split(&mut resets);
        // // Since we're using Ws2812Direct, we need to be careful to not call our task too often below.
        // let mut neopixels = Ws2812Direct::new(
        //     pins.d5.into_mode(),
        //     &mut pio,
        //     sm0,
        //     clocks.peripheral_clock.freq(),
        // );
        // // Don't forget to drive the power enable pin high!
        // let mut pwr_pin = pins.d10.into_push_pull_output();
        // pwr_pin.set_high().unwrap();
        // // Light up a few pixels
        // let pixels: [RGB8; 3] = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
        // neopixels.write(pixels.iter().cloned()).unwrap();

        /**********************************************************************
        Setup the I2C peripheral 
        **********************************************************************/
        let scl = pins.scl.into_mode::<FunctionI2C>();
        let sda = pins.sda.into_mode::<FunctionI2C>();
        let i2c1 = I2C::i2c1(
            cx.device.I2C1,
            sda,
            scl,
            fugit::RateExtU32::kHz(400),
            &mut resets,
            &clocks.system_clock,
        );

        // Setup the OLED display
        let display_size = DisplaySize::Display64x128;
        let display_rot  = DisplayRotation::Rotate270;
        
        let mut display: GraphicsMode<_> = Builder::new()
            .with_size(display_size)
            .with_rotation(display_rot)
            .connect_i2c(i2c1)
            .into();
        display.init().unwrap();

        use embedded_graphics::{
            // primitives::{Rectangle, PrimitiveStyle},
            mono_font::{ascii::FONT_6X9, MonoTextStyle},
            pixelcolor::BinaryColor,
            prelude::*,
            text::Text,
        };
        let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On);
        let text = Text::new("RTIC testing", Point::new(10, 10), style);
        text.draw(&mut display).unwrap();
        display.flush().unwrap();
        let args = core::format_args!("testing {}", 1);
        let _ = print_fmt::spawn_after(1500.millis(), (0, 1.2));
        
        // Set up IMU
        // let mut imu = Lis3dh::new_i2c(i2c1, lis3dh::SlaveAddr::Default).unwrap();

        // source: https://github.com/BenBergman/lis3dh-rs/blob/master/examples/cpx.rs
        let mut imu_tracker = Tracker::new(3700.0);
        

        

        /**********************************************************************
        Setup tasks!
        **********************************************************************/
        // Blink 5 times on startup, and print a welcome message
        
        blink::spawn(5).unwrap();
        print::spawn_after(1000.millis(), "Welcome to Carsten's RTIC App!\n").unwrap();
        // Start the heartbeat in 3 seconds
        heartbeat::spawn_after(1500.millis()).unwrap();
        // Start the OLED animation after 2 seconds
        update_oled::spawn_after(2.secs()).unwrap();
        // Clear the NeoMatrix after 3 seconds
        //neomatrix_update::spawn_after(3.secs()).unwrap();
        // Start IMU communication
        //update_imu::spawn_after(1600.millis()).unwrap();

        // Return the resource structs
        (
            DataCommon {
                usb_manager: usb_manager,
            },
            DataLocal {
                led: led_pin,
                display: display,
                pixel: (10, 25),
                dirs: (true, true),
                // imu: imu,
                // imu_tracker: imu_tracker,
                // neopixel: neopixels,
                // neopixel_idx: 0,
            },
            init::Monotonics(systick_monotonic::Systick::new(cx.core.SYST, 125_000_000)),
        )
    }


    /**************************************************************************
    USB Interrupt task -- keeps the host happy and reads any available serial data
    **************************************************************************/
    #[task(binds = USBCTRL_IRQ, shared = [usb_manager])]
    fn usb_task(cx: usb_task::Context) {
        let mut usb_manager = cx.shared.usb_manager;
        (usb_manager).lock(
            |usb_manager_l| {
                usb_manager_l.interrupt();
            }
        );
    }

    /**************************************************************************
    Print Task -- toggle the LED and prints the state to the serial port.
    **************************************************************************/
    #[task(shared = [usb_manager])]
    fn print(cx: print::Context, s: &'static str) {
        let mut usb_manager = cx.shared.usb_manager;
        usb_manager.lock(
            |usb_manager_l| {
                usb_manager_l.write(s);
                //write!(usb_manager_l, "test {}", 1.2_f64);
            }
        );
    }

    #[task(shared = [usb_manager])]
    fn print_fmt(cx: print_fmt::Context, args: (u8, f64)) {
        let mut usb_manager = cx.shared.usb_manager;
        usb_manager.lock(
            |usb_manager_l| {
                write!(usb_manager_l, "{}; {}\n", args.0, args.1);
            }
        );
    }


    // write!()

    /**************************************************************************
    Heartbeat Task -- once started, the heartbeat will print to serial port
        every 2 seconds.
    **************************************************************************/
    #[task]
    fn heartbeat(_cx: heartbeat::Context) {
        // blink::spawn(3).unwrap();
        print::spawn("heartbeat!\n").unwrap();
        heartbeat::spawn_after(1000.millis()).unwrap();
    }

    /**************************************************************************
    LED Task -- Blinks the onboard LED n times
    **************************************************************************/
    const BLINK_DUR: u64 = 120;  // = on_time = off_time (in ms)
    #[task(local = [led])]
    fn blink(cx: blink::Context, n: u8) {
        if n == 0 {
            return;
        } else if cx.local.led.is_set_low().unwrap() {
            cx.local.led.set_high().unwrap();
            blink::spawn_after(BLINK_DUR.millis(), n).unwrap();
        } else {
            cx.local.led.set_low().unwrap();
            blink::spawn_after(BLINK_DUR.millis(), n-1).unwrap();
        }
    }

    /**************************************************************************
    OLED Update Task -- clears the OLED and sets a pixel bouncing around
    **************************************************************************/
    #[task(local = [display, pixel, dirs])]
    fn update_oled(cx: update_oled::Context) {
        // Get the display and its dimensions
        let d = cx.local.display;
        let (w, h) = d.get_dimensions();
        // Get the current pixel position and direction
        let (mut x, mut y) = cx.local.pixel;
        let (mut x_dir, mut y_dir) = cx.local.dirs;
        // increment or decrement the pixel location
        // TODO -- learn about addition and subtraction 
        let delta: u8 = 1;
        if x_dir { x += delta } else { x -= delta }
        if y_dir { y += delta } else { y -= delta }
        // check the bounds
        if x <= 0 || x >= w-1 { x_dir = !x_dir }
        if y <= 0 || y >= h-1 { y_dir = !y_dir }
        *cx.local.pixel = (x, y);
        *cx.local.dirs  = (x_dir, y_dir);
        d.clear();
        d.set_pixel(x as u32, y as u32, 1u8);
        match d.flush() {
            Ok(_) => {},
            Err(_) => {},
        };
        update_oled::spawn_after(5.millis()).unwrap();
    }

    /**************************************************************************
    IMU update task
    **************************************************************************/
    // #[task(local = [imu, imu_tracker])]
    // fn update_imu(cx: update_imu::Context) {
    //     // Update accelerometer reading
    //     // Source: https://github.com/BenBergman/lis3dh-rs/blob/master/examples/cpx.rs
    //     let accel: accelerometer::vector::I16x3 = cx.local.imu.accel_raw().unwrap();
    //     let orientation = cx.local.imu_tracker.update(accel);
    //     //print::spawn_after(100.millis(),"Orientation: ").unwrap();
    //     print::spawn(match orientation {
    //         LandscapeUp     => "LandscapeUp\n",
    //         LandscapeDown   => "LandscapeDown\n",
    //         PortraitUp      => "PortraitUp\n",
    //         PortraitDown    => "PortraitDown\n",
    //         _ => "unknown\n"
    //     }).unwrap();
    //     update_imu::spawn_after(1000.millis()).unwrap();
    // }

    /**************************************************************************
    NeoMatrix update task --
    **************************************************************************/
    // const NM_UPDATE_DUR: u64 = 100; // refresh period for NeoMatrix in ms
    // #[task(local = [neopixel, neopixel_idx])]
    // fn neomatrix_update(cx: neomatrix_update::Context) {
    //     let nm = cx.local.neopixel;
    //     let mut idx = *cx.local.neopixel_idx;
    //     if idx == 63 {
    //         idx = 0;
    //     } else {
    //         idx += 1;
    //     }
    //     *cx.local.neopixel_idx = idx;
    //     let mut pixels: [RGB8; 64] = [RGB8::new(0, 0, 0); 64];
    //     (pixels[idx as usize].r, pixels[idx as usize].b) = (50, 100);
    //     nm.write(pixels.iter().cloned()).unwrap();
    //     neomatrix_update::spawn_after(NM_UPDATE_DUR.millis()).unwrap();
    // }

} // mod app