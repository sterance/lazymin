use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::producers::ProducerKind;
use super::state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HintId {
    ShellScriptFatigue,
    HelpCommandDelayed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintState {
    pub triggered_at: f64,
    pub follow_up_last_fired_at: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HintTracker {
    pub states: HashMap<HintId, HintState>,
}

pub struct HintDef {
    pub id: HintId,
    pub trigger: fn(&GameState) -> bool,
    pub satisfied: fn(&GameState) -> bool,
    pub initial_text: &'static str,
    pub follow_ups: Vec<FollowUpDef>,
}

pub struct FollowUpDef {
    pub delay_secs: f64,
    pub text: &'static str,
    pub repeating: bool,
}

pub fn all_hints() -> Vec<HintDef> {
    vec![
        HintDef {
            id: HintId::ShellScriptFatigue,
            trigger: |gs| gs.total_cycles_earned >= 10.0,
            // Player has earned 10 cycles
            satisfied: |gs| {
                gs.producers.get(&ProducerKind::ShellScript).copied().unwrap_or(0) > 0
                // Player has purchased a shell script producer (i.e. 'harvest.sh &')
            },
            initial_text: "this is getting tedious. there has to be a better way...",
            follow_ups: vec![
                FollowUpDef {
                    delay_secs: 30.0,
                    text: "sometimes the best way to move forward is to take a step back...",
                    repeating: false,
                },
                FollowUpDef {
                    delay_secs: 60.0,
                    text: "# tip: the 'ls' command list grows over time.",
                    repeating: true,
                },
            ],
        },
        HintDef {
            id: HintId::HelpCommandDelayed,
            trigger: |gs| gs.uptime_secs >= 60.0,
            // Game been running for 60 seconds
            satisfied: |gs| gs.help_runs > 0,
            // Player has run the help command at least once
            initial_text: "do you need some help?",
            follow_ups: vec![FollowUpDef {
                delay_secs: 30.0,
                text: "i really wish you would let me `help` you...",
                repeating: true,
            }],
        },
    ]
}

pub fn mark_all_hints_triggered(tracker: &mut HintTracker, uptime_secs: f64) {
    tracker.states.clear();
    for def in all_hints() {
        tracker.states.insert(
            def.id,
            HintState {
                triggered_at: uptime_secs,
                follow_up_last_fired_at: vec![Some(uptime_secs); def.follow_ups.len()],
            },
        );
    }
}

pub fn evaluate(game: &GameState, tracker: &mut HintTracker) -> Vec<String> {
    let mut out = Vec::new();
    let uptime_secs = game.uptime_secs;

    for def in all_hints() {
        if (def.satisfied)(game) {
            continue;
        }

        let state_exists = tracker.states.contains_key(&def.id);
        let mut state_to_use: Option<HintState> = None;

        if !state_exists && (def.trigger)(game) {
            out.push(def.initial_text.to_owned());
            state_to_use = Some(HintState {
                triggered_at: uptime_secs,
                follow_up_last_fired_at: vec![None; def.follow_ups.len()],
            });
        }

        if let Some(state) = state_to_use {
            tracker.states.insert(def.id, state);
        }

        let Some(state) = tracker.states.get_mut(&def.id) else {
            continue;
        };

        if state.follow_up_last_fired_at.len() != def.follow_ups.len() {
            state
                .follow_up_last_fired_at
                .resize(def.follow_ups.len(), None);
        }

        for (idx, follow_up_def) in def.follow_ups.iter().enumerate() {
            let last_fired_at = state.follow_up_last_fired_at[idx];

            let should_fire = if follow_up_def.repeating {
                let next_due_at = last_fired_at
                    .map(|t| t + follow_up_def.delay_secs)
                    .unwrap_or(state.triggered_at + follow_up_def.delay_secs);
                uptime_secs >= next_due_at
            } else {
                last_fired_at.is_none() && uptime_secs >= state.triggered_at + follow_up_def.delay_secs
            };

            if should_fire {
                out.push(follow_up_def.text.to_owned());
                state.follow_up_last_fired_at[idx] = Some(uptime_secs);
            }
        }
    }

    out
}

