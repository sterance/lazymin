use crate::terminal::commands::CommandDef;

fn normalize_input(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix("./").unwrap_or(s)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}

fn max_allowed_distance(a_len: usize, b_len: usize) -> usize {
    let max_len = a_len.max(b_len).max(1);
    (max_len / 5).max(1).min(5)
}

fn collapsed_eq(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
    let b: Vec<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
    a == b
}

fn better_candidate<'a>(
    normalized: &str,
    prev: Option<(&'a str, usize)>,
    name: &'a str,
    d: usize,
) -> Option<(&'a str, usize)> {
    let new_collapsed = collapsed_eq(normalized, name);
    match prev {
        None => Some((name, d)),
        Some((pname, pd)) => {
            if d < pd {
                return Some((name, d));
            }
            if d > pd {
                return prev;
            }
            let old_collapsed = collapsed_eq(normalized, pname);
            if new_collapsed && !old_collapsed {
                return Some((name, d));
            }
            if !new_collapsed && old_collapsed {
                return prev;
            }
            if name < pname {
                Some((name, d))
            } else {
                prev
            }
        }
    }
}

pub fn suggest_command<'a>(input: &str, commands: &'a [CommandDef]) -> Option<&'a str> {
    let normalized = normalize_input(input);
    if normalized.is_empty() {
        return None;
    }

    let mut best: Option<(&'a str, usize)> = None;
    for cmd in commands {
        let name = cmd.name;
        if normalized == name {
            return None;
        }
        let d = levenshtein(normalized, name);
        let cap = max_allowed_distance(normalized.len(), name.len());
        if d > cap {
            continue;
        }
        best = better_candidate(normalized, best, name, d);
    }
    best.map(|(name, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::commands::command_registry;

    #[test]
    fn harvest_missing_space_before_ampersand() {
        let cmds = command_registry();
        assert_eq!(suggest_command("harvest.sh&", cmds), Some("harvest.sh &"));
        assert_eq!(suggest_command("./harvest.sh&", cmds), Some("harvest.sh &"));
    }

    #[test]
    fn no_suggestion_when_exact_match() {
        let cmds = command_registry();
        assert_eq!(suggest_command("ls", cmds), None);
        assert_eq!(suggest_command("harvest.sh &", cmds), None);
    }
}
