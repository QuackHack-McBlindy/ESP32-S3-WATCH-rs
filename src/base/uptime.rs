// BASE/UPTIME


fn format_uptime(dur: embassy_time::Duration) -> heapless::String<16> {
    let total_secs = dur.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    heapless::String::from_iter(format_args!("{:02}D:{:02}H:{:02}M", days, hours, minutes))
}
