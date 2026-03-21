use crate::app::{App, OutputStyle, TerminalLine};
use crate::format::fmt_cycles;
use crate::game::log::push_log;
use crate::game::producers::{producer_cost, producer_def, ProducerKind};

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

fn cmd_help(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();
    let hidden_in_help = ["harvest.sh", "harvest.sh &", "help"];

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
    app.game.cycles += yield_cycles;
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

fn shell_script_cost(app: &App) -> f64 {
    let def = producer_def(ProducerKind::ShellScript);
    let owned = app
        .game
        .producers
        .get(&ProducerKind::ShellScript)
        .copied()
        .unwrap_or(0);
    producer_cost(def, owned)
}

fn cmd_buy_shell_script(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let price = shell_script_cost(app);
    app.game.cycles -= price;

    let owned = app
        .game
        .producers
        .entry(ProducerKind::ShellScript)
        .and_modify(|count| *count += 1)
        .or_insert(1);

    let rate = producer_def(ProducerKind::ShellScript).base_cycles_per_s;

    push_log(
        &mut app.game.log,
        app.game.uptime_secs,
        format!("shell script purchased -- +{rate:.0} cycles/s"),
    );

    vec![
        TerminalLine::Output {
            text: format!("[{owned}] harvest.sh &  -- +{rate:.0} cycles/s"),
            style: OutputStyle::System,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_ps_aux(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = vec![TerminalLine::Output {
        text: "PID   COMMAND".to_owned(),
        style: OutputStyle::Info,
    }];

    let scripts = app
        .game
        .producers
        .get(&ProducerKind::ShellScript)
        .copied()
        .unwrap_or(0);

    if scripts == 0 {
        out.push(TerminalLine::Output {
            text: "      no background jobs running".to_owned(),
            style: OutputStyle::System,
        });
    } else {
        for i in 1..=scripts {
            out.push(TerminalLine::Output {
                text: format!("{i:<6}harvest.sh &"),
                style: OutputStyle::System,
            });
        }
    }

    out.push(TerminalLine::Blank);
    out
}

fn cmd_ls(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let hidden_in_ls = ["help", "clear", "ls", "exit", "harvest.sh &"];
    let names: Vec<&str> = command_registry()
        .iter()
        .filter(|cmd| !(cmd.locked)(app))
        .filter(|cmd| !hidden_in_ls.contains(&cmd.name))
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

