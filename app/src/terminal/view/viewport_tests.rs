use std::cell::Cell;

use instant::Instant;
use pathfinder_geometry::vector::vec2f;
use warpui::App;

use super::*;
use crate::terminal::model::test_utils::TestBlockListBuilder;
use crate::test_util::terminal::{
    add_window_with_id_and_terminal, initialize_app_for_terminal_view,
};

thread_local! {
    pub(super) static WORK: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

#[test]
fn l0_08_transcript_construction_work() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        for count in [100, 1000, 10000] {
            let restored = (0..count)
                .map(|i| {
                    SerializedBlock::new_for_test(
                        format!("echo {i}").into_bytes(),
                        ("row\r\n".repeat(19) + "row").into_bytes(),
                    )
                    .into()
                })
                .collect::<Vec<_>>();
            terminal.update(&mut app, |view, _| {
                *view.model.lock().block_list_mut() = TestBlockListBuilder::new()
                    .with_block_sizes(crate::terminal::model::block::BlockSize {
                        size: SizeInfo::new_without_font_metrics(30, 80),
                        ..crate::terminal::model::test_utils::block_size()
                    })
                    .with_honor_ps1(true)
                    .with_restored_blocks(&restored)
                    .build();
                assert_eq!(
                    view.model.lock().block_list().blocks()[0]
                        .output_grid()
                        .len_displayed(),
                    20
                );
            });
            terminal.update(&mut app, |view, _| {
                view.size_info = SizeInfo::new(
                    vec2f(800., 600.),
                    10_f32.into_pixels(),
                    20_f32.into_pixels(),
                    0_f32.into_pixels(),
                    0_f32.into_pixels(),
                );
                view.model
                    .lock()
                    .block_list_mut()
                    .set_next_gap_height_in_lines(30.into_lines());
            });
            for clears in 0..3 {
                if clears > 0 {
                    terminal.update(&mut app, |view, ctx| {
                        view.model.lock().clear_visible_screen();
                        view.handle_model_event(&ModelEvent::TerminalClear, ctx);
                        assert_eq!(view.model.lock().block_list().blocks().len(), count + 1);
                    });
                }
                for fraction in [0., 0.5, 1.] {
                    terminal.read(&app, |view, _| {
                        let height = view
                            .model
                            .lock()
                            .block_list()
                            .block_heights()
                            .summary()
                            .height
                            .as_f64();
                        view.transcript_scroll
                            .scroll_to((fraction * (height as f32 * 20. - 600.)).into_pixels());
                    });
                    for trial in 0..3 {
                        WORK.with(|work| work.set((0, 0)));
                        let start = Instant::now();
                        terminal.read(&app, |view, ctx| drop(view.render_blocks(ctx)));
                        let elapsed = start.elapsed().as_micros();
                        let (scanned, grids) = WORK.with(Cell::get);
                        eprintln!(
                            "L0_RENDER_WORK count={count} clears={clears} fraction={fraction} trial={trial} scanned={scanned} grids={grids} microseconds={elapsed}"
                        );
                        assert!(
                            scanned <= 8,
                            "only viewport plus overscan should be visited"
                        );
                        assert!(grids <= 12, "only visible grids should be constructed");
                        if clears == 0 {
                            assert!(grids > 0);
                        }
                    }
                }
            }
        }
    });
}

#[test]
fn l0_08_hidden_history_updates_height_and_does_not_expand_viewport_work() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let restored = (0..1000)
            .map(|_| SerializedBlock::new_for_test(b"echo row".to_vec(), b"row".to_vec()).into())
            .collect::<Vec<_>>();
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&restored));
        terminal.update(&mut app, |view, _| {
            let mut model = view.model.lock();
            let list = model.block_list_mut();
            let original = list.block_heights().summary().height;
            let ids = list
                .blocks()
                .iter()
                .take(999)
                .map(|block| block.id().clone())
                .collect::<Vec<_>>();
            for id in &ids {
                assert_eq!(list.toggle_visibility_of_block(id), Some(false));
            }
            let hidden = list.block_heights().summary().height;
            assert!(hidden < original);
            list.unhide_block(&ids[0]);
            assert!(list.block_heights().summary().height > hidden);
            assert_eq!(list.toggle_visibility_of_block(&ids[0]), Some(false));
            assert_eq!(list.block_heights().summary().height, hidden);
            view.transcript_scroll.scroll_to(0_f32.into_pixels());
        });
        WORK.with(|work| work.set((0, 0)));
        terminal.read(&app, |view, ctx| drop(view.render_blocks(ctx)));
        let (scanned, grids) = WORK.with(Cell::get);
        assert!(
            scanned <= 3,
            "hidden prefix must be skipped by the height index"
        );
        assert!(grids > 0 && grids <= 2);
    });
}

#[test]
fn l0_08_find_uses_the_same_padding_and_gap_coordinates_as_selection() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let restored = (0..8)
            .map(|_| {
                SerializedBlock::new_for_test(
                    b"print lines".to_vec(),
                    b"one\r\ntwo\r\nthree".to_vec(),
                )
                .into()
            })
            .collect::<Vec<_>>();
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, Some(&restored));
        terminal.update(&mut app, |view, ctx| {
            view.handle_wakeup(None, ctx);
            let point = WithinBlock::new(Point { row: 1, col: 0 }, BlockIndex(6), GridType::Output);
            let canonical =
                BlockListPoint::from_within_block_point(&point, view.model.lock().block_list()).row;
            let matched = BlockGridMatch {
                is_filtered: false,
                block_index: point.block_index,
                grid_type: point.grid,
                range: point.inner..=point.inner,
            };
            view.handle_wakeup(Some(matched), ctx);
            let expected = (canonical.as_f64() as f32 * view.size_info.cell_height_px().as_f32()
                - view.size_info.pane_height_px / 2.)
                .max(0.);
            assert!((view.transcript_scroll.scroll_start().as_f32() - expected).abs() < 0.01);
        });
    });
}

fn configure_reading_fixture(view: &mut TerminalView, ctx: &mut ViewContext<TerminalView>) {
    let output = (0..50)
        .map(|i| format!("ROW-{i:03} {}\r\n", "x".repeat(90)))
        .collect::<String>();
    let restored = (0..3)
        .map(|i| {
            SerializedBlock::new_for_test(
                format!("history-{i}").into_bytes(),
                output.clone().into_bytes(),
            )
            .into()
        })
        .collect::<Vec<_>>();
    let size = SizeInfo::new(
        vec2f(800., 600.),
        10_f32.into_pixels(),
        20_f32.into_pixels(),
        0_f32.into_pixels(),
        0_f32.into_pixels(),
    );
    *view.model.lock().block_list_mut() = TestBlockListBuilder::new()
        .with_block_sizes(crate::terminal::model::block::BlockSize {
            size,
            ..crate::terminal::model::test_utils::block_size()
        })
        .with_honor_ps1(true)
        .with_restored_blocks(&restored)
        .build();
    view.size_info = size;
    view.handle_wakeup(None, ctx);
    let target = reading_target(view);
    view.transcript_scroll
        .scroll_to((target + 5.).into_pixels());
    drop(view.render_blocks(ctx));
}

fn reading_target(view: &TerminalView) -> f32 {
    let model = view.model.lock();
    let list = model.block_list();
    let block = &list.blocks()[1];
    let grid = block.output_grid();
    let row = (0..grid.len_displayed())
        .find(|&row| {
            grid.grid_handler().row(row).is_some_and(|row| {
                row[..]
                    .iter()
                    .map(|cell| cell.c.to_string())
                    .collect::<String>()
                    .starts_with("ROW-010")
            })
        })
        .expect("target line must survive layout changes");
    BlockListPoint::from_within_block_point(
        &WithinBlock::new(Point { row, col: 0 }, BlockIndex(1), GridType::Output),
        list,
    )
    .row
    .as_f64() as f32
        * view.size_info.cell_height_px().as_f32()
}

#[test]
fn l0_08_reading_anchor_survives_height_change_above_viewport() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, ctx| {
            configure_reading_fixture(view, ctx);
            let id = view.model.lock().block_list().blocks()[0].id().clone();
            view.model
                .lock()
                .block_list_mut()
                .toggle_visibility_of_block(&id);
            view.handle_wakeup(None, ctx);
            assert!(
                (view.transcript_scroll.scroll_start().as_f32() - reading_target(view) - 5.).abs()
                    < 0.1,
                "hiding an earlier block must preserve the same reading row and fractional offset"
            );
            view.model.lock().block_list_mut().unhide_block(&id);
            view.handle_wakeup(None, ctx);
            assert!(
                (view.transcript_scroll.scroll_start().as_f32() - reading_target(view) - 5.).abs()
                    < 0.1
            );
        });
    });
}

#[test]
fn l0_08_reading_anchor_survives_soft_wrap_resize() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let (_, terminal) = add_window_with_id_and_terminal(&mut app, None);
        terminal.update(&mut app, |view, ctx| {
            configure_reading_fixture(view, ctx);
            view.after_layout(vec2f(400., 600.), ctx);
            assert!(
                (view.transcript_scroll.scroll_start().as_f32()
                    - reading_target(view)
                    - view.size_info.cell_height_px().as_f32() * 0.25)
                    .abs()
                    < 0.1,
                "resize must follow the same text through soft wrapping"
            );
            drop(view.render_blocks(ctx));
            view.after_layout(vec2f(800., 600.), ctx);
            assert!(
                (view.transcript_scroll.scroll_start().as_f32()
                    - reading_target(view)
                    - view.size_info.cell_height_px().as_f32() * 0.25)
                    .abs()
                    < 0.1
            );
        });
    });
}
