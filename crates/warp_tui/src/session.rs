use std::path::PathBuf;

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use warpui::SingletonEntity as _;
use warpui_core::platform::{TerminationMode, WindowStyle};
use warpui_core::runtime::{TuiDriverStartupError, TuiFocusPolicy, spawn_tui_driver};
use warpui_core::{AddWindowOptions, AppContext};

use crate::root_view::RootTuiView;
use crate::session_registry::TuiSessions;
use crate::terminal_background::probe_and_select_theme;

const CLI_VERSION: &str = match option_env!("GIT_RELEASE_TAG") {
    Some(version) => version,
    None => "v0.0.0.0.0.0",
};

#[derive(Debug, Parser)]
#[command(name = "term4u-tui", version = CLI_VERSION)]
struct TuiArgs {
    #[command(subcommand)]
    command: Option<TuiCommand>,
}

#[derive(Debug, Subcommand)]
enum TuiCommand {
    /// Print the JSON schema for the current Term4u channel's settings and exit.
    DumpSettingsSchema { output_path: Option<PathBuf> },
}

pub fn run() -> Result<()> {
    if let Some(result) = warp::run_tui_worker_if_requested() {
        return result;
    }
    let args = match TuiArgs::try_parse() {
        Ok(args) => args,
        Err(error) if error.kind() == ErrorKind::DisplayVersion => {
            println!("{CLI_VERSION}");
            return Ok(());
        }
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(anyhow::Error::new(error)),
    };
    if let Some(TuiCommand::DumpSettingsSchema { output_path }) = args.command {
        warp::features::init_feature_flags();
        return warp::settings::dump_settings_schema(output_path.as_deref());
    }

    warp::run_tui(Box::new(init))
}

fn init(ctx: &mut AppContext) {
    crate::keybindings::init(ctx);
    let selected_theme = warp::settings::TuiThemeSettings::as_ref(ctx).selected_theme();
    let theme = probe_and_select_theme(selected_theme);
    warp::tui_export::Appearance::handle(ctx).update(ctx, |appearance, ctx| {
        appearance.set_theme(theme, ctx);
    });

    let (window_id, root) = ctx.add_tui_window(
        AddWindowOptions {
            window_style: WindowStyle::NotStealFocus,
            ..Default::default()
        },
        RootTuiView::new,
    );
    match spawn_tui_driver(
        ctx,
        window_id,
        root.clone(),
        TuiFocusPolicy::PresentedTree,
        false,
        false,
    ) {
        Ok(driver) => {
            let sessions = ctx.add_singleton_model(|_| TuiSessions::new(driver));
            root.update(ctx, |_, ctx| {
                ctx.subscribe_to_model(&sessions, |_, _, _, ctx| ctx.notify());
            });
            TuiSessions::create_local_terminal_session(
                &sessions,
                window_id,
                true,
                std::env::current_dir().ok(),
                ctx,
            );
            root.update(ctx, |root, ctx| root.activate(ctx));
        }
        Err(error) => handle_tui_driver_startup_error(error, ctx),
    }
}

fn handle_tui_driver_startup_error(error: TuiDriverStartupError, ctx: &mut AppContext) {
    match error {
        TuiDriverStartupError::TerminalDisconnected(error) => {
            log::error!("failed to start the TUI driver: {error}");
            ctx.terminate_app(TerminationMode::ForceTerminate, None);
        }
        TuiDriverStartupError::Unexpected(error) => {
            ctx.terminate_app(
                TerminationMode::ForceTerminate,
                Some(Err(anyhow::Error::new(error))),
            );
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
