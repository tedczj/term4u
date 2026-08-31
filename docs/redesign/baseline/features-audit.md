# M2 default feature 审计

- 基线：`b5f4f0f6`
- 范围：`app/Cargo.toml` 的 202 个 `default` feature
- 结论：69 项“本地保留”已全部进入 `local_only`；133 项“云删除”明确不进入；无待定项。

| # | feature | 分类 | 依据 |
|---:|---|---|---|
| 1 | `agent_mode` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 2 | `osc_hyperlinks` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 3 | `viewing_shared_sessions` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 4 | `render_continuous_block_selections_with_single_border` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 5 | `shared_with_me` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 6 | `session_sharing_acls` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 7 | `external_agent_mode_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 8 | `shell_selector` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 9 | `minimalist_ui` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 10 | `avatar_in_tab_bar` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 11 | `full_screen_zen_mode` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 12 | `workflow_aliases` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 13 | `ligatures` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 14 | `dynamic_workflow_enums` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 15 | `rect_selection` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 16 | `reload_stale_conversation_files` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 17 | `loginless_conversion` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 18 | `warp_packs` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 19 | `ai_rules` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 20 | `suggested_rules` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 21 | `default_waterfall_mode` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 22 | `am_workflows` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 23 | `autoupdate_ui_revamp` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 24 | `render_agent_mode_output_markdown` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 25 | `agent_mode_primary_xml` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 26 | `agent_mode_pre_plan_xml` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 27 | `command_correction_key` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 28 | `kitty_images` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 29 | `global_ai_analytics_collection` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 30 | `grep_tool` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 31 | `validate_autosuggestions` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 32 | `clear_autosuggestion_on_escape` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 33 | `file_retrieval_tools` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 34 | `mcp_server` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 35 | `fast_forward_autoexecute_button` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 36 | `image_as_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 37 | `command_palette_file_search` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 38 | `usage_based_pricing` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 39 | `ai_context_menu` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 40 | `at_menu_outside_of_ai_mode` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 41 | `agent_management_view` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 42 | `agent_management_details_view` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 43 | `interactive_conversation_management_view` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 44 | `shared_block_title_generation` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 45 | `tab_close_button_on_left` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 46 | `ai_resume_button` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 47 | `code_find_replace` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 48 | `ai_context_menu_code` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 49 | `agent_decides_command_execution` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 50 | `linked_code_blocks` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 51 | `drive_objects_as_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 52 | `search_codebase_ui` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 53 | `profiles_design_revamp` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 54 | `multi_profile` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 55 | `tabbed_editor_view` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 56 | `allow_opening_file_links_using_editor_env` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 57 | `read_image_files` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 58 | `selection_as_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 59 | `undo_closed_panes` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 60 | `revert_diff_hunk` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 61 | `code_review_save_changes` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 62 | `create_project_flow` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 63 | `get_started_tab` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 64 | `file_tree` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 65 | `vim_code_editor` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 66 | `code_launch_modal` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 67 | `allow_ignoring_input_suggestions` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 68 | `mcp_oauth` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 69 | `expand_edit_to_pane` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 70 | `fallback_model_load_output_messaging` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 71 | `api_key_management` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 72 | `summarization_cancellation_confirmation` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 73 | `ui_zoom` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 74 | `discard_per_file_and_all_changes` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 75 | `auto_open_code_review_pane` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 76 | `diff_set_as_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 77 | `summarize_conversation_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 78 | `inline_code_review` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 79 | `web_search_ui` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 80 | `agent_shared_sessions` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 81 | `integration_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 82 | `artifact_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 83 | `conversation_api` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 84 | `cloud_environments` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 85 | `create_environment_slash_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 86 | `code_review_find` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 87 | `async_find` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 88 | `shared_session_long_running_commands` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 89 | `mcp_grouped_server_context` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 90 | `fork_from_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 91 | `context_window_usage_v2` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 92 | `context_window_usage_breakdown` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 93 | `ambient_agents_command_line` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 94 | `ambient_agents_image_upload` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 95 | `scheduled_ambient_agents` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 96 | `warp_managed_secrets` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 97 | `v4a_file_diffs` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 98 | `classic_completions` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 99 | `force_classic_completions` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 100 | `team_api_keys` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 101 | `named_agents` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 102 | `agent_tips` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 103 | `pluggable_notifications` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 104 | `agent_onboarding` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 105 | `account_first_onboarding` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 106 | `global_search` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 107 | `cloud_conversations` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 108 | `list_skills` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 109 | `ask_user_question` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 110 | `bundled_skills` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 111 | `ambient_agents_rtc` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 112 | `cloud_mode` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 113 | `cloud_mode_from_local_session` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 114 | `cloud_mode_image_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 115 | `agent_mode_computer_use` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 116 | `background_computer_use` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 117 | `oz_platform_skills` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 118 | `oz_identity_federation` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 119 | `sync_ambient_plans` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 120 | `conversation_artifacts` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 121 | `agent_view` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 122 | `agent_view_block_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 123 | `agent_view_conversation_list_view` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 124 | `inline_slash_commands` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 125 | `inline_history_menu` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 126 | `inline_model_selector` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 127 | `oz_launch_modal` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 128 | `open_warp_launch_modal` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 129 | `orchestration_launch_modal` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 130 | `agent_cli_launch_modal` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 131 | `new_tab_styling` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 132 | `richtext_multiselect` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 133 | `inline_profile_selector` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 134 | `web_fetch_ui` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 135 | `oz_changelog_updates` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 136 | `skill_arguments` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 137 | `incremental_auto_reload` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 138 | `active_conversation_requires_interaction` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 139 | `figma_detection` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 140 | `file_based_mcp` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 141 | `inline_repo_menu` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 142 | `kitty_keyboard_protocol` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 143 | `inline_menu_headers` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 144 | `github_pr_prompt_chip` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 145 | `conversations_as_context` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 146 | `markdown_tables` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 147 | `blocklist_markdown_table_rendering` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 148 | `markdown_mermaid` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 149 | `blocklist_markdown_images` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 150 | `pr_comments_v2` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 151 | `pr_comments_skill` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 152 | `revert_to_checkpoints` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 153 | `rewind_slash_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 154 | `hoa_code_review` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 155 | `warpify_footer` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 156 | `agent_toolbar_editor` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 157 | `configurable_toolbar` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 158 | `transfer_control_tool` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 159 | `hoa_notifications` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 160 | `open_code_notifications` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 161 | `cli_agent_rich_input` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 162 | `vertical_tabs` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 163 | `vertical_tabs_summary_mode` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 164 | `tab_configs` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 165 | `grouped_tabs` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 166 | `pinned_tabs` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 167 | `agent_harness` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 168 | `hoa_onboarding_flow` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 169 | `hoa_remote_control` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 170 | `codex_notifications` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 171 | `codex_plugin` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 172 | `trim_trailing_blank_lines` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 173 | `skip_firebase_anonymous_user` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 174 | `settings_file` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 175 | `directory_tab_colors` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 176 | `git_credential_refresh` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 177 | `oz_handoff` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 178 | `handoff_local_cloud` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 179 | `pending_user_query_indicator` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 180 | `queue_slash_command` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 181 | `queued_prompts_v2` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 182 | `cloud_mode_input_v2` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 183 | `cloud_mode_setup_v2` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 184 | `handoff_cloud_cloud` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 185 | `remote_codebase_indexing` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 186 | `solo_user_byok` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 187 | `custom_model_routers` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 188 | `supergrok` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 189 | `gemini_enterprise` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 190 | `billing_and_usage_page_v2` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 191 | `remote_code_review` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 192 | `git_operations_in_code_review` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 193 | `terminal_lifecycle_recovery` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 194 | `cloud_runners` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 195 | `cloud_agent_runners` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 196 | `file_backed_execution_profiles` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 197 | `well_known_mcp_ids` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 198 | `factory_mcp` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 199 | `wait_for_events_parent_registration` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 200 | `orchestration_unified_stack` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
| 201 | `ime_marked_text` | 本地保留 | 终端、编辑器、文件、主题/布局、补全、设置、本地 MCP 或本地代码审查能力 |
| 202 | `ctrl_c_cancels_third_party_harness` | 云删除 | 依赖云 Agent、账号、共享、Drive、远程服务、更新、在线内容或数据上报路径 |
