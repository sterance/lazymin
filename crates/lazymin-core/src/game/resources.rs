use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::format::{fmt_bandwidth, fmt_bytes, fmt_watts};

use super::producers::producer_def;
use super::producers::ProducerKind;

pub const STARTING_RAM_MB: f64 = 16.0;
pub const STARTING_DISK_MB: f64 = 512.0;
pub const STARTING_BANDWIDTH_MBPS: f64 = 0.0;
pub const STARTING_WATTS: f64 = 10.0;
pub const BASE_ENTROPY_PER_SEC: f64 = 0.01;

pub const KERNEL_RAM_MB: f64 = 4.0;
pub const KERNEL_DISK_MB: f64 = 100.0;
pub const KERNEL_WATTS: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HardwareTier {
    Consumer = 0,
    Business = 1,
    Supplier = 2,
    Innovator = 3,
    Futurologist = 4,
}

impl HardwareTier {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn next(self) -> Option<HardwareTier> {
        match self {
            Self::Consumer => Some(Self::Business),
            Self::Business => Some(Self::Supplier),
            Self::Supplier => Some(Self::Innovator),
            Self::Innovator => Some(Self::Futurologist),
            Self::Futurologist => None,
        }
    }
}

impl Default for HardwareTier {
    fn default() -> Self {
        Self::Consumer
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HardwareDef {
    pub kind: ResourceKind,
    pub cap_delta: f64,
    pub base_cost: f64,
    pub watts: f64,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct TierHardwareValues {
    pub cap_delta: f64,
    pub base_cost: f64,
}

// per-tier hardware values: [tier][resource_index]
// resource order: Ram=0, Disk=1, Bandwidth=2, Watts=3
const TIER_HARDWARE: [[TierHardwareValues; 4]; 5] = [
    // Consumer (tier 1) - matches existing values
    [
        TierHardwareValues { cap_delta: 16.0, base_cost: 50.0 },
        TierHardwareValues { cap_delta: 256.0, base_cost: 100.0 },
        TierHardwareValues { cap_delta: 5.0, base_cost: 200.0 },
        TierHardwareValues { cap_delta: 10.0, base_cost: 75.0 },
    ],
    // Business (tier 2) - ~8x
    [
        TierHardwareValues { cap_delta: 128.0, base_cost: 400.0 },
        TierHardwareValues { cap_delta: 2_048.0, base_cost: 800.0 },
        TierHardwareValues { cap_delta: 40.0, base_cost: 1_600.0 },
        TierHardwareValues { cap_delta: 80.0, base_cost: 600.0 },
    ],
    // Supplier (tier 3) - ~8x from Business
    [
        TierHardwareValues { cap_delta: 1_024.0, base_cost: 3_200.0 },
        TierHardwareValues { cap_delta: 16_384.0, base_cost: 6_400.0 },
        TierHardwareValues { cap_delta: 320.0, base_cost: 12_800.0 },
        TierHardwareValues { cap_delta: 640.0, base_cost: 4_800.0 },
    ],
    // Innovator (tier 4) - ~8x from Supplier
    [
        TierHardwareValues { cap_delta: 8_192.0, base_cost: 25_600.0 },
        TierHardwareValues { cap_delta: 131_072.0, base_cost: 51_200.0 },
        TierHardwareValues { cap_delta: 2_560.0, base_cost: 102_400.0 },
        TierHardwareValues { cap_delta: 5_120.0, base_cost: 38_400.0 },
    ],
    // Futurologist (tier 5) - ~8x from Innovator
    [
        TierHardwareValues { cap_delta: 65_536.0, base_cost: 204_800.0 },
        TierHardwareValues { cap_delta: 1_048_576.0, base_cost: 409_600.0 },
        TierHardwareValues { cap_delta: 20_480.0, base_cost: 819_200.0 },
        TierHardwareValues { cap_delta: 40_960.0, base_cost: 307_200.0 },
    ],
];

const HARDWARE_WATTS: [f64; 4] = [1.0, 0.5, 2.0, 0.0];
const HARDWARE_LABELS: [&str; 4] = ["ram", "disk", "bandwidth", "power"];
const HARDWARE_KINDS: [ResourceKind; 4] = [
    ResourceKind::Ram,
    ResourceKind::Disk,
    ResourceKind::Bandwidth,
    ResourceKind::Watts,
];

fn resource_hw_index(kind: ResourceKind) -> usize {
    match kind {
        ResourceKind::Ram => 0,
        ResourceKind::Disk => 1,
        ResourceKind::Bandwidth => 2,
        ResourceKind::Watts => 3,
        _ => panic!("not a hardware resource kind"),
    }
}

pub fn tiered_hardware_def(tier: HardwareTier, kind: ResourceKind) -> HardwareDef {
    let idx = resource_hw_index(kind);
    let tv = &TIER_HARDWARE[tier.index()][idx];
    HardwareDef {
        kind,
        cap_delta: tv.cap_delta,
        base_cost: tv.base_cost,
        watts: HARDWARE_WATTS[idx],
        label: HARDWARE_LABELS[idx],
    }
}

pub fn all_hardware_kinds() -> &'static [ResourceKind] {
    &HARDWARE_KINDS
}

pub fn hardware_def_for_tier(tier: HardwareTier, kind: ResourceKind) -> HardwareDef {
    tiered_hardware_def(tier, kind)
}

// tier flavour names for apt install descriptions
const TIER_ITEM_NAMES: [[&str; 4]; 5] = [
    // Consumer
    ["ram stick", "hard drive", "network card", "power supply"],
    // Business
    ["ram pallet", "drive rack", "switch module", "ups unit"],
    // Supplier
    ["ram warehouse", "storage array", "backbone port", "generator unit"],
    // Innovator
    ["ram substrate", "molecular storage node", "photonic link", "fusion tap"],
    // Futurologist
    ["ram lattice", "compressed-matter store", "quantum channel", "stellar tap"],
];

pub fn apt_install_hardware_description(tier: HardwareTier, kind: ResourceKind) -> String {
    let hw = tiered_hardware_def(tier, kind);
    let idx = resource_hw_index(kind);
    let item_name = TIER_ITEM_NAMES[tier.index()][idx];
    match kind {
        ResourceKind::Ram => format!("{item_name} (+{})", fmt_bytes(hw.cap_delta)),
        ResourceKind::Disk => format!("{item_name} (+{})", fmt_bytes(hw.cap_delta)),
        ResourceKind::Bandwidth => format!("{item_name} (+{})", fmt_bandwidth(hw.cap_delta)),
        ResourceKind::Watts => format!("{item_name} (+{})", fmt_watts(hw.cap_delta)),
        ResourceKind::Cycles | ResourceKind::Entropy => {
            unreachable!("apt install hardware description: not a capacity hardware kind")
        }
    }
}

pub fn total_hardware_watts(purchases: &HashMap<ResourceKind, u32>) -> f64 {
    HARDWARE_KINDS
        .iter()
        .map(|&kind| {
            let count = purchases.get(&kind).copied().unwrap_or(0) as f64;
            count * HARDWARE_WATTS[resource_hw_index(kind)]
        })
        .sum()
}

pub fn total_power_draw(purchases: &HashMap<ResourceKind, u32>) -> f64 {
    total_hardware_watts(purchases) + KERNEL_WATTS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    Cycles,
    Ram,
    Disk,
    Bandwidth,
    Watts,
    Entropy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePool {
    pub amounts: HashMap<ResourceKind, f64>,
    pub caps: HashMap<ResourceKind, f64>,
    pub rates: HashMap<ResourceKind, f64>,
}

impl ResourcePool {
    pub fn new() -> Self {
        let mut amounts = HashMap::new();
        amounts.insert(ResourceKind::Cycles, 0.0);
        amounts.insert(ResourceKind::Ram, 0.0);
        amounts.insert(ResourceKind::Disk, 0.0);
        amounts.insert(ResourceKind::Bandwidth, 0.0);
        amounts.insert(ResourceKind::Watts, 0.0);
        amounts.insert(ResourceKind::Entropy, 0.0);

        let mut caps = HashMap::new();
        caps.insert(ResourceKind::Ram, STARTING_RAM_MB);
        caps.insert(ResourceKind::Disk, STARTING_DISK_MB);
        caps.insert(ResourceKind::Bandwidth, STARTING_BANDWIDTH_MBPS);
        caps.insert(ResourceKind::Watts, STARTING_WATTS);

        let mut rates = HashMap::new();
        rates.insert(ResourceKind::Cycles, 0.0);
        rates.insert(ResourceKind::Entropy, BASE_ENTROPY_PER_SEC);

        Self {
            amounts,
            caps,
            rates,
        }
    }

    pub fn get(&self, kind: ResourceKind) -> f64 {
        self.amounts.get(&kind).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, kind: ResourceKind, value: f64) {
        self.amounts.insert(kind, value);
    }

    pub fn cap(&self, kind: ResourceKind) -> Option<f64> {
        self.caps.get(&kind).copied()
    }

    pub fn set_cap(&mut self, kind: ResourceKind, value: f64) {
        self.caps.insert(kind, value.max(0.0));
    }

    pub fn add_cap(&mut self, kind: ResourceKind, delta: f64) {
        let next = self.cap(kind).unwrap_or(0.0) + delta;
        self.set_cap(kind, next);
    }

    pub fn can_afford(&self, cost: f64) -> bool {
        self.get(ResourceKind::Cycles) >= cost
    }

    pub fn deduct(&mut self, cost: f64) {
        let next = self.get(ResourceKind::Cycles) - cost;
        self.set(ResourceKind::Cycles, next.max(0.0));
    }

    pub fn advance(&mut self, delta_secs: f64) {
        if delta_secs <= 0.0 {
            return;
        }

        for (kind, rate) in self.rates.clone() {
            let next = self.get(kind) + (rate * delta_secs);
            self.set(kind, next);
        }
    }

    pub fn clamp_to_caps(&mut self) {
        for (kind, cap) in self.caps.clone() {
            let current = self.get(kind);
            if current > cap {
                self.set(kind, cap);
            }
        }
    }
}

pub fn total_reserved_ram(producers: &HashMap<ProducerKind, u32>) -> f64 {
    KERNEL_RAM_MB
        + producers
            .iter()
            .map(|(kind, count)| producer_def(*kind).ram_mb * (*count as f64))
            .sum::<f64>()
}

pub fn total_reserved_disk(producers: &HashMap<ProducerKind, u32>) -> f64 {
    KERNEL_DISK_MB
        + producers
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(kind, _)| producer_def(*kind).disk_mb)
            .sum::<f64>()
}

pub fn total_reserved_bandwidth(producers: &HashMap<ProducerKind, u32>) -> f64 {
    producers
        .iter()
        .map(|(kind, count)| producer_def(*kind).bw_mbps * (*count as f64))
        .sum()
}

#[cfg(test)]
mod apt_install_description_tests {
    use super::*;

    #[test]
    fn ram_disk_use_fmt_bytes_style() {
        let ram = apt_install_hardware_description(HardwareTier::Consumer, ResourceKind::Ram);
        assert!(ram.contains("ram stick"));
        assert!(ram.ends_with(')'));
        let disk = apt_install_hardware_description(HardwareTier::Consumer, ResourceKind::Disk);
        assert!(disk.contains("hard drive"));
        assert!(disk.contains("GB") || disk.contains("MB"));
    }

    #[test]
    fn bandwidth_and_watts_suffixes() {
        let bw = apt_install_hardware_description(HardwareTier::Consumer, ResourceKind::Bandwidth);
        assert!(bw.contains("Mbps"));
        let w = apt_install_hardware_description(HardwareTier::Consumer, ResourceKind::Watts);
        assert!(w.contains('W'));
    }

    #[test]
    fn tier_changes_flavour_text() {
        let consumer = apt_install_hardware_description(HardwareTier::Consumer, ResourceKind::Ram);
        let business = apt_install_hardware_description(HardwareTier::Business, ResourceKind::Ram);
        assert!(consumer.contains("ram stick"));
        assert!(business.contains("ram pallet"));
    }
}
