use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use crate::api::sync::{SyncApi, SyncRequest as ApiSyncRequest};

pub struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel::<SyncResult>();
        
        app
            .init_resource::<SyncState>()
            .insert_resource(SyncChannelSender(Mutex::new(tx)))
            .insert_resource(SyncChannelReceiver(Mutex::new(rx)))
            .add_event::<SyncRequestEvent>()
            .add_event::<SyncStartedEvent>()
            .add_event::<SyncProgressEvent>()
            .add_event::<SyncCompletedEvent>()
            .add_event::<SyncErrorEvent>()
            .add_systems(Update, (
                handle_sync_requests,
                process_sync_results,
            ));
    }
}

#[derive(Resource, Default)]
pub struct SyncState {
    pub is_syncing: bool,
    pub current_sync_id: Option<Uuid>,
    pub progress: Option<SyncProgress>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SyncProgress {
    pub total_files: usize,
    pub processed_files: usize,
    pub tasks_created: usize,
    pub tasks_skipped: usize,
}

#[derive(Resource)]
pub struct SyncChannelSender(Mutex<Sender<SyncResult>>);

#[derive(Resource)]
pub struct SyncChannelReceiver(Mutex<Receiver<SyncResult>>);

#[allow(dead_code)]
enum SyncResult {
    Started { sync_id: Uuid },
    Progress { sync_id: Uuid, progress: SyncProgress },
    Completed { response: SyncResponse },
    Error { error: String },
}

#[derive(Event)]
pub struct SyncRequestEvent {
    pub project_id: Uuid,
    pub request: SyncRequest,
    pub token: String,
}

#[derive(Event)]
#[allow(dead_code)]
pub struct SyncStartedEvent {
    pub sync_id: Uuid,
}

#[derive(Event)]
#[allow(dead_code)]
pub struct SyncProgressEvent {
    pub sync_id: Uuid,
    pub progress: SyncProgress,
}

#[derive(Event)]
#[allow(dead_code)]
pub struct SyncCompletedEvent {
    pub sync_id: Uuid,
    pub response: SyncResponse,
}

#[derive(Event)]
pub struct SyncErrorEvent {
    pub error: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SyncRequest {
    pub prefix: Option<String>,
    pub file_extensions: Option<Vec<String>>,
    pub overwrite_existing: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SyncResponse {
    pub sync_id: Uuid,
    pub total_files: usize,
    pub tasks_created: usize,
    pub tasks_skipped: usize,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}


fn handle_sync_requests(
    mut sync_requests: EventReader<SyncRequestEvent>,
    sender: Res<SyncChannelSender>,
    mut sync_state: ResMut<SyncState>,
) {
    for request_event in sync_requests.read() {
        if sync_state.is_syncing {
            if let Ok(tx) = sender.0.lock() {
                let _ = tx.send(SyncResult::Error {
                    error: "Sync already in progress".to_string(),
                });
            }
            continue;
        }
        
        sync_state.is_syncing = true;
        
        let project_id = request_event.project_id;
        let request = request_event.request.clone();
        let token = request_event.token.clone();
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        if let Ok(tx) = sender.0.lock() {
            let tx = tx.clone();
            
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    let result = execute_sync(project_id, request, token, api_base_url).await;
                    match result {
                        Ok(response) => {
                            let _ = tx.send(SyncResult::Completed { response });
                        }
                        Err(error) => {
                            let _ = tx.send(SyncResult::Error { error });
                        }
                    }
                });
            });
        }
    }
}

async fn execute_sync(
    project_id: Uuid,
    request: SyncRequest,
    token: String,
    _api_base_url: String,
) -> Result<SyncResponse, String> {
    let sync_api = SyncApi::new();
    
    // Convert from local SyncRequest to API SyncRequest
    let api_request = ApiSyncRequest {
        prefix: request.prefix,
        file_extensions: request.file_extensions,
        overwrite_existing: request.overwrite_existing,
    };
    
    let api_response = sync_api.start_sync(&token, project_id, &api_request).await
        .map_err(|e| e.to_string())?;
    
    // Convert from API SyncResponse to local SyncResponse
    let local_response = SyncResponse {
        sync_id: api_response.sync_id,
        total_files: api_response.total_files,
        tasks_created: api_response.tasks_created,
        tasks_skipped: api_response.tasks_skipped,
        errors: api_response.errors,
        started_at: api_response.started_at,
        completed_at: api_response.completed_at,
    };
    
    Ok(local_response)
}


fn process_sync_results(
    receiver: Res<SyncChannelReceiver>,
    mut sync_state: ResMut<SyncState>,
    mut started_events: EventWriter<SyncStartedEvent>,
    mut progress_events: EventWriter<SyncProgressEvent>,
    mut completed_events: EventWriter<SyncCompletedEvent>,
    mut error_events: EventWriter<SyncErrorEvent>,
) {
    if let Ok(rx) = receiver.0.lock() {
        while let Ok(result) = rx.try_recv() {
            match result {
                SyncResult::Started { sync_id } => {
                    sync_state.current_sync_id = Some(sync_id);
                    started_events.write(SyncStartedEvent { sync_id });
                }
                SyncResult::Progress { sync_id, progress } => {
                    sync_state.progress = Some(progress.clone());
                    progress_events.write(SyncProgressEvent { sync_id, progress });
                }
                SyncResult::Completed { response } => {
                    sync_state.is_syncing = false;
                    sync_state.current_sync_id = None;
                    sync_state.progress = None;
                    completed_events.write(SyncCompletedEvent {
                        sync_id: response.sync_id,
                        response,
                    });
                }
                SyncResult::Error { error } => {
                    sync_state.is_syncing = false;
                    sync_state.current_sync_id = None;
                    sync_state.progress = None;
                    error_events.write(SyncErrorEvent { error });
                }
            }
        }
    }
}