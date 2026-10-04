//! Native authority primitives for the Forme desktop shell.

pub mod commands;
pub mod identity;
pub mod preview;
mod security;
pub mod storage;
pub mod target;
pub mod worker;

#[cfg(feature = "desktop")]
pub fn run() {
    use commands::{
        CommandError, NativeCommands, NativePreviewCommands, PreviewBuildRequest,
        PreviewBuildResponse, ProjectCompareAndSwapRequest, ProjectCompareAndSwapResponse,
        ProjectLoadResponse, TargetPublishRequest, TargetPublishResponse,
    };
    use preview::PreviewServer;
    use std::{fs, io};
    use target::{NativeTargets, TargetReview};
    use tauri::{Manager, State};
    use worker::NativeProductWorker;

    #[tauri::command]
    fn project_load(
        commands: State<'_, NativeCommands>,
    ) -> Result<Option<ProjectLoadResponse>, CommandError> {
        commands.project_load()
    }

    #[tauri::command]
    fn project_compare_and_swap(
        commands: State<'_, NativeCommands>,
        request: ProjectCompareAndSwapRequest,
    ) -> Result<ProjectCompareAndSwapResponse, CommandError> {
        commands.project_compare_and_swap(request)
    }

    #[tauri::command]
    fn identity_create(commands: State<'_, NativeCommands>) -> Result<String, CommandError> {
        commands.identity_create()
    }

    #[tauri::command]
    fn preview_build(
        commands: State<'_, NativePreviewCommands>,
        storage: State<'_, NativeCommands>,
        request: PreviewBuildRequest,
    ) -> Result<PreviewBuildResponse, CommandError> {
        let revision = request.revision.clone();
        storage.with_current_project(&revision, |project| {
            commands.preview_build(project, request)
        })
    }

    #[tauri::command]
    fn target_configure(
        commands: State<'_, NativePreviewCommands>,
    ) -> Result<TargetReview, CommandError> {
        let path = rfd::FileDialog::new()
            .set_title("Choose an empty Forme publication folder")
            .pick_folder()
            .ok_or_else(|| CommandError::new("TARGET_CANCELLED"))?;
        commands.target_configure_path(path)
    }

    #[tauri::command]
    fn target_list(
        commands: State<'_, NativePreviewCommands>,
    ) -> Result<Vec<TargetReview>, CommandError> {
        commands.target_list()
    }

    #[tauri::command]
    fn target_publish(
        commands: State<'_, NativePreviewCommands>,
        storage: State<'_, NativeCommands>,
        request: TargetPublishRequest,
    ) -> Result<TargetPublishResponse, CommandError> {
        let revision = request.revision.clone();
        storage.with_current_project(&revision, |project| {
            commands.target_publish(project, request)
        })
    }

    #[tauri::command]
    fn workspace_dispose(commands: State<'_, NativePreviewCommands>) -> Result<(), CommandError> {
        commands.workspace_dispose()
    }

    let preview_server =
        PreviewServer::bind().expect("the Forme preview server could not bind to random loopback");
    let preview_url = serde_json::to_string(&preview_server.url())
        .expect("the loopback preview URL should serialize");
    let initialization_script = format!(
        "Object.defineProperty(globalThis,'__FORME_PREVIEW_URL__',{{value:{preview_url},writable:false,configurable:false}});"
    );

    tauri::Builder::default()
        .append_invoke_initialization_script(initialization_script)
        .setup(move |application| {
            let data_root = application.path().app_local_data_dir()?;
            fs::create_dir_all(&data_root)?;
            let commands = NativeCommands::open(data_root.join("authoring-v1")).map_err(|_| {
                io::Error::other("the native authoring profile could not be opened safely")
            })?;
            let lifecycle = commands.lifecycle();
            application.manage(commands);
            let suffix = if cfg!(windows) { ".exe" } else { "" };
            let worker_path = application
                .path()
                .resource_dir()?
                .join(format!("forme-product-worker{suffix}"));
            let sandbox_launcher_path = application
                .path()
                .resource_dir()?
                .join("forme-sandbox-macos");
            let workspace_root = data_root.join("product-workspaces-v1");
            let worker = NativeProductWorker::open(
                worker_path,
                env!("FORME_PRODUCT_WORKER_SHA256").to_owned(),
                sandbox_launcher_path,
                env!("FORME_SANDBOX_LAUNCHER_SHA256").to_owned(),
                workspace_root.clone(),
            )
            .map_err(|_| io::Error::other("the bundled product worker failed identity checks"))?;
            let targets = NativeTargets::new(vec![data_root, workspace_root])
                .map_err(|_| io::Error::other("the target exclusions could not be established"))?;
            let preview = NativePreviewCommands::open_with_server_targets_and_lifecycle(
                worker,
                preview_server,
                targets,
                lifecycle,
            );
            application.manage(preview);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            project_load,
            project_compare_and_swap,
            identity_create,
            preview_build,
            target_configure,
            target_list,
            target_publish,
            workspace_dispose
        ])
        .run(tauri::generate_context!())
        .expect("the Forme desktop runtime could not start");
}
