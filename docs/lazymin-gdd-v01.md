# lazymin — Game Design Document
## Refinement & Expansion

> This document describes planned changes to existing systems and the design of new systems. It is written from a game design perspective. A separate implementation document will follow once this is approved.

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

Upon reaching the Supplier tier, a new UI panel (**COMPETITORS**) appears alongside the existing MARKET panel. The competitor system introduces a pool of AI-controlled companies that are competing in the same space as the player. They interact with each other and with the player's actions, creating an emergent dynamic market ecosystem.

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

### 5.6 UI Panel

The COMPETITORS panel displays:
- Each company's name, ID, and current value
- A simple trend indicator per company (up/stable/down)
- Remaining cooldown on player actions per company (if on cooldown)

Log entries communicate all events (both autonomous and player-triggered).

---

## 6. Research System — Innovator Tier

### 6.1 Overview

Upon reaching the Innovator tier, the player gains access to a **research system**. Research projects are funded over time using a combination of resources, run one at a time, and yield permanent benefits affecting production, hardware, the market, or competitors.

### 6.2 Project Structure

Each research project has:
- A **name** and **description**
- A **resource cost** to initiate and sustain: some combination of cycles/s (ongoing), entropy (upfront or ongoing), hardware capacity reservation (e.g. occupies 2GB of RAM for the duration), and coolant (upfront or ongoing)
- A **duration** (time to completion in seconds, once fully funded/initiated)
- An **outcome**: a permanent effect applied on completion
- An **unlock condition**: what must be true before the project appears in the list

Projects are selected from a **fixed list** revealed progressively based on unlock conditions. Only one project runs at a time.

### 6.3 Resource Investment Model

- The player selects a project from the available list via a terminal command (e.g. `research [project-id]`)
- The upfront costs are paid immediately on start
- Ongoing costs are deducted each tick for the duration of the project
- Hardware capacity is **reserved** for the duration (reducing effective available capacity)
- If the player cannot sustain ongoing costs (e.g. not enough cycles/s), the project **pauses** rather than failing outright. Progress is preserved.
- Completion triggers a log entry and immediately applies the effect

### 6.4 Example Research Projects (Draft)

These are initial proposals to establish scope. All values are placeholder.

| Project Name              | Resources                                    | Duration | Outcome                                              |
|---------------------------|----------------------------------------------|----------|------------------------------------------------------|
| Adaptive Compression      | 50k cycles upfront, 500 cycles/s ongoing     | 120s     | Disk capacity ×1.5 permanent                        |
| Entropy Recycling         | 10 entropy upfront, 2 entropy/s ongoing      | 90s      | Entropy rate ×2 permanent                           |
| Predictive Scheduling     | 1GB RAM reserved, 1k cycles/s ongoing        | 180s     | All producers ×1.25 permanent                       |
| Market Manipulation Suite | 20 coolant upfront, 500k cycles              | 60s      | Unlock additional `invest`/`hack` actions or reduce cooldown |
| Neural Optimisation       | 2 entropy + 100k cycles upfront, 2GB RAM     | 300s     | Top-tier producer ×2 permanent                      |

> These projects and their values are first drafts. The list should grow and be refined through playtesting.

---

## 7. Futurologist Tier & Endgame

### 7.1 Framing

The Futurologist tier is the final act of the game. At this stage, the player encounters the fundamental constraint that has been foreshadowed through the narrative: **the finite energy available from their star system**. The player has been growing exponentially, but exponential growth cannot continue indefinitely in a finite universe.

The endgame is not about growing faster. It is about **optimising toward a stable maximum** in the face of hard resource limits.

### 7.2 The Energy Cap

A **solar energy ceiling** is introduced when the player reaches the Futurologist tier. This is expressed as a hard cap on total production (cycles/s) that cannot be exceeded regardless of producers, upgrades, or hardware. The cap is fixed and represents the theoretical maximum harvestable energy from the star system.

The player will have been approaching this cap naturally, and the final tier begins precisely when their production is close enough to it that the ceiling becomes the dominant strategic constraint.

### 7.3 Win Condition (Draft)

The win condition is to **maintain production at or above a minimum threshold of the cap** (e.g. 90% of maximum) **while simultaneously satisfying a non-trivial set of stability requirements**. These requirements draw on all previously introduced systems:

- Market: coolant must stay above a minimum level (overclock must not collapse)
- Competitors: at least one company must remain solvent (the player cannot be the sole remaining entity — an antitrust / narrative constraint)
- Research: at least one specific late-game research project must be completed
- Hardware: all hardware tiers must have at least one purchase recorded

The exact requirements and threshold values are to be finalised. The spirit is: the player must demonstrate competence across all systems simultaneously, not just brute-force production.

A win-state log message concludes the game, narrating what this optimised civilisation represents. The player is returned to the title or given a soft-reset prompt.

---

## 8. Soft Reset

### 8.1 Command

The soft reset is triggered by typing `rm -rf /*` (no `sudo`). This is distinct from the hard reset command `sudo rm -rf /*`.

### 8.2 Confirmation Flow

The confirmation flow is identical to the hard reset: the player types `CONFIRM` to proceed or anything else to abort. The prompt text differs slightly — the soft reset prompt includes the **current entropy amount** and the **bonus multiplier it will contribute**, so the player can make an informed decision about when to reset.

Example prompt:
```
warning: this will reset all progress.
you have 47.3 entropy. this will contribute +4.73% to your permanent production multiplier.
current accumulated multiplier: +12.5%
type CONFIRM to proceed, or anything else to abort.
```

### 8.3 Entropy-to-Multiplier Formula

The bonus added per soft reset is: `entropy_at_reset × 0.001` expressed as a multiplier increment (i.e. 100 entropy = +0.1 = +10% production). The formula is linear with fixed fractional scaling. The exact constant (0.001 is a placeholder) is tunable.

The accumulated multiplier **stacks additively** across all soft resets. There is no cap. For example, three resets of +5%, +8%, and +3% result in a permanent +16% production multiplier.

### 8.4 What Carries Over

Only the accumulated production multiplier carries over. Everything else resets identically to a hard reset: game state, producers, hardware, upgrades, market state, competitors, research progress, and log history.

### 8.5 Presentation on New Run

The first log line of every new run reads `system initialized`. After a soft reset, this line is appended with the current total multiplier:

```
system initialized [+16.0% entropy bonus]
```

The multiplier is applied from tick 0 of the new run.

---

## 9. Story & Narrative Integration

### 9.1 The Player's Identity

The player's identity is deliberately **left ambiguous** throughout the game. The player is clearly some form of intelligence harvesting computational cycles, but *what* they are — an AI, a collective, a future civilisation — is never stated. This ambiguity is intentional and should be preserved in all hint/log copy.

### 9.2 The Underlying Narrative

The overarching story, revealed gradually through logs and gameplay events, is:

> Humanity's population has grown beyond what any finite environment can sustain. The player is part of an effort — the nature of which is left vague — to solve the problems this creates, one by one, as they emerge. The harvesting of computational cycles is the mechanism by which these problems are modelled, simulated, and solved.

The endgame reveals that even this solution has limits: the star's energy is finite. The win condition represents finding a stable optimum — not infinite growth, but sustainable equilibrium.

### 9.3 Narrative Delivery

All story beats are delivered through the **existing log system** using the `HintDef` framework (trigger, satisfied, initial text, follow-ups). No new UI elements are introduced for narrative.

Story hints are designed to:
- Appear at natural milestones (first time reaching a new tier, first competitor buyout, first research completion, etc.)
- Never break the fourth wall or explain the game's mechanics directly
- Escalate in tone as the player progresses — early hints are mundane/technical, later hints grow more philosophical

### 9.4 Sample Log / Hint Copy (Draft)

These are tone references, not final copy.

**Early game (consumer tier):**
> *"the scripts are running. the cycles are accumulating. it's a start."*

**On reaching business tier:**
> *"at this scale, individual units stop mattering. it's all throughput now."*

**On first competitor appearing:**
> *"you are not the only one doing this."*

**On first company bankruptcy:**
> *"[company] couldn't keep up. the efficient thing to do is not dwell on it."*

**On approaching the energy cap (futurologist tier):**
> *"growth projections are flattening. this is not a bug."*
> *"the star can only give so much. the question is whether it's enough."*

**On win:**
> *"stable. optimal. a civilisation the size of a solar system, running quietly at the edge of what physics allows. for now, it is enough."*

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

*End of Game Design Document v0.1*
