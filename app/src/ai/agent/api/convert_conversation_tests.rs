use std::collections::HashMap;

use warp_multi_agent_api::message::ToolCallResult;
use warp_multi_agent_api::message::tool_call_result::Result;
use warp_multi_agent_api::{CallMcpToolResult, ReadMcpResourceResult};

use super::convert_tool_call_result_to_input;
use crate::ai::agent::task::TaskId;

fn convert(result: Result) -> Option<crate::ai::agent::AIAgentInput> {
    convert_tool_call_result_to_input(
        &TaskId::new("task".to_owned()),
        &ToolCallResult {
            tool_call_id: "tool-call".to_owned(),
            context: None,
            result: Some(result),
        },
        &HashMap::new(),
        &mut HashMap::new(),
    )
}

#[test]
fn historical_mcp_results_are_ignored() {
    assert!(convert(Result::ReadMcpResource(ReadMcpResourceResult::default())).is_none());
    assert!(convert(Result::CallMcpTool(CallMcpToolResult::default())).is_none());
}
