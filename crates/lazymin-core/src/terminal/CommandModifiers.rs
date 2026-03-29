// registry entries map input to named effects (`bypasses_permission_lock`, etc.); permission_lock.rs,
// execute.rs, and highlight.rs apply it (sudo bypass, -max / ` *n` purchase loops, etc.).

use std::num::NonZeroU32;

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

#[allow(non_upper_case_globals)]
pub const bypasses_permission_lock: ModifierKind = ModifierKind::Sudo;
#[allow(non_upper_case_globals)]
pub const enables_max_purchase_loop: ModifierKind = ModifierKind::Max;

#[derive(Clone, Copy)]
struct PrefixModifier {
    prefix: &'static str,
    effect: ModifierKind,
}

#[derive(Clone, Copy)]
enum PurchaseLoopSuffix {
    RepeatCount,
    Max,
}

#[allow(non_upper_case_globals)]
const enables_purchase_loop_repeat: PurchaseLoopSuffix = PurchaseLoopSuffix::RepeatCount;
#[allow(non_upper_case_globals)]
const enables_purchase_loop_max: PurchaseLoopSuffix = PurchaseLoopSuffix::Max;

#[derive(Clone, Copy)]
enum SuffixModifier {
    StarRepeat {
        #[allow(dead_code)]
        suffix: &'static str,
        effect: PurchaseLoopSuffix,
    },
    Literal {
        suffix: &'static str,
        effect: PurchaseLoopSuffix,
    },
}


// ----- PREFIX MODIFIERS -----

static PREFIX_MODIFIERS: &[PrefixModifier] = &[PrefixModifier {
    prefix: "sudo ",
    effect: bypasses_permission_lock,
}];


// ----- SUFFFIX MODIFIERS -----

static SUFFIX_MODIFIERS: &[SuffixModifier] = &[
    SuffixModifier::StarRepeat {
        suffix: " *n",
        effect: enables_purchase_loop_repeat,
    },
    SuffixModifier::Literal {
        suffix: " -max",
        effect: enables_purchase_loop_max,
    },
];





#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandModifiers(u8);

impl CommandModifiers {
    pub fn has(self, k: ModifierKind) -> bool {
        self.0 & (1u8 << k as u8) != 0
    }

    fn insert(&mut self, k: ModifierKind) {
        self.0 |= 1u8 << k as u8;
    }

    fn remove(&mut self, k: ModifierKind) {
        self.0 &= !(1u8 << k as u8);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PurchaseRepeat {
    #[default]
    Once,
    Max,
    Times(NonZeroU32),
}

fn strip_star_repeat_suffix(s: &str) -> Option<(&str, NonZeroU32)> {
    let mut i = s.len();
    while i > 0 && s.as_bytes()[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i == s.len() {
        return None;
    }
    let digit_start = i;
    if digit_start == 0 || s.as_bytes()[digit_start - 1] != b'*' {
        return None;
    }
    if digit_start < 2 || s.as_bytes()[digit_start - 2] != b' ' {
        return None;
    }
    let prefix_len = digit_start - 2;
    if prefix_len == 0 {
        return None;
    }
    let n: u32 = s[digit_start..].parse().ok()?;
    let n = NonZeroU32::new(n)?;
    Some((&s[..prefix_len], n))
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

fn strip_repeat_suffixes<'a>(mut s: &'a str, mods: &mut CommandModifiers) -> (PurchaseRepeat, &'a str) {
    let mut saw_max = false;
    let mut times: Option<NonZeroU32> = None;

    loop {
        let mut progressed = false;
        for def in SUFFIX_MODIFIERS {
            match def {
                SuffixModifier::StarRepeat { effect, .. } => {
                    debug_assert!(matches!(effect, PurchaseLoopSuffix::RepeatCount));
                    if let Some((rest, n)) = strip_star_repeat_suffix(s) {
                        if !rest.is_empty() {
                            s = rest;
                            times = Some(n);
                            progressed = true;
                            break;
                        }
                    }
                }
                SuffixModifier::Literal { suffix, effect } => {
                    debug_assert!(matches!(effect, PurchaseLoopSuffix::Max));
                    if let Some(rest) = s.strip_suffix(suffix) {
                        if !rest.is_empty() {
                            s = rest;
                            mods.insert(enables_max_purchase_loop);
                            saw_max = true;
                            progressed = true;
                            break;
                        }
                    }
                }
            }
        }
        if !progressed {
            break;
        }
    }

    let repeat = if let Some(n) = times {
        mods.remove(ModifierKind::Max);
        PurchaseRepeat::Times(n)
    } else if saw_max {
        PurchaseRepeat::Max
    } else {
        PurchaseRepeat::Once
    };

    (repeat, s)
}

pub fn resolve_modifiers(trimmed: &str) -> (CommandModifiers, PurchaseRepeat, &str) {
    if is_known_command_or_upgrade(trimmed) {
        return (CommandModifiers::default(), PurchaseRepeat::Once, trimmed);
    }

    let mut mods = CommandModifiers::default();
    let (repeat, s) = strip_repeat_suffixes(trimmed, &mut mods);

    if is_known_command_or_upgrade(s) {
        return (mods, repeat, s);
    }

    let mut s = s;
    for def in PREFIX_MODIFIERS {
        if let Some(rest) = s.strip_prefix(def.prefix) {
            if !rest.is_empty() {
                mods.insert(def.effect);
                s = rest;
                break;
            }
        }
    }

    (mods, repeat, s)
}
