use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::services::{FileStore, WorkspaceService};
use crate::utils::auth::get_user_id_from_request;
use crate::utils::AppError;

const LEDGER_EXT: &str = ".ledger";

pub fn ledger_files_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ledger-files")
            .route("", web::get().to(list_files))
            .route("/content", web::get().to(read_file))
            .route("/content", web::put().to(write_file))
            .route("/validate", web::post().to(validate_content)),
    );
}

#[derive(Debug, Serialize)]
struct LedgerFileEntry {
    path: String,
    label: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    bytes: u64,
}

#[derive(Debug, Deserialize)]
struct PathQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct WriteRequest {
    content: String,
}

#[derive(Debug, Serialize)]
struct ReadResponse {
    path: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ValidateResponse {
    ok: bool,
    output: String,
}

/// Resolve a user-supplied relative path to an on-disk ledger file the user may edit.
///
/// Only two shapes are permitted:
/// - `users/user-{uuid}-*.ledger` where `{uuid}` is the caller's own user id
/// - `workspaces/workspace-{uuid}/*.ledger` where `{uuid}` is a workspace the caller owns
///
/// Rejects absolute paths, traversal components (`..`, `.`), and non-`.ledger` files.
fn resolve_writable_path(
    rel_path: &str,
    user_id: &Uuid,
    file_store: &FileStore,
    ws_service: &WorkspaceService,
) -> Result<PathBuf, AppError> {
    let path = Path::new(rel_path);
    if path.is_absolute() {
        return Err(AppError::BadRequest("Path must be relative".into()));
    }

    let components: Vec<&str> = path
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_str().ok_or(()),
            _ => Err(()),
        })
        .collect::<Result<_, _>>()
        .map_err(|_| AppError::BadRequest("Invalid path".into()))?;

    let Some(file_name) = components.last() else {
        return Err(AppError::BadRequest("Invalid path".into()));
    };
    if !file_name.ends_with(LEDGER_EXT) {
        return Err(AppError::BadRequest("Only .ledger files are editable".into()));
    }

    let data_root = Path::new(file_store.data_path());

    let resolved = match components.as_slice() {
        ["users", name] => {
            let expected_prefix = format!("user-{}-", user_id);
            if !name.starts_with(&expected_prefix) {
                return Err(AppError::Forbidden(
                    "You can only edit your own ledger files".into(),
                ));
            }
            data_root.join("users").join(name)
        }
        ["workspaces", ws_dir, name] => {
            let ws_id = ws_dir
                .strip_prefix("workspace-")
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or_else(|| AppError::BadRequest("Invalid workspace directory".into()))?;
            let workspace = ws_service
                .get_workspace(&ws_id)?
                .ok_or_else(|| AppError::NotFound("Workspace not found".into()))?;
            if workspace.owner_id != *user_id {
                return Err(AppError::Forbidden(
                    "Only the workspace owner can edit its ledger files".into(),
                ));
            }
            data_root.join("workspaces").join(ws_dir).join(name)
        }
        _ => return Err(AppError::BadRequest("Invalid path".into())),
    };

    // Defense in depth: ensure the resolved path is inside data_root even after symlink
    // resolution. Parent directory must exist (we never create ledger files here).
    if let (Ok(canon_root), Ok(canon_parent)) = (data_root.canonicalize(), resolved.parent().unwrap_or(data_root).canonicalize()) {
        if !canon_parent.starts_with(&canon_root) {
            return Err(AppError::BadRequest("Invalid path".into()));
        }
    }

    Ok(resolved)
}

async fn list_files(
    req: HttpRequest,
    file_store: web::Data<FileStore>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".into()))?;

    let data_root = Path::new(file_store.data_path());
    let mut entries: Vec<LedgerFileEntry> = Vec::new();

    let user_prefix = format!("user-{}-", user_id);
    collect_ledger_files(&data_root.join("users"), |name| {
        name.starts_with(&user_prefix)
    })
    .into_iter()
    .for_each(|(name, bytes)| {
        entries.push(LedgerFileEntry {
            path: format!("users/{}", name),
            label: name,
            workspace_id: None,
            workspace_name: None,
            bytes,
        });
    });

    for workspace in ws_service.list_workspaces(&user_id)? {
        if workspace.owner_id != user_id {
            continue;
        }
        let ws_dir_name = format!("workspace-{}", workspace.id);
        let ws_dir = data_root.join("workspaces").join(&ws_dir_name);
        for (name, bytes) in collect_ledger_files(&ws_dir, |_| true) {
            entries.push(LedgerFileEntry {
                path: format!("workspaces/{}/{}", ws_dir_name, name),
                label: name,
                workspace_id: Some(workspace.id),
                workspace_name: Some(workspace.name.clone()),
                bytes,
            });
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(HttpResponse::Ok().json(entries))
}

/// List `.ledger` files in `dir` whose names pass `filter`. Returns `(name, bytes)` tuples.
/// Missing directories yield an empty list — the caller may not have created them yet.
fn collect_ledger_files(dir: &Path, filter: impl Fn(&str) -> bool) -> Vec<(String, u64)> {
    let Ok(iter) = fs::read_dir(dir) else { return Vec::new() };
    iter.flatten()
        .filter_map(|de| {
            let name = de.file_name().to_string_lossy().into_owned();
            if name.ends_with(LEDGER_EXT) && filter(&name) {
                let bytes = de.metadata().map(|m| m.len()).unwrap_or(0);
                Some((name, bytes))
            } else {
                None
            }
        })
        .collect()
}

async fn read_file(
    req: HttpRequest,
    query: web::Query<PathQuery>,
    file_store: web::Data<FileStore>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".into()))?;

    let resolved = resolve_writable_path(&query.path, &user_id, &file_store, &ws_service)?;
    let content = file_store
        .read_ledger_file(&resolved)?
        .ok_or_else(|| AppError::NotFound("File not found".into()))?;

    Ok(HttpResponse::Ok().json(ReadResponse {
        path: query.path.clone(),
        content,
    }))
}

async fn write_file(
    req: HttpRequest,
    query: web::Query<PathQuery>,
    body: web::Json<WriteRequest>,
    file_store: web::Data<FileStore>,
    ws_service: web::Data<WorkspaceService>,
) -> Result<HttpResponse, AppError> {
    let user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".into()))?;

    let resolved = resolve_writable_path(&query.path, &user_id, &file_store, &ws_service)?;
    if !resolved.exists() {
        return Err(AppError::NotFound("File not found".into()));
    }

    file_store.write_ledger_file(&resolved, &body.content)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "path": query.path,
        "bytes": body.content.len(),
    })))
}

async fn validate_content(
    req: HttpRequest,
    body: web::Json<WriteRequest>,
) -> Result<HttpResponse, AppError> {
    let _user_id = get_user_id_from_request(&req)
        .map_err(|_| AppError::Unauthorized("Not authenticated".into()))?;

    let mut tmp = NamedTempFile::with_suffix(".ledger")
        .map_err(|e| AppError::Internal(format!("Failed to create temp file: {}", e)))?;
    tmp.write_all(body.content.as_bytes())
        .map_err(|e| AppError::Internal(format!("Failed to write temp file: {}", e)))?;
    tmp.flush()
        .map_err(|e| AppError::Internal(format!("Failed to flush temp file: {}", e)))?;

    let tmp_path_str = tmp.path().to_string_lossy().into_owned();

    let output = Command::new("ledger")
        .arg("-f")
        .arg(tmp.path())
        .arg("balance")
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run ledger: {}", e)))?;

    // NamedTempFile's Drop handles cleanup even if we panic below.
    let ok = output.status.success();
    let stream = if ok { &output.stdout } else { &output.stderr };
    let cleaned = String::from_utf8_lossy(stream).replace(&tmp_path_str, "<buffer>");

    Ok(HttpResponse::Ok().json(ValidateResponse {
        ok,
        output: cleaned,
    }))
}
