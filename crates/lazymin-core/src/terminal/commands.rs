#[path = "CommandDefs.rs"]
mod command_defs;
#[path = "locks.rs"]
mod locks;

use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::{fmt_bandwidth, fmt_bytes, fmt_bytes_rate, fmt_cycles, fmt_cycles_rate, fmt_watts};
use crate::game::producers::{
    all_producers, producer_cost, producer_def, ProducerKind,
};
use crate::game::resources::{
    all_hardware, apt_install_hardware_description, hardware_def, total_power_draw,
    total_reserved_bandwidth, total_reserved_disk, total_reserved_ram, ResourceKind,
    KERNEL_DISK_MB, KERNEL_RAM_MB, KERNEL_WATTS,
};
use crate::game::save;
use crate::game::tick::{
    coolant_unit_price, disk_log_growth_rate, grant_cycle_burst, remote_cycle_rate,
};
use crate::game::upgrades::{
    apply_upgrade_purchase, burst_upgrade_cost, effective_disk_cap, is_burst_upgrade,
    refresh_unlock_threshold_tracking, upgrade_by_command, upgrade_unlocked, UpgradeKind,
};

use command_defs::registry_command;
pub use command_defs::{command_registry, CommandDef};

const HELP_ORDER: &[&str] = &[
    "ls",
    "clear",
    "exit",
    "",
    "ps aux",
    "du",
    "ifconfig",
    "lshw",
    "",
    "apt install",
    "apt update",
    "apt upgrade",
];

const LS_ORDER: &[&str] = &[
    "harvest.sh",
    "harvest.sh &",
    "crontab harvest.cron",
    "systemctl start harvestd",
    "systemctl enable harvest.service",
    "insmod harvest.ko",
    "virsh start harvest-vm",
    "init 5",
];

const APT_INSTALL_ORDER: &[&str] = &[
    "apt install ram",
    "apt install hdd",
    "apt install nic",
    "apt install psu",
];

const APT_UPDATE_ORDER: &[&str] = &[
    "cat /dev/urandom > /dev/null",
    "openssl rand -base64 32",
    "uuidgen",
    "mktemp -d",
    "dd if=/dev/urandom of=/dev/sda",
    "reboot --firmware",
    "jvacuum",
];

#[cfg(target_arch = "wasm32")]
const BROWSER_SAVE_LOG_TEXT: &str = "progress saved to browser storage";

const UPGRADES_ORDER: &[&str] = &[
    "alias harvest='harvest.sh'",    
    "shellcheck harvest.sh",
    "run-parts /etc/cron.hourly",
    "sudo visudo",
    "systemctl set-default multi-user.target",
    "mount -t tmpfs",
    "upsc myups",
    "zstd --train",
    "logrotate",
    "bpftrace -e 'tracepoint:*'",
    "numactl --interleave=all",
    "rngd --feed-random",
    "gpg --gen-key",
    "ssh remote harvest",
    "ssh market",
    "ssh-keygen -t ed25519",
    "certbot renew",
    "haveged --run",
    "stress-ng --cpu 0",
    "fault-inject enable",
    "init 0 && init 6",
];

fn producer_cost_for(app: &App, kind: ProducerKind) -> f64 {
    let def = producer_def(kind);
    let owned = app.game.producers.get(&kind).copied().unwrap_or(0);
    let mut p = producer_cost(def, owned);
    if let Some(f) = app.game.pending_producer_cost_factors.front() {
        p *= *f;
    }
    p
}

fn format_upgrade_cost(cycles: f64, entropy: f64) -> String {
    let mut parts = Vec::new();
    if cycles > 0.0 {
        parts.push(format!("{} cycles", fmt_cycles(cycles)));
    }
    if entropy > 0.0 {
        parts.push(format!("{entropy:.2} ent"));
    }
    if parts.is_empty() {
        "free".to_owned()
    } else {
        parts.join(" + ")
    }
}

fn format_upgrade_cost_suffix(cycles: f64, entropy: f64) -> String {
    let mut parts = Vec::new();
    if cycles > 0.0 {
        parts.push(format!("-{} cycles", fmt_cycles(cycles)));
    }
    if entropy > 0.0 {
        parts.push(format!("-{entropy:.2} ent"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

fn upgrade_cost_by_command(app: &App, command: &str) -> Option<(f64, f64)> {
    let u = upgrade_by_command(command)?;
    let (cycles, entropy) = if is_burst_upgrade(u.kind) {
        let bought = app.game.burst_purchase_counts.get(&u.kind).copied().unwrap_or(0);
        burst_upgrade_cost(u, bought)
    } else {
        (u.cycles_cost, u.entropy_cost)
    };
    Some((cycles, entropy))
}

fn buy_producer(app: &mut App, kind: ProducerKind) -> Vec<TerminalLine> {
    let def = producer_def(kind);
    let owned_before = app.game.producers.get(&kind).copied().unwrap_or(0);

    let reserved_ram = total_reserved_ram(&app.game.producers);
    let ram_cap = app.game.resources.cap(ResourceKind::Ram).unwrap_or(0.0);

    if reserved_ram + def.ram_mb > ram_cap {
        app.game.hit_resource_gate = true;
        let free_ram = (ram_cap - reserved_ram).max(0.0);
        return vec![
            TerminalLine::Output {
                text: format!(
                    "insufficient memory (need {}, have {} free)",
                    fmt_bytes(def.ram_mb),
                    fmt_bytes(free_ram)
                ),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    if owned_before == 0 {
        let reserved_disk = total_reserved_disk(&app.game.producers);
        let disk_cap = effective_disk_cap(&app.game);
        if reserved_disk + def.disk_mb + app.game.disk_log_usage > disk_cap + 1e-6 {
            app.game.hit_resource_gate = true;
            let free = (disk_cap - reserved_disk - app.game.disk_log_usage).max(0.0);
            return vec![
                TerminalLine::Output {
                    text: format!(
                        "insufficient disk space (need {}, have {} free)",
                        fmt_bytes(def.disk_mb),
                        fmt_bytes(free)
                    ),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ];
        }
    }

    if def.bw_mbps > 0.0 {
        let bw_cap = app
            .game
            .resources
            .cap(ResourceKind::Bandwidth)
            .unwrap_or(0.0);
        let reserved_bw = total_reserved_bandwidth(&app.game.producers);
        if reserved_bw + def.bw_mbps > bw_cap + 1e-6 {
            app.game.hit_resource_gate = true;
            let free = (bw_cap - reserved_bw).max(0.0);
            return vec![
                TerminalLine::Output {
                    text: format!(
                        "insufficient bandwidth (need {}, have {} free)",
                        fmt_bandwidth(def.bw_mbps),
                        fmt_bandwidth(free)
                    ),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ];
        }
    }

    let mut price = producer_cost(def, owned_before);
    if let Some(f) = app.game.pending_producer_cost_factors.pop_front() {
        price *= f;
    }
    app.game.resources.deduct(price);

    let owned_count = {
        let e = app.game.producers.entry(kind).or_insert(0);
        *e += 1;
        *e
    };

    if owned_before == 0 {
        app.game.ever_owned_producers.insert(kind);
    }
    refresh_unlock_threshold_tracking(&mut app.game);

    vec![
        TerminalLine::Output {
            text: format!(
                "[{owned_count}] {}  -- +{:.0} cycles/s",
                def.command, def.base_cycles_per_s
            ),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cap_upgrade_cost(base_cost: f64, purchases: u32) -> f64 {
    base_cost * 1.05_f64.powi(purchases as i32)
}

fn capacity_cost_basis_count(app: &App, kind: ResourceKind) -> u32 {
    app.game
        .hardware_cost_basis
        .get(&kind)
        .copied()
        .unwrap_or(0)
}

fn capacity_command_cost_for(app: &App, kind: ResourceKind) -> f64 {
    use crate::game::upgrades::{ram_hardware_cost_multiplier, watt_hardware_cost_multiplier};
    let base = hardware_def(kind).base_cost;
    let count = capacity_cost_basis_count(app, kind);
    let mut c = cap_upgrade_cost(base, count);
    match kind {
        ResourceKind::Ram => c *= ram_hardware_cost_multiplier(&app.game),
        ResourceKind::Watts => c *= watt_hardware_cost_multiplier(&app.game),
        _ => {}
    }
    if let Some(f) = app.game.next_hardware_discount {
        c *= f;
    }
    c
}

fn buy_capacity(app: &mut App, kind: ResourceKind) -> Vec<TerminalLine> {
    let hw = hardware_def(kind);
    let cost = capacity_command_cost_for(app, kind);
    let watts_cap = app.game.resources.cap(ResourceKind::Watts).unwrap_or(0.0);
    let used_watts = total_power_draw(&app.game.capacity_purchases);

    if hw.watts > 0.0 && used_watts + hw.watts > watts_cap {
        let free_watts = (watts_cap - used_watts).max(0.0);
        return vec![
                TerminalLine::Output {
                    text: format!(
                        "power budget exceeded (need {}, have {} free)",
                        fmt_watts(hw.watts),
                        fmt_watts(free_watts)
                    ),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    app.game.resources.deduct(cost);
    app.game.next_hardware_discount = None;
    app.game.resources.add_cap(kind, hw.cap_delta);
    app.game
        .capacity_purchases
        .entry(kind)
        .and_modify(|count| *count += 1)
        .or_insert(1);
    app.game
        .hardware_cost_basis
        .entry(kind)
        .and_modify(|count| *count += 1)
        .or_insert(1);

    let cap = app.game.resources.cap(kind).unwrap_or(0.0);
    let (cap_str, free_str) = match kind {
        ResourceKind::Ram => {
            let reserved = total_reserved_ram(&app.game.producers);
            let free = (cap - reserved).max(0.0);
            (fmt_bytes(cap), fmt_bytes(free))
        }
        ResourceKind::Disk => {
            let reserved = total_reserved_disk(&app.game.producers);
            let used = reserved + app.game.disk_log_usage;
            let free = (cap - used).max(0.0);
            (fmt_bytes(cap), fmt_bytes(free))
        }
        ResourceKind::Bandwidth => {
            let reserved = total_reserved_bandwidth(&app.game.producers);
            let free = (cap - reserved).max(0.0);
            (fmt_bandwidth(cap), fmt_bandwidth(free))
        }
        ResourceKind::Watts => {
            let used = total_power_draw(&app.game.capacity_purchases);
            let free = (cap - used).max(0.0);
            (fmt_watts(cap), fmt_watts(free))
        }
        ResourceKind::Cycles | ResourceKind::Entropy => unreachable!(
            "buy_capacity should only be called for capacity hardware kinds"
        ),
    };

    vec![
        TerminalLine::Output {
            text: format!("{} capacity now {cap_str}, {free_str} free", hw.label),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_help(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.game.help_runs += 1;
    let mut out = Vec::new();
    let mut pending_blank = false;
    for &name in HELP_ORDER {
        if name.is_empty() {
            pending_blank = true;
            continue;
        }
        let Some(cmd) = registry_command(name) else {
            continue;
        };
        if (cmd.locked)(app) {
            continue;
        }
        if pending_blank && !out.is_empty() {
            out.push(TerminalLine::Blank);
            pending_blank = false;
        }
        out.push(TerminalLine::Output {
            text: format!("{} - {}", cmd.name, command_player_description(cmd)),
            style: OutputStyle::Info,
        });
    }

    out.push(TerminalLine::Blank);
    out
}

fn cmd_harvest(_: &str, app: &mut App) -> Vec<TerminalLine> {
    use crate::game::upgrades::manual_harvest_multiplier;
    let yield_cycles = 1.0 * manual_harvest_multiplier(&app.game);
    let cycles = app.game.resources.get(ResourceKind::Cycles) + yield_cycles;
    app.game.resources.set(ResourceKind::Cycles, cycles);
    app.game.total_cycles_earned += yield_cycles;
    app.game.manual_runs += 1;

    vec![
        TerminalLine::Output {
            text: format!("harvested {} cycles", fmt_cycles(yield_cycles)),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

macro_rules! define_producer_command {
    ($cmd_fn:ident, $cost_fn:ident, $kind:expr) => {
        fn $cmd_fn(_: &str, app: &mut App) -> Vec<TerminalLine> {
            buy_producer(app, $kind)
        }

        fn $cost_fn(app: &App) -> f64 {
            producer_cost_for(app, $kind)
        }
    };
}

macro_rules! define_capacity_command {
    ($cmd_fn:ident, $cost_fn:ident, $kind:expr) => {
        fn $cmd_fn(_: &str, app: &mut App) -> Vec<TerminalLine> {
            buy_capacity(app, $kind)
        }

        fn $cost_fn(app: &App) -> f64 {
            capacity_command_cost_for(app, $kind)
        }
    };
}

define_producer_command!(
    cmd_buy_shell_script,
    shell_script_cost,
    ProducerKind::ShellScript
);
define_producer_command!(cmd_buy_cron_job, cron_job_cost, ProducerKind::CronJob);
define_producer_command!(cmd_buy_daemon, daemon_cost, ProducerKind::Daemon);
define_producer_command!(
    cmd_buy_service_unit,
    service_unit_cost,
    ProducerKind::ServiceUnit
);
define_producer_command!(
    cmd_buy_kernel_module,
    kernel_module_cost,
    ProducerKind::KernelModule
);
define_producer_command!(cmd_buy_hypervisor, hypervisor_cost, ProducerKind::Hypervisor);
define_producer_command!(
    cmd_buy_os_takeover,
    os_takeover_cost,
    ProducerKind::OsTakeover
);

define_capacity_command!(cmd_buy_ram, apt_ram_cost, ResourceKind::Ram);
define_capacity_command!(cmd_buy_disk, apt_disk_cost, ResourceKind::Disk);
define_capacity_command!(cmd_buy_bw, apt_bw_cost, ResourceKind::Bandwidth);
define_capacity_command!(cmd_buy_watts, apt_watts_cost, ResourceKind::Watts);

pub(super) fn market_buy_cost(app: &App) -> f64 {
    coolant_unit_price(&app.game)
}

pub(super) fn cmd_market_buy(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let price = coolant_unit_price(&app.game);
    app.game.resources.deduct(price);
    app.game.coolant += 1.0;
    vec![
        TerminalLine::Output {
            text: format!(
                "coolant +1 (now {:.0}) -- -{} cycles",
                app.game.coolant,
                fmt_cycles(price)
            ),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn apt_install_resource(name: &str) -> Option<ResourceKind> {
    match name {
        "apt install ram" => Some(ResourceKind::Ram),
        "apt install hdd" => Some(ResourceKind::Disk),
        "apt install nic" => Some(ResourceKind::Bandwidth),
        "apt install psu" => Some(ResourceKind::Watts),
        _ => None,
    }
}

fn command_player_description(cmd: &CommandDef) -> String {
    apt_install_resource(cmd.name)
        .map(apt_install_hardware_description)
        .unwrap_or_else(|| cmd.description.to_owned())
}

fn cmd_apt_install(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    for &name in APT_INSTALL_ORDER {
        let Some(kind) = apt_install_resource(name) else {
            continue;
        };
        let Some(cmd) = registry_command(name) else {
            continue;
        };
        if (cmd.locked)(app) {
            continue;
        }
        let next = capacity_command_cost_for(app, kind);
        let owned = app
            .game
            .capacity_purchases
            .get(&kind)
            .copied()
            .unwrap_or(0);
        out.push(TerminalLine::Output {
            text: format!(
                "[{owned}] {} - {} (next: {} cycles)",
                name,
                command_player_description(cmd),
                fmt_cycles(next)
            ),
            style: OutputStyle::Info,
        });
    }

    out.push(TerminalLine::Blank);
    out
}

fn cmd_apt_update(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    for &name in APT_UPDATE_ORDER {
        let Some(u) = upgrade_by_command(name) else {
            continue;
        };
        if !upgrade_unlocked(&app.game, u.kind) {
            continue;
        }
        let bought = app
            .game
            .burst_purchase_counts
            .get(&u.kind)
            .copied()
            .unwrap_or(0);
        let (cy, ent) = burst_upgrade_cost(u, bought);

        let cost_str = format_upgrade_cost(cy, ent);

        out.push(TerminalLine::Output {
            text: format!("[{bought}] {} - {} ({})", u.command, u.description, cost_str),
            style: OutputStyle::Info,
        });
    }
    out.push(TerminalLine::Blank);
    out
}

fn cmd_ps_aux(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let ram_cap = app
        .game
        .resources
        .cap(ResourceKind::Ram)
        .unwrap_or(0.0)
        .max(1e-9);

    let mut out = vec![TerminalLine::Output {
        text: format!("{:<6}{:<36}{:<12}{:>5}", "PID", "COMMAND", "RSS", "%MEM"),
        style: OutputStyle::Info,
    }];

    let k_pct = (KERNEL_RAM_MB / ram_cap) * 100.0;
    out.push(TerminalLine::Output {
        text: format!(
            "{:<6}{:<36}{:<12}{k_pct:>5.1}",
            1,
            "[kernel]",
            fmt_bytes(KERNEL_RAM_MB),
        ),
        style: OutputStyle::System,
    });

    let mut pid = 1000_u32;
    let mut any_user = false;
    for def in all_producers() {
        let count = app.game.producers.get(&def.kind).copied().unwrap_or(0);
        for _ in 0..count {
            any_user = true;
            let pct = (def.ram_mb / ram_cap) * 100.0;
            out.push(TerminalLine::Output {
                text: format!(
                    "{pid:<6}{:<36}{:<12}{pct:>5.1}",
                    def.command,
                    fmt_bytes(def.ram_mb),
                ),
                style: OutputStyle::System,
            });
            pid += 1;
        }
    }

    if !any_user {
        out.push(TerminalLine::Output {
            text: format!("{:<6}{:<36}", "", "no userland background jobs"),
            style: OutputStyle::System,
        });
    }

    out.push(TerminalLine::Blank);
    out.push(TerminalLine::Output {
        text: "`pkill [PID]` kills running processes".to_owned(),
        style: OutputStyle::System,
    });
    out.push(TerminalLine::Blank);
    out
}

fn cmd_pkill(input: &str, app: &mut App) -> Vec<TerminalLine> {
    let pmsg = |body: &str| format!("pkill: {body}");

    let mut parts = input.split_whitespace();
    let _ = parts.next();

    let pid_tok = match parts.next() {
        Some(tok) => tok,
        None => {
            return vec![
                TerminalLine::Output {
                    text: pmsg("specify process to kill, e.g. `pkill [PID]`"),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ];
        }
    };

    let pid: u32 = match pid_tok.parse() {
        Ok(pid) => pid,
        Err(_) => {
            let stripped: String = pid_tok
                .chars()
                .filter(|c| *c != '[' && *c != ']')
                .collect();
            if pid_tok.contains('[') || pid_tok.contains(']') {
                if let Ok(s_pid) = stripped.parse::<u32>() {
                    if s_pid == 1 {
                        return vec![
                            TerminalLine::Output {
                                text: pmsg("cannot kill kernel"),
                                style: OutputStyle::Error,
                            },
                            TerminalLine::Blank,
                        ];
                    }
                    if s_pid >= 1000 {
                        return vec![
                            TerminalLine::Output {
                                text: pmsg(&format!("did you mean `pkill {s_pid}`?")),
                                style: OutputStyle::Error,
                            },
                            TerminalLine::Blank,
                        ];
                    }
                }
            }
            return vec![
                TerminalLine::Output {
                    text: pmsg("invalid PID"),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ];
        }
    };

    if pid == 1 {
        return vec![
            TerminalLine::Output {
                text: pmsg("cannot kill kernel"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    if pid < 1000 {
        return vec![
            TerminalLine::Output {
                text: pmsg("invalid PID"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    let mut idx = pid - 1000;
    let mut target_kind: Option<ProducerKind> = None;
    for def in all_producers() {
        let count = app.game.producers.get(&def.kind).copied().unwrap_or(0);
        if idx < count {
            target_kind = Some(def.kind);
            break;
        }
        idx = idx.saturating_sub(count);
    }

    let Some(kind) = target_kind else {
        return vec![
            TerminalLine::Output {
                text: pmsg("invalid PID"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    };

    let count = app.game.producers.get(&kind).copied().unwrap_or(0);
    if count == 0 {
        return vec![
            TerminalLine::Output {
                text: pmsg("invalid PID"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    let ram_mb = producer_def(kind).ram_mb;
    if count == 1 {
        app.game.producers.remove(&kind);
    } else {
        app.game.producers.insert(kind, count - 1);
    }

    vec![
        TerminalLine::Output {
            text: pmsg(&format!("[{pid}] killed, {} ram freed", fmt_bytes(ram_mb))),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_du(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = vec![TerminalLine::Output {
        text: format!("{:<44}{}", "FILESYSTEM", "SIZE"),
        style: OutputStyle::Info,
    }];

    let mut reserved_total = KERNEL_DISK_MB;
    out.push(TerminalLine::Output {
        text: format!("{:<44}{}", "/boot/vmlinuz", fmt_bytes(KERNEL_DISK_MB)),
        style: OutputStyle::System,
    });

    for def in all_producers() {
        let count = app.game.producers.get(&def.kind).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        reserved_total += def.disk_mb;
        out.push(TerminalLine::Output {
            text: format!("{:<44}{}", def.command, fmt_bytes(def.disk_mb)),
            style: OutputStyle::System,
        });
    }

    let logs = app.game.disk_log_usage;
    if logs > 0.0 {
        let log_rate = disk_log_growth_rate(&app.game);
        let rate_suffix = if log_rate > 0.0 {
            format!("  (+{})", fmt_bytes_rate(log_rate))
        } else {
            String::new()
        };
        out.push(TerminalLine::Output {
            text: format!("{:<44}{}{}", "/var/log", fmt_bytes(logs), rate_suffix),
            style: OutputStyle::System,
        });
    }

    let disk_cap = effective_disk_cap(&app.game);
    let used = reserved_total + logs;

    out.push(TerminalLine::Output {
        text: format!("{:<44}{} / {}", "total", fmt_bytes(used), fmt_bytes(disk_cap)),
        style: OutputStyle::System,
    });

    out.push(TerminalLine::Blank);
    let jvacuum_cost_suffix = upgrade_cost_by_command(app, "jvacuum")
        .map(|(cycles, entropy)| format_upgrade_cost_suffix(cycles, entropy))
        .unwrap_or_default();
    out.push(TerminalLine::Output {
        text: format!("`jvacuum` clears log disk usage{jvacuum_cost_suffix}"),
        style: OutputStyle::System,
    });
    out.push(TerminalLine::Blank);
    out
}

fn cmd_ifconfig(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = vec![TerminalLine::Output {
        text: format!("{:<44}{}", "IFACE", "TX"),
        style: OutputStyle::Info,
    }];

    let cap = app
        .game
        .resources
        .cap(ResourceKind::Bandwidth)
        .unwrap_or(0.0);
    let reserved = total_reserved_bandwidth(&app.game.producers);
    let mut any = false;

    for def in all_producers() {
        if def.bw_mbps <= 0.0 {
            continue;
        }
        let count = app.game.producers.get(&def.kind).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        any = true;
        let mbps = def.bw_mbps * (count as f64);
        out.push(TerminalLine::Output {
            text: format!(
                "{:<44}{}",
                format!("{}  (×{count})", def.command),
                fmt_bandwidth(mbps)
            ),
            style: OutputStyle::System,
        });
    }

    if app.game.remote_channel_active {
        any = true;
        let spare = (cap - reserved).max(0.0);
        let remote_rate = remote_cycle_rate(&app.game);
        out.push(TerminalLine::Output {
            text: format!(
                "{:<44}{} (+{} cycles/s)",
                "ssh remote harvest",
                fmt_bandwidth(spare),
                fmt_cycles_rate(remote_rate)
            ),
            style: OutputStyle::System,
        });
    }

    if !any {
        out.push(TerminalLine::Output {
            text: "no active interfaces".to_owned(),
            style: OutputStyle::System,
        });
    } else {
        out.push(TerminalLine::Output {
            text: format!(
                "{:<44}{} / {}",
                "total",
                fmt_bandwidth(reserved),
                fmt_bandwidth(cap)
            ),
            style: OutputStyle::System,
        });
    }

    out.push(TerminalLine::Blank);
    out
}

fn cmd_lshw(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = vec![TerminalLine::Output {
        text: format!("{:<44}{}", "DEVICE", "POWER"),
        style: OutputStyle::Info,
    }];

    let mut total_w = KERNEL_WATTS;
    out.push(TerminalLine::Output {
        text: format!("{:<44}{}", "kernel", fmt_watts(KERNEL_WATTS)),
        style: OutputStyle::System,
    });

    for hw in all_hardware() {
        if hw.watts <= 0.0 {
            continue;
        }
        let count = app
            .game
            .capacity_purchases
            .get(&hw.kind)
            .copied()
            .unwrap_or(0);
        if count == 0 {
            continue;
        }
        let w = hw.watts * (count as f64);
        total_w += w;
        out.push(TerminalLine::Output {
            text: format!(
                "{:<44}{}",
                format!("{}  (×{count})", hw.label),
                fmt_watts(w)
            ),
            style: OutputStyle::System,
        });
    }

    let watts_cap = app.game.resources.cap(ResourceKind::Watts).unwrap_or(0.0);
    out.push(TerminalLine::Output {
        text: format!(
            "{:<44}{} / {}",
            "total",
            fmt_watts(total_w),
            fmt_watts(watts_cap)
        ),
        style: OutputStyle::System,
    });

    out.push(TerminalLine::Blank);
    out
}

fn cmd_ls(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let hide_harvest_bg = app.game.total_cycles_earned < 10.0;

    let names: Vec<&str> = LS_ORDER
        .iter()
        .copied()
        .filter(|&name| {
            let Some(cmd) = registry_command(name) else {
                return false;
            };
            if (cmd.locked)(app) {
                return false;
            }
            if name == "harvest.sh &" && hide_harvest_bg {
                return false;
            }
            true
        })
        .collect();

    let listing = names
        .into_iter()
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(" ");

    vec![
        TerminalLine::Output {
            text: listing,
            style: OutputStyle::Info,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_hello(_: &str, _: &mut App) -> Vec<TerminalLine> {
    const ART: &[&str] = &[
        r"  ....,       ,....",
        r".' ,,, '.   .' ,,, '.",
        r" .`   `.     .`   `.",
        r": ..... :   : ..... :",
        r":`~'-'-`:   :`-'-'~`:",
        r" `.~-`.'     `.~`'.'",
        r"   ```   ___   ```",
        r"       ( . . )",
        r"",
        r"        .._..",
        r"      .'     '.",
        r"     `.~~~~~~~.`",
        r"       `-...-`",
        r"",
        r"        HELLO",
    ];
    ART.iter()
        .map(|line| {
            let leading = line.len() - line.trim_start().len();
            let text = format!(
                "{}{}",
                "\u{00A0}".repeat(leading),
                &line[leading..]
            );
            TerminalLine::Output {
                text,
                style: OutputStyle::Literal,
            }
        })
        .chain(std::iter::once(TerminalLine::Blank))
        .collect()
}

pub(super) fn cmd_mute(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.game.sound_muted = true;
    vec![
        TerminalLine::Output {
            text: "sound muted".to_owned(),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

pub(super) fn cmd_unmute(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.game.sound_muted = false;
    vec![
        TerminalLine::Output {
            text: "sound unmuted".to_owned(),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_clear(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.terminal.clear_lines();
    Vec::new()
}

fn cmd_sudo_rm(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.pending_reset = true;
    vec![
        TerminalLine::Output {
            text: "warning: this will permanently erase all progress.".to_owned(),
            style: OutputStyle::Error,
        },
        TerminalLine::Output {
            text: "type CONFIRM to proceed, or anything else to abort.".to_owned(),
            style: OutputStyle::Info,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_exit(_: &str, app: &mut App) -> Vec<TerminalLine> {
    if let Err(e) = save::save(&app.game) {
        return vec![
            TerminalLine::Output {
                text: format!("save failed: {e}"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }
    #[cfg(target_arch = "wasm32")]
    {
        use crate::game::log::push_log;
        app.game
            .log
            .retain(|entry| entry.text != BROWSER_SAVE_LOG_TEXT);
        push_log(
            &mut app.game.log,
            app.game.uptime_secs,
            BROWSER_SAVE_LOG_TEXT,
        );
    }
    app.should_quit = true;
    Vec::new()
}

fn cmd_upgrade_unused(_: &str, _: &mut App) -> Vec<TerminalLine> {
    Vec::new()
}

fn cmd_upgrades(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    for &name in UPGRADES_ORDER {
        let Some(u) = upgrade_by_command(name) else {
            continue;
        };
        if !upgrade_unlocked(&app.game, u.kind) {
            continue;
        }
        let cost_str = format_upgrade_cost(u.cycles_cost, u.entropy_cost);
        out.push(TerminalLine::Output {
            text: format!("{} - {} ({})", u.command, u.description, cost_str),
            style: OutputStyle::Info,
        });
    }
    out.push(TerminalLine::Blank);
    out
}

pub fn run_purchased_upgrade(
    app: &mut App,
    trimmed: &str,
    bypass_unlock: bool,
) -> Option<Vec<TerminalLine>> {
    let u = upgrade_by_command(trimmed)?;
    if !bypass_unlock && !upgrade_unlocked(&app.game, u.kind) {
        return Some(vec![
            TerminalLine::Output {
                text: "upgrade: Permission denied".to_owned(),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ]);
    }
    let (cy, ent) = if is_burst_upgrade(u.kind) {
        let bought = app
            .game
            .burst_purchase_counts
            .get(&u.kind)
            .copied()
            .unwrap_or(0);
        burst_upgrade_cost(u, bought)
    } else {
        (u.cycles_cost, u.entropy_cost)
    };
    let have_c = app.game.resources.get(ResourceKind::Cycles);
    let have_e = app.game.resources.get(ResourceKind::Entropy);
    if have_c < cy {
        return Some(vec![
            TerminalLine::Output {
                text: format!(
                    "insufficient cycles (need {}, have {})",
                    fmt_cycles(cy),
                    fmt_cycles(have_c)
                ),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ]);
    }
    if have_e + 1e-9 < ent {
        return Some(vec![
            TerminalLine::Output {
                text: format!("insufficient entropy (need {ent:.2}, have {have_e:.2})"),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ]);
    }
    app.game.resources.deduct(cy);
    let next_e = (have_e - ent).max(0.0);
    app.game.resources.set(ResourceKind::Entropy, next_e);

    apply_upgrade_purchase(&mut app.game, u.kind, ent);
    if u.kind == UpgradeKind::CatDevUrandom {
        grant_cycle_burst(&mut app.game, 60.0);
    }

    Some(vec![
        TerminalLine::Output {
            text: format!(
                "{} -- {}{}",
                u.command,
                u.description,
                format_upgrade_cost_suffix(cy, ent)
            ),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ])
}