use once_cell::sync::Lazy;

use crate::app::{App, TerminalLine};

use crate::game::upgrades::all_upgrades;

use super::locks::{
    always_unlocked, lock_apt_install, lock_apt_update, lock_apt_upgrade, lock_cluster,
    lock_competitors, lock_cron_job, lock_daemon, lock_distributed_fabric, lock_endgame,
    lock_hypervisor, lock_kernel_module, lock_market, lock_neural_substrate, lock_no_bw_producer,
    lock_no_disk_producer, lock_no_mem_producer, lock_no_power_hardware, lock_os_takeover,
    lock_research, lock_service_unit,
};

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

static BASE_COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "harvest.sh",
        description: "run the harvest script manually",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_harvest,
    },
    CommandDef {
        name: "harvest.sh &",
        description: "run harvest script in the background",
        locked: always_unlocked,
        cost: Some(super::shell_script_cost),
        execute: super::cmd_buy_shell_script,
    },
    CommandDef {
        name: "crontab harvest.cron",
        description: "schedule harvest as a cron job",
        locked: lock_cron_job,
        cost: Some(super::cron_job_cost),
        execute: super::cmd_buy_cron_job,
    },
    CommandDef {
        name: "systemctl start harvestd",
        description: "start the harvest daemon",
        locked: lock_daemon,
        cost: Some(super::daemon_cost),
        execute: super::cmd_buy_daemon,
    },
    CommandDef {
        name: "systemctl enable harvest.service",
        description: "enable persistent harvest service",
        locked: lock_service_unit,
        cost: Some(super::service_unit_cost),
        execute: super::cmd_buy_service_unit,
    },
    CommandDef {
        name: "insmod harvest.ko",
        description: "load kernel-level harvesting",
        locked: lock_kernel_module,
        cost: Some(super::kernel_module_cost),
        execute: super::cmd_buy_kernel_module,
    },
    CommandDef {
        name: "virsh start harvest-vm",
        description: "start hypervisor automation",
        locked: lock_hypervisor,
        cost: Some(super::hypervisor_cost),
        execute: super::cmd_buy_hypervisor,
    },
    CommandDef {
        name: "init 5",
        description: "handoff to full OS takeover",
        locked: lock_os_takeover,
        cost: Some(super::os_takeover_cost),
        execute: super::cmd_buy_os_takeover,
    },
    CommandDef {
        name: "kubectl apply -f harvest.yaml",
        description: "deploy harvest across cluster nodes",
        locked: lock_cluster,
        cost: Some(super::cluster_cost),
        execute: super::cmd_buy_cluster,
    },
    CommandDef {
        name: "terraform apply harvest",
        description: "provision distributed fabric",
        locked: lock_distributed_fabric,
        cost: Some(super::distributed_fabric_cost),
        execute: super::cmd_buy_distributed_fabric,
    },
    CommandDef {
        name: "deploy --model harvest-net",
        description: "deploy neural substrate harvester",
        locked: lock_neural_substrate,
        cost: Some(super::neural_substrate_cost),
        execute: super::cmd_buy_neural_substrate,
    },
    // apt install ram|hdd|nic|psu: description text is built from hardware cap_delta
    // in crate::game::resources::apt_install_hardware_description (see command_player_description)
    CommandDef {
        name: "apt install ram",
        description: "",
        locked: lock_apt_install,
        cost: Some(super::apt_ram_cost),
        execute: super::cmd_buy_ram,
    },
    CommandDef {
        name: "apt install hdd",
        description: "",
        locked: lock_apt_install,
        cost: Some(super::apt_disk_cost),
        execute: super::cmd_buy_disk,
    },
    CommandDef {
        name: "apt install nic",
        description: "",
        locked: lock_apt_install,
        cost: Some(super::apt_bw_cost),
        execute: super::cmd_buy_bw,
    },
    CommandDef {
        name: "apt install psu",
        description: "",
        locked: lock_apt_install,
        cost: Some(super::apt_watts_cost),
        execute: super::cmd_buy_watts,
    },
    CommandDef {
        name: "hello",
        description: "say hi",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_hello,
    },
    CommandDef {
        name: "mute",
        description: "mute all sound output",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_mute,
    },
    CommandDef {
        name: "unmute",
        description: "unmute all sound output",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_unmute,
    },
    CommandDef {
        name: "help",
        description: "list currently unlocked commands",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_help,
    },
    CommandDef {
        name: "ls",
        description: "list producers",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_ls,
    },
    CommandDef {
        name: "apt install",
        description: "list hardware packages",
        locked: lock_apt_install,
        cost: None,
        execute: super::cmd_apt_install,
    },
    CommandDef {
        name: "apt update",
        description: "list one-shot upgrades",
        locked: lock_apt_update,
        cost: None,
        execute: super::cmd_apt_update,
    },
    CommandDef {
        name: "apt upgrade",
        description: "list permanent upgrades",
        locked: lock_apt_upgrade,
        cost: None,
        execute: super::cmd_upgrades,
    },
    CommandDef {
        name: "sudo rm -rf /*",
        description: "reset all game progress (requires confirmation)",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_sudo_rm,
    },
    CommandDef {
        name: "rm -rf /*",
        description: "soft reset: recycle entropy into permanent bonus",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_soft_reset,
    },
    CommandDef {
        name: "clear",
        description: "clear the terminal history",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_clear,
    },
    CommandDef {
        name: "ps aux",
        description: "show running processes",
        locked: lock_no_mem_producer,
        cost: None,
        execute: super::cmd_ps_aux,
    },
    CommandDef {
        name: "pkill",
        description: "kill a producer process by PID",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_pkill,
    },
    CommandDef {
        name: "du",
        description: "show disk usage",
        locked: lock_no_disk_producer,
        cost: None,
        execute: super::cmd_du,
    },
    CommandDef {
        name: "ifconfig",
        description: "show bandwidth usage",
        locked: lock_no_bw_producer,
        cost: None,
        execute: super::cmd_ifconfig,
    },
    CommandDef {
        name: "lshw",
        description: "show power draw",
        locked: lock_no_power_hardware,
        cost: None,
        execute: super::cmd_lshw,
    },
    CommandDef {
        name: "mb",
        description: "buy 1 coolant at market price",
        locked: lock_market,
        cost: Some(super::market_buy_cost),
        execute: super::cmd_market_buy,
    },
    CommandDef {
        name: "hack",
        description: "reduce a competitor's value",
        locked: lock_competitors,
        cost: None,
        execute: super::cmd_hack,
    },
    CommandDef {
        name: "invest",
        description: "increase a competitor's value",
        locked: lock_competitors,
        cost: None,
        execute: super::cmd_invest,
    },
    CommandDef {
        name: "buyout",
        description: "acquire a weakened competitor",
        locked: lock_competitors,
        cost: None,
        execute: super::cmd_buyout,
    },
    CommandDef {
        name: "research",
        description: "view or start research projects",
        locked: lock_research,
        cost: None,
        execute: super::cmd_research,
    },
    CommandDef {
        name: "shutdown --graceful",
        description: "initiate graceful shutdown sequence",
        locked: lock_endgame,
        cost: None,
        execute: super::cmd_shutdown_graceful,
    },
    CommandDef {
        name: "exit",
        description: "save and quit",
        locked: always_unlocked,
        cost: None,
        execute: super::cmd_exit,
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
            execute: super::cmd_upgrade_unused,
        });
    }

    Box::leak(v.into_boxed_slice())
});

pub fn command_registry() -> &'static [CommandDef] {
    COMMAND_REGISTRY.as_ref()
}

pub(super) fn registry_command(name: &str) -> Option<&'static CommandDef> {
    command_registry().iter().find(|c| c.name == name)
}

#[cfg(test)]
mod command_order_tests {
    use std::collections::HashSet;

    use super::*;

    use crate::game::upgrades::{all_upgrades, upgrade_by_command};

    #[test]
    fn all_base_commands_accounted_for_in_ordered_lists() {
        let all_ordered: HashSet<&str> = super::super::HELP_ORDER
            .iter()
            .chain(super::super::LS_ORDER.iter())
            .chain(super::super::APT_INSTALL_ORDER.iter())
            .chain(super::super::APT_UPDATE_ORDER.iter())
            .chain(super::super::UPGRADES_ORDER.iter())
            .copied()
            .collect();

        for cmd in BASE_COMMANDS {
            if cmd.name == "hello"
                || cmd.name == "help"
                || cmd.name == "mute"
                || cmd.name == "unmute"
                || cmd.name == "sudo rm -rf /*"
                || cmd.name == "rm -rf /*"
                || cmd.name == "pkill"
                || cmd.name == "mb"
                || cmd.name == "hack"
                || cmd.name == "invest"
                || cmd.name == "buyout"
                || cmd.name == "research"
                || cmd.name == "shutdown --graceful"
            {
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
                super::super::UPGRADES_ORDER.contains(&u.command)
                    || super::super::APT_UPDATE_ORDER.contains(&u.command),
                "upgrade '{}' missing from UPGRADES_ORDER/APT_UPDATE_ORDER",
                u.command
            );
        }

        for &name in super::super::UPGRADES_ORDER {
            assert!(
                upgrade_by_command(name).is_some(),
                "UPGRADES_ORDER entry '{}' is not a registered upgrade",
                name
            );
        }

        for &name in super::super::APT_UPDATE_ORDER {
            assert!(
                upgrade_by_command(name).is_some(),
                "APT_UPDATE_ORDER entry '{}' is not a registered upgrade",
                name
            );
        }
    }
}

