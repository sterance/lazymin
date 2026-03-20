use super::commands::command_registry;

pub enum InputHighlight {
    Unknown,
    PartialMatch,
    Ready,
}

pub fn classify_input(input: &str) -> InputHighlight {
    let normalized = input.trim_end();
    if normalized.is_empty() {
        return InputHighlight::Unknown;
    }

    let mut partial = false;
    for cmd in command_registry() {
        if cmd.name == normalized {
            return InputHighlight::Ready;
        }
        if cmd.name.starts_with(normalized) {
            partial = true;
        }
    }

    if partial {
        InputHighlight::PartialMatch
    } else {
        InputHighlight::Unknown
    }
}

