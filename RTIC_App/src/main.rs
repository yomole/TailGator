#![no_std]
#![no_main]

use rtic::app;
use defmt_rtt as _;
use panic_halt as _;

#[app(device = adafruit_feather_rp2040::pac, peripherals = true, dispatchers = [RTC_IRQ])]
mod app {

    // Debug imports
    use defmt::{info, error};

    // Board specific imports
    use adafruit_feather_rp2040::{
        hal,
        hal::{
            prelude::*,
            clocks::init_clocks_and_plls,
            Clock,
            watchdog::Watchdog,
            Sio,
            gpio,
            spi,
            i2c,
        },
        Pins,
        XOSC_CRYSTAL_FREQ,
    };
    
    use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};

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

    // SdCard imports
    use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};
    use embedded_sdmmc::sdcard::DummyCsPin;
    use embedded_sdmmc::filesystem::Mode;
    use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};


    // Dummy timesource for creating files
    #[derive(Default)]
    pub struct DummyTimesource();

    impl TimeSource for DummyTimesource {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp {
                year_since_1970: 0,
                zero_indexed_month: 0,
                zero_indexed_day: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            }
        }
    }

    type LedPin = gpio::Pin<
        gpio::bank0::Gpio13,
        gpio::FunctionSioOutput,
        gpio::PullDown>;

    type LeakDetectorPin = gpio::Pin<
        gpio::bank0::Gpio24,
        gpio::FunctionSioInput,
        gpio::PullDown>;


    type Spi0Bus = hal::Spi<
        spi::Enabled,
        hal::pac::SPI0,
        (   gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSpi, gpio::PullNone>,
            gpio::Pin<gpio::bank0::Gpio20, gpio::FunctionSpi, gpio::PullUp>,
            gpio::Pin<gpio::bank0::Gpio18, gpio::FunctionSpi, gpio::PullNone>)>;
    
    type SdCardReader = SdCard<
        ExclusiveDevice<
            Spi0Bus,
            DummyCsPin,
            NoDelay>,
        gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
        hal::Timer>;

    type SdCardVolumeMgr = embedded_sdmmc::VolumeManager<
        SdCardReader, 
        DummyTimesource, 
        4, 4, 1>;

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
        lis3dh::Lis3dhI2C<i2c::I2C<hal::pac::I2C1,
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
        led_pin: LedPin,
        leak_detector_pin: LeakDetectorPin,
        display: OledDisplay,
        pixel: (u8, u8),
        dirs: (bool, bool),
        // imu:  Lis3dhIMU,
        // imu_tracker: Tracker,
        sd_card_volume_mgr: SdCardVolumeMgr,
    }

    // Systick magic
    use systick_monotonic::ExtU64;
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    /**************************************************************************
    Init routine
    **************************************************************************/
    #[init]
    fn init(cx: init::Context) -> (DataShared, DataLocal, init::Monotonics) {

        info!("initializing");
        
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

        // GPIO setup -------------------------------------------------------------------
        let sio = Sio::new(cx.device.SIO);
        // initialize the pins to default state
        let pins = Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );
        let led_pin = pins.d13.into_push_pull_output();
        let leak_detector_pin = pins.d24.into_pull_down_input();

        

        // Peripheral setup -------------------------------------------------------------

        // I2C
        let scl = pins.scl.into_function::<gpio::FunctionI2C>();
        let sda = pins.sda.into_function::<gpio::FunctionI2C>();
        let i2c1 = i2c::I2C::i2c1(
            cx.device.I2C1,
            sda,
            scl,
            fugit::RateExtU32::kHz(400),
            &mut resets,
            &clocks.system_clock,
        );

        // OLED display
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


        // Set up SPI and SD Card reader
        let spi_sclk: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.sclk.reconfigure();
        let spi_mosi: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.mosi.reconfigure();
        let spi_miso: gpio::Pin<_, gpio::FunctionSpi, gpio::PullUp> = pins.miso.reconfigure();
        let spi_cs = pins.d25.into_push_pull_output();
        
        let spi_bus = spi::Spi::<_, _, _, 8>::new(cx.device.SPI0, (spi_mosi, spi_miso, spi_sclk));

        let spi_bus = spi_bus.init(
            &mut resets,
            clocks.peripheral_clock.freq(),
            400.kHz(), // card initialization happens at low baud rate
            embedded_hal::spi::MODE_0,
        );

        let spi_device = ExclusiveDevice::new(spi_bus, DummyCsPin, NoDelay);

        let mut delay = rp2040_hal::Timer::new(
            cx.device.TIMER,
            &mut resets,
            &clocks,
        );
        

        let sdcard = SdCard::new(spi_device, spi_cs, delay);
        let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());
        
        info!("Init SD card controller and retrieve card size...");
        match volume_mgr.device().num_bytes() {
            Ok(size) => info!("card size is {} bytes", size),
            Err(e) => {
                error!("Error retrieving card size: {}", defmt::Debug2Format(&e));
            }
        }

        volume_mgr
            .device()
            .spi(|spi_device| spi_device.bus_mut().set_baudrate(clocks.peripheral_clock.freq(), 16.MHz()));

        

        // Task setup -------------------------------------------------------------------
        info!("starting");
        blink::spawn(5).unwrap();

        // Start OLED animation
        update_oled::spawn_after(2000.millis()).unwrap();

        // Start heartbeat
        heartbeat::spawn_after(3000.millis()).unwrap();
        
        // Start IMU communication
        //update_imu::spawn_after(1600.millis()).unwrap();

        // Test logging to SD card
        test_log::spawn_after(3200.millis()).unwrap();

        // Return the resources
        (
            DataShared {

            },
            DataLocal {
                led_pin: led_pin,
                leak_detector_pin: leak_detector_pin,
                display: display,
                pixel: (10, 25),
                dirs: (true, true),
                // imu: imu,
                // imu_tracker: imu_tracker,
                sd_card_volume_mgr: volume_mgr,
            },
            init::Monotonics(systick_monotonic::Systick::new(cx.core.SYST, 125_000_000)),
        )
    }


    #[task (local=[leak_detector_pin])]
    fn heartbeat(cx: heartbeat::Context) {
        // blink::spawn(2).unwrap();
        info!("heartbeat");
        if cx.local.leak_detector_pin.is_low().unwrap() {
            info!("leak pin low");
            blink::spawn(1).unwrap();
        } else {
            info!("leak pin high");
            blink::spawn(2).unwrap();
        }
        heartbeat::spawn_after(1000.millis()).unwrap();
    }

    /**************************************************************************
    LED Blink Task
    **************************************************************************/
    const BLINK_DUR: u64 = 120;  // = on_time = off_time (in ms)
    #[task(local = [led_pin])]
    fn blink(cx: blink::Context, n: u8) {
        if n == 0 {
            return;
        } else if cx.local.led_pin.is_set_low().unwrap() {
            cx.local.led_pin.set_high().unwrap();
            blink::spawn_after(BLINK_DUR.millis(), n).unwrap();
        } else {
            cx.local.led_pin.set_low().unwrap();
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
    //     //info!("IMU orientation:");
    //     info!(match {
    //         LandscapeUp     => "LandscapeUp\n",
    //         LandscapeDown   => "LandscapeDown\n",
    //         PortraitUp      => "PortraitUp\n",
    //         PortraitDown    => "PortraitDown\n",
    //         _ => "unknown\n"
    //     });
    //     update_imu::spawn_after(1000.millis()).unwrap();
    // }



    // Write a test message to a log file via the volume manager.
    #[task(local = [sd_card_volume_mgr])]
    fn test_log(cx: test_log::Context) {
        info!("Getting Volume 0...");
        let mut volume = match cx.local.sd_card_volume_mgr.open_volume(VolumeIdx(0)) {
            Ok(v) => v,
            Err(e) => {
                error!("Error getting volume 0: {}", defmt::Debug2Format(&e));
                loop{}
            }
        };

        let mut dir = match volume.open_root_dir() {
            Ok(dir) => dir,
            Err(e) => {
                error!("Error opening root dir: {}", defmt::Debug2Format(&e));
                loop{}
            }
        };

        let _file = match dir.open_file_in_dir("log.txt", Mode::ReadWriteCreateOrTruncate) {
            Ok(mut file) => {
                file.write(b"test log data").unwrap();
                info!("Wrote successfully to file!");
            }
            Err(e) => {
                error!("Error opening file 'log.txt': {}", defmt::Debug2Format(&e));
                loop{}
            }
        };
    }

} // mod app