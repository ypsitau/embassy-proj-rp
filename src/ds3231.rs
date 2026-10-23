use embedded_hal_async as hal_async;

pub struct Driver<I2C> {
    i2c: I2C,
}

impl<I2C: hal_async::i2c::I2c> Driver<I2C> {
    pub const I2C_ADDR: u8 = 0x68;
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }
    pub async fn read(&mut self) -> Result<(u16, u8, u8, u8, u8, u8), I2C::Error> {
        fn bcd_to_dec(value: u8) -> u8 { (value >> 4) * 10 + (value & 0x0f) }
        let mut regs = [0u8; 7];
        self.i2c.write_read(Self::I2C_ADDR, &[0x00], &mut regs).await?;
        let year = 2000 + bcd_to_dec(regs[6]) as i32;
        let month = time::Month::try_from(bcd_to_dec(regs[5])).unwrap();
        let day = bcd_to_dec(regs[4]); 
        let hour = bcd_to_dec(regs[2]);
        let minute = bcd_to_dec(regs[1]);
        let second = bcd_to_dec(regs[0] & 0x7f);
        Ok((year as u16, month as u8, day, hour, minute, second))
    }
}
