use std::ops::Range;

use pathfinder_geometry::vector::vec2f;
use vim::vim::VimMode;
use warp_core::features::FeatureFlag;
use warpui::keymap::Keystroke;
use warpui::platform::WindowStyle;
use warpui::{App, Event, ViewHandle};

use super::initialize_app;
use crate::editor::{DisplayPoint, EditorOptions, EditorView};

#[test]
fn test_set_marked_text() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);

        app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut editor = EditorView::new_with_base_text("", Default::default(), ctx);

            // Simulate typing in "nihao" into the IME and then selecting "你好" as the candidate.
            editor.set_marked_text("nihao", &(5..5), ctx);
            assert_eq!(editor.selected_text(ctx), "nihao");
            editor.ime_commit("你好", ctx);
            assert_eq!(editor.buffer_text(ctx), "你好");

            editor.user_insert(", I am Teddy ", ctx);
            assert_eq!(editor.buffer_text(ctx), "你好, I am Teddy ".to_owned());

            // Simulate typing in "xiong" into the IME and selecting "熊" as the candidate.
            editor.set_marked_text("xiong", &(5..5), ctx);
            assert_eq!(editor.selected_text(ctx), "xiong");
            editor.ime_commit("熊", ctx);
            assert_eq!(editor.buffer_text(ctx), "你好, I am Teddy 熊".to_owned());

            editor
        });
    });
}

#[test]
fn test_set_marked_text_multiple_empty_selections() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);

        app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut editor = EditorView::new_with_base_text(" is ", Default::default(), ctx);

            // Set two cursors: one at the beginning and one at the end.
            editor
                .select_ranges(
                    vec![
                        DisplayPoint::new(0, 0)..DisplayPoint::new(0, 0),
                        DisplayPoint::new(0, 4)..DisplayPoint::new(0, 4),
                    ],
                    ctx,
                )
                .unwrap();
            assert_eq!(editor.selections(ctx).len(), 2);

            // Simulate typing in "pyaar" into the IME and then selecting "प्यार" as the candidate.
            editor.set_marked_text("pyaar", &(5..5), ctx);
            for selected_text in editor.selected_text_strings(ctx).iter() {
                assert_eq!(selected_text, "pyaar");
            }
            editor.ime_commit("प्यार", ctx);
            assert_eq!(editor.buffer_text(ctx), "प्यार is प्यार".to_owned());

            editor
        });
    });
}

#[test]
fn test_set_marked_text_multiple_nonempty_selections() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);

        app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut editor =
                EditorView::new_with_base_text("love is love", Default::default(), ctx);

            // Select both instances of "love" in the buffer text.
            editor
                .select_ranges(
                    vec![
                        DisplayPoint::new(0, 0)..DisplayPoint::new(0, 4),
                        DisplayPoint::new(0, 8)..DisplayPoint::new(0, 12),
                    ],
                    ctx,
                )
                .unwrap();
            assert_eq!(editor.selections(ctx).len(), 2);

            // Simulate typing in "pyaar" into the IME and then selecting "प्यार" as the candidate.
            editor.set_marked_text("pyaar", &(5..5), ctx);
            for selected_text in editor.selected_text_strings(ctx).iter() {
                assert_eq!(selected_text, "pyaar");
            }
            editor.ime_commit("प्यार", ctx);
            assert_eq!(editor.buffer_text(ctx), "प्यार is प्यार".to_owned());

            editor
        });
    });
}

#[test]
fn test_set_marked_text_vim_normal_mode() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);

        app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let editor_options = EditorOptions {
                supports_vim_mode: true,
                ..Default::default()
            };
            let mut editor = EditorView::new_with_base_text(
                "This text should remain unchanged",
                editor_options,
                ctx,
            );

            editor
                .select_ranges(vec![DisplayPoint::new(0, 0)..DisplayPoint::new(0, 0)], ctx)
                .unwrap();

            // Set vim to normal mode.
            editor.vim_keystroke(&Keystroke::parse("escape").unwrap(), ctx);
            assert_eq!(editor.vim_mode(ctx), Some(VimMode::Normal));

            // Simulate typing in "Om Shanti Om" into the IME and then selecting "ॐ शांति ॐ" as the candidate.
            // Since we're in normal mode, we don't expect the text to change at all.
            editor.set_marked_text("om shanti om", &(10..10), ctx);
            assert_eq!(editor.selected_text(ctx), "");
            assert_eq!(
                editor.buffer_text(ctx),
                "This text should remain unchanged".to_owned()
            );
            editor.ime_commit("ॐ शांति ॐ", ctx);
            assert_eq!(
                editor.buffer_text(ctx),
                "This text should remain unchanged".to_owned()
            );

            editor
        });
    });
}

#[test]
fn test_set_marked_text_vim_insert_mode() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);

        app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let editor_options = EditorOptions {
                supports_vim_mode: true,
                ..Default::default()
            };
            let mut editor = EditorView::new_with_base_text(
                " is the best Bollywood movie ever created.",
                editor_options,
                ctx,
            );

            editor
                .select_ranges(vec![DisplayPoint::new(0, 0)..DisplayPoint::new(0, 0)], ctx)
                .unwrap();

            // Set vim to normal mode.
            assert_eq!(editor.vim_mode(ctx), Some(VimMode::Insert));

            // Simulate typing in "Om Shanti Om" into the IME and then selecting "ॐ शांति ॐ" as the candidate.
            // Since we're in insert mode, we don't expect the text to be inserted.
            editor.set_marked_text("om shanti om", &(10..10), ctx);
            assert_eq!(editor.selected_text(ctx), "om shanti om");
            assert_eq!(
                editor.buffer_text(ctx),
                "om shanti om is the best Bollywood movie ever created.".to_owned()
            );
            editor.ime_commit("ॐ शांति ॐ", ctx);
            assert_eq!(
                editor.buffer_text(ctx),
                "ॐ शांति ॐ is the best Bollywood movie ever created.".to_owned()
            );

            editor
        });
    });
}

#[test]
fn l0_07_marked_caret_uses_selected_row_in_soft_wrapped_text() {
    use std::sync::Arc;

    use warpui::text_layout::TextFrame;

    use crate::editor::soft_wrap::{FrameLayouts, SoftWrapPoint};

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);
        let (_, editor) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            EditorView::new_with_base_text("abc ", Default::default(), ctx)
        });
        set_marked_text_from_platform(&editor, "0123456789 0123456789", 2..2, &mut app);
        editor.read(&app, |editor, ctx| {
            // One logical line is laid out over three visual rows. Mock frame newlines
            // split glyph runs without adding buffer characters.
            let frames = FrameLayouts::new(
                vec![Arc::new(TextFrame::mock("abc 012345\n6789 012345\n6789"))],
                0,
                3,
            );
            assert_eq!(frames.num_lines(), 3);
            let selection = editor
                .editor_model
                .as_ref(ctx)
                .all_drawable_selections_intersecting_range(
                    DisplayPoint::new(0, 0)..editor.max_point(ctx),
                    ctx,
                )
                .next()
                .unwrap();
            assert_eq!(
                frames.to_soft_wrap_point(selection.range.end, selection.clamp_direction),
                Some(SoftWrapPoint::new(0, 6)),
            );
            assert_eq!(editor.selected_text(ctx), "0123456789 0123456789");
        });
    });
}

#[test]
fn l0_07_marked_caret_maps_selection_across_newlines_and_emoji() {
    use crate::editor::view::position_id_for_cursor;

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);
        let (actual_window, actual) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            EditorView::new_with_base_text("前缀 ", Default::default(), ctx)
        });
        let (expected_window, expected) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            let mut editor =
                EditorView::new_with_base_text("前缀 中文\n😀x\n尾部", Default::default(), ctx);
            let point = DisplayPoint::new(1, 1);
            editor.select_ranges(vec![point..point], ctx).unwrap();
            editor
        });
        set_marked_text_from_platform(&actual, "中文\n😀x\n尾部", 3..4, &mut app);
        let mut positions = Vec::new();
        for (window, editor) in [(actual_window, actual), (expected_window, expected)] {
            let position = app.update(|ctx| {
                let presenter = ctx.presenter(window).unwrap();
                presenter.borrow_mut().position_cache_mut().start();
                presenter
                    .borrow_mut()
                    .build_scene(vec2f(100., 400.), 1., None, ctx);
                presenter.borrow_mut().position_cache_mut().end();
                presenter
                    .borrow()
                    .position_cache()
                    .get_position(position_id_for_cursor(editor.id()))
                    .expect("rendered editor must publish its caret")
            });
            positions.push(position);
        }
        assert_eq!(positions[0], positions[1]);
    });
}

#[test]
fn l0_07_marked_caret_controls_horizontal_and_vertical_autoscroll() {
    use std::sync::Arc;

    use warpui::text_layout::TextFrame;

    use crate::editor::soft_wrap::FrameLayouts;
    use crate::editor::view::ScrollState;

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let _guard = FeatureFlag::ImeMarkedText.override_enabled(true);
        let (_, editor) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            EditorView::new_with_base_text("abc ", Default::default(), ctx)
        });
        set_marked_text_from_platform(&editor, "0123456789 0123456789", 2..2, &mut app);
        editor.read(&app, |editor, ctx| {
            let snapshot = editor.snapshot(ctx);
            let scroll = ScrollState::from(editor);
            let frames = FrameLayouts::new(
                vec![Arc::new(TextFrame::mock("abc 012345\n6789 012345\n6789"))],
                0,
                3,
            );
            *scroll.scroll_position.lock() = vec2f(0., 2.);
            *scroll.autoscroll_requested.lock() = true;
            assert!(snapshot.autoscroll_vertically(&scroll, 3., 1., 0., &frames, ctx));
            assert_eq!(scroll.scroll_position().y(), 0.);

            let frame = TextFrame::mock_with_positions("abc 0123456789 0123456789", 10.);
            *scroll.scroll_position.lock() = vec2f(10., 0.);
            snapshot.autoscroll_horizontally(
                &scroll,
                0,
                100.,
                250.,
                10.,
                frame.lines().iter().collect(),
                ctx,
            );
            assert_eq!(scroll.scroll_position().x(), 3.);
        });
    });
}

fn set_marked_text_from_platform(
    editor: &ViewHandle<EditorView>,
    text: &str,
    selected_range: Range<usize>,
    app: &mut App,
) {
    editor.update(app, |editor, ctx| {
        editor.move_to_buffer_end(ctx);
        ctx.focus_self();
    });
    let window = app.read(|ctx| editor.window_id(ctx));
    let handled = app.update(|ctx| {
        let presenter = ctx.presenter(window).unwrap();
        presenter.borrow_mut().position_cache_mut().start();
        presenter
            .borrow_mut()
            .build_scene(vec2f(100., 400.), 1., None, ctx);
        presenter.borrow_mut().position_cache_mut().end();
        ctx.simulate_window_event(
            Event::SetMarkedText {
                marked_text: text.to_owned(),
                selected_range,
            },
            window,
            presenter,
        )
    });
    assert!(
        handled,
        "focused editor must accept platform composition events"
    );
    editor.read(app, |editor, ctx| {
        assert_eq!(editor.selected_text(ctx), text)
    });
}
