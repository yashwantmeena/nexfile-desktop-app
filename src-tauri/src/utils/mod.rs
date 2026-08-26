pub(crate) fn percentage(value: i64, total: i64) -> u8 {
    if value <= 0 || total <= 0 {
        return 0;
    }

    let percent = (i128::from(value) * 100 / i128::from(total)).min(100);
    u8::try_from(percent).unwrap_or(100)
}
