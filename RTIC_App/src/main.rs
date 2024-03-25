#![no_std]
#![no_main]

use rtic::app;
use defmt_rtt as _;
use panic_halt as _;

// Second-stage bootloader ------------------------------------------------------------------------
#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

// RTIC App ---------------------------------------------------------------------------------------
#[app(device = rp2040_hal::pac, peripherals = true, dispatchers = [RTC_IRQ])]
mod app {
    // Debug imports
    use defmt::{trace, info, warn, error};

    // Peripheral sharing imports
    use core::cell::RefCell;
    use critical_section::Mutex;
    use embedded_hal_bus::i2c::CriticalSectionDevice as I2cCriticalSectionDev;

    // Board specific imports
    use rp2040_hal as hal;
    use rp2040_hal::pac as pac;

    const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

    use hal::{
        Clock,
        clocks::init_clocks_and_plls,
        watchdog::Watchdog,
        Sio,
        gpio::bank0::{
            Gpio2,      // Sda
            Gpio3,      // Scl
            Gpio13,     // LED
            Gpio18,     // SPI0 SCK
            Gpio19,     // SPI0 MOSI
            Gpio20,     // SPI0 MISO
            Gpio24,     // Leak Detector
            Gpio25,     // SPI0 CS (custom)
        },
        gpio::{
            Pin,
            FunctionI2C,
            FunctionSpi,
            FunctionSioInput,
            FunctionSioOutput,
            PullUp,
            PullDown,
            PullNone,
        },
        i2c::I2C,
        spi::Spi,
        // pio::PIOExt,
    };

    // GPIO pin imports
    use embedded_hal::digital::{
        InputPin,
        OutputPin,
        StatefulOutputPin
    };

    // GPIO pin type aliases
    type LedPin = Pin<
        Gpio13,
        FunctionSioOutput,
        PullDown>;

    type LeakDetectorPin = Pin<
        Gpio24,
        FunctionSioInput,
        PullDown>;

    // Timing imports
    use fugit::RateExtU32;

    // OLED imports
    use sh1107::{
        prelude::*,
        mode::GraphicsMode,
        interface::i2c::I2cInterface,
        Builder
    };

    use embedded_graphics::{
        mono_font::{
            ascii::FONT_6X9 as font,
            MonoTextStyleBuilder,
        },
        primitives::{
            Line,
            PrimitiveStyle,
        },
        pixelcolor::BinaryColor,
        prelude::*,
        text::Text,
    };

    // OLED type aliases
    type Scl = Pin<Gpio3, FunctionI2C, PullUp>;
    type Sda = Pin<Gpio2, FunctionI2C, PullUp>;
    type I2cWithPins = I2C<pac::I2C1, (Sda, Scl)>;
    type OledDisplay = GraphicsMode<I2cInterface<I2cCriticalSectionDev<'static, I2cWithPins>>>;

    // IMU imports
    use lis3dh::{Lis3dh, Lis3dhI2C, SlaveAddr};
    use lis3dh::accelerometer::Accelerometer;

    // IMU type aliases
    type I2cBus = Mutex<RefCell<I2cWithPins>>;
    type Lis3dhAccelerometer = Lis3dh<Lis3dhI2C<I2cCriticalSectionDev<'static, I2cWithPins>>>;


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

    // SdCard type aliases
    type Spi0Bus = hal::Spi<
        hal::spi::Enabled,
        hal::pac::SPI0,
        (   Pin<Gpio19, FunctionSpi, PullNone>,
            Pin<Gpio20, FunctionSpi, PullUp>,
            Pin<Gpio18, FunctionSpi, PullNone>)
    >;
    type SdCardReader = SdCard<
        ExclusiveDevice<
            Spi0Bus,
            DummyCsPin,
            NoDelay>,
        Pin<Gpio25, FunctionSioOutput, PullDown>,
        hal::Timer
    >;
    type SdCardVolumeMgr = embedded_sdmmc::VolumeManager<
        SdCardReader, 
        DummyTimesource, 
        4, 4, 1
    >;



    // Resources ----------------------------------------------------------------------------------
    #[shared]
    struct DataShared {
        accel: Option<Lis3dhAccelerometer>,
        display: Option<OledDisplay>,
    }

    #[local]
    struct DataLocal {
        // Components
        led_pin: LedPin,
        leak_detector_pin: LeakDetectorPin,
        pixel: (u8, u8),
        dirs: (bool, bool),
        sd_card_volume_mgr: Option<SdCardVolumeMgr>,
    }

    // Systick magic
    use systick_monotonic::ExtU64;
    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    // Init function ------------------------------------------------------------------------------
    #[init(local = [i2c_bus: Option<I2cBus> = None])]
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

        // GPIO setup -----------------------------------------------------------------------------
        let sio = Sio::new(cx.device.SIO);
        // initialize the pins to default state
        let pins = hal::gpio::Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );
        let led_pin = pins.gpio13.into_push_pull_output();
        let leak_detector_pin = pins.gpio24.into_pull_down_input();

        // Peripheral setup -----------------------------------------------------------------------

        // I2C
        let scl = pins.gpio3.into_function::<FunctionI2C>().into_pull_type::<PullUp>();
        let sda = pins.gpio2.into_function::<FunctionI2C>().into_pull_type::<PullUp>();
        let i2c1 = I2C::i2c1(
            cx.device.I2C1,
            sda,
            scl,
            fugit::RateExtU32::kHz(400),
            &mut resets,
            &clocks.system_clock,
        );
        let i2c_bus: &'static _ =
            cx.local.i2c_bus.insert(
                critical_section::Mutex::new(RefCell::new(i2c1))
            );

        // OLED display
        let oled_i2c = I2cCriticalSectionDev::new(i2c_bus);
        let display_size = DisplaySize::Display64x128;
        let display_rot  = DisplayRotation::Rotate270;
        let mut disp: GraphicsMode<_> = Builder::new()
            .with_size(display_size)
            .with_rotation(display_rot)
            .connect_i2c(oled_i2c)
            .into();
        let mut display: Option<OledDisplay> = None;
        match disp.init() {
            Ok(_)   => { trace!("OLED init succeeded"); display = Some(disp); },
            Err(e)  => error!("OLED init failed: {}", defmt::Debug2Format(&e)),
        }

        // IMU
        let lis3dh_i2c = I2cCriticalSectionDev::new(i2c_bus);
        let mut lis3dh: Option<Lis3dhAccelerometer> = None;
        match Lis3dh::new_i2c(lis3dh_i2c, SlaveAddr::Default) {
            Ok(imu) => { trace!("IMU init suceeded"); lis3dh = Some(imu) },
            Err(e)  => error!("IMU init failed: {}", defmt::Debug2Format(&e)),
        }
        if let Some(ref mut imu) = lis3dh {
            match imu.set_range(lis3dh::Range::G8) {
                Ok(_)   => trace!("IMU set range succeeded"),
                Err(e)  => error!("IMU set range failed: {}", defmt::Debug2Format(&e))
            };
        }

        // SPI
        let spi_sclk: Pin<_, FunctionSpi, PullNone> = pins.gpio18.reconfigure();
        let spi_mosi: Pin<_, FunctionSpi, PullNone> = pins.gpio19.reconfigure();
        let spi_miso: Pin<_, FunctionSpi, PullUp> = pins.gpio20.reconfigure();
        let spi_cs = pins.gpio25.into_push_pull_output();

        let spi_bus = Spi::<_, _, _, 8>::new(cx.device.SPI0, (spi_mosi, spi_miso, spi_sclk));

        let spi_bus = spi_bus.init(
            &mut resets,
            clocks.peripheral_clock.freq(),
            400.kHz(), // card initialization happens at low baud rate
            embedded_hal::spi::MODE_0,
        );

        let spi_device = ExclusiveDevice::new(spi_bus, DummyCsPin, NoDelay);

        let delay = rp2040_hal::Timer::new(
            cx.device.TIMER,
            &mut resets,
            &clocks,
        );

        // SD Card reader
        let sdcard = SdCard::new(spi_device, spi_cs, delay);
        let mut v_mgr = VolumeManager::new(sdcard, DummyTimesource::default());
        let mut volume_mgr: Option<SdCardVolumeMgr> = None;
        
        match v_mgr.device().num_bytes() {
            Ok(size) => { info!("SD card size is {} bytes", size); volume_mgr = Some(v_mgr) },
            Err(e) => {
                error!("Error retrieving card size: {}", defmt::Debug2Format(&e));
            }
        }

        if let Some(ref mut volume_mgr) = volume_mgr {
            volume_mgr
                .device()
                .spi(|spi_device| spi_device.bus_mut().set_baudrate(clocks.peripheral_clock.freq(), 16.MHz()));
        }

        // Task setup -----------------------------------------------------------------------------
        info!("Spawning tasks...");
        blink::spawn(5).unwrap();

        // Start IMU communication
        update_imu::spawn().unwrap();

        // Start OLED animation
        update_oled::spawn_after(2000.millis()).unwrap();
        
        // Start heartbeat
        heartbeat::spawn_after(3000.millis()).unwrap();

        // Test logging to SD card
        test_log::spawn_after(3200.millis()).unwrap();

        // Return the resources
        (
            DataShared {
                accel: lis3dh,
                display: display,
            },
            DataLocal {
                led_pin: led_pin,
                leak_detector_pin: leak_detector_pin,
                pixel: (10, 25),
                dirs: (true, true),
                sd_card_volume_mgr: volume_mgr,
            },
            init::Monotonics(systick_monotonic::Systick::new(cx.core.SYST, 125_000_000)),
        )
    }


   
    // Heartbeat task -----------------------------------------------------------------------------
    #[task (local=[leak_detector_pin])]
    fn heartbeat(cx: heartbeat::Context) {
        trace!("heartbeat");
        if cx.local.leak_detector_pin.is_low().unwrap() {
            trace!("leak pin low");
            blink::spawn(1).unwrap();
        } else {
            warn!("leak pin high! leak detected!");
            blink::spawn(2).unwrap();
        }
        heartbeat::spawn_after(1000.millis()).unwrap();
    }

    // LED blink task -----------------------------------------------------------------------------
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

    // OLED update task ---------------------------------------------------------------------------
    const FEATHER_DISPLAY_SIZE: Size = Size::new(128, 64);
    const START_POSITION_1: Point = Point::new(0, font.character_size.height as i32 - 3);
    const SEPARATOR_POSITION: Point = Point::new(START_POSITION_1.x, START_POSITION_1.y + 4);
    const START_POSITION_2: Point = Point::new(SEPARATOR_POSITION.x, SEPARATOR_POSITION.y + font.character_size.height as i32);
    const START_POSITION_3: Point = Point::new(START_POSITION_2.x, START_POSITION_2.y + font.character_size.height as i32);
    
    #[task(shared = [display], local = [pixel, dirs])]
    fn update_oled(cx: update_oled::Context) {
        // Change these values to view the interface prototypes //
        let num_can_devices: u8 = 1;                            //
        let leak_detected: bool = true;                         //
        // ---------------------------------------------------- //

        // 0 = nominal, 1 = warn, 2 = err
        let mut system_state = 0;
        if num_can_devices == 0 { system_state = 1;}
        if leak_detected { system_state = 2; }

        // Font styles
        let normal = MonoTextStyleBuilder::new()
            .font(&font)
            .text_color(BinaryColor::On)
            .build();

        let underline = MonoTextStyleBuilder::from(&normal)
            .underline()
            .build();

        let highlight = MonoTextStyleBuilder::from(&normal)
            .background_color(BinaryColor::On)
            .text_color(BinaryColor::Off)
            .build();


        let mut d = cx.shared.display;
        d.lock(|d_l| {
            if let Some(ref mut d_l) = d_l {
                
                // Clear the display buffer
                d_l.clear();

                // Process system state
                let mut position = Point::new(0, 0);
                match Text::new("Status: ", START_POSITION_1, normal).draw(d_l) {
                    Ok(pos) => position = pos,
                    Err(e) => error!("{}", defmt::Debug2Format(&e))
                };
                match system_state {
                    0 => match Text::new("nominal", position, normal).draw(d_l) { Ok(_) => (), Err(e) => error!("{}", defmt::Debug2Format(&e))},
                    1 => match Text::new("warning", position, underline).draw(d_l) { Ok(_) => (), Err(e) => error!("{}", defmt::Debug2Format(&e))},
                    2 => match Text::new("critical", position, underline).draw(d_l) { Ok(_) => (), Err(e) => error!("{}", defmt::Debug2Format(&e))},
                    _ => match Text::new("", position, normal).draw(d_l) { Ok(_) => (), Err(e) => error!("{}", defmt::Debug2Format(&e))},
                };

                // Draw a separating line
                match Line::new(
                    SEPARATOR_POSITION, 
                    Point::new(SEPARATOR_POSITION.x + FEATHER_DISPLAY_SIZE.width as i32, SEPARATOR_POSITION.y))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(d_l) {
                    Ok(_) => (),
                    Err(e) => error!("{}", defmt::Debug2Format(&e)),
                };

                // List the number of CAN devices
                // Note we expect to have 3-4 devices in practice, so fully supporting up to 10 (with partial support for more) is sufficient
                match Text::new("CAN devices: ", START_POSITION_2, normal).draw(d_l) {
                    Ok(pos) => position = pos,
                    Err(e) => error!("{}", defmt::Debug2Format(&e)),
                }

                let can_dev_msg = match num_can_devices {
                    0 => "0", 1 => "1", 2 => "2", 3 => "3", 4 => "4", 5 => "5", 6 => "6", 7 => "7", 8 => "8", 9 => "9", 10 => "10",
                    _ => ">10"
                };
                match Text::new(can_dev_msg, position, normal).draw(d_l) {
                    Ok(_) => (),
                    Err(e) => error!("{}", defmt::Debug2Format(&e)),
                }
            
                if leak_detected {
                    match Text::new("LEAK DETECTED!", START_POSITION_3, highlight).draw(d_l) {
                            Ok(pos) => position = pos,
                            Err(e) => error!("{}", defmt::Debug2Format(&e)),
                        }
                    }

                match d_l.flush() {
                    Ok(_) => { trace!("OLED flush succeeded"); },
                    Err(e) => {error!("OLED flush failed: {}", defmt::Debug2Format(&e))},
                };
            }
        });
        update_oled::spawn_after(50.millis()).unwrap();
    }


    // IMU update task ----------------------------------------------------------------------------
    #[task(shared = [accel])]
    fn update_imu(cx: update_imu::Context) {
        let mut accel = cx.shared.accel;
        accel.lock(|accel_l| {
            if let Some(ref mut accel_l) = accel_l {
                match accel_l.accel_norm() {
                    Ok(accel_vec)   => info!("accel x is now: {:?}", accel_vec.x),
                    Err(e)  => warn!("unable to read accel from IMU: {}", defmt::Debug2Format(&e)),
                }
            }
        });
        update_imu::spawn_after(1000u64.millis()).unwrap();
    }

    // Write a test message to a log file via the volume manager. ---------------------------------
    #[task(local = [sd_card_volume_mgr])]
    fn test_log(cx: test_log::Context) {
        trace!("Getting SD card volume 0...");
        if let Some(ref mut volume_mgr) = cx.local.sd_card_volume_mgr {
            let mut volume = match volume_mgr.open_volume(VolumeIdx(0)) {
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
                    match file.write(b"test log data") {
                        Ok(_) => info!("Wrote successfully to file!"),
                        Err(e) => error!("{}", defmt::Debug2Format(&e)),
                    }
                }
                Err(e) => {
                    error!("Error opening file 'log.txt': {}", defmt::Debug2Format(&e));
                    loop{}
                }
            };
        }
        
    }
} // mod app
