#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProducerKind {
    ShellScript,
}

#[derive(Debug, Clone, Copy)]
pub struct ProducerDef {
    pub kind: ProducerKind,
    pub name: &'static str,
    pub command: &'static str,
    pub base_cycles_per_s: f64,
    pub base_cost: f64,
    pub unlock_threshold: f64,
}

const PRODUCERS: [ProducerDef; 1] = [ProducerDef {
    kind: ProducerKind::ShellScript,
    name: "Shell Script",
    command: "harvest.sh &",
    base_cycles_per_s: 1.0,
    base_cost: 10.0,
    unlock_threshold: 0.0,
}];

pub fn all_producers() -> &'static [ProducerDef] {
    &PRODUCERS
}

pub fn producer_def(kind: ProducerKind) -> &'static ProducerDef {
    all_producers()
        .iter()
        .find(|def| def.kind == kind)
        .expect("producer kind must exist in registry")
}

pub fn producer_cost(def: &ProducerDef, owned: u32) -> f64 {
    def.base_cost * 1.15_f64.powi(owned as i32)
}
