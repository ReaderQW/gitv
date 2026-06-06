use eframe::egui;
use giter_core::GitRepo;
use rfd::FileDialog;
use std::path::PathBuf;

// fn main() -> Result<(), eframe::Error> {
//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
//         ..Default::default()
//     };

//     eframe::run_native(
//         "Giter",
//         options,
//         Box::new(|_cc| Ok(Box::new(GiterApp::default()))),
//     )
// }

// struct GiterApp {
//     repo_path: String,
//     repo: Option<GitRepo>,
//     commits: Vec<giter_core::CommitInfo>,
//     branches: Vec<giter_core::BranchInfo>,
//     status_message: String,
// }
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Giter",
        options,
        Box::new(|_cc| Box::new(GiterApp::default())),
    )
}

struct GiterApp {
    repo_path: String,
    repo: Option<GitRepo>,
    commits: Vec<giter_core::CommitInfo>,
    branches: Vec<giter_core::BranchInfo>,
    status_message: String,
}


impl Default for GiterApp {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            repo: None,
            commits: Vec::new(),
            branches: Vec::new(),
            status_message: String::from("Welcome to Giter! Open a git repository to begin."),
        }
    }
}

impl eframe::App for GiterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.heading("Giter");
                ui.separator();

                if ui.button("Open Repository").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        self.open_repo(path);
                    }
                }
            });
        });

        egui::SidePanel::left("side_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Branches");
                ui.separator();
                if self.repo.is_some() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for branch in &self.branches {
                            let label = if branch.is_head {
                                format!("* {}", branch.name)
                            } else {
                                branch.name.clone()
                            };
                            ui.label(label);
                        }
                    });
                } else {
                    ui.label("No repository opened");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Commits");
            ui.separator();

            if !self.status_message.is_empty() && self.repo.is_none() {
                ui.label(&self.status_message);
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("commits_grid")
                    .striped(true)
                    .min_col_width(80.0)
                    .show(ui, |ui| {
                        ui.strong("Hash");
                        ui.strong("Author");
                        ui.strong("Date");
                        ui.strong("Message");
                        ui.end_row();

                        for commit in &self.commits {
                            let short_hash = &commit.hash[..commit.hash.len().min(8)];
                            ui.label(short_hash);
                            ui.label(&commit.author_name);
                            ui.label(&commit.datetime[..commit.datetime.len().min(10)]);
                            let msg = if commit.message.len() > 50 {
                                format!("{}...", &commit.message[..50])
                            } else {
                                commit.message.clone()
                            };
                            ui.label(msg);
                            ui.end_row();
                        }
                    });
            });
        });
    }
}

impl GiterApp {
    fn open_repo(&mut self, path: PathBuf) {
        let path_str = path.to_string_lossy().to_string();
        self.repo_path = path_str.clone();

        match GitRepo::open(&path_str) {
            Ok(repo) => {
                self.status_message = format!("Opened repository: {}", path_str);
                self.branches = repo.branches().unwrap_or_default();
                self.commits = repo.commits(Some(100)).unwrap_or_default();
                self.repo = Some(repo);
            }
            Err(e) => {
                self.status_message = format!("Failed to open repository: {}", e);
                self.repo = None;
                self.commits.clear();
                self.branches.clear();
            }
        }
    }
}
