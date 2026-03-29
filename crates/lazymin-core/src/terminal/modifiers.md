# Command modifiers

Player input is normalized with trailing whitespace trimmed, then resolved to a **base command or upgrade string** plus metadata. Resolution is implemented in `CommandModifiers.rs` (`resolve_modifiers`).

## Result of resolution

`resolve_modifiers(trimmed)` returns:

1. **`CommandModifiers`** — bit flags for effects that apply to this run (currently `sudo` and, transiently, `-max` during parsing).
2. **`PurchaseRepeat`** — how many times a paid (or costless repeat) purchase loop should run: `Once`, `Max` (`-max`), or `Times(n)` (` *n`).
3. **`effective`** — the command or upgrade string after stripping recognized modifiers.

Downstream code (`execute.rs`, `highlight.rs`, `permission_lock.rs`) uses this tuple; purchase loops and cycle checks live in `execute.rs` / `max_purchase.rs`.

## Early exit: already a known command

If the **entire** trimmed line matches a known registry command or upgrade command, resolution stops immediately: no suffix or prefix stripping, `(default modifiers, PurchaseRepeat::Once, full input)`.

Examples: a bare `sudo visudo`, or any string that is exactly a valid command name. This avoids treating substrings of special inputs (e.g. `sudo rm -rf /*`) as modifiers.

## Suffixes (stripped first, in a loop)

Suffixes are removed from the **end** of the string, repeatedly, until one pass removes nothing.

Each **iteration** applies, in order:

1. **Repeat count** — if the line ends with a ` *` + ASCII digits segment (space, asterisk, digits only to end of string), strip it. The prefix left after the strip must be non-empty. Digits must parse to a **non-zero** unsigned integer (`*0` does not match).
2. **Max** — else if the line ends with the literal ` -max`, strip that suffix; the remainder must be non-empty.

Then the loop runs again on the shortened string. So both orders work, e.g. `cmd *3 -max` and `cmd -max *3` reduce to `cmd` after enough iterations. Repeated chunks (e.g. `cmd *2 -max *2`) are stripped until stable.

If multiple ` *n` segments appear (unusual), stripping always removes the **rightmost** ` *n` first; each removal overwrites the stored count, so the **leftmost** ` *n` in the original input ends up determining `n` for `PurchaseRepeat::Times(n)`.

### `PurchaseRepeat` after suffix stripping

- If **any** repeat-count suffix ` *n` was stripped, **`PurchaseRepeat::Times(n)`**. The last successful strip of ` *n` sets `n`. The `-max` bit is **not** left set on modifiers when `Times` applies.
- Else if **` -max` was stripped at least once**, **`PurchaseRepeat::Max`**, and the max modifier bit is present as appropriate for parsing.
- Else **`PurchaseRepeat::Once`**.

So `-max` and ` *n` together always yield **`Times(n)`**, not `Max`.

## Prefix (stripped only if needed)

If, after all suffix stripping, the string is **not** yet a known command or upgrade, a single prefix may be stripped:

- **`sudo `** — removes the leading literal `sudo ` when the remainder is non-empty, and records the sudo modifier (permission bypass for locked commands where applicable).

Only one prefix pass is defined; there is no stacking of multiple prefix types.

## Typical combinations

For a registry command like `apt install ram` or `harvest.sh`, these shapes are supported (non-exhaustive; see unit tests `resolve_modifiers_*_all_suffix_prefix_permutations`):

| Prefix | Suffix(es) | Effective command | Repeat |
|--------|------------|-------------------|--------|
| (none) | (none) | base | Once |
| `sudo ` | (none) | base without `sudo ` | Once |
| (none) | ` -max` | base | Max |
| `sudo ` | ` -max` | base | Max |
| (none) | ` *n` | base | Times(n) |
| `sudo ` | ` *n` | base | Times(n) |
| (none) | ` *n -max` or ` -max *n` | base | Times(n) |
| `sudo ` | ` *n -max` or ` -max *n` | base | Times(n) |

For upgrades whose canonical command string **includes** `sudo ` (e.g. `sudo visudo`), the full string may be recognized **after** suffix stripping but **before** stripping an extra `sudo ` prefix; in those cases the effective string may still start with `sudo `.

## Execution (summary)

- **`PurchaseRepeat::Max`** with a command that has a cycle cost: repeat purchase until a resource gate or error, with summary lines as in `max_purchase.rs` (`capped by:` when stopped early).
- **`PurchaseRepeat::Times(n)`** with a cost: same loop, but at most `n` successful purchases; if all `n` succeed, the summary line has no `capped by:` clause.
- **`PurchaseRepeat::Times(n)`** with no cost: run the command body up to `n` times, with analogous success vs capped summaries.
- **`PurchaseRepeat::Max`** with no cost: a single execution (same as unmodified command for costless commands).

Exact output formatting and edge cases are covered by tests in `execute.rs` and behavior in `max_purchase.rs`.
