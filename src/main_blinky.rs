#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let gpio_led = {
        let p = rp::init(Default::default());
        rp::gpio::Output::new(p.PIN_25, rp::gpio::Level::Low)
    };
    let fut_task_blinky = task_blinky(gpio_led);
    fut_task_blinky.await;
}

async fn task_blinky(mut gpio_led: impl embedded_hal_1::digital::OutputPin) {
    loop {
        gpio_led.set_high().ok();
        Timer::after_secs(1).await;
        gpio_led.set_low().ok();
        Timer::after_secs(1).await;
    }
}
