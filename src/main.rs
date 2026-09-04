#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
{% if use_usb_driver -%}
use embassy_usb as usb;
{% endif -%}
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
{% if use_usb_driver %}
rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => rp::usb::InterruptHandler<rp::peripherals::USB>;
});
{% endif %}
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    {% if use_usb_driver -%}
    let usb_driver = rp::usb::Driver::new(p.USB, Irqs);
    let mut usb_builder = {
        const VID: u16 = 0xc0de;
        const PID: u16 = 0xcafe;
        const CONFIG_DESCRIPTOR_SIZE: usize = 256;
        const BOS_DESCRIPTOR_SIZE: usize = 256;
        const MSOS_DESCRIPTOR_SIZE: usize = 256;
        const CONTROL_BUF_SIZE: usize = 64;
        let mut config = usb::Config::new(VID, PID);
        config.manufacturer = Some("Embassy");
        config.product = Some("{{project-name}}");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = CONTROL_BUF_SIZE as u8;
        let config_descriptor_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; CONFIG_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; CONFIG_DESCRIPTOR_SIZE])
        };
        let bos_descriptor_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; BOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; BOS_DESCRIPTOR_SIZE])
        };
        let msos_descriptor_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; MSOS_DESCRIPTOR_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; MSOS_DESCRIPTOR_SIZE])
        };
        let control_buf = { // should be replaced by make_static macro when it becomes available
            static STATIC_CELL: StaticCell<[u8; CONTROL_BUF_SIZE]> = StaticCell::new();
            STATIC_CELL.init([0; CONTROL_BUF_SIZE])
        };
        //let device_handler = { // should be replaced by make_static macro when it becomes available
        //    static STATIC_CELL: StaticCell<DeviceHandler> = StaticCell::new();
        //    STATIC_CELL.init(DeviceHandler::new())
        //};
        let usb_builder = usb::Builder::new(usb_driver, config,
            config_descriptor_buf, bos_descriptor_buf, msos_descriptor_buf, control_buf);
        //usb_builder.handler(device_handler);
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
        let mut first = true;
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
                if first {
                    if let Err(e) = cdc_sender.write_packet(b"\r\nEcho via USB\r\n").await { break e; }
                    first = false;
                }
                if let Err(e) = cdc_sender.write_packet(buf_read).await { break e; }
            };
            if e != usb::driver::EndpointError::Disabled { break; }
        };
    };
    {% endif -%}
    let fut_gpio = async {
        let mut gpio_led = rp::gpio::Output::new(p.PIN_25, rp::gpio::Level::Low);
        loop {
            info!("led on!");
            gpio_led.set_high();
            Timer::after_secs(1).await;
            info!("led off!");
            gpio_led.set_low();
            Timer::after_secs(1).await;
        }
    };
    {% if use_usb_driver -%}
    embassy_futures::join::join3(fut_usb, fut_echo, fut_gpio).await;
{% else -%}
    fut_gpio.await;
{% endif -%}
}
