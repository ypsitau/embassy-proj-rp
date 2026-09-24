pub fn calc_divider_and_top(clk_sys_freq: u32, freq: u32) -> (f32, u16) {
    let mut divider_rough: u32 = 1;
    let mut top_rtn: u16 = u16::MAX;
    if freq == 0 { return (divider_rough as f32, top_rtn); }
    let freqdiv: f32 = clk_sys_freq as f32 / freq as f32;
    let mut freqdiff_min: u32 = u32::MAX;
    for divider in 1..=256_u32 {
        let top_plus_1: u32 = (freqdiv / divider as f32) as u32;
        if !(1..=65_536).contains(&top_plus_1) { continue; }
        let top: u16 = (top_plus_1 - 1) as u16;
        let freq_actual: u32 = clk_sys_freq / (divider * top_plus_1);
        let freqdiff: u32 = freq_actual.abs_diff(freq);
        if freqdiff_min > freqdiff {
            freqdiff_min = freqdiff;
            divider_rough = divider;
            top_rtn = top;
        }
    }
    let mut divider_rtn: f32 = divider_rough as f32;
    for divider_frac in 0..=15_u32 {
        let divider: f32 = divider_rough as f32 + divider_frac as f32 / 16.0;
        let top_plus_1: u32 = (freqdiv / divider) as u32;
        if !(1..=65_536).contains(&top_plus_1) { continue; }
        let top: u16 = (top_plus_1 - 1) as u16;
        let freq_actual: u32 = (clk_sys_freq as f32 / (divider * top_plus_1 as f32)) as u32;
        let freqdiff: u32 = freq_actual.abs_diff(freq);
        if freqdiff_min > freqdiff {
            freqdiff_min = freqdiff;
            divider_rtn = divider;
            top_rtn = top;
        }
    }
    (divider_rtn, top_rtn)
}

#[allow(dead_code)]
pub fn calc_freq(clk_sys_freq: u32, divider: f32, top: u16) -> u32 {
    let divisor = divider * (u32::from(top) + 1) as f32;
    (clk_sys_freq as f32 / divisor) as u32
}
