use crate::app::{App, OutputStyle, TerminalLine};

pub type CommandLocked = fn(&App) -> bool;
pub type CommandExecute = fn(&str, &mut App) -> Vec<TerminalLine>;

pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
    pub locked: CommandLocked,
    pub execute: CommandExecute,
}

fn always_unlocked(_: &App) -> bool {
    false
}

fn cmd_help(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let mut out = Vec::new();

    let cmds = command_registry();
    for cmd in cmds {
        if (cmd.locked)(app) || cmd.name == "harvest.sh" {
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
            text: "harvested 1 cycles".to_owned(),
            style: OutputStyle::Success,
        },
        TerminalLine::Blank,
    ]
}

fn cmd_ls(_: &str, app: &mut App) -> Vec<TerminalLine> {
    let hidden_in_ls = ["help", "clear", "ls", "exit"];
    let names: Vec<&str> = command_registry()
        .iter()
        .filter(|cmd| !(cmd.locked)(app))
        .filter(|cmd| !hidden_in_ls.contains(&cmd.name))
        .map(|cmd| cmd.name)
        .collect();

    let listing = names
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .join("  ");

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
        execute: cmd_harvest,
    },
    CommandDef {
        name: "help",
        description: "list currently unlocked commands",
        locked: always_unlocked,
        execute: cmd_help,
    },
    CommandDef {
        name: "ls",
        description: "list commands as files",
        locked: always_unlocked,
        execute: cmd_ls,
    },
    CommandDef {
        name: "clear",
        description: "clear the terminal history",
        locked: always_unlocked,
        execute: cmd_clear,
    },
    CommandDef {
        name: "exit",
        description: "save and quit",
        locked: always_unlocked,
        execute: cmd_exit,
    },
];

pub fn command_registry() -> &'static [CommandDef] {
    COMMANDS
}

