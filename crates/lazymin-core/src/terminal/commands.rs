use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::{fmt_cycles, fmt_mb};
use crate::game::log::push_log;
use crate::game::producers::{all_producers, producer_cost, producer_def, producer_unlocked, ProducerKind};
use crate::game::resources::{
    all_hardware, hardware_def, total_power_draw, total_reserved_bandwidth, total_reserved_disk,
    total_reserved_ram, ResourceKind, KERNEL_DISK_MB, KERNEL_RAM_MB, KERNEL_WATTS,
};
use crate::game::tick::grant_cycle_burst;
use crate::game::upgrades::{
    all_upgrades, apply_upgrade_purchase, effective_disk_cap, upgrade_by_command, upgrade_unlocked,
    UpgradeKind,
};
use once_cell::sync::Lazy;

pub type CommandLocked = fn(&App) -> bool;
pub type CommandCost = fn(&App) -> f64;
pub type CommandExecute = fn(&str, &mut App) -> Vec<TerminalLine>;

#[derive(Clone, Copy)]
pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub locked: CommandLocked,
    pub cost: Option<CommandCost>,
    pub execute: CommandExecute,
}

const HELP_ORDER: &[&str] = &[
    "ls",
    "ps aux",
    "du",
    "ifconfig",
    "lshw",
    "apt install",
    "apt upgrade",
    "clear",
    "exit",
];

const LS_ORDER: &[&str] = &[
    "ps aux",
    "du",
    "ifconfig",
    "lshw",
    "harvest.sh",
    "harvest.sh &",
    "crontab -e",
    "systemctl start harvestd",
    "systemctl enable harvest.service",
    "insmod harvest.ko",
    "virsh start harvest-vm",
    "init 5",
    "ssh remote harvest",
];

const APT_INSTALL_ORDER: &[&str] = &[
    "apt install ram",
    "apt install hdd",
    "apt install nic",
    "apt install psu",
];

const UPGRADES_ORDER: &[&str] = &[
    "shellcheck harvest.sh",
    "alias harvest='harvest.sh'",
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
    "cat /dev/urandom > /dev/null",
    "shuf --random-source=/dev/urandom",
    "openssl rand -base64 32",
    "uuidgen",
    "gpg --gen-key",
    "ssh-keygen -t ed25519",
    "mktemp -d",
    "dd if=/dev/urandom of=/dev/sda",
    "certbot renew",
    "haveged --run",
    "stress-ng --cpu 0",
    "fault-inject enable",
    "reboot --firmware",
    "init 0 && init 6",
    "journald --vacuum-size",
];

fn always_unlocked(_: &App) -> bool {
    false
}

fn locked_producer(app: &App, kind: ProducerKind) -> bool {
    !producer_unlocked(
        app.game.total_cycles_earned,
        &app.game.producers,
        kind,
    )
}

fn producer_cost_for(app: &App, kind: ProducerKind) -> f64 {
    let def = producer_def(kind);
    let owned = app.game.producers.get(&kind).copied().unwrap_or(0);
    let mut p = producer_cost(def, owned);
    if let Some(f) = app.game.pending_producer_cost_factors.front() {
        p *= *f;
    }
    p
}

fn buy_producer(app: &mut App, kind: ProducerKind) -> Vec<TerminalLine> {
    let def = producer_def(kind);

    let reserved_ram = total_reserved_ram(&app.game.producers);
    let ram_cap = app.game.resources.cap(ResourceKind::Ram).unwrap_or(0.0);

    if reserved_ram + def.ram_mb > ram_cap {
        let free_ram = (ram_cap - reserved_ram).max(0.0);
        return vec![
            TerminalLine::Output {
                text: format!(
                    "insufficient memory (need {:.0} MB, have {:.0} MB free)",
                    def.ram_mb, free_ram
                ),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    let reserved_disk = total_reserved_disk(&app.game.producers);
    let disk_cap = effective_disk_cap(&app.game);
    if reserved_disk + def.disk_mb + app.game.disk_log_usage > disk_cap + 1e-6 {
        let free = (disk_cap - reserved_disk - app.game.disk_log_usage).max(0.0);
        return vec![
            TerminalLine::Output {
                text: format!(
                    "insufficient disk space (need {:.0} MB, have {:.0} MB free)",
                    def.disk_mb, free
                ),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }

    if def.bw_mbps > 0.0 {
        let bw_cap = app.game.resources.cap(ResourceKind::Bandwidth).unwrap_or(0.0);
        let reserved_bw = total_reserved_bandwidth(&app.game.producers);
        if reserved_bw + def.bw_mbps > bw_cap + 1e-6 {
            let free = (bw_cap - reserved_bw).max(0.0);
            return vec![
                TerminalLine::Output {
                    text: format!(
                        "insufficient bandwidth (need {:.1} Mbps, have {:.1} Mbps free)",
                        def.bw_mbps, free
                    ),
                    style: OutputStyle::Error,
                },
                TerminalLine::Blank,
            ];
        }
    }

    let owned_before = app.game.producers.get(&kind).copied().unwrap_or(0);
    let mut price = producer_cost(def, owned_before);
    if let Some(f) = app.game.pending_producer_cost_factors.pop_front() {
        price *= f;
    }
    app.game.resources.deduct(price);

    let owned = app
        .game
        .producers
        .entry(kind)
        .and_modify(|count| *count += 1)
        .or_insert(1);

    push_log(
        &mut app.game.log,
        app.game.uptime_secs,
        format!("{} purchased -- +{:.0} cycles/s", def.name.to_lowercase(), def.base_cycles_per_s),
    );

    vec![
        TerminalLine::Output {
            text: format!("[{owned}] {}  -- +{:.0} cycles/s", def.command, def.base_cycles_per_s),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cap_upgrade_cost(base_cost: f64, purchases: u32) -> f64 {
    base_cost * 1.15_f64.powi(purchases as i32)
}

fn capacity_cost_basis_count(app: &App, kind: ResourceKind) -> u32 {
    app.game.hardware_cost_basis.get(&kind).copied().unwrap_or(0)
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
                    "power budget exceeded (need {:.1} W, have {:.1} W free)",
                    hw.watts, free_watts
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
    push_log(
        &mut app.game.log,
        app.game.uptime_secs,
        format!("{} capacity expanded", hw.label),
    );

    vec![
        TerminalLine::Output {
            text: format!("{} capacity now {:.0}", hw.label, cap),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_help(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    for &name in HELP_ORDER {
        let Some(cmd) = registry_command(name) else {
            continue;
        };
        if (cmd.locked)(app) {
            continue;
        }
        out.push(TerminalLine::Output {
            text: format!("{} - {}", cmd.name, cmd.description),
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

fn cmd_buy_shell_script(_: &str, app: &mut App) -> Vec<TerminalLine> {
    buy_producer(app, ProducerKind::ShellScript)
}

fn cmd_buy_cron_job(_: &str, app: &mut App) -> Vec<TerminalLine> { buy_producer(app, ProducerKind::CronJob) }
fn cmd_buy_daemon(_: &str, app: &mut App) -> Vec<TerminalLine> { buy_producer(app, ProducerKind::Daemon) }
fn cmd_buy_service_unit(_: &str, app: &mut App) -> Vec<TerminalLine> { buy_producer(app, ProducerKind::ServiceUnit) }
fn cmd_buy_kernel_module(_: &str, app: &mut App) -> Vec<TerminalLine> { buy_producer(app, ProducerKind::KernelModule) }
fn cmd_buy_hypervisor(_: &str, app: &mut App) -> Vec<TerminalLine> { buy_producer(app, ProducerKind::Hypervisor) }
fn cmd_buy_os_takeover(_: &str, app: &mut App) -> Vec<TerminalLine> { buy_producer(app, ProducerKind::OsTakeover) }

fn lock_cron_job(app: &App) -> bool { locked_producer(app, ProducerKind::CronJob) }
fn lock_daemon(app: &App) -> bool { locked_producer(app, ProducerKind::Daemon) }
fn lock_service_unit(app: &App) -> bool { locked_producer(app, ProducerKind::ServiceUnit) }
fn lock_kernel_module(app: &App) -> bool { locked_producer(app, ProducerKind::KernelModule) }
fn lock_hypervisor(app: &App) -> bool { locked_producer(app, ProducerKind::Hypervisor) }
fn lock_os_takeover(app: &App) -> bool { locked_producer(app, ProducerKind::OsTakeover) }

fn shell_script_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::ShellScript) }
fn cron_job_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::CronJob) }
fn daemon_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::Daemon) }
fn service_unit_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::ServiceUnit) }
fn kernel_module_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::KernelModule) }
fn hypervisor_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::Hypervisor) }
fn os_takeover_cost(app: &App) -> f64 { producer_cost_for(app, ProducerKind::OsTakeover) }

fn apt_ram_cost(app: &App) -> f64 { capacity_command_cost_for(app, ResourceKind::Ram) }
fn apt_disk_cost(app: &App) -> f64 { capacity_command_cost_for(app, ResourceKind::Disk) }
fn apt_bw_cost(app: &App) -> f64 { capacity_command_cost_for(app, ResourceKind::Bandwidth) }
fn apt_watts_cost(app: &App) -> f64 { capacity_command_cost_for(app, ResourceKind::Watts) }

fn cmd_buy_ram(_: &str, app: &mut App) -> Vec<TerminalLine> {
    buy_capacity(app, ResourceKind::Ram)
}
fn cmd_buy_disk(_: &str, app: &mut App) -> Vec<TerminalLine> {
    buy_capacity(app, ResourceKind::Disk)
}
fn cmd_buy_bw(_: &str, app: &mut App) -> Vec<TerminalLine> {
    buy_capacity(app, ResourceKind::Bandwidth)
}
fn cmd_buy_watts(_: &str, app: &mut App) -> Vec<TerminalLine> {
    buy_capacity(app, ResourceKind::Watts)
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

fn cmd_apt_install(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    for &name in APT_INSTALL_ORDER {
        let Some(kind) = apt_install_resource(name) else {
            continue;
        };
        let Some(cmd) = registry_command(name) else {
            continue;
        };
        let next = capacity_command_cost_for(app, kind);
        out.push(TerminalLine::Output {
            text: format!(
                "{} - {} (next: {} cycles)",
                name,
                cmd.description,
                fmt_cycles(next)
            ),
            style: OutputStyle::Info,
        });
    }

    out.push(TerminalLine::Blank);
    out
}

fn cmd_ps_aux(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let ram_cap = app.game.resources.cap(ResourceKind::Ram).unwrap_or(0.0).max(1e-9);

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
            fmt_mb(KERNEL_RAM_MB),
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
                    fmt_mb(def.ram_mb),
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
    out
}

fn cmd_du(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = vec![TerminalLine::Output {
        text: format!("{:<44}{}", "FILESYSTEM", "SIZE"),
        style: OutputStyle::Info,
    }];

    let mut reserved_total = KERNEL_DISK_MB;
    out.push(TerminalLine::Output {
        text: format!("{:<44}{}", "/boot/vmlinuz", fmt_mb(KERNEL_DISK_MB)),
        style: OutputStyle::System,
    });

    for def in all_producers() {
        let count = app.game.producers.get(&def.kind).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        let mb = def.disk_mb * (count as f64);
        reserved_total += mb;
        out.push(TerminalLine::Output {
            text: format!(
                "{:<44}{}",
                format!("{}  (×{count})", def.command),
                fmt_mb(mb)
            ),
            style: OutputStyle::System,
        });
    }

    let logs = app.game.disk_log_usage;
    if logs > 0.0 {
        out.push(TerminalLine::Output {
            text: format!("{:<44}{}", "/var/log", fmt_mb(logs)),
            style: OutputStyle::System,
        });
    }

    let disk_cap = effective_disk_cap(&app.game);
    let used = reserved_total + logs;

    out.push(TerminalLine::Output {
        text: format!(
            "{:<44}{} / {}",
            "total",
            fmt_mb(used),
            fmt_mb(disk_cap)
        ),
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

    let cap = app.game.resources.cap(ResourceKind::Bandwidth).unwrap_or(0.0);
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
                "{:<44}{:.1} Mbps",
                format!("{}  (×{count})", def.command),
                mbps
            ),
            style: OutputStyle::System,
        });
    }

    if app.game.remote_channel_active {
        any = true;
        let spare = (cap - reserved).max(0.0);
        out.push(TerminalLine::Output {
            text: format!("{:<44}{:.1} Mbps free", "ssh remote harvest", spare),
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
            text: format!("{:<44}{:.1} / {:.1} Mbps", "total", reserved, cap),
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
        text: format!("{:<44}{:.1} W", "kernel", KERNEL_WATTS),
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
                "{:<44}{:.1} W",
                format!("{}  (×{count})", hw.label),
                w
            ),
            style: OutputStyle::System,
        });
    }

    let watts_cap = app.game.resources.cap(ResourceKind::Watts).unwrap_or(0.0);
    out.push(TerminalLine::Output {
        text: format!("{:<44}{:.1} / {:.1} W", "total", total_w, watts_cap),
        style: OutputStyle::System,
    });

    out.push(TerminalLine::Blank);
    out
}

fn cmd_ls(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let hide_harvest_bg = app
        .game
        .producers
        .get(&ProducerKind::ShellScript)
        .copied()
        .unwrap_or(0)
        == 0;

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

fn cmd_clear(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.terminal.clear_lines();
    Vec::new()
}

fn cmd_exit(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.should_quit = true;
    Vec::new()
}

fn cmd_upgrade_unused(_: &str, _: &mut App) -> Vec<TerminalLine> {
    Vec::new()
}

fn journald_vacuum_cost(app: &App) -> f64 {
    500.0 * 1.15_f64.powi(app.game.journald_vacuum_count as i32)
}

fn cmd_journald_vacuum(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let cost = journald_vacuum_cost(app);
    if !app.game.resources.can_afford(cost) {
        return vec![
            TerminalLine::Output {
                text: format!(
                    "insufficient cycles (need {}, have {})",
                    fmt_cycles(cost),
                    fmt_cycles(app.game.resources.get(ResourceKind::Cycles))
                ),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ];
    }
    app.game.resources.deduct(cost);
    app.game.disk_log_usage = 0.0;
    app.game.journald_vacuum_count += 1;
    push_log(
        &mut app.game.log,
        app.game.uptime_secs,
        "log volume reclaimed (disk usage from logs cleared)",
    );
    vec![
        TerminalLine::Output {
            text: "vacuum complete: log-backed disk usage reset to 0".to_owned(),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn lock_ssh_remote(app: &App) -> bool {
    app.game.remote_channel_active
        || app
            .game
            .capacity_purchases
            .get(&ResourceKind::Bandwidth)
            .copied()
            .unwrap_or(0)
            < 1
}

fn cmd_ssh_remote(_: &str, app: &mut App) -> Vec<TerminalLine> {
    app.game.remote_channel_active = true;
    push_log(
        &mut app.game.log,
        app.game.uptime_secs,
        "remote harvest channel active (spare bandwidth -> cycles)",
    );
    vec![
        TerminalLine::Output {
            text: "ssh: remote harvest tunnel established".to_owned(),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_upgrades(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    for &name in UPGRADES_ORDER {
        if name == "journald --vacuum-size" {
            let journald_cost = journald_vacuum_cost(app);
            out.push(TerminalLine::Output {
                text: format!(
                    "journald --vacuum-size - clear accumulated log disk usage (next: {} cycles)",
                    fmt_cycles(journald_cost)
                ),
                style: OutputStyle::Info,
            });
            continue;
        }
        let Some(u) = upgrade_by_command(name) else {
            continue;
        };
        if !upgrade_unlocked(&app.game, u.kind) {
            continue;
        }
        let mut parts = Vec::new();
        if u.cycles_cost > 0.0 {
            parts.push(format!("{} cycles", fmt_cycles(u.cycles_cost)));
        }
        if u.entropy_cost > 0.0 {
            parts.push(format!("{:.2} ent", u.entropy_cost));
        }
        let cost_str = if parts.is_empty() {
            "free".to_owned()
        } else {
            parts.join(" + ")
        };
        out.push(TerminalLine::Output {
            text: format!("{} - {} ({})", u.command, u.description, cost_str),
            style: OutputStyle::Info,
        });
    }
    out.push(TerminalLine::Blank);
    out
}

pub fn run_purchased_upgrade(app: &mut App, trimmed: &str) -> Option<Vec<TerminalLine>> {
    let u = upgrade_by_command(trimmed)?;
    if !upgrade_unlocked(&app.game, u.kind) {
        return Some(vec![
            TerminalLine::Output {
                text: "bash: upgrade: Permission denied".to_owned(),
                style: OutputStyle::Error,
            },
            TerminalLine::Blank,
        ]);
    }
    let cy = u.cycles_cost;
    let ent = u.entropy_cost;
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

    apply_upgrade_purchase(&mut app.game, u.kind);
    if u.kind == UpgradeKind::CatDevUrandom {
        grant_cycle_burst(&mut app.game, 60.0);
    }

    push_log(
        &mut app.game.log,
        app.game.uptime_secs,
        format!("upgrade installed: {}", u.command),
    );
    Some(vec![
        TerminalLine::Output {
            text: format!("{} -- {}", u.command, u.description),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ])
}

static BASE_COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "harvest.sh",
        description: "run the harvest script manually",
        locked: always_unlocked,
        cost: None,
        execute: cmd_harvest,
    },
    CommandDef {
        name: "harvest.sh &",
        description: "run harvest script in the background",
        locked: always_unlocked,
        cost: Some(shell_script_cost),
        execute: cmd_buy_shell_script,
    },
    CommandDef {
        name: "crontab -e",
        description: "schedule harvest as a cron job",
        locked: lock_cron_job,
        cost: Some(cron_job_cost),
        execute: cmd_buy_cron_job,
    },
    CommandDef {
        name: "systemctl start harvestd",
        description: "start the harvest daemon",
        locked: lock_daemon,
        cost: Some(daemon_cost),
        execute: cmd_buy_daemon,
    },
    CommandDef {
        name: "systemctl enable harvest.service",
        description: "enable persistent harvest service",
        locked: lock_service_unit,
        cost: Some(service_unit_cost),
        execute: cmd_buy_service_unit,
    },
    CommandDef {
        name: "insmod harvest.ko",
        description: "load kernel-level harvesting",
        locked: lock_kernel_module,
        cost: Some(kernel_module_cost),
        execute: cmd_buy_kernel_module,
    },
    CommandDef {
        name: "virsh start harvest-vm",
        description: "start hypervisor automation",
        locked: lock_hypervisor,
        cost: Some(hypervisor_cost),
        execute: cmd_buy_hypervisor,
    },
    CommandDef {
        name: "init 5",
        description: "handoff to full OS takeover",
        locked: lock_os_takeover,
        cost: Some(os_takeover_cost),
        execute: cmd_buy_os_takeover,
    },
    CommandDef {
        name: "apt install ram",
        description: "expand ram capacity (+256 MB)",
        locked: always_unlocked,
        cost: Some(apt_ram_cost),
        execute: cmd_buy_ram,
    },
    CommandDef {
        name: "apt install hdd",
        description: "expand disk capacity (+1 GB)",
        locked: always_unlocked,
        cost: Some(apt_disk_cost),
        execute: cmd_buy_disk,
    },
    CommandDef {
        name: "apt install nic",
        description: "expand bandwidth capacity (+10 Mbps)",
        locked: always_unlocked,
        cost: Some(apt_bw_cost),
        execute: cmd_buy_bw,
    },
    CommandDef {
        name: "apt install psu",
        description: "expand power capacity (+50 W)",
        locked: always_unlocked,
        cost: Some(apt_watts_cost),
        execute: cmd_buy_watts,
    },
    CommandDef {
        name: "help",
        description: "list currently unlocked commands",
        locked: always_unlocked,
        cost: None,
        execute: cmd_help,
    },
    CommandDef {
        name: "ls",
        description: "list commands",
        locked: always_unlocked,
        cost: None,
        execute: cmd_ls,
    },
    CommandDef {
        name: "apt install",
        description: "list hardware packages",
        locked: always_unlocked,
        cost: None,
        execute: cmd_apt_install,
    },
    CommandDef {
        name: "apt upgrade",
        description: "list package upgrades",
        locked: always_unlocked,
        cost: None,
        execute: cmd_upgrades,
    },
    CommandDef {
        name: "journald --vacuum-size",
        description: "clear accumulated log disk usage (cost scales)",
        locked: always_unlocked,
        cost: Some(journald_vacuum_cost),
        execute: cmd_journald_vacuum,
    },
    CommandDef {
        name: "ssh remote harvest",
        description: "use spare bandwidth for bonus cycles/s",
        locked: lock_ssh_remote,
        cost: None,
        execute: cmd_ssh_remote,
    },
    CommandDef {
        name: "clear",
        description: "clear the terminal history",
        locked: always_unlocked,
        cost: None,
        execute: cmd_clear,
    },
    CommandDef {
        name: "ps aux",
        description: "show running processes",
        locked: always_unlocked,
        cost: None,
        execute: cmd_ps_aux,
    },
    CommandDef {
        name: "du",
        description: "show disk usage",
        locked: always_unlocked,
        cost: None,
        execute: cmd_du,
    },
    CommandDef {
        name: "ifconfig",
        description: "show bandwidth usage",
        locked: always_unlocked,
        cost: None,
        execute: cmd_ifconfig,
    },
    CommandDef {
        name: "lshw",
        description: "show power draw",
        locked: always_unlocked,
        cost: None,
        execute: cmd_lshw,
    },
    CommandDef {
        name: "exit",
        description: "save and quit",
        locked: always_unlocked,
        cost: None,
        execute: cmd_exit,
    },
];

static COMMAND_REGISTRY: Lazy<&'static [CommandDef]> = Lazy::new(|| {
    let mut v: Vec<CommandDef> = BASE_COMMANDS.to_vec();
    for u in all_upgrades() {
        v.push(CommandDef {
            name: u.command,
            description: u.description,
            locked: always_unlocked,
            cost: None,
            execute: cmd_upgrade_unused,
        });
    }
    Box::leak(v.into_boxed_slice())
});

pub fn command_registry() -> &'static [CommandDef] {
    COMMAND_REGISTRY.as_ref()
}

fn registry_command(name: &str) -> Option<&'static CommandDef> {
    command_registry().iter().find(|c| c.name == name)
}

#[cfg(test)]
mod command_order_tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn all_base_commands_accounted_for_in_ordered_lists() {
        let all_ordered: HashSet<&str> = HELP_ORDER
            .iter()
            .chain(LS_ORDER.iter())
            .chain(APT_INSTALL_ORDER.iter())
            .chain(UPGRADES_ORDER.iter())
            .copied()
            .collect();

        for cmd in BASE_COMMANDS {
            if cmd.name == "help" {
                continue;
            }
            assert!(
                all_ordered.contains(cmd.name),
                "command '{}' is not in any *_ORDER list — add it or exclude it in this test",
                cmd.name
            );
        }

        for u in all_upgrades() {
            assert!(
                UPGRADES_ORDER.contains(&u.command),
                "upgrade '{}' missing from UPGRADES_ORDER",
                u.command
            );
        }

        for &name in UPGRADES_ORDER {
            if name == "journald --vacuum-size" {
                continue;
            }
            assert!(
                upgrade_by_command(name).is_some(),
                "UPGRADES_ORDER entry '{}' is not a registered upgrade",
                name
            );
        }
    }
}

