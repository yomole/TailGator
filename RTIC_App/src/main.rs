#![no_std]
#![no_main]

use rtic::app;
use panic_halt as _;

#[app(device = adafruit_feather_rp2040::pac, peripherals = true, dispatchers = [RTC_IRQ])]
mod app {

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
    use embedded_hal::digital::{
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
        I2cInterface<hal::i2c::I2C<hal::pac::I2C1, 
        (   hal::gpio::Pin<hal::gpio::bank0::Gpio2, hal::gpio::FunctionI2C, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio3, hal::gpio::FunctionI2C, hal::gpio::PullDown>)>>
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
        (   hal::gpio::Pin<hal::gpio::bank0::Gpio2, hal::gpio::FunctionI2C, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio3, hal::gpio::FunctionI2C, hal::gpio::PullDown>)
    >>>;

    /**************************************************************************
    Resources
    **************************************************************************/

    #[shared]
    struct DataShared {}

    #[local]
    struct DataLocal {
        led: hal::gpio::Pin<hal::gpio::bank0::Gpio13, hal::gpio::FunctionSioOutput, hal::gpio::PullDown>,
        display: OledDisplay,
        pixel: (u8, u8),
        dirs: (bool, bool),
        // imu:  Lis3dhIMU,
        // imu_tracker: Tracker,
    }

    use systick_monotonic::ExtU64;
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    /**************************************************************************
    Init routine
    **************************************************************************/
    #[init]
    fn init(cx: init::Context) -> (DataShared, DataLocal, init::Monotonics) {
        
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
        let led_pin = pins.d13.into_push_pull_output();
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
        let scl = pins.scl.into_function::<FunctionI2C>();
        let sda = pins.sda.into_function::<FunctionI2C>();
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

        // Set up IMU
        // let mut imu = Lis3dh::new_i2c(i2c1, lis3dh::SlaveAddr::Default).unwrap();

        // source: https://github.com/BenBergman/lis3dh-rs/blob/master/examples/cpx.rs
        let mut imu_tracker = Tracker::new(3700.0);
        

        

        /**********************************************************************
        Setup tasks!
        **********************************************************************/
        blink::spawn(5).unwrap();
        // info!(...);
        heartbeat::spawn_after(3000.millis()).unwrap();
        
        // Start OLED animation
        update_oled::spawn_after(2000.millis()).unwrap();
        
        // Start IMU communication
        //update_imu::spawn_after(1600.millis()).unwrap();

        // Return the resource structs
        (
            DataShared {

            },
            DataLocal {
                led: led_pin,
                display: display,
                pixel: (10, 25),
                dirs: (true, true),
                // imu: imu,
                // imu_tracker: imu_tracker,
            },
            init::Monotonics(systick_monotonic::Systick::new(cx.core.SYST, 125_000_000)),
        )
    }


    #[task]
    fn heartbeat(_cx: heartbeat::Context) {
        blink::spawn(2).unwrap();
        // print::spawn("heartbeat!\n").unwrap();
        heartbeat::spawn_after(1000.millis()).unwrap();
    }

    /**************************************************************************
    LED Blink Task
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
    OLED Update Task
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

} // mod app