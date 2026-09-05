use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Copy, Clone, Eq, PartialOrd, Sequence, Hash, Serialize, Deserialize)]
pub enum GridType {
    Prompt,
    Rprompt,
    PromptAndCommand,
    Output,
}
