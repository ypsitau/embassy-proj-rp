use defmt::info;
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
    info!("Starting task_pin_in_out");
    loop {
        let is_pushed = pin_sw.is_low().unwrap();
        if is_pushed {
            info!("Button pressed");
            pin_led.set_high().ok();
        } else {
            info!("Button released");
            pin_led.set_low().ok();
        }
        pin_sw.wait_for_any_edge().await.ok();
        time::Timer::after_millis(30).await;    // debounce delay
    }
}