use chrono::{DateTime, Local};

use super::block::SerializedBlock;

#[derive(Clone, Debug)]
pub enum SerializedBlockListItem {
    Command { block: Box<SerializedBlock> },
}

impl SerializedBlockListItem {
    pub fn start_ts(&self) -> Option<DateTime<Local>> {
        match self {
            Self::Command { block } => block.start_ts,
        }
    }
}

impl From<SerializedBlock> for SerializedBlockListItem {
    fn from(block: SerializedBlock) -> Self {
        Self::Command {
            block: Box::new(block),
        }
    }
}

impl From<persistence::model::Block> for SerializedBlockListItem {
    fn from(block: persistence::model::Block) -> Self {
        SerializedBlock::from(block).into()
    }
}
