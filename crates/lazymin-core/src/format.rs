pub fn fmt_cycles(value: f64) -> String {
    let abs = value.abs();

    if abs < 1_000.0 {
        return format!("{value:.0}");
    }

    let (divisor, suffix) = if abs >= 1_000_000_000.0 {
        (1_000_000_000.0, "G")
    } else if abs >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else {
        (1_000.0, "k")
    };

    let scaled = value / divisor;
    format!("{scaled:.1}{suffix}")
}

pub fn fmt_cycles_rate(value: f64) -> String {
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

pub fn fmt_mb(value_mb: f64) -> String {
    if value_mb.abs() >= 1024.0 {
        let gb = value_mb / 1024.0;
        format!("{gb:.1} GB")
    } else {
        format!("{value_mb:.0} MB")
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
    fn mb_formatting() {
        assert_eq!(fmt_mb(16.0), "16 MB");
        assert_eq!(fmt_mb(1024.0), "1.0 GB");
        assert_eq!(fmt_mb(1536.0), "1.5 GB");
    }
}
