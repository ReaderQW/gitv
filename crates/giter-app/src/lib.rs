#![allow(dead_code)]

mod state;

use state::app_settings::AppSettings;
use state::scan_result::{ScanResultStore, ScanState};
use state::selected_repos::SelectedRepos;
use std::sync::Mutex;
use chrono::Datelike;

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Open a native folder picker dialog and return the chosen path.
#[tauri::command]
fn pick_folder() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择一个 Git 仓库")
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
}

/// Return the persisted application settings.
#[tauri::command]
fn get_settings(settings: tauri::State<'_, Mutex<AppSettings>>) -> Result<AppSettings, String> {
    settings.lock().map(|s| s.clone()).map_err(|e| e.to_string())
}

/// Persist new application settings.
#[tauri::command]
fn update_settings(
    new_settings: AppSettings,
    settings: tauri::State<'_, Mutex<AppSettings>>,
) -> Result<(), String> {
    let mut s = settings.lock().map_err(|e| e.to_string())?;
    *s = new_settings;
    s.save()?;
    Ok(())
}

/// Return the list of selected repo paths.
#[tauri::command]
fn get_repos(repos: tauri::State<'_, Mutex<SelectedRepos>>) -> Result<Vec<String>, String> {
    let repos = repos.lock().map_err(|e| e.to_string())?;
    Ok(repos.all().iter().map(|p| p.to_string_lossy().to_string()).collect())
}

/// Add a path to the selected-repos collection.
#[tauri::command]
fn add_repo(path: String, repos: tauri::State<'_, Mutex<SelectedRepos>>) -> Result<(), String> {
    let mut repos = repos.lock().map_err(|e| e.to_string())?;
    repos.add(std::path::PathBuf::from(&path));
    Ok(())
}

/// Remove a path from the selected-repos collection.
#[tauri::command]
fn remove_repo(path: String, repos: tauri::State<'_, Mutex<SelectedRepos>>) -> Result<(), String> {
    let mut repos = repos.lock().map_err(|e| e.to_string())?;
    repos.remove(std::path::Path::new(&path));
    Ok(())
}

/// Return all scan results as (path, state) pairs.
#[tauri::command]
fn get_scan_results(
    store: tauri::State<'_, Mutex<ScanResultStore>>,
) -> Result<Vec<(String, ScanState)>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    Ok(store.all())
}

/// Return contributor commit counts for a scanned repo.
#[tauri::command]
fn get_contributors(
    path: String,
    store: tauri::State<'_, Mutex<ScanResultStore>>,
) -> Result<Vec<(String, usize)>, String> {
    let store = store.lock().map_err(|e| e.to_string())?;
    match store.get(std::path::Path::new(&path)) {
        ScanState::Completed(stats) => {
            Ok(giter_core::analysis::contributors::contributor_commits(&stats.commits))
        }
        _ => Ok(vec![]),
    }
}

/// Return trend data (commits per day/week/month + code churn) for a scanned repo.
#[tauri::command]
fn get_trends(
    path: String,
    store: tauri::State<'_, Mutex<ScanResultStore>>,
) -> Result<
    (
        Vec<(String, usize)>,
        Vec<(String, usize)>,
        Vec<(String, usize)>,
        Vec<(String, i32, i32)>,
    ),
    String,
> {
    use chrono::NaiveDate;
    fn fmt_date(d: &NaiveDate) -> String {
        format!("{}-{:02}-{:02}", d.year(), d.month(), d.day())
    }

    let store = store.lock().map_err(|e| e.to_string())?;
    match store.get(std::path::Path::new(&path)) {
        ScanState::Completed(stats) => {
            let by_day: Vec<_> = giter_core::analysis::trends::commit_trend_by_day(&stats.commits)
                .into_iter()
                .map(|(d, c)| (fmt_date(&d), c))
                .collect();
            let by_week: Vec<_> = giter_core::analysis::trends::commit_trend_by_week(&stats.commits)
                .into_iter()
                .map(|(d, c)| (fmt_date(&d), c))
                .collect();
            let by_month = giter_core::analysis::trends::commit_trend_by_month(&stats.commits)
                .unwrap_or_default()
                .into_iter()
                .map(|(d, c)| (fmt_date(&d), c))
                .collect();
            let churn: Vec<_> = giter_core::analysis::code_churn::code_churn_over_time(&stats.changes)
                .into_iter()
                .map(|(d, ins, del)| (fmt_date(&d), ins, del))
                .collect();
            Ok((by_day, by_week, by_month, churn))
        }
        _ => Ok((vec![], vec![], vec![], vec![])),
    }
}

/// Scan a single repository and store the result.
#[tauri::command]
fn scan_repo(path: String, store: tauri::State<'_, Mutex<ScanResultStore>>) -> Result<(), String> {
    // Mark as Scanning
    {
        let mut s = store.lock().map_err(|e| e.to_string())?;
        s.set_state(&path, ScanState::Scanning);
    }

    let repo_path = std::path::PathBuf::from(&path);
    let result = giter_core::git::scanner::scan_repository(&repo_path);

    let mut s = store.lock().map_err(|e| e.to_string())?;
    match result {
        Ok(data) => {
            let stats = giter_core::model::RepoStats {
                repo_path: path.clone(),
                repo_name: data.repo_name,
                commits: data.commits,
                changes: data.changes,
                snapshots: Vec::new(),
            };
            s.set_state(&path, ScanState::Completed(stats));
        }
        Err(e) => {
            s.set_state(&path, ScanState::Failed(e.to_string()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(AppSettings::load()))
        .manage(Mutex::new(SelectedRepos::new()))
        .manage(Mutex::new(ScanResultStore::new()))
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            get_settings,
            update_settings,
            get_repos,
            add_repo,
            remove_repo,
            get_scan_results,
            get_contributors,
            get_trends,
            scan_repo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running giter application");
}
