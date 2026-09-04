pub use cloud_object_models::{
    AmbientAgentEnvironment, BaseImage, CloudAmbientAgentEnvironment,
    CloudAmbientAgentEnvironmentModel, GcpProviderConfig, GithubRepo, ProvidersConfig, SourceRepo,
};

use crate::cloud_object::model::generic_string_model::StringModel;
use crate::cloud_object::model::json_model::JsonModel;
use crate::cloud_object::{
    GenericStringObjectFormat, GenericStringObjectUniqueKey, JsonObjectType, Revision,
};
use crate::server::sync_queue::QueueItem;

impl StringModel for AmbientAgentEnvironment {
    type CloudObjectType = CloudAmbientAgentEnvironment;

    fn model_type_name(&self) -> &'static str {
        "Cloud environment"
    }

    fn should_enforce_revisions() -> bool {
        true
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::CloudEnvironment)
    }

    fn display_name(&self) -> String {
        self.name.clone()
    }

    fn update_object_queue_item(
        &self,
        revision_ts: Option<Revision>,
        object: &CloudAmbientAgentEnvironment,
    ) -> QueueItem {
        QueueItem::UpdateCloudEnvironment {
            model: object.model().clone().into(),
            id: object.id,
            revision: revision_ts.or(object.metadata.revision),
        }
    }

    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }

    fn should_show_activity_toasts() -> bool {
        false
    }

    fn warn_if_unsaved_at_quit() -> bool {
        true
    }
}

impl JsonModel for AmbientAgentEnvironment {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::CloudEnvironment
    }
}

pub(crate) fn sort_environments_by_recency(environments: &mut [CloudAmbientAgentEnvironment]) {
    environments.sort_by(|a, b| {
        b.metadata
            .last_task_run_ts
            .cmp(&a.metadata.last_task_run_ts)
            .then_with(|| {
                a.model()
                    .string_model
                    .name
                    .to_lowercase()
                    .cmp(&b.model().string_model.name.to_lowercase())
            })
    });
}
