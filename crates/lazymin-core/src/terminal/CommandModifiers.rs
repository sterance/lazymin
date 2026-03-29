// registry entries only define how input is parsed; runtime behavior for each kind is wired in
// execute.rs / highlight.rs (sudo bypass, -max purchase loop, etc.).

use crate::game::upgrades::upgrade_by_command;
use crate::terminal::commands::command_registry;

fn is_known_command_or_upgrade(s: &str) -> bool {
    upgrade_by_command(s).is_some() || command_registry().iter().any(|cmd| cmd.name == s)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierKind {
    Sudo = 0,
    Max = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandModifiers(u8);

impl CommandModifiers {
    pub fn has(self, k: ModifierKind) -> bool {
        self.0 & (1u8 << k as u8) != 0
    }

    fn insert(&mut self, k: ModifierKind) {
        self.0 |= 1u8 << k as u8;
    }
}

impl FromIterator<ModifierKind> for CommandModifiers {
    fn from_iter<T: IntoIterator<Item = ModifierKind>>(iter: T) -> Self {
        let mut m = Self::default();
        for k in iter {
            m.insert(k);
        }
        m
    }
}

#[derive(Clone, Copy)]
struct SuffixModifier {
    kind: ModifierKind,
    suffix: &'static str,
}

#[derive(Clone, Copy)]
struct PrefixModifier {
    kind: ModifierKind,
    prefix: &'static str,
}

static SUFFIX_MODIFIERS: &[SuffixModifier] = &[
    // repeat buy until a resource gate
    SuffixModifier {
        kind: ModifierKind::Max,
        suffix: " -max",
    },
];

static PREFIX_MODIFIERS: &[PrefixModifier] = &[
    // skip certain unlock checks for this run
    PrefixModifier {
        kind: ModifierKind::Sudo,
        prefix: "sudo ",
    },
];

pub fn resolve_modifiers(trimmed: &str) -> (CommandModifiers, &str) {
    if is_known_command_or_upgrade(trimmed) {
        return (CommandModifiers::default(), trimmed);
    }

    let mut mods = CommandModifiers::default();
    let mut s = trimmed;

    for def in SUFFIX_MODIFIERS {
        if let Some(rest) = s.strip_suffix(def.suffix) {
            if !rest.is_empty() {
                mods.insert(def.kind);
                s = rest;
            }
        }
    }

    if is_known_command_or_upgrade(s) {
        return (mods, s);
    }

    for def in PREFIX_MODIFIERS {
        if let Some(rest) = s.strip_prefix(def.prefix) {
            if !rest.is_empty() {
                mods.insert(def.kind);
                s = rest;
                break;
            }
        }
    }

    (mods, s)
}
