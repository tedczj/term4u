use warpui::units::IntoLines;
use warpui::{App, TypedActionView};

use super::*;
use crate::terminal::find::BlockGridMatch;
use crate::terminal::model::ansi::{Handler, Mode};
use crate::terminal::model::block::SerializedBlock;
use crate::terminal::model::index::Point;
use crate::terminal::model::terminal_model::BlockIndex;
use crate::terminal::view::TerminalAction;
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

#[test]
fn l0_08_collapse_keeps_payload_and_command_and_updates_height() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let blocks = [SerializedBlock::new_for_test(
            b"print lines".to_vec(),
            b"one\r\ntwo\r\nthree".to_vec(),
        )
        .into()];
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&blocks));
        terminal.update(&mut app, |view, ctx| {
            let (id, height, output) = {
                let model = view.model.lock();
                let list = model.block_list();
                (
                    list.blocks()[0].id().clone(),
                    list.block_heights().summary().height,
                    list.blocks()[0].output_to_string(),
                )
            };
            view.handle_action(
                &TerminalAction::LocalOutput(LocalOutputAction::ToggleBlockOutput(id.clone())),
                ctx,
            );
            {
                let model = view.model.lock();
                let list = model.block_list();
                let block = list.block_with_id(&id).unwrap();
                assert!(block.should_hide_output_grid());
                assert!(!block.should_hide_command_grid());
                assert_eq!(block.output_to_string(), output);
                assert!(list.block_heights().summary().height < height);
            }
            view.handle_action(
                &TerminalAction::LocalOutput(LocalOutputAction::ToggleBlockOutput(id.clone())),
                ctx,
            );
            assert_eq!(
                view.model
                    .lock()
                    .block_list()
                    .block_heights()
                    .summary()
                    .height,
                height
            );
        });
    });
}

#[test]
fn l0_08_find_expands_collapsed_output_before_mapping_the_match() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let blocks = [SerializedBlock::new_for_test(
            b"print lines".to_vec(),
            b"one\r\ntwo\r\nthree".to_vec(),
        )
        .into()];
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&blocks));
        terminal.update(&mut app, |view, ctx| {
            let id = view.model.lock().block_list().blocks()[0].id().clone();
            view.handle_local_output_action(&LocalOutputAction::ToggleBlockOutput(id.clone()), ctx);
            view.handle_wakeup(
                Some(BlockGridMatch {
                    is_filtered: false,
                    block_index: BlockIndex(0),
                    grid_type: GridType::Output,
                    range: Point::new(1, 0)..=Point::new(1, 2),
                }),
                ctx,
            );
            assert!(
                !view
                    .model
                    .lock()
                    .block_list()
                    .block_with_id(&id)
                    .unwrap()
                    .should_hide_output_grid()
            );
        });
    });
}

#[test]
fn l0_08_active_and_stale_block_collapse_requests_are_harmless() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, ctx| {
            let id = view.model.lock().block_list().active_block().id().clone();
            let count = view.model.lock().block_list().blocks().len();
            view.handle_local_output_action(&LocalOutputAction::ToggleBlockOutput(id.clone()), ctx);
            assert!(
                !view
                    .model
                    .lock()
                    .block_list()
                    .active_block()
                    .should_hide_output_grid()
            );
            let missing = BlockId::new();
            assert!(
                view.model
                    .lock()
                    .block_list()
                    .block_with_id(&missing)
                    .is_none()
            );
            view.handle_local_output_action(&LocalOutputAction::ToggleBlockOutput(missing), ctx);
            assert_eq!(view.model.lock().block_list().blocks().len(), count);
        });
    });
}

#[test]
fn l0_08_transcript_clear_preserves_draft_and_restored_blocks() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let blocks =
            [SerializedBlock::new_for_test(b"echo kept".to_vec(), b"kept".to_vec()).into()];
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&blocks));
        terminal.update(&mut app, |view, ctx| {
            view.input
                .update(ctx, |input, ctx| input.insert_text("draft 中文", ctx));
            let (id, count) = {
                let mut model = view.model.lock();
                model
                    .block_list_mut()
                    .set_next_gap_height_in_lines(30.into_lines());
                (
                    model.block_list().blocks()[0].id().clone(),
                    model.block_list().blocks().len(),
                )
            };
            view.handle_action(
                &TerminalAction::LocalOutput(LocalOutputAction::ClearVisible),
                ctx,
            );
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "draft 中文");
            assert_eq!(view.model.lock().block_list().blocks().len(), count);
            assert_eq!(
                view.model
                    .lock()
                    .block_list()
                    .block_with_id(&id)
                    .unwrap()
                    .output_to_string(),
                "kept"
            );
        });
    });
}

#[test]
fn l0_08_native_screen_rejects_transcript_actions_and_keeps_contents() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let blocks =
            [SerializedBlock::new_for_test(b"echo kept".to_vec(), b"kept".to_vec()).into()];
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&blocks));
        terminal.update(&mut app, |view, ctx| {
            let id = view.model.lock().block_list().blocks()[0].id().clone();
            view.model.lock().set_mode(Mode::SwapScreen {
                save_cursor_and_clear_screen: true,
            });
            let height = view
                .model
                .lock()
                .block_list()
                .block_heights()
                .summary()
                .height;
            for action in [
                LocalOutputAction::ClearVisible,
                LocalOutputAction::PreviousBlock,
                LocalOutputAction::NextBlock,
                LocalOutputAction::ToggleBlockOutput(id.clone()),
            ] {
                view.handle_local_output_action(&action, ctx);
            }
            let model = view.model.lock();
            assert_eq!(model.block_list().block_heights().summary().height, height);
            assert!(
                !model
                    .block_list()
                    .block_with_id(&id)
                    .unwrap()
                    .should_hide_output_grid()
            );
            assert!(model.block_list().selection().is_none());
        });
    });
}

#[test]
fn l0_08_block_navigation_skips_hidden_blocks_and_does_not_edit_draft() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let blocks = (0..3)
            .map(|i| {
                SerializedBlock::new_for_test(
                    format!("echo {i}").into_bytes(),
                    format!("row{i}").into_bytes(),
                )
                .into()
            })
            .collect::<Vec<_>>();
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&blocks));
        terminal.update(&mut app, |view, ctx| {
            let hidden = view.model.lock().block_list().blocks()[1].id().clone();
            view.model
                .lock()
                .block_list_mut()
                .toggle_visibility_of_block(&hidden);
            view.input
                .update(ctx, |input, ctx| input.insert_text("unchanged", ctx));
            view.handle_local_output_action(&LocalOutputAction::PreviousBlock, ctx);
            assert!(view.selected_text(ctx).unwrap().contains("row2"));
            view.handle_local_output_action(&LocalOutputAction::PreviousBlock, ctx);
            assert!(view.selected_text(ctx).unwrap().contains("row0"));
            view.handle_local_output_action(&LocalOutputAction::NextBlock, ctx);
            assert!(view.selected_text(ctx).unwrap().contains("row2"));
            assert_eq!(view.input.as_ref(ctx).buffer_text(ctx), "unchanged");
        });
    });
}
