#[inline]
pub fn canonicalize_zero(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else {
        x
    }
}

const CYCLE_COMPACT_SUFFIXES: [&str; 8] = ["k", "M", "G", "T", "P", "E", "Z", "Y"];

fn cycles_si_tier(abs: f64) -> usize {
    let mut tier = 0usize;
    let mut threshold = 1000.0;
    while tier < CYCLE_COMPACT_SUFFIXES.len() && abs >= threshold {
        tier += 1;
        threshold *= 1000.0;
    }
    tier
}

pub fn fmt_cycles(value: f64) -> String {
    let v = canonicalize_zero(value);
    let abs = v.abs();

    if abs < 1_000.0 {
        return format!("{v:.0}");
    }

    let tier = cycles_si_tier(abs);
    let divisor = 1000.0_f64.powi(tier as i32);
    let scaled = v / divisor;
    let suffix = CYCLE_COMPACT_SUFFIXES[tier - 1];
    format!("{scaled:.1}{suffix}")
}

pub fn fmt_cycles_rate(value: f64) -> String {
    let value = canonicalize_zero(value);
    let abs = value.abs();
    if abs >= 1000.0 {
        return fmt_cycles(value);
    }
    if abs.fract().abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

const STORAGE_UNIT_LABELS: [&str; 6] = ["MB", "GB", "TB", "PB", "EB", "ZB"];

fn storage_tier(abs_mb: f64) -> usize {
    let mut tier = 0usize;
    let mut threshold = 1024.0;
    while tier < 5 && abs_mb >= threshold {
        tier += 1;
        threshold *= 1024.0;
    }
    tier
}

pub fn fmt_bytes(value_mb: f64) -> String {
    let v = canonicalize_zero(value_mb);
    let abs = v.abs();
    let tier = storage_tier(abs);
    let divisor = 1024.0_f64.powi(tier as i32);
    let scaled = v / divisor;
    let label = STORAGE_UNIT_LABELS[tier];
    if tier == 0 && scaled.fract().abs() < 1e-9 {
        format!("{:.0} {label}", scaled)
    } else if tier == 0 {
        format!("{:.1} {label}", scaled)
    } else {
        format!("{:.1} {label}", scaled)
    }
}

const STORAGE_RATE_UNIT_LABELS: [&str; 8] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s", "PB/s", "EB/s", "ZB/s"];

pub fn fmt_bytes_rate(value_mb_per_s: f64) -> String {
    let bytes_per_s = canonicalize_zero(value_mb_per_s * 1024.0 * 1024.0);
    let abs = bytes_per_s.abs();
    let mut tier = 0usize;
    let mut threshold = 1024.0;
    while tier + 1 < STORAGE_RATE_UNIT_LABELS.len() && abs >= threshold {
        tier += 1;
        threshold *= 1024.0;
    }
    let divisor = 1024.0_f64.powi(tier as i32);
    let scaled = bytes_per_s / divisor;
    let label = STORAGE_RATE_UNIT_LABELS[tier];
    if scaled.fract().abs() < 1e-9 {
        format!("{scaled:.0} {label}")
    } else {
        format!("{scaled:.1} {label}")
    }
}

const BANDWIDTH_UNIT_LABELS: [&str; 8] = [
    "Mbps", "Gbps", "Tbps", "Pbps", "Ebps", "Zbps", "Ybps", "Ybps",
];

fn si_tier_from_base(abs: f64) -> usize {
    let mut tier = 0usize;
    let mut threshold = 1000.0;
    while tier + 1 < BANDWIDTH_UNIT_LABELS.len() && abs >= threshold {
        tier += 1;
        threshold *= 1000.0;
    }
    tier
}

pub fn fmt_bandwidth(mbps: f64) -> String {
    let v = canonicalize_zero(mbps);
    let abs = v.abs();
    let tier = si_tier_from_base(abs);
    let divisor = 1000.0_f64.powi(tier as i32);
    let scaled = v / divisor;
    let label = BANDWIDTH_UNIT_LABELS[tier];
    if tier == 0 && scaled.fract().abs() < 1e-9 {
        format!("{:.0} {label}", scaled)
    } else {
        format!("{:.1} {label}", scaled)
    }
}

const WATT_UNIT_LABELS: [&str; 8] = ["W", "kW", "MW", "GW", "TW", "PW", "YW", "YW"];

pub fn fmt_watts(watts: f64) -> String {
    let v = canonicalize_zero(watts);
    let abs = v.abs();
    let tier = si_tier_from_base(abs);
    let divisor = 1000.0_f64.powi(tier as i32);
    let scaled = v / divisor;
    let label = WATT_UNIT_LABELS[tier];
    if tier == 0 && scaled.fract().abs() < 1e-9 {
        format!("{:.0} {label}", scaled)
    } else {
        format!("{:.1} {label}", scaled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_values() {
        assert_eq!(fmt_cycles(0.0), "0");
        assert_eq!(fmt_cycles(1.0), "1");
        assert_eq!(fmt_cycles(999.0), "999");
    }

    #[test]
    fn thousands() {
        assert_eq!(fmt_cycles(1_000.0), "1.0k");
        assert_eq!(fmt_cycles(1_200.0), "1.2k");
        assert_eq!(fmt_cycles(12_000.0), "12.0k");
        assert_eq!(fmt_cycles(130_000.0), "130.0k");
    }

    #[test]
    fn millions() {
        assert_eq!(fmt_cycles(1_400_000.0), "1.4M");
        assert_eq!(fmt_cycles(20_000_000.0), "20.0M");
    }

    #[test]
    fn billions() {
        assert_eq!(fmt_cycles(1_000_000_000.0), "1.0G");
        assert_eq!(fmt_cycles(2_500_000_000.0), "2.5G");
    }

    #[test]
    fn cycles_extended_si_suffixes() {
        assert_eq!(fmt_cycles(2e12), "2.0T");
        assert_eq!(fmt_cycles(3e15), "3.0P");
        assert_eq!(fmt_cycles(4e18), "4.0E");
        assert_eq!(fmt_cycles(5e21), "5.0Z");
        assert_eq!(fmt_cycles(6e24), "6.0Y");
    }

    #[test]
    fn cycles_extreme_y_catchall() {
        let s = fmt_cycles(1e30);
        assert!(s.ends_with('Y'));
    }

    #[test]
    fn rate_small() {
        assert_eq!(fmt_cycles_rate(0.0), "0");
        assert_eq!(fmt_cycles_rate(12.3), "12.3");
        assert_eq!(fmt_cycles_rate(999.0), "999");
    }

    #[test]
    fn rate_compact() {
        assert_eq!(fmt_cycles_rate(1_000.0), "1.0k");
        assert_eq!(fmt_cycles_rate(1_200.0), "1.2k");
        assert_eq!(fmt_cycles_rate(1_400_000.0), "1.4M");
    }

    #[test]
    fn bytes_mb_gb_tb_pb() {
        assert_eq!(fmt_bytes(16.0), "16 MB");
        assert_eq!(fmt_bytes(1024.0), "1.0 GB");
        assert_eq!(fmt_bytes(1536.0), "1.5 GB");
        let mb_per_tb = 1024.0_f64.powi(2);
        assert_eq!(fmt_bytes(mb_per_tb), "1.0 TB");
        let mb_per_pb = 1024.0_f64.powi(3);
        assert_eq!(fmt_bytes(mb_per_pb), "1.0 PB");
        let mb_per_eb = 1024.0_f64.powi(4);
        assert_eq!(fmt_bytes(mb_per_eb), "1.0 EB");
        let mb_per_zb = 1024.0_f64.powi(5);
        assert_eq!(fmt_bytes(mb_per_zb), "1.0 ZB");
    }

    #[test]
    fn bytes_extreme_zb_catchall() {
        let huge_mb = 1024.0_f64.powi(6);
        let s = fmt_bytes(huge_mb);
        assert!(s.contains("ZB"));
    }

    #[test]
    fn bytes_rate_scales_small_medium_large() {
        assert_eq!(fmt_bytes_rate(0.0005), "524.3 B/s");
        assert_eq!(fmt_bytes_rate(0.5), "512 KB/s");
        assert_eq!(fmt_bytes_rate(5.0), "5 MB/s");
        assert_eq!(fmt_bytes_rate(2048.0), "2 GB/s");
    }

    #[test]
    fn bandwidth_tiers() {
        assert_eq!(fmt_bandwidth(10.0), "10 Mbps");
        assert_eq!(fmt_bandwidth(1500.0), "1.5 Gbps");
        assert_eq!(fmt_bandwidth(2_000_000.0), "2.0 Tbps");
        assert_eq!(fmt_bandwidth(3e9), "3.0 Pbps");
        assert_eq!(fmt_bandwidth(4e12), "4.0 Ebps");
        assert_eq!(fmt_bandwidth(5e15), "5.0 Zbps");
        assert_eq!(fmt_bandwidth(6e18), "6.0 Ybps");
    }

    #[test]
    fn bandwidth_extreme_last_unit_catchall() {
        let s = fmt_bandwidth(1e30);
        assert!(s.contains("Ybps"));
    }

    #[test]
    fn watts_tiers() {
        assert_eq!(fmt_watts(50.0), "50 W");
        assert_eq!(fmt_watts(1500.0), "1.5 kW");
        assert_eq!(fmt_watts(2_000_000.0), "2.0 MW");
        assert_eq!(fmt_watts(3e9), "3.0 GW");
        assert_eq!(fmt_watts(4e12), "4.0 TW");
        assert_eq!(fmt_watts(5e15), "5.0 PW");
        assert_eq!(fmt_watts(6e18), "6.0 YW");
    }

    #[test]
    fn watts_extreme_last_unit_catchall() {
        let s = fmt_watts(1e30);
        assert!(s.contains("YW"));
    }

    #[test]
    fn canonicalize_zero_avoids_negative_zero_display() {
        assert_eq!(format!("{:.0}", canonicalize_zero(-0.0)), "0");
        assert_eq!(fmt_cycles_rate(-0.0), "0");
        assert_eq!(fmt_bytes(-0.0), "0 MB");
    }

    #[test]
    fn formatters_handle_f64_max_without_panic() {
        let x = f64::MAX;
        let _ = fmt_bytes(x);
        let _ = fmt_bandwidth(x);
        let _ = fmt_watts(x);
        let _ = fmt_cycles(x);
    }
}
