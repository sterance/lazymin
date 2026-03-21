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
    if scaled.fract().abs() < 0.05 {
        format!("{scaled:.0}{suffix}")
    } else {
        format!("{scaled:.1}{suffix}")
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
        assert_eq!(fmt_cycles(1_000.0), "1k");
        assert_eq!(fmt_cycles(1_200.0), "1.2k");
        assert_eq!(fmt_cycles(12_000.0), "12k");
        assert_eq!(fmt_cycles(130_000.0), "130k");
    }

    #[test]
    fn millions() {
        assert_eq!(fmt_cycles(1_400_000.0), "1.4M");
        assert_eq!(fmt_cycles(20_000_000.0), "20M");
    }

    #[test]
    fn billions() {
        assert_eq!(fmt_cycles(1_000_000_000.0), "1G");
        assert_eq!(fmt_cycles(2_500_000_000.0), "2.5G");
    }
}
