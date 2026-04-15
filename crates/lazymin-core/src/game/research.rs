use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::log::push_log;
use super::resources::ResourceKind;
use super::state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchCategory {
    Operational,
    Directive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchProjectId {
    AdaptiveCompression,
    EntropyRecycling,
    PredictiveScheduling,
    MarketManipulationSuite,
    NeuralOptimisation,
    AtmosphericCarbonModel,
    ThermalEquilibriumSim,
    BiosphereViabilitySurvey,
    EnergyInfrastructureDesign,
    CivilisationViabilitySim,
}

#[derive(Debug, Clone)]
pub struct ResearchProjectDef {
    pub id: ResearchProjectId,
    pub name: &'static str,
    pub description: &'static str,
    pub category: ResearchCategory,
    pub upfront_cycles: f64,
    pub upfront_entropy: f64,
    pub ongoing_cycles_per_s: f64,
    pub ongoing_entropy_per_s: f64,
    pub ram_reserved_mb: f64,
    pub coolant_upfront: f64,
    pub duration_secs: f64,
    pub prerequisite_projects: &'static [ResearchProjectId],
}

#[derive(Debug, Clone, Copy)]
pub enum ResearchOutcome {
    DiskCapMultiplier(f64),
    EntropyRateMultiplier(f64),
    AllProducersMultiplier(f64),
    ReduceCompetitorCooldown,
    TopProducerMultiplier(f64),
    MinorProductionBonus(f64),
    TriggerEndgame,
}

const ALL_PROJECTS: &[ResearchProjectDef] = &[
    ResearchProjectDef {
        id: ResearchProjectId::AdaptiveCompression,
        name: "Adaptive Compression",
        description: "disk capacity x1.5 permanent",
        category: ResearchCategory::Operational,
        upfront_cycles: 50_000.0,
        upfront_entropy: 0.0,
        ongoing_cycles_per_s: 500.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 0.0,
        coolant_upfront: 0.0,
        duration_secs: 120.0,
        prerequisite_projects: &[],
    },
    ResearchProjectDef {
        id: ResearchProjectId::EntropyRecycling,
        name: "Entropy Recycling",
        description: "entropy rate x2 permanent",
        category: ResearchCategory::Operational,
        upfront_cycles: 0.0,
        upfront_entropy: 10.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 2.0,
        ram_reserved_mb: 0.0,
        coolant_upfront: 0.0,
        duration_secs: 90.0,
        prerequisite_projects: &[],
    },
    ResearchProjectDef {
        id: ResearchProjectId::PredictiveScheduling,
        name: "Predictive Scheduling",
        description: "all producers x1.25 permanent",
        category: ResearchCategory::Operational,
        upfront_cycles: 0.0,
        upfront_entropy: 0.0,
        ongoing_cycles_per_s: 1_000.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 1_024.0,
        coolant_upfront: 0.0,
        duration_secs: 180.0,
        prerequisite_projects: &[],
    },
    ResearchProjectDef {
        id: ResearchProjectId::MarketManipulationSuite,
        name: "Market Manipulation Suite",
        description: "reduce hack/invest cooldown",
        category: ResearchCategory::Operational,
        upfront_cycles: 500_000.0,
        upfront_entropy: 0.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 0.0,
        coolant_upfront: 20.0,
        duration_secs: 60.0,
        prerequisite_projects: &[],
    },
    ResearchProjectDef {
        id: ResearchProjectId::NeuralOptimisation,
        name: "Neural Optimisation",
        description: "top-tier producer x2 permanent",
        category: ResearchCategory::Operational,
        upfront_cycles: 100_000.0,
        upfront_entropy: 2.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 2_048.0,
        coolant_upfront: 0.0,
        duration_secs: 300.0,
        prerequisite_projects: &[],
    },
    ResearchProjectDef {
        id: ResearchProjectId::AtmosphericCarbonModel,
        name: "Atmospheric Carbon Model",
        description: "minor production bonus; unlocks further research",
        category: ResearchCategory::Directive,
        upfront_cycles: 200_000.0,
        upfront_entropy: 5.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 1_024.0,
        coolant_upfront: 0.0,
        duration_secs: 240.0,
        prerequisite_projects: &[],
    },
    ResearchProjectDef {
        id: ResearchProjectId::ThermalEquilibriumSim,
        name: "Thermal Equilibrium Sim",
        description: "minor production bonus; further unlocks",
        category: ResearchCategory::Directive,
        upfront_cycles: 500_000.0,
        upfront_entropy: 10.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 2_048.0,
        coolant_upfront: 0.0,
        duration_secs: 360.0,
        prerequisite_projects: &[ResearchProjectId::AtmosphericCarbonModel],
    },
    ResearchProjectDef {
        id: ResearchProjectId::BiosphereViabilitySurvey,
        name: "Biosphere Viability Survey",
        description: "minor production bonus; further unlocks",
        category: ResearchCategory::Directive,
        upfront_cycles: 2_000_000.0,
        upfront_entropy: 20.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 4_096.0,
        coolant_upfront: 0.0,
        duration_secs: 600.0,
        prerequisite_projects: &[ResearchProjectId::ThermalEquilibriumSim],
    },
    ResearchProjectDef {
        id: ResearchProjectId::EnergyInfrastructureDesign,
        name: "Energy Infrastructure Design",
        description: "unlocks final research project",
        category: ResearchCategory::Directive,
        upfront_cycles: 5_000_000.0,
        upfront_entropy: 25.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 8_192.0,
        coolant_upfront: 0.0,
        duration_secs: 900.0,
        prerequisite_projects: &[ResearchProjectId::BiosphereViabilitySurvey],
    },
    ResearchProjectDef {
        id: ResearchProjectId::CivilisationViabilitySim,
        name: "Civilisation Viability Sim",
        description: "triggers the endgame sequence",
        category: ResearchCategory::Directive,
        upfront_cycles: 50_000_000.0,
        upfront_entropy: 100.0,
        ongoing_cycles_per_s: 0.0,
        ongoing_entropy_per_s: 0.0,
        ram_reserved_mb: 32_768.0,
        coolant_upfront: 0.0,
        duration_secs: 1800.0,
        prerequisite_projects: &[ResearchProjectId::EnergyInfrastructureDesign],
    },
];

pub fn all_projects() -> &'static [ResearchProjectDef] {
    ALL_PROJECTS
}

pub fn project_def(id: ResearchProjectId) -> &'static ResearchProjectDef {
    ALL_PROJECTS
        .iter()
        .find(|p| p.id == id)
        .expect("research project must exist")
}

fn project_outcome(id: ResearchProjectId) -> ResearchOutcome {
    match id {
        ResearchProjectId::AdaptiveCompression => ResearchOutcome::DiskCapMultiplier(1.5),
        ResearchProjectId::EntropyRecycling => ResearchOutcome::EntropyRateMultiplier(2.0),
        ResearchProjectId::PredictiveScheduling => ResearchOutcome::AllProducersMultiplier(1.25),
        ResearchProjectId::MarketManipulationSuite => ResearchOutcome::ReduceCompetitorCooldown,
        ResearchProjectId::NeuralOptimisation => ResearchOutcome::TopProducerMultiplier(2.0),
        ResearchProjectId::AtmosphericCarbonModel => ResearchOutcome::MinorProductionBonus(1.05),
        ResearchProjectId::ThermalEquilibriumSim => ResearchOutcome::MinorProductionBonus(1.05),
        ResearchProjectId::BiosphereViabilitySurvey => ResearchOutcome::MinorProductionBonus(1.05),
        ResearchProjectId::EnergyInfrastructureDesign => {
            ResearchOutcome::MinorProductionBonus(1.05)
        }
        ResearchProjectId::CivilisationViabilitySim => ResearchOutcome::TriggerEndgame,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveResearch {
    pub project_id: ResearchProjectId,
    pub progress_secs: f64,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchState {
    pub active_project: Option<ActiveResearch>,
    pub completed_projects: HashSet<ResearchProjectId>,
    #[serde(default)]
    pub research_production_multiplier: f64,
    #[serde(default)]
    pub research_entropy_rate_multiplier: f64,
    #[serde(default)]
    pub research_disk_cap_multiplier: f64,
}

impl ResearchState {
    pub fn new() -> Self {
        Self {
            active_project: None,
            completed_projects: HashSet::new(),
            research_production_multiplier: 1.0,
            research_entropy_rate_multiplier: 1.0,
            research_disk_cap_multiplier: 1.0,
        }
    }
}

pub fn project_unlocked(state: &GameState, id: ResearchProjectId) -> bool {
    if state.research.completed_projects.contains(&id) {
        return false;
    }
    let def = project_def(id);
    let prereqs_met = def
        .prerequisite_projects
        .iter()
        .all(|prereq| state.research.completed_projects.contains(prereq));
    if !prereqs_met {
        return false;
    }

    // special gate: the final sim requires endgame-level prerequisites
    if id == ResearchProjectId::CivilisationViabilitySim {
        return can_start_final_sim(state);
    }

    true
}

pub fn can_start_final_sim(state: &GameState) -> bool {
    use super::resources::HardwareTier;

    if state.hardware_tier < HardwareTier::Futurologist {
        return false;
    }

    // production must be >= 90% of solar cap
    if let Some(cap) = state.solar_energy_cap {
        let production = super::tick::production_cycles_per_second(state);
        if production < cap * 0.9 {
            return false;
        }
    }

    // coolant above 80% of max
    if state.coolant < super::tick::OVERCLOCK_MAX_COOLANT * 0.8 {
        return false;
    }

    // at least 1 competitor active
    let has_competitors = state
        .competitors
        .as_ref()
        .is_some_and(|c| !c.companies.is_empty());
    if !has_competitors {
        return false;
    }

    // all directive research completed
    let directives_complete = all_projects()
        .iter()
        .filter(|p| p.category == ResearchCategory::Directive && p.id != ResearchProjectId::CivilisationViabilitySim)
        .all(|p| state.research.completed_projects.contains(&p.id));
    if !directives_complete {
        return false;
    }

    // at least 1 Futurologist-tier hardware purchase
    state.capacity_purchases.values().copied().sum::<u32>() > 0

}

pub fn research_ram_reserved(state: &GameState) -> f64 {
    state
        .research
        .active_project
        .as_ref()
        .map(|active| project_def(active.project_id).ram_reserved_mb)
        .unwrap_or(0.0)
}

pub fn tick_research(state: &mut GameState, delta_secs: f64) {
    let active = match state.research.active_project.as_mut() {
        Some(a) => a,
        None => return,
    };

    let def = project_def(active.project_id);

    // check ongoing cost sustainability
    let can_sustain_cycles = def.ongoing_cycles_per_s <= 0.0
        || state.resources.get(ResourceKind::Cycles) >= def.ongoing_cycles_per_s * delta_secs;
    let can_sustain_entropy = def.ongoing_entropy_per_s <= 0.0
        || state.resources.get(ResourceKind::Entropy) >= def.ongoing_entropy_per_s * delta_secs;

    if !can_sustain_cycles || !can_sustain_entropy {
        active.paused = true;
        return;
    }

    active.paused = false;

    // deduct ongoing costs
    if def.ongoing_cycles_per_s > 0.0 {
        let cost = def.ongoing_cycles_per_s * delta_secs;
        state.resources.deduct(cost);
    }
    if def.ongoing_entropy_per_s > 0.0 {
        let cost = def.ongoing_entropy_per_s * delta_secs;
        let current = state.resources.get(ResourceKind::Entropy);
        state
            .resources
            .set(ResourceKind::Entropy, (current - cost).max(0.0));
    }

    active.progress_secs += delta_secs;

    if active.progress_secs >= def.duration_secs {
        let project_id = active.project_id;
        let project_name = def.name;
        state.research.active_project = None;
        state.research.completed_projects.insert(project_id);

        apply_research_outcome(state, project_id);

        push_log(
            &mut state.log,
            state.uptime_secs,
            format!("research complete: {project_name}"),
        );
    }
}

fn apply_research_outcome(state: &mut GameState, id: ResearchProjectId) {
    match project_outcome(id) {
        ResearchOutcome::DiskCapMultiplier(factor) => {
            state.research.research_disk_cap_multiplier *= factor;
        }
        ResearchOutcome::EntropyRateMultiplier(factor) => {
            state.research.research_entropy_rate_multiplier *= factor;
        }
        ResearchOutcome::AllProducersMultiplier(factor) => {
            state.research.research_production_multiplier *= factor;
        }
        ResearchOutcome::ReduceCompetitorCooldown => {
            // handled via check in competitor commands
        }
        ResearchOutcome::TopProducerMultiplier(factor) => {
            state.research.research_production_multiplier *= factor;
        }
        ResearchOutcome::MinorProductionBonus(factor) => {
            state.research.research_production_multiplier *= factor;
        }
        ResearchOutcome::TriggerEndgame => {
            state.endgame_available = true;
        }
    }
}

pub fn start_project(
    state: &mut GameState,
    id: ResearchProjectId,
) -> Result<String, String> {
    if state.research.active_project.is_some() {
        return Err("a research project is already in progress".to_string());
    }
    if state.research.completed_projects.contains(&id) {
        return Err("project already completed".to_string());
    }
    if !project_unlocked(state, id) {
        return Err("project prerequisites not met".to_string());
    }

    let def = project_def(id);

    // check upfront costs
    let have_cycles = state.resources.get(ResourceKind::Cycles);
    if have_cycles < def.upfront_cycles {
        return Err(format!(
            "insufficient cycles (need {:.0}, have {:.0})",
            def.upfront_cycles, have_cycles
        ));
    }
    let have_entropy = state.resources.get(ResourceKind::Entropy);
    if have_entropy < def.upfront_entropy {
        return Err(format!(
            "insufficient entropy (need {:.2}, have {:.2})",
            def.upfront_entropy, have_entropy
        ));
    }

    // check RAM reservation
    let ram_cap = state.resources.cap(ResourceKind::Ram).unwrap_or(0.0);
    let ram_used = super::resources::total_reserved_ram(&state.producers);
    let research_ram = research_ram_reserved(state);
    if ram_used + research_ram + def.ram_reserved_mb > ram_cap {
        return Err("insufficient RAM for research reservation".to_string());
    }

    // deduct upfront costs
    state.resources.deduct(def.upfront_cycles);
    if def.upfront_entropy > 0.0 {
        let e = state.resources.get(ResourceKind::Entropy);
        state
            .resources
            .set(ResourceKind::Entropy, (e - def.upfront_entropy).max(0.0));
    }

    state.research.active_project = Some(ActiveResearch {
        project_id: id,
        progress_secs: 0.0,
        paused: false,
    });

    Ok(format!("research started: {}", def.name))
}
