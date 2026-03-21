use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProducerKind {
    ShellScript,
    CronJob,
    Daemon,
    ServiceUnit,
    KernelModule,
    Hypervisor,
    OsTakeover,
}

#[derive(Debug, Clone, Copy)]
pub struct ProducerDef {
    pub kind: ProducerKind,
    pub name: &'static str,
    pub command: &'static str,
    pub base_cycles_per_s: f64,
    pub base_cost: f64,
    pub ram_mb: f64,
    pub unlock_threshold: f64,
}

const PRODUCERS: [ProducerDef; 7] = [
    ProducerDef {
        kind: ProducerKind::ShellScript,
        name: "Shell Script",
        command: "harvest.sh &",
        base_cycles_per_s: 1.0,
        base_cost: 10.0,
        ram_mb: 1.0,
        unlock_threshold: 0.0,
    },
    ProducerDef {
        kind: ProducerKind::CronJob,
        name: "Cron Job",
        command: "crontab -e",
        base_cycles_per_s: 8.0,
        base_cost: 100.0,
        ram_mb: 4.0,
        unlock_threshold: 50.0,
    },
    ProducerDef {
        kind: ProducerKind::Daemon,
        name: "Daemon",
        command: "systemctl start harvestd",
        base_cycles_per_s: 47.0,
        base_cost: 1_100.0,
        ram_mb: 16.0,
        unlock_threshold: 500.0,
    },
    ProducerDef {
        kind: ProducerKind::ServiceUnit,
        name: "Service Unit",
        command: "systemctl enable harvest.service",
        base_cycles_per_s: 260.0,
        base_cost: 12_000.0,
        ram_mb: 64.0,
        unlock_threshold: 6_000.0,
    },
    ProducerDef {
        kind: ProducerKind::KernelModule,
        name: "Kernel Module",
        command: "insmod harvest.ko",
        base_cycles_per_s: 1_400.0,
        base_cost: 130_000.0,
        ram_mb: 256.0,
        unlock_threshold: 65_000.0,
    },
    ProducerDef {
        kind: ProducerKind::Hypervisor,
        name: "Hypervisor",
        command: "virsh start harvest-vm",
        base_cycles_per_s: 7_800.0,
        base_cost: 1_400_000.0,
        ram_mb: 1_024.0,
        unlock_threshold: 700_000.0,
    },
    ProducerDef {
        kind: ProducerKind::OsTakeover,
        name: "OS Takeover",
        command: "init 5",
        base_cycles_per_s: 44_000.0,
        base_cost: 20_000_000.0,
        ram_mb: 4_096.0,
        unlock_threshold: 10_000_000.0,
    },
];

pub fn all_producers() -> &'static [ProducerDef] {
    &PRODUCERS
}

pub fn producer_def(kind: ProducerKind) -> &'static ProducerDef {
    all_producers()
        .iter()
        .find(|def| def.kind == kind)
        .expect("producer kind must exist in registry")
}

pub fn producer_cost(def: &ProducerDef, owned: u32) -> f64 {
    def.base_cost * 1.15_f64.powi(owned as i32)
}

pub fn previous_tier(kind: ProducerKind) -> Option<ProducerKind> {
    match kind {
        ProducerKind::ShellScript => None,
        ProducerKind::CronJob => Some(ProducerKind::ShellScript),
        ProducerKind::Daemon => Some(ProducerKind::CronJob),
        ProducerKind::ServiceUnit => Some(ProducerKind::Daemon),
        ProducerKind::KernelModule => Some(ProducerKind::ServiceUnit),
        ProducerKind::Hypervisor => Some(ProducerKind::KernelModule),
        ProducerKind::OsTakeover => Some(ProducerKind::Hypervisor),
    }
}

pub fn producer_unlocked(
    total_cycles_earned: f64,
    producers: &HashMap<ProducerKind, u32>,
    kind: ProducerKind,
) -> bool {
    let def = producer_def(kind);
    if total_cycles_earned < def.unlock_threshold {
        return false;
    }
    if let Some(prev) = previous_tier(kind) {
        let owned = producers.get(&prev).copied().unwrap_or(0);
        if owned < 1 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_script_never_requires_previous_tier() {
        let p = HashMap::new();
        assert!(producer_unlocked(0.0, &p, ProducerKind::ShellScript));
    }

    #[test]
    fn cron_job_requires_cycles_and_shell_script() {
        let mut p = HashMap::new();
        assert!(!producer_unlocked(100.0, &p, ProducerKind::CronJob));
        p.insert(ProducerKind::ShellScript, 1);
        assert!(producer_unlocked(100.0, &p, ProducerKind::CronJob));
    }

    #[test]
    fn daemon_requires_cron_not_only_shell() {
        let mut p = HashMap::new();
        p.insert(ProducerKind::ShellScript, 1);
        assert!(!producer_unlocked(1_000.0, &p, ProducerKind::Daemon));
        p.insert(ProducerKind::CronJob, 1);
        assert!(producer_unlocked(1_000.0, &p, ProducerKind::Daemon));
    }
}
