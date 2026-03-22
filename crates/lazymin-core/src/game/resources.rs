use std::collections::HashMap;

use super::producers::producer_def;
use super::producers::ProducerKind;

pub const STARTING_RAM_MB: f64 = 16.0;
pub const STARTING_DISK_MB: f64 = 1024.0;
pub const STARTING_BANDWIDTH_MBPS: f64 = 0.0;
pub const STARTING_WATTS: f64 = 5.0;
pub const BASE_ENTROPY_PER_SEC: f64 = 0.01;

pub const KERNEL_RAM_MB: f64 = 4.0;
pub const KERNEL_DISK_MB: f64 = 100.0;
pub const KERNEL_WATTS: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
pub struct HardwareDef {
    pub kind: ResourceKind,
    pub cap_delta: f64,
    pub base_cost: f64,
    pub watts: f64,
    pub label: &'static str,
}

const HARDWARE: [HardwareDef; 4] = [
    HardwareDef {
        kind: ResourceKind::Ram,
        cap_delta: 256.0,
        base_cost: 50.0,
        watts: 1.0,
        label: "ram",
    },
    HardwareDef {
        kind: ResourceKind::Disk,
        cap_delta: 1024.0,
        base_cost: 100.0,
        watts: 0.5,
        label: "disk",
    },
    HardwareDef {
        kind: ResourceKind::Bandwidth,
        cap_delta: 10.0,
        base_cost: 200.0,
        watts: 2.0,
        label: "bandwidth",
    },
    HardwareDef {
        kind: ResourceKind::Watts,
        cap_delta: 50.0,
        base_cost: 75.0,
        watts: 0.0,
        label: "power",
    },
];

pub fn all_hardware() -> &'static [HardwareDef] {
    &HARDWARE
}

pub fn hardware_def(kind: ResourceKind) -> &'static HardwareDef {
    all_hardware()
        .iter()
        .find(|def| def.kind == kind)
        .expect("resource kind must map to capacity hardware")
}

pub fn total_hardware_watts(purchases: &HashMap<ResourceKind, u32>) -> f64 {
    all_hardware()
        .iter()
        .map(|def| {
            let count = purchases.get(&def.kind).copied().unwrap_or(0) as f64;
            count * def.watts
        })
        .sum()
}

pub fn total_power_draw(purchases: &HashMap<ResourceKind, u32>) -> f64 {
    total_hardware_watts(purchases) + KERNEL_WATTS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Cycles,
    Ram,
    Disk,
    Bandwidth,
    Watts,
    Entropy,
}

#[derive(Debug, Clone)]
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
            .map(|(kind, count)| producer_def(*kind).disk_mb * (*count as f64))
            .sum::<f64>()
}

pub fn total_reserved_bandwidth(producers: &HashMap<ProducerKind, u32>) -> f64 {
    producers
        .iter()
        .map(|(kind, count)| producer_def(*kind).bw_mbps * (*count as f64))
        .sum()
}
