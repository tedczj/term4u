use std::collections::HashMap;
use std::fmt;

use channel_versions::Changelog;
use markdown_parser::FormattedText;
use warpui::assets::asset_cache::AssetSource;
use warpui::{Entity, ModelContext, SingletonEntity};

pub struct ChangelogModel {
    pub changelog: ChangelogState,
    pub parsed_changelog: HashMap<String, FormattedText>,
    pub oz_updates: Vec<FormattedText>,
    pub image: Option<AssetSource>,
}

impl ChangelogModel {
    pub fn new<T>(ignored: T) -> Self {
        let _ = ignored;
        Self {
            changelog: ChangelogState::None,
            parsed_changelog: HashMap::new(),
            oz_updates: Vec::new(),
            image: None,
        }
    }

    pub fn check_for_changelog(
        &mut self,
        request_type: ChangelogRequestType,
        ctx: &mut ModelContext<Self>,
    ) {
        ctx.emit(Event::ChangelogRequestFailed { request_type });
    }

    pub fn is_check_pending(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChangelogHeader {
    NewFeatures,
    Improvements,
    BugFixes,
}

impl fmt::Display for ChangelogHeader {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(match self {
            Self::NewFeatures => "New features",
            Self::Improvements => "Improvements",
            Self::BugFixes => "Bug fixes",
        })
    }
}

#[derive(Debug)]
pub enum Event {
    ChangelogRequestComplete {
        request_type: ChangelogRequestType,
        changelog: Changelog,
    },
    ChangelogRequestFailed {
        request_type: ChangelogRequestType,
    },
    ImageRequestComplete,
}

#[derive(Debug)]
pub enum ChangelogRequestType {
    WindowLaunch,
    UserAction,
}

pub enum ChangelogState {
    None,
    Pending,
    Some(Changelog),
}

impl Entity for ChangelogModel {
    type Event = Event;
}

impl SingletonEntity for ChangelogModel {}
