pub mod time;

pub(crate) fn percentage(value: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }

    let percent = (u128::from(value) * 100 / u128::from(total)).min(100);
    u8::try_from(percent).unwrap_or(100)
}
