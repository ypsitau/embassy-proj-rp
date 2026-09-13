{% case template -%}
{% when "GPIO In/Out" -%}
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time as time;
use embedded_hal_1 as hal;
use embedded_hal_async as hal_async;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let (pin_sw, pin_led) = {
        let p = rp::init(Default::default());
        let pin_sw = rp::gpio::Input::new(p.PIN_15, rp::gpio::Pull::Up);
        let pin_led = rp::gpio::Output::new(p.PIN_25, rp::gpio::Level::Low);
        (pin_sw, pin_led)
    };
    let fut_task_pin_in_out = task_pin_in_out(pin_sw, pin_led);
    fut_task_pin_in_out.await;
}

async fn task_pin_in_out<InputPin, OutputPin>(mut pin_sw: InputPin, mut pin_led: OutputPin)
where
    InputPin: hal::digital::InputPin + hal_async::digital::Wait,
    OutputPin: hal::digital::OutputPin,
{
    loop {
        let is_pushed = pin_sw.is_low().unwrap();
        if is_pushed {
            pin_led.set_high().ok();
        } else {
            pin_led.set_low().ok();
        }
        pin_sw.wait_for_any_edge().await.ok();
        time::Timer::after_millis(30).await;    // debounce delay
    }
}
{% when "USB device CDC" -%}
#![no_std]
#![no_main]

use core::sync::atomic;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_usb as usb;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => rp::usb::InterruptHandler<rp::peripherals::USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let usb_driver = {
        // Raspberry Pi Pico initialization
        let p = rp::init(Default::default());
        rp::usb::Driver::new(p.USB, Irqs)
    };
    // Launch a task
    let fut_task_usb = task_usb(usb_driver);
    fut_task_usb.await;
}

//-----------------------------------------------------------------------------
// USB task
//-----------------------------------------------------------------------------
async fn task_usb(usb_driver: impl usb::driver::Driver<'static>) {
    let mut usb_builder = {
        const VID: u16 = 0xc0de;
        const PID: u16 = 0xcafe;
        const CONFIG_DESCRIPTOR_SIZE: usize = 256;
        const BOS_DESCRIPTOR_SIZE: usize = 256;
        const MSOS_DESCRIPTOR_SIZE: usize = 256;
        const CONTROL_BUF_SIZE: usize = 64;
        let mut usb_config = usb::Config::new(VID, PID);
        usb_config.manufacturer = Some("Embassy");
        usb_config.product = Some("hoge");
        usb_config.serial_number = Some("12345678");
        usb_config.max_power = 100;
        usb_config.max_packet_size_0 = CONTROL_BUF_SIZE as u8;
        let config_descriptor_buf = {
            static STATIC_CELL: StaticCell<[u8; CONFIG_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; CONFIG_DESCRIPTOR_SIZE])
        };
        let bos_descriptor_buf = {
            static STATIC_CELL: StaticCell<[u8; BOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; BOS_DESCRIPTOR_SIZE])
        };
        let msos_descriptor_buf = {
            static STATIC_CELL: StaticCell<[u8; MSOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; MSOS_DESCRIPTOR_SIZE])
        };
        let control_buf = {
            static STATIC_CELL: StaticCell<[u8; CONTROL_BUF_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0u8; CONTROL_BUF_SIZE])
        };
        let usb_handler = {
            static STATIC_CELL: StaticCell<USBHandler> = StaticCell::new();
            STATIC_CELL.init(USBHandler::new())
        };
        let mut usb_builder = usb::Builder::new(usb_driver, usb_config,
            config_descriptor_buf, bos_descriptor_buf, msos_descriptor_buf, control_buf);
        usb_builder.handler(usb_handler);
        usb_builder
    };
    let cdc_driver = {
        let state = {
            static STATE: StaticCell<usb::class::cdc_acm::State> = StaticCell::new();
            STATE.init(usb::class::cdc_acm::State::new())
        };
        let max_packet_size = 64;
        usb::class::cdc_acm::CdcAcmClass::new(&mut usb_builder, state, max_packet_size)
    };
    let mut usb_device = usb_builder.build();
    let fut_usb = usb_device.run();
    let fut_echo = async {
        let (mut cdc_sender, mut cdc_receiver) = cdc_driver.split();
        let buf = {
            static STATIC_CELL: StaticCell<[u8; 64]> = StaticCell::new();
            STATIC_CELL.init([0u8; 64])
        };
        loop {
            cdc_receiver.wait_connection().await;
            info!("Connected");
            let e = loop {
                let buf_read = match cdc_receiver.read_packet(buf).await {
                    Ok(n) => &buf[..n], Err(e) => break e,
                };
                if let Err(e) = cdc_sender.write_packet(buf_read).await { break e; }
            };
            if e != usb::driver::EndpointError::Disabled { break; }
        };
    };
    embassy_futures::join::join(fut_usb, fut_echo).await;
}

//-----------------------------------------------------------------------------
// USBHandler
//-----------------------------------------------------------------------------
struct USBHandler {
    configured: atomic::AtomicBool,
}

impl USBHandler {
    fn new() -> Self {
        USBHandler { configured: atomic::AtomicBool::new(false), }
    }
}

impl usb::Handler for USBHandler {
    /// Called when the USB device has been enabled or disabled.
    fn enabled(&mut self, enabled: bool) {
        info!("usb::Handler.enabled({})", enabled);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called after a USB reset after the bus reset sequence is complete.
    fn reset(&mut self) {
        info!("usb::Handler.reset()");
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has set the address of the device to `addr`.
    fn addressed(&mut self, addr: u8) {
        info!("usb::Handler.addressed(addr: {})", addr);
        self.configured.store(false, atomic::Ordering::Relaxed);
    }
    /// Called when the host has enabled or disabled the configuration of the device.
    fn configured(&mut self, configured: bool) {
        info!("usb::Handler.configured(configured: {})", configured);
        self.configured.store(configured, atomic::Ordering::Relaxed);
    }
}
{% endcase -%}
