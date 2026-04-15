use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::log::push_log;
use super::state::GameState;

const MIN_COMPANIES: usize = 1;
const MAX_COMPANIES: usize = 5;
const INITIAL_COMPANIES: usize = 3;
const SPAWN_DELAY_SECS: f64 = 60.0;
const EVENT_INTERVAL_SECS: f64 = 30.0;
const BUYOUT_VALUE_THRESHOLD_FRACTION: f64 = 0.3;
const HACK_INVEST_COOLDOWN_SECS: f64 = 30.0;
const HACK_VALUE_REDUCTION: f64 = 0.15;
const INVEST_VALUE_INCREASE: f64 = 0.10;
const BUYOUT_PRODUCTION_BONUS: f64 = 0.05;

const COMPANY_NAMES: &[&str] = &[
    "Nexon Systems",
    "Parallax Corp",
    "Cerulean Industries",
    "Vantage Digital",
    "Obsidian Works",
    "Helix Dynamics",
    "Stratum Global",
    "Meridian Logic",
    "Candela Networks",
    "Forge Collective",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: char,
    pub name: String,
    pub value: f64,
    pub value_history: VecDeque<f64>,
    pub hack_cooldown_until: f64,
    pub invest_cooldown_until: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanyTrend {
    Growing,
    Stable,
    Declining,
}

impl Company {
    pub fn trend(&self) -> CompanyTrend {
        if self.value_history.len() < 2 {
            return CompanyTrend::Stable;
        }
        let recent = self.value;
        let oldest = self.value_history.front().copied().unwrap_or(recent);
        let delta = recent - oldest;
        let threshold = oldest.abs() * 0.05;
        if delta > threshold {
            CompanyTrend::Growing
        } else if delta < -threshold {
            CompanyTrend::Declining
        } else {
            CompanyTrend::Stable
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorPool {
    pub companies: Vec<Company>,
    pub next_spawn_at: Option<f64>,
    pub event_accumulator: f64,
    pub total_buyouts: u32,
    names_used: Vec<String>,
}

impl Default for CompetitorPool {
    fn default() -> Self {
        Self {
            companies: Vec::new(),
            next_spawn_at: None,
            event_accumulator: 0.0,
            total_buyouts: 0,
            names_used: Vec::new(),
        }
    }
}

impl CompetitorPool {
    pub fn initialize(uptime_secs: f64, rng: &mut impl FnMut() -> f64) -> Self {
        let mut pool = Self::default();
        for _ in 0..INITIAL_COMPANIES {
            pool.spawn_company(uptime_secs, rng);
        }
        pool
    }

    fn next_id(&self) -> char {
        let used: Vec<char> = self.companies.iter().map(|c| c.id).collect();
        ('A'..='Z').find(|c| !used.contains(c)).unwrap_or('?')
    }

    fn pick_name(&mut self, rng: &mut impl FnMut() -> f64) -> String {
        let available: Vec<&&str> = COMPANY_NAMES
            .iter()
            .filter(|n| !self.names_used.iter().any(|u| u == **n))
            .collect();
        if available.is_empty() {
            let idx = (rng() * COMPANY_NAMES.len() as f64).floor() as usize;
            return COMPANY_NAMES[idx.min(COMPANY_NAMES.len() - 1)].to_string();
        }
        let idx = (rng() * available.len() as f64).floor() as usize;
        let name = available[idx.min(available.len() - 1)].to_string();
        self.names_used.push(name.clone());
        name
    }

    pub fn spawn_company(&mut self, _uptime_secs: f64, rng: &mut impl FnMut() -> f64) {
        let id = self.next_id();
        let name = self.pick_name(rng);
        let base_value = 500.0 + rng() * 500.0;
        self.companies.push(Company {
            id,
            name,
            value: base_value,
            value_history: VecDeque::new(),
            hack_cooldown_until: 0.0,
            invest_cooldown_until: 0.0,
        });
    }

    pub fn company_by_id(&self, id: char) -> Option<&Company> {
        self.companies.iter().find(|c| c.id == id)
    }

    pub fn company_by_id_mut(&mut self, id: char) -> Option<&mut Company> {
        self.companies.iter_mut().find(|c| c.id == id)
    }
}

pub fn tick_competitors(state: &mut GameState, delta_secs: f64) {
    if state.competitors.is_none() {
        return;
    }

    // handle pending spawns - take pool out temporarily
    let mut pool = state.competitors.take().unwrap();

    if let Some(spawn_at) = pool.next_spawn_at {
        if state.uptime_secs >= spawn_at && pool.companies.len() < MAX_COMPANIES {
            let uptime = state.uptime_secs;
            let r1 = state.roll_unit();
            let r2 = state.roll_unit();
            let r3 = state.roll_unit();
            let mut calls = [r1, r2, r3].into_iter();
            let mut rng = || calls.next().unwrap_or(0.5);
            pool.spawn_company(uptime, &mut rng);
            let name = pool.companies.last().unwrap().name.clone();
            let id = pool.companies.last().unwrap().id;
            pool.next_spawn_at = None;
            push_log(
                &mut state.log,
                state.uptime_secs,
                format!("[{id}] {name} has entered the market"),
            );
        }
    }

    pool.event_accumulator += delta_secs;
    if pool.event_accumulator < EVENT_INTERVAL_SECS {
        state.competitors = Some(pool);
        return;
    }
    pool.event_accumulator -= EVENT_INTERVAL_SECS;

    if pool.companies.len() < 2 {
        if pool.companies.len() < MIN_COMPANIES && pool.next_spawn_at.is_none() {
            pool.next_spawn_at = Some(state.uptime_secs + SPAWN_DELAY_SECS);
        }
        state.competitors = Some(pool);
        return;
    }

    for i in 0..pool.companies.len() {
        let roll = state.roll_unit();
        let drift = (roll - 0.5) * 0.1 * pool.companies[i].value;
        pool.companies[i].value = (pool.companies[i].value + drift).max(10.0);
        let new_value = pool.companies[i].value;
        pool.companies[i].value_history.push_back(new_value);
        while pool.companies[i].value_history.len() > 10 {
            pool.companies[i].value_history.pop_front();
        }
    }

    let event_roll = state.roll_unit();
    if event_roll < 0.25 && pool.companies.len() >= 2 {
        let (max_idx, min_idx) = {
            let mut max_i = 0;
            let mut min_i = 0;
            for i in 1..pool.companies.len() {
                if pool.companies[i].value > pool.companies[max_i].value {
                    max_i = i;
                }
                if pool.companies[i].value < pool.companies[min_i].value {
                    min_i = i;
                }
            }
            (max_i, min_i)
        };
        if max_idx != min_idx && pool.companies.len() > MIN_COMPANIES {
            let absorbed_value = pool.companies[min_idx].value * 0.5;
            let acquirer_name = pool.companies[max_idx].name.clone();
            let acquired_name = pool.companies[min_idx].name.clone();
            pool.companies[max_idx].value += absorbed_value;
            pool.companies.remove(min_idx);
            push_log(
                &mut state.log,
                state.uptime_secs,
                format!("{acquirer_name} acquired {acquired_name}"),
            );
            if pool.companies.len() < MIN_COMPANIES + 1 && pool.next_spawn_at.is_none() {
                pool.next_spawn_at = Some(state.uptime_secs + SPAWN_DELAY_SECS);
            }
        }
    } else if event_roll < 0.50 && pool.companies.len() >= 2 {
        let idx_a = (state.roll_unit() * pool.companies.len() as f64).floor() as usize;
        let idx_a = idx_a.min(pool.companies.len() - 1);
        let idx_b = (state.roll_unit() * pool.companies.len() as f64).floor() as usize;
        let idx_b = idx_b.min(pool.companies.len() - 1);
        if idx_a != idx_b {
            let loss = pool.companies[idx_b].value * 0.1;
            let name_a = pool.companies[idx_a].name.clone();
            let name_b = pool.companies[idx_b].name.clone();
            pool.companies[idx_b].value = (pool.companies[idx_b].value - loss).max(10.0);
            push_log(
                &mut state.log,
                state.uptime_secs,
                format!("{name_a} locked out {name_b}'s primary supplier"),
            );
        }
    } else if event_roll < 0.65 {
        let min_idx = pool
            .companies
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.value.partial_cmp(&b.value).unwrap())
            .map(|(i, _)| i);
        if let Some(idx) = min_idx {
            if pool.companies[idx].value < 50.0 && pool.companies.len() > MIN_COMPANIES {
                let name = pool.companies[idx].name.clone();
                pool.companies.remove(idx);
                push_log(
                    &mut state.log,
                    state.uptime_secs,
                    format!("{name} has filed for bankruptcy and dissolved"),
                );
                if pool.companies.len() < MIN_COMPANIES + 1 && pool.next_spawn_at.is_none() {
                    pool.next_spawn_at = Some(state.uptime_secs + SPAWN_DELAY_SECS);
                }
            }
        }
    }

    state.competitors = Some(pool);
}

pub fn hack_cost(state: &GameState) -> f64 {
    crate::game::tick::production_cycles_per_second(state) * 10.0
}

pub fn invest_cost(state: &GameState) -> f64 {
    crate::game::tick::production_cycles_per_second(state) * 10.0
}

pub fn buyout_cost(state: &GameState, company: &Company) -> f64 {
    company.value * crate::game::tick::production_cycles_per_second(state).max(1.0) * 0.1
}

pub fn can_buyout(company: &Company) -> bool {
    let max_for_buyout = 1000.0 * BUYOUT_VALUE_THRESHOLD_FRACTION;
    company.value < max_for_buyout
}

pub fn hack_company(state: &mut GameState, id: char) -> Result<String, String> {
    // validate first with immutable borrows
    {
        let pool = state.competitors.as_ref().ok_or("no competitors")?;
        let company = pool
            .company_by_id(id)
            .ok_or_else(|| format!("unknown company: {id}"))?;
        if state.uptime_secs < company.hack_cooldown_until {
            let remaining = company.hack_cooldown_until - state.uptime_secs;
            return Err(format!("hack on cooldown ({remaining:.0}s remaining)"));
        }
    }

    let cost = hack_cost(state);
    if !state.resources.can_afford(cost) {
        return Err("insufficient cycles".to_string());
    }

    state.resources.deduct(cost);
    let pool = state.competitors.as_mut().unwrap();
    let company = pool.company_by_id_mut(id).unwrap();
    let reduction = company.value * HACK_VALUE_REDUCTION;
    company.value = (company.value - reduction).max(10.0);
    company.hack_cooldown_until = state.uptime_secs + HACK_INVEST_COOLDOWN_SECS;
    let name = company.name.clone();
    Ok(format!("{name} value reduced by {:.0}", reduction))
}

pub fn invest_company(state: &mut GameState, id: char) -> Result<String, String> {
    {
        let pool = state.competitors.as_ref().ok_or("no competitors")?;
        let company = pool
            .company_by_id(id)
            .ok_or_else(|| format!("unknown company: {id}"))?;
        if state.uptime_secs < company.invest_cooldown_until {
            let remaining = company.invest_cooldown_until - state.uptime_secs;
            return Err(format!("invest on cooldown ({remaining:.0}s remaining)"));
        }
    }

    let cost = invest_cost(state);
    if !state.resources.can_afford(cost) {
        return Err("insufficient cycles".to_string());
    }

    state.resources.deduct(cost);
    let pool = state.competitors.as_mut().unwrap();
    let company = pool.company_by_id_mut(id).unwrap();
    let increase = company.value * INVEST_VALUE_INCREASE;
    company.value += increase;
    company.invest_cooldown_until = state.uptime_secs + HACK_INVEST_COOLDOWN_SECS;
    let name = company.name.clone();
    Ok(format!("{name} value increased by {:.0}", increase))
}

pub fn buyout_company(state: &mut GameState, id: char) -> Result<String, String> {
    let (cost, name) = {
        let pool = state.competitors.as_ref().ok_or("no competitors")?;
        let company = pool
            .company_by_id(id)
            .ok_or_else(|| format!("unknown company: {id}"))?;

        if !can_buyout(company) {
            return Err("company value too high for buyout".to_string());
        }
        if pool.companies.len() <= MIN_COMPANIES {
            return Err(
                "cannot proceed. autonomous process count would fall below minimum viable threshold."
                    .to_string(),
            );
        }
        (buyout_cost(state, company), company.name.clone())
    };

    if !state.resources.can_afford(cost) {
        return Err("insufficient cycles".to_string());
    }

    state.resources.deduct(cost);
    let pool = state.competitors.as_mut().unwrap();
    pool.companies.retain(|c| c.id != id);
    pool.total_buyouts += 1;

    if pool.companies.len() < MIN_COMPANIES + 1 && pool.next_spawn_at.is_none() {
        pool.next_spawn_at = Some(state.uptime_secs + SPAWN_DELAY_SECS);
    }

    Ok(format!(
        "{name} acquired. +{:.0}% permanent production bonus",
        BUYOUT_PRODUCTION_BONUS * 100.0
    ))
}

pub fn total_buyout_multiplier(state: &GameState) -> f64 {
    let buyouts = state
        .competitors
        .as_ref()
        .map(|c| c.total_buyouts)
        .unwrap_or(0);
    1.0 + (buyouts as f64) * BUYOUT_PRODUCTION_BONUS
}
