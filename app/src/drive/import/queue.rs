use std::collections::HashMap;

use warpui::{Entity, ModelContext, SingletonEntity};

use super::nodes::{self, FileId};
use crate::cloud_object::{CloudObjectEventEntrypoint, Owner};
use crate::drive::folders::FolderId;
use crate::notebooks::CloudNotebookModel;
use crate::server::cloud_objects::update_manager::{InitiatedBy, UpdateManager};
use crate::server::ids::{ClientId, SyncId};
use crate::workflows::workflow::Workflow;
use crate::workflows::workflow_enum::WorkflowEnum;

pub(super) enum ImportQueueEvent {
    FileSavedLocally(FileId),
}

#[derive(Debug)]
pub(super) enum ParentId {
    FolderToUpload(ClientId),
    InitialFolder(Option<SyncId>),
}

#[derive(Debug)]
pub(super) struct ImportQueueArgs {
    pub(super) owner: Owner,
    pub(super) parent_id: ParentId,
    pub(super) content: RequestContent,
}

#[derive(Debug)]
pub(super) enum RequestContent {
    Folder {
        name: String,
        client_id: ClientId,
        folder_id: nodes::FolderId,
    },
    Notebook {
        title: String,
        data: String,
        client_id: ClientId,
        file_id: FileId,
    },
    Workflow {
        workflows: Vec<(Workflow, ClientId)>,
        workflow_enums: HashMap<ClientId, WorkflowEnum>,
        file_id: FileId,
    },
}

#[derive(Default)]
struct FileCompletionCounter {
    client_id_to_file_id: HashMap<ClientId, FileId>,
    file_id_to_counter: HashMap<FileId, usize>,
}

impl FileCompletionCounter {
    fn add_entry(&mut self, client_id: ClientId, file_id: FileId) {
        self.client_id_to_file_id.insert(client_id, file_id);
        *self.file_id_to_counter.entry(file_id).or_insert(0) += 1;
    }
}

pub(super) struct ImportQueue {
    queue: Vec<ImportQueueArgs>,
    client_to_server_id: HashMap<ClientId, Option<FolderId>>,
    client_to_node_folder_id: HashMap<ClientId, nodes::FolderId>,
    file_completion: FileCompletionCounter,
}

impl ImportQueue {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let _update_manager = UpdateManager::handle(ctx);
        Self {
            queue: Vec::new(),
            client_to_server_id: HashMap::default(),
            file_completion: Default::default(),
            client_to_node_folder_id: HashMap::default(),
        }
    }

    // Whether all dependencies of an item has been sync-ed.
    fn dependency_synced(&self, item: &ImportQueueArgs) -> bool {
        match &item.parent_id {
            ParentId::FolderToUpload(id) => self
                .client_to_server_id
                .get(id)
                .map(|item| item.is_some())
                .unwrap_or(false),
            ParentId::InitialFolder(_) => true,
        }
    }

    // Enqueue a new request to the import queue.
    pub fn enqueue(&mut self, arg: ImportQueueArgs, ctx: &mut ModelContext<Self>) {
        // Update internal tracker of the object.
        match &arg.content {
            RequestContent::Folder {
                client_id,
                folder_id,
                ..
            } => {
                self.client_to_server_id.insert(*client_id, None);
                self.client_to_node_folder_id.insert(*client_id, *folder_id);
            }
            RequestContent::Notebook {
                client_id, file_id, ..
            } => self.file_completion.add_entry(*client_id, *file_id),
            RequestContent::Workflow {
                workflows, file_id, ..
            } => {
                for (_, client_id) in workflows {
                    self.file_completion.add_entry(*client_id, *file_id);
                }
            }
        }

        self.queue.push(arg);
        self.dequeue(ctx);
    }

    // Dequeue a new request from the import queue.
    pub fn dequeue(&mut self, ctx: &mut ModelContext<Self>) {
        if self.queue.is_empty() {
            return;
        }

        if let Some(idx) = self
            .queue
            .iter()
            .position(|item| self.dependency_synced(item))
        {
            let dequeued_item = self.queue.remove(idx);
            let parent_id = match dequeued_item.parent_id {
                ParentId::FolderToUpload(client_id) => Some(SyncId::ServerId(
                    self.client_to_server_id
                        .get(&client_id)
                        .expect("Client id entry should exist")
                        .expect("Server id entry should exist")
                        .into(),
                )),
                ParentId::InitialFolder(folder_id) => folder_id,
            };

            match dequeued_item.content {
                RequestContent::Folder {
                    name, client_id, ..
                } => {
                    UpdateManager::handle(ctx).update(ctx, |update_manager, ctx| {
                        update_manager.create_folder(
                            name,
                            dequeued_item.owner,
                            client_id,
                            parent_id,
                            false,
                            InitiatedBy::User,
                            ctx,
                        );
                    });
                }
                RequestContent::Notebook {
                    title,
                    data,
                    client_id,
                    file_id,
                } => {
                    UpdateManager::handle(ctx).update(ctx, |update_manager, ctx| {
                        update_manager.create_notebook(
                            client_id,
                            dequeued_item.owner,
                            parent_id,
                            CloudNotebookModel {
                                title,
                                data,
                                ai_document_id: None,
                                conversation_id: None,
                            },
                            CloudObjectEventEntrypoint::ImportModal,
                            false,
                            ctx,
                        );
                    });
                    ctx.emit(ImportQueueEvent::FileSavedLocally(file_id));
                }
                RequestContent::Workflow {
                    workflows,
                    workflow_enums,
                    file_id,
                } => {
                    UpdateManager::handle(ctx).update(ctx, |update_manager, ctx| {
                        // Create any new workflow enums
                        for (client_id, workflow_enum) in workflow_enums {
                            update_manager.create_workflow_enum(
                                workflow_enum,
                                dequeued_item.owner,
                                client_id,
                                CloudObjectEventEntrypoint::ImportModal,
                                false,
                                ctx,
                            );
                        }

                        // Create the workflow
                        for (workflow, client_id) in workflows {
                            update_manager.create_workflow(
                                workflow,
                                dequeued_item.owner,
                                parent_id,
                                client_id,
                                CloudObjectEventEntrypoint::ImportModal,
                                false,
                                ctx,
                            );
                        }
                    });
                    ctx.emit(ImportQueueEvent::FileSavedLocally(file_id));
                }
            }
            self.dequeue(ctx);
        }
    }
}

impl Entity for ImportQueue {
    type Event = ImportQueueEvent;
}
