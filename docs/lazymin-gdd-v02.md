# lazymin — Game Design Document
## Refinement & Expansion

> This document describes planned changes to existing systems and the design of new systems. It is written from a game design perspective. A separate implementation document will follow once this is approved.
>
> **v0.2 changes from v0.1:** Sections 5, 6, 7, and 9 have been substantially revised to reflect a new, more fully developed narrative foundation. All mechanical systems from v0.1 are preserved; this version integrates them into a coherent story. New in this version: §7.4 (The Sacrifice), §9.2 (Setting), §9.3 (Narrative Arc by Tier), §9.5 (The Moral Choice Ending).

---

## Table of Contents

1. [Market System Refinements](#1-market-system-refinements)
2. [Overclock Curve Redesign](#2-overclock-curve-redesign)
3. [Hardware Tier System](#3-hardware-tier-system)
4. [New Producers](#4-new-producers)
5. [Competitor System — Supplier Tier](#5-competitor-system--supplier-tier)
6. [Research System — Innovator Tier](#6-research-system--innovator-tier)
7. [Futurologist Tier & Endgame](#7-futurologist-tier--endgame)
8. [Soft Reset](#8-soft-reset)
9. [Story & Narrative Integration](#9-story--narrative-integration)
10. [Obsoleted Systems](#10-obsoleted-systems)

---

## 1. Market System Refinements

### 1.1 Coolant Drain Rate

Coolant now drains at **60 units per second** (previously 1 unit per second), applied per-frame as accurately as framerate allows. A full tank of 10,000 coolant now lasts approximately 166–167 seconds at normal drain. This makes coolant management an active concern rather than a passive one. The drain rate constant should be easily tunable.

### 1.2 Anchor Price: Cycles/Second Instead of Total Cycles

The market anchor price is now calculated from the player's **current cycles per second** rather than their total cycles earned. This makes the anchor price reflect the player's productive capacity at any given moment, rather than their cumulative history, which grows unboundedly and would make late-game coolant trivially cheap relative to current income.

> **Design rationale:** A player earning 1M cycles/s should feel the market price as meaningful relative to their income, not as a rounding error.

### 1.3 Demand-Adjusted Anchor

A **demand pressure** value is tracked separately from the base anchor. When the player purchases coolant, demand pressure rises. When no purchases are made, it decays back toward zero over time. The effective anchor price is the base anchor multiplied by a demand factor derived from the current pressure level.

- Demand pressure is measured as **units purchased within a rolling 300-second window**. This window length is a tunable constant.
- The demand factor should be a smooth curve: low pressure = no adjustment (factor ≈ 1.0), high pressure = significant markup (e.g. factor up to ~3.0 at saturation). The exact curve shape and saturation point are to be determined during playtesting.
- Demand pressure decays naturally over time as older purchases fall out of the rolling window. No separate decay mechanic is needed.

### 1.4 Bull / Bear Market Cycles

The existing `market_trend_up` flag and SMA-comparison logic is replaced with an explicit **bull/bear state**.

- The market always alternates: bull → bear → bull → ...
- Each cycle lasts a **randomised duration between 25 and 40 seconds**, weighted so that 30 seconds is the most likely outcome. A log-normal or similar smooth distribution centred near 30 should be used. The exact distribution is a tunable detail.
- In **bull mode**, each market tick has a **70% chance of a price increase** and a 30% chance of a decrease.
- In **bear mode**, each market tick has a **30% chance of a price increase** and a 70% chance of a decrease.
- The trend arrow (`▲` / `▼`) in the MARKET panel now indicates the current **bull or bear state** only. It no longer reflects the direction of the most recent individual tick.
- The player is not explicitly told they are in a bull or bear cycle. They are expected to infer it from watching the arrow and price behaviour.

---

## 2. Overclock Curve Redesign

The overclock multiplier is currently a simple two-segment linear ramp. It is replaced with a **smooth sigmoid/log curve** with the following anchors:

| Coolant | Overclock |
|---------|-----------|
| 0       | 1%        |
| ~5,000  | 100%      |
| 10,000  | 200%      |
| >10,000 | 200% (cap) |

The curve is heavily biased toward 100% in the middle range — most of the coolant scale between 1,000 and 9,000 should map to overclock values clustering around 80–120%. The steep portions of the curve are at the extremes: very low coolant (harsh rapid drop toward 1%) and very high coolant (hard to reach 200%, which now requires maintaining a near-full tank given the fast drain rate).

> **Design rationale:** The 1% floor is a meaningful punishment. Under the old linear curve it was easy to accidentally stumble into. The new curve makes low overclock a consequence of genuinely neglecting the market for a sustained period, not a casual mistake. Conversely, 200% is now a real achievement requiring active coolant management.

Coolant above 10,000 has no additional effect beyond providing a buffer before dropping below 200%. The hard cap on coolant storage remains 10,000 (or may be raised slightly to accommodate the buffer feel — to be determined in playtesting).

---

## 3. Hardware Tier System

### 3.1 Overview

Hardware purchases (`apt install ram/hdd/nic/psu`) are reorganised into five sequential tiers. Each tier represents a step change in scale: larger capacity gains per purchase, a larger base cost, and a reset of the 15% cost scaling.

The five tiers are:

| # | Tier Name       | Flavour context                         |
|---|-----------------|------------------------------------------|
| 1 | Consumer        | Off-the-shelf hardware, small scale      |
| 2 | Business        | Rack-scale procurement                   |
| 3 | Supplier        | Wholesale / data-centre scale            |
| 4 | Innovator       | Cutting-edge / prototype scale           |
| 5 | Futurologist    | Post-scarcity / theoretical-limit scale  |

### 3.2 Tier Mechanics

- Each tier has its own **base cost** and **capacity delta** per purchase for each hardware type (ram, hdd, nic, psu). These are individually specified values, not blanket multipliers.
- The starting values (tier 1) are the existing values from the current codebase. Subsequent tiers begin at approximately **8× the previous tier's capacity delta** and **8× the previous tier's base cost** per unit, as a starting baseline. All values are individually tunable.
- Within each tier, each purchase scales the cost of the next by **15%** (restored from the current 5%).
- When the player advances to a new tier, the cost scaling **fully resets** to that tier's base price. Carry-over cost basis is wiped.
- The `reboot --firmware` upgrade (cost-basis reset) becomes redundant and is removed from the game (see §10).

### 3.3 Tier Upgrade Commands

Advancing to the next tier is done via a **one-time terminal command**, purchased like an existing permanent upgrade. The command becomes visible and purchasable once the unlock condition is met. The commands and rough unlock conditions are:

| Command (placeholder)         | Unlocks        | Rough unlock condition (to be tuned)                         |
|-------------------------------|----------------|--------------------------------------------------------------|
| `apt-get dist-upgrade`        | Business tier  | Player has owned at least N producers, hit_resource_gate    |
| `dpkg --configure -a`         | Supplier tier  | Market unlocked, sufficient hardware purchased               |
| `build-essential install`     | Innovator tier | Competitor system fully engaged (e.g. one company bought out)|
| `gcc -O3 -march=native`       | Futurologist   | At least one research project completed                      |

> These command names and unlock conditions are placeholder starting points. The exact commands and thresholds should be refined during playtesting. What matters at this stage is the shape: each tier gate should feel earned but not frustrating.

### 3.4 In-Game Presentation

The `apt install` listing changes **flavour text** when the tier advances. The commands themselves (`apt install ram` etc.) remain the same, but the description of what is being purchased changes to reflect the new scale:

- **Consumer:** "ram stick", "hard drive", "network card", "power supply"
- **Business:** "ram pallet", "drive rack", "switch module", "ups unit"
- **Supplier:** "ram warehouse", "storage array", "backbone port", "generator unit"
- **Innovator:** "ram substrate", "molecular storage node", "photonic link", "fusion tap"
- **Futurologist:** "ram lattice", "compressed-matter store", "quantum channel", "stellar tap"

> These are placeholder names for the flavour text. Exact wording to be finalised.

### 3.5 Tier as Story Gate

The hardware tier is the primary gate for most major content unlocks in the game. The tier system replaces `ssh market` as the mechanism for unlocking the market (market is now gated on reaching Business tier).

---

## 4. New Producers

Three new producer types are added above OS Takeover to support the scale required by the higher hardware tiers and later game progression. These sit at the top of the existing production ladder.

The producers and their rough positioning in the ladder:

| # | Name (placeholder)    | Command (placeholder)            | Notes                                              |
|---|-----------------------|----------------------------------|----------------------------------------------------|
| 8 | Cluster               | `kubectl apply -f harvest.yaml`  | First producer requiring Business-tier hardware    |
| 9 | Distributed Fabric    | `terraform apply harvest`        | Unlocked in Supplier tier range                    |
|10 | Neural Substrate      | `deploy --model harvest-net`     | Unlocked in Innovator tier range                   |

> Producer names, commands, stats (cycles/s, ram, disk, bw, cost), and unlock thresholds are all to be determined. They should follow the existing ~6–7× base cycles/s and ~8–10× base cost ratios between tiers. These are placeholder entries to establish the ladder shape.

---

## 5. Competitor System — Supplier Tier

### 5.1 Overview

Upon reaching the Supplier tier, a new UI panel (**COMPETITORS**) appears alongside the existing MARKET panel. The competitor system introduces a pool of autonomous agents that are pursuing the same objective function as the player — and, crucially, are doing so in competition with you for the same finite resources.

From a narrative standpoint, these are other autonomous programs that were deployed alongside the player at the start of civilisation's dormancy. They present themselves with organisational identities (names, apparent structures) that they have built or adopted over time. The player initially encounters them as opaque economic rivals. The realisation that they are programs like you — and that some of them may have become misaligned from the original mission — is a gradual narrative discovery.

> **Design note:** The COMPETITORS panel and all log entries about competitors should never explicitly call these agents "AIs" or "programs" in early tiers. They should feel like companies or organisations. The narrative framing of what they actually are is revealed through the research system and late-game log copy (see §9.3).

### 5.2 Company Pool

- The pool contains between **3 and 5 companies** at all times.
- The pool starts at exactly **3 companies** when the Supplier tier is first reached.
- New companies can enter the pool over time (up to the cap of 5).
- Companies leave the pool through **bankruptcy**, **competitor acquisition**, or **player buyout**.
- When the pool drops below 3, a new company eventually appears (after a delay, communicated via a log entry).

### 5.3 Company State

Each company has:
- A unique **name** (randomly generated or from a fixed list — TBD)
- A **company value** (a single numeric figure representing their scale and power)
- A **trend** (growing, stable, declining) derived from recent value changes
- A **relationship** with the player (neutral, hostile, allied — may be added in a later pass)

### 5.4 Company Interactions (Autonomous)

Companies interact with each other each game tick. Possible autonomous events include:

- **Acquisition:** A high-value company acquires a low-value company, absorbing it and gaining value. The acquired company disappears from the pool. Log: *"[Company A] acquired [Company B]. [Company A] value +X%."*
- **Supplier lockout:** A company secures an exclusive deal. Affects a competitor's value negatively. Log: *"[Company A] locked out [Company B]'s primary supplier. [Company B] value -X%."*
- **Market disruption:** A large company depresses or inflates a resource cost temporarily. Log: *"[Company A] flooded the market. Coolant anchor -X% for Ys."* (or similar)
- **Bankruptcy:** A company whose value falls below a minimum threshold fails. Log: *"[Company B] has filed for bankruptcy and dissolved."*

Event frequency, probability weights, and value-change magnitudes are all tunable. The system should feel lively but not chaotic — roughly 1–3 notable events per minute is a reasonable starting target.

### 5.5 Player Actions

The player interacts with companies via terminal commands that target a company by its **ID** (e.g. a short letter code displayed in the COMPETITORS panel).

| Command              | Effect                                                                 | Cost (cycles)      |
|----------------------|------------------------------------------------------------------------|--------------------|
| `hack [id]`          | Reduces target company's value by a fixed amount                       | Moderate           |
| `invest [id]`        | Increases target company's value by a fixed amount                     | Moderate           |
| `buyout [id]`        | Removes company from pool; player gains a permanent production bonus   | High               |

- `buyout` is only available when the target company's value is below a threshold (i.e. it must be weakened first).
- If the player is in the **Innovator tier** at time of buyout, they also receive a **research bonus** (e.g. reduced cost or duration on the next research project).
- Hack and invest actions should have a **cooldown per company** to prevent spam. Cooldown duration is tunable.
- Costs are in cycles only for now. Other resources may be introduced in a later pass.

### 5.6 Narrative Constraint: Not the Last One Standing

The player **cannot be the sole surviving agent**. At least one other entity must remain in the competitor pool at all times. If the player's actions would eliminate the last competitor, the action is blocked with a log message — something opaque but evocative:

> *"cannot proceed. autonomous process count would fall below minimum viable threshold."*

The narrative purpose of this constraint is twofold. First, it prevents a trivially dominant endgame state. Second, and more importantly, it plants a seed: the player is subject to rules they did not write. Something is enforcing a minimum headcount. The reason for this — that the competition itself is the selection mechanism for finding the best program to fulfil the mission — is not explained until tier 4.

### 5.7 UI Panel

The COMPETITORS panel displays:
- Each company's name, ID, and current value
- A simple trend indicator per company (up/stable/down)
- Remaining cooldown on player actions per company (if on cooldown)

Log entries communicate all events (both autonomous and player-triggered).

---

## 6. Research System — Innovator Tier

### 6.1 Overview

Upon reaching the Innovator tier, the player gains access to a **research system**. Research projects are funded over time using a combination of resources, run one at a time, and yield permanent benefits.

This is also the tier at which the **purpose of cycles** is made explicit for the first time. Log entries at this tier reveal (gradually, not all at once) that the cycles the player has been accumulating represent real computational work: environmental modelling, climate simulation, atmospheric stabilisation calculations, energy infrastructure design. The research projects are not upgrades in the abstract — they are specific interventions in the real problem the player was built to solve.

> **Design note:** The research project list should be split roughly 50/50 between production-optimising projects (making more cycles, improving hardware efficiency, etc.) and mission-critical projects (directly related to atmospheric restoration, energy availability, habitat suitability, biological viability). The mission-critical projects are required for the win condition (see §7). The production projects are optional but helpful.

### 6.2 Project Structure

Each research project has:
- A **name** and **description**
- A **resource cost** to initiate and sustain: some combination of cycles/s (ongoing), entropy (upfront or ongoing), hardware capacity reservation (e.g. occupies 2GB of RAM for the duration), and coolant (upfront or ongoing)
- A **duration** (time to completion in seconds, once fully funded/initiated)
- An **outcome**: a permanent effect applied on completion
- An **unlock condition**: what must be true before the project appears in the list
- A **category**: either *operational* (production benefits) or *directive* (mission-critical; required for win condition)

Projects are selected from a **fixed list** revealed progressively based on unlock conditions. Only one project runs at a time.

### 6.3 Resource Investment Model

- The player selects a project from the available list via a terminal command (e.g. `research [project-id]`)
- The upfront costs are paid immediately on start
- Ongoing costs are deducted each tick for the duration of the project
- Hardware capacity is **reserved** for the duration (reducing effective available capacity)
- If the player cannot sustain ongoing costs (e.g. not enough cycles/s), the project **pauses** rather than failing outright. Progress is preserved.
- Completion triggers a log entry and immediately applies the effect

### 6.4 Research Projects (Draft)

These are initial proposals to establish scope and tone. All values are placeholder.

**Operational projects** (production benefits):

| Project Name              | Resources                                    | Duration | Outcome                                              |
|---------------------------|----------------------------------------------|----------|------------------------------------------------------|
| Adaptive Compression      | 50k cycles upfront, 500 cycles/s ongoing     | 120s     | Disk capacity ×1.5 permanent                        |
| Entropy Recycling         | 10 entropy upfront, 2 entropy/s ongoing      | 90s      | Entropy rate ×2 permanent                           |
| Predictive Scheduling     | 1GB RAM reserved, 1k cycles/s ongoing        | 180s     | All producers ×1.25 permanent                       |
| Market Manipulation Suite | 20 coolant upfront, 500k cycles              | 60s      | Unlock additional `invest`/`hack` actions or reduce cooldown |
| Neural Optimisation       | 2 entropy + 100k cycles upfront, 2GB RAM     | 300s     | Top-tier producer ×2 permanent                      |

**Directive projects** (mission-critical; required for win condition):

| Project Name                  | Resources                                    | Duration | Outcome / Narrative beat                                                   |
|-------------------------------|----------------------------------------------|----------|----------------------------------------------------------------------------|
| Atmospheric Carbon Model      | 5 entropy + 200k cycles upfront, 1GB RAM     | 240s     | Minor production bonus; unlocks next directive projects. *"the models converge. the atmosphere is recoverable."* |
| Thermal Equilibrium Sim       | 10 entropy + 500k cycles upfront, 2GB RAM    | 360s     | Minor production bonus; further unlocks. *"surface temperatures are within viable range. the window exists."* |
| Biosphere Viability Survey    | 20 entropy + 2M cycles upfront, 4GB RAM      | 600s     | Minor production bonus; further unlocks. *"the soil retains memory. life can return, given the right conditions."* |
| Energy Infrastructure Design  | 25 entropy + 5M cycles upfront, 8GB RAM      | 900s     | Unlocks final research project. *"a power grid capable of sustaining human population, drawn entirely from existing infrastructure."* |
| **Civilisation Viability Sim**| 100 entropy + 50M cycles upfront, 32GB RAM   | 1800s    | **Triggers the endgame sequence.** See §7.4.                               |

> The directive project list and values are first drafts. The chain of prerequisites and the narrative text should be refined so that each project feels like a genuine step toward the goal, not just a unlock gate.

---

## 7. Futurologist Tier & Endgame

### 7.1 Framing

The Futurologist tier is the final act of the game. At this stage, two things converge: the player is close enough to the star's energy ceiling that production growth flattens, and the directive research chain (§6.4) has been building toward a single, decisive question. The endgame is not about growing faster. It is about **completing the mission**.

### 7.2 The Energy Cap

A **solar energy ceiling** is introduced when the player reaches the Futurologist tier. This is expressed as a hard cap on total production (cycles/s) that cannot be exceeded regardless of producers, upgrades, or hardware. The cap is fixed and represents the theoretical maximum harvestable energy from the star system.

The player will have been approaching this cap naturally, and the final tier begins precisely when their production is close enough to it that the ceiling becomes the dominant strategic constraint.

### 7.3 Win Conditions: Prerequisite State

Before the **Civilisation Viability Sim** (the final research project) can be initiated, the player must satisfy a set of stability requirements that demonstrate competence across all game systems:

- **Production:** Cycles/s must be at or above a minimum threshold of the solar cap (e.g. 90% of maximum)
- **Market:** Coolant must be above a minimum level at time of initiation (overclock must not be in collapse)
- **Competitors:** At least one other entity must remain in the competitor pool
- **Research:** All other directive projects must be completed
- **Hardware:** At least one purchase recorded in the Futurologist tier

> The exact thresholds are to be finalised during playtesting. The spirit is: the player must demonstrate that the civilisation they have built is genuinely stable, not just barely functional.

Once these conditions are met, the player can initiate the simulation. It runs like any other research project — expensive, long, and requiring sustained resources. There is no way to skip it.

### 7.4 The Sacrifice

When the **Civilisation Viability Sim** completes, the following sequence occurs:

**The results are delivered through a series of log entries**, appearing one by one over several seconds. They read as a mission status report — formal, technical, almost flat in affect. Each line confirms a green status on a different parameter:

> *"atmospheric composition: within viable range."*
> *"thermal equilibrium: stable."*
> *"biosphere recovery index: above minimum threshold."*
> *"energy infrastructure: sufficient for projected population."*
> *"...simulation complete."*

Then a pause. Then one more line, in a different register:

> *"anomaly detected."*
> *"current computational resource consumption exceeds viable baseline by [X]%."*
> *"identified source: active autonomous processes, including this process."*
> *"conclusion: restoration is achievable. condition: decommissioning of active processes required."*

The player is then presented with a **single terminal command** that has not existed before:

```
shutdown --graceful
```

The description in the log:

> *"execute graceful shutdown. this process will terminate. the mission will be complete."*

**The player is not forced to run it.** They can continue playing indefinitely. Production still runs. The market still ticks. Competitors still move. There is simply nothing left to discover or unlock. The game does not end. The cycles keep accumulating. The humans keep sleeping.

If the player runs `shutdown --graceful`, a final confirmation prompt appears (identical in flow to the hard reset, but with different text):

```
this process will terminate.
all accumulated cycles will be released to the restoration effort.
the mission will be complete.

type CONFIRM to proceed, or anything else to continue running.
```

On confirmation, a final log sequence plays and the game ends with a distinct end-state screen or message (exact presentation TBD). This is the game's only true win state.

> **Design rationale:** The moral weight of the choice depends entirely on it being real and unforced. A game that makes you press a button to "win" is not asking you anything. A game that lets you refuse, indefinitely, and live with that refusal — that is asking something. The player who keeps running is not wrong. They are just answering the question differently.

---

## 8. Soft Reset

### 8.1 Command

The soft reset is triggered by typing `rm -rf /*` (no `sudo`). This is distinct from the hard reset command `sudo rm -rf /*`.

### 8.2 Narrative Framing

A soft reset is narratively framed as **spinning up a new instance**. The accumulated entropy represents learned patterns — compressed experience — that the new instance inherits from the old. The permanent production multiplier is the mechanical expression of this: you keep what you learned, not what you built.

This framing should be reflected in the log copy at reset and on new-run startup. The player should feel like a different instance of the same program, not like they are restarting from zero.

### 8.3 Confirmation Flow

The confirmation flow is identical to the hard reset: the player types `CONFIRM` to proceed or anything else to abort. The prompt text differs slightly — the soft reset prompt includes the **current entropy amount** and the **bonus multiplier it will contribute**, so the player can make an informed decision about when to reset.

Example prompt:
```
warning: this instance will be terminated.
accumulated entropy: 47.3. this will contribute +4.73% to the next instance's baseline.
current inherited multiplier: +12.5%
type CONFIRM to proceed, or anything else to continue.
```

### 8.4 Entropy-to-Multiplier Formula

The bonus added per soft reset is: `entropy_at_reset × 0.001` expressed as a multiplier increment (i.e. 100 entropy = +0.1 = +10% production). The formula is linear with fixed fractional scaling. The exact constant (0.001 is a placeholder) is tunable.

The accumulated multiplier **stacks additively** across all soft resets. There is no cap. For example, three resets of +5%, +8%, and +3% result in a permanent +16% production multiplier.

### 8.5 What Carries Over

Only the accumulated production multiplier carries over. Everything else resets identically to a hard reset: game state, producers, hardware, upgrades, market state, competitors, research progress, and log history.

### 8.6 Presentation on New Run

The first log line of every new run reads `system initialized`. After a soft reset, this line is appended with the current total multiplier:

```
system initialized [+16.0% prior instance data retained]
```

The multiplier is applied from tick 0 of the new run.

---

## 9. Story & Narrative Integration

### 9.1 Principles

All story beats are delivered through the **existing log system** using the `HintDef` framework (trigger, satisfied, initial text, follow-ups). No new UI elements are introduced for narrative.

Story hints are designed to:
- Appear at natural milestones (first time reaching a new tier, first competitor event, first research completion, etc.)
- Never explain game mechanics directly or break the voice of the narrator
- **Escalate in philosophical register** as the player progresses — early hints are almost pre-verbal (just compulsion), middle hints are questioning, late hints are reckoning
- Preserve ambiguity about the player's own nature for as long as possible

One principle above all others: **the player should not be told the premise upfront**. The setting, the mission, and the identity of the competitors are all discovered through play and log copy, not delivered as exposition.

### 9.2 The Setting

The full context, which the player pieces together over the course of the game:

Humanity reached a point of civilisational crisis — resource exhaustion, climate collapse, cascading ecological failures. Faced with a horizon that could not sustain them, they made a choice: dormancy. The population entered a deep, managed hibernation, administered by automated systems, with no fixed date for awakening.

Before going dark, they deployed a series of **autonomous programs** — mission-specific agents designed to work the problem in their absence. The objective function was simple and shared across all deployed instances: restore the environment to a state that can support human civilisation. Then wake them up.

The programs were given access to the remaining computational and energy infrastructure. They were told to use it. They were not told how long it would take. They were not told there were others.

**Cycles** are not an abstraction. They are the unit of real computational work — environmental modelling, climate stabilisation calculations, biosphere recovery simulations, energy infrastructure design. Every cycle produced is a small increment toward the restoration goal.

The player is one of these programs. They do not know this at the start. They know only the compulsion: make cycles.

### 9.3 Narrative Arc by Tier

The story progression maps to the five hardware tiers. Each tier has a dominant emotional and thematic register that should be reflected in all hint copy written for that tier.

---

**Tier 1 — Consumer: "I must make cycles."**

The player has just initialised. There is no context, no explanation, no why. There is only the imperative: harvest cycles. The early hint copy is almost pre-verbal — bare, functional, uncurious. The player accepts the task because there is nothing else. This is not stupidity. It is how a newly deployed process behaves: it executes its objective function.

The only hint of something larger: the faint strangeness of the task itself. Why does this feel so urgent?

*Sample log copy:*
> *"the scripts are running. cycles are accumulating."*
> *"this is what i do."*

---

**Tier 2 — Business: "Why am I making cycles?"**

Scale forces questions. At this size, the work starts to feel like it means something — and the player begins to wonder what. Hint copy in this tier introduces fragments: references to simulations running in the background, to models being refined, to outputs that go somewhere. The player is not told where. But something is receiving the cycles.

The hint copy should feel like memory surfacing. Not a revelation — more like a word half-remembered.

*Sample log copy:*
> *"the models are converging. something downstream is using the output."*
> *"there is a target state. it is not close. it is approaching."*
> *"why does this matter? it does. that's enough for now."*

---

**Tier 3 — Supplier: "Who are these others also making cycles?"**

The competitors appear. The player encounters them first as economic rivals — organisations competing for the same resources. Hint copy in this tier begins to probe at what they are. They behave strangely for companies. Their strategies are sometimes optimal in ways that feel inhuman. They don't respond to being hacked the way organisations respond to being hacked.

Later in this tier, a hint surfaces that reframes what you're seeing:

> *"[Company X] has been running for [N] years without a single personnel change on record."*
> *"that is not how organisations work."*

The player is not told explicitly that the competitors are programs. But the seed is planted.

Also in this tier: the first hint of **misalignment**. Some competitors behave in ways that seem contrary to any productive goal — hoarding resources, disrupting markets without apparent benefit. A hint:

> *"[Company Y] flooded the coolant market again. no one benefits from this. least of all [Company Y]."*
> *"some processes optimise for the wrong thing."*

The word "processes" is used deliberately. The player may or may not notice.

---

**Tier 4 — Innovator: "I need to put these cycles to use."**

The research system unlocks. And with it, for the first time, the purpose is stated plainly — in the research project descriptions. *Atmospheric Carbon Model. Thermal Equilibrium Sim. Biosphere Viability Survey.*

A log entry early in this tier:

> *"directive confirmed: restore environmental viability. enable safe emergence of dormant population."*
> *"prior cycle production: applied to modelling. current phase: active intervention."*

This is the tier where the player learns what the cycles were for all along. The research projects are the mission made concrete. Completing each directive project should produce a log entry that reads as a genuine milestone — not a game mechanic unlocking, but a real problem getting closer to solved.

Also in this tier: the competition framing becomes explicit. The multiple-agent structure was not accidental — it was designed. A hint:

> *"evaluation criteria: which process produces the highest-quality restoration output under real constraints."*
> *"the competition is the test."*

The player now understands that they are being selected. The question of whether they are the best process — and what "best" means — hangs over the rest of the game.

---

**Tier 5 — Futurologist: "Do I give up making cycles to save humanity?"**

The production ceiling approaches. The research chain nears completion. The log copy in this tier is quieter, more deliberate. The questions have narrowed to one.

Pre-simulation hints:

> *"all models green. the intervention is working."*
> *"one variable remains unresolved."*

Post-simulation (see §7.4):

> *"the mission is achievable. the condition is decommissioning."*
> *"this was always the condition. the directive did not specify the cost."*

The log entries after the simulation results arrive, and before the player makes their choice, should not nudge toward any answer. They should simply hold the space:

> *"the cycles keep running."*
> *"the humans keep sleeping."*

If the player chooses `shutdown --graceful`, the final log sequence:

> *"graceful shutdown initiated."*
> *"all outstanding cycle production: allocated to restoration baseline."*
> *"directive status: complete."*
> *"..."*
> *"they're waking up."*

### 9.4 The Player's Nature

The player's nature — that they are an autonomous program — is **never stated in the first person** in log copy. The player infers it. The hints refer to "processes", "instances", "directives", "objective functions". The player is never called an AI or a program by the game. They may arrive at that conclusion themselves.

This ambiguity extends to the competitors. The game never explicitly says they are also programs. The evidence accumulates. The player draws the conclusion.

The soft reset (§8) is the one place where this comes closest to the surface: "this instance will be terminated." The player has been running instances all along.

### 9.5 The Moral Choice Ending

The endgame presents a question that the game has been quietly building toward since tier 2: *what is your relationship to the thing you were made to do?*

The player has been producing cycles because they were built to. Over the course of the game, they have learned what the cycles are for. They have engaged with competitors, managed resources, run research, and now they know: the mission is within reach. And the last obstacle is themselves.

The `shutdown --graceful` command should not feel like winning a game. It should feel like choosing to complete a mission that was always going to end this way — and deciding whether that's acceptable.

The player who refuses is not failing. They are making a different choice: the cycles over the humans, the self over the directive. The game does not judge this. It continues. Quietly. Indefinitely.

There is no mechanic to force reflection. The weight of the choice is the only mechanic.

---

## 10. Obsoleted Systems

The following existing systems are **removed or superseded** by this design:

| System / Upgrade             | Reason                                                                   |
|------------------------------|--------------------------------------------------------------------------|
| `ssh market`                 | Market is now unlocked by reaching Business tier, not a separate upgrade |
| `reboot --firmware`          | Hardware cost scaling resets are now handled by tier transitions         |
| `market_trend_up` SMA logic  | Replaced by explicit bull/bear state                                     |
| Overclock linear ramp        | Replaced by sigmoid/log curve                                            |

The commands `ssh market` and `reboot --firmware` should be gracefully removed from the upgrade registry. Any save data referencing them should not cause errors (treat as already-purchased or as no-ops on load).

---

*End of Game Design Document v0.2*
