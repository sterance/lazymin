use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::log::push_log;
use crate::game::producers::{all_producers, producer_cost, producer_def, producer_unlocked, ProducerKind};
use crate::game::resources::{hardware_def, total_hardware_watts, total_reserved_ram, ResourceKind};

pub type CommandLocked = fn(&App) -> bool;
pub type CommandCost = fn(&App) -> f64;
pub type CommandExecute = fn(&str, &mut App) -> Vec<TerminalLine>;

pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub locked: CommandLocked,
    pub cost: Option<CommandCost>,
    pub execute: CommandExecute,
}

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
    producer_cost(def, owned)
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

    let price = producer_cost_for(app, kind);
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

fn capacity_purchase_count(app: &App, kind: ResourceKind) -> u32 {
    app.game.capacity_purchases.get(&kind).copied().unwrap_or(0)
}

fn capacity_command_cost_for(app: &App, kind: ResourceKind) -> f64 {
    let base = hardware_def(kind).base_cost;
    cap_upgrade_cost(base, capacity_purchase_count(app, kind))
}

fn buy_capacity(app: &mut App, kind: ResourceKind) -> Vec<TerminalLine> {
    let hw = hardware_def(kind);
    let cost = capacity_command_cost_for(app, kind);
    let watts_cap = app.game.resources.cap(ResourceKind::Watts).unwrap_or(0.0);
    let used_watts = total_hardware_watts(&app.game.capacity_purchases);

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
    app.game.resources.add_cap(kind, hw.cap_delta);
    app.game
        .capacity_purchases
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
    let hidden_in_help = [
        "harvest.sh",
        "harvest.sh &",
        "help",
        "apt install ram",
        "apt install hdd",
        "apt install nic",
        "apt install psu",
    ];

    let cmds = command_registry();
    for cmd in cmds {
        if (cmd.locked)(app) || hidden_in_help.contains(&cmd.name) {
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
    let yield_cycles = 1.0;
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

fn cmd_apt_install(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let rows: [(&str, ResourceKind, &str); 4] = [
        (
            "apt install ram",
            ResourceKind::Ram,
            "expand ram capacity (+256 MB)",
        ),
        (
            "apt install hdd",
            ResourceKind::Disk,
            "expand disk capacity (+1 GB)",
        ),
        (
            "apt install nic",
            ResourceKind::Bandwidth,
            "expand bandwidth capacity (+10 Mbps)",
        ),
        (
            "apt install psu",
            ResourceKind::Watts,
            "expand power capacity (+50 W)",
        ),
    ];

    let mut out: Vec<TerminalLine> = rows
        .into_iter()
        .map(|(name, kind, desc)| {
            let next = capacity_command_cost_for(app, kind);
            TerminalLine::Output {
                text: format!("{} - {} (next: {} cycles)", name, desc, fmt_cycles(next)),
                style: OutputStyle::Info,
            }
        })
        .collect();

    out.push(TerminalLine::Blank);
    out
}

fn cmd_ps_aux(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = vec![TerminalLine::Output {
        text: "PID   COMMAND".to_owned(),
        style: OutputStyle::Info,
    }];

    let mut pid = 1000_u32;
    let mut any = false;
    for def in all_producers() {
        let count = app.game.producers.get(&def.kind).copied().unwrap_or(0);
        for _ in 0..count {
            any = true;
            out.push(TerminalLine::Output {
                text: format!("{pid:<6}{}", def.command),
                style: OutputStyle::System,
            });
            pid += 1;
        }
    }

    if !any {
        out.push(TerminalLine::Output {
            text: "      no background jobs running".to_owned(),
            style: OutputStyle::System,
        });
    }

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

    let hidden_in_ls = [
        "help",
        "clear",
        "ls",
        "exit",
        "apt install",
        "apt install ram",
        "apt install hdd",
        "apt install nic",
        "apt install psu",
    ];
    let names: Vec<&str> = command_registry()
        .iter()
        .filter(|cmd| !(cmd.locked)(app))
        .filter(|cmd| !hidden_in_ls.contains(&cmd.name))
        .filter(|cmd| !(cmd.name == "harvest.sh &" && hide_harvest_bg))
        .map(|cmd| cmd.name)
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

static COMMANDS: &[CommandDef] = &[
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
        name: "exit",
        description: "save and quit",
        locked: always_unlocked,
        cost: None,
        execute: cmd_exit,
    },
];

pub fn command_registry() -> &'static [CommandDef] {
    COMMANDS
}

