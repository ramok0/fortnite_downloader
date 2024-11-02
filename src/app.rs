use std::{collections::HashMap, fmt::format, hash::Hash, path::PathBuf, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};

use eframe::App;
use egui::{RichText, Spinner, TextEdit, Widget};
use epic_manifest_parser_rs::{manifest::{chunk_info::FChunkInfo, file_manifest::FFileManifest, FManifest}, ParseResult};

use crate::{downloader::DownloadManager, epic, get_client, launcher};



pub struct Application {
    tags: Option<Vec<String>>,
    download_tags: HashMap<String, bool>,
    manifest: Option<FManifest>,
    max_threads: usize,
    max_chunk_concurrency: usize,
    download_path: Option<PathBuf>,
    error: Option<Box<dyn std::error::Error>>,
    base_url: String,
    files_number:usize,
    game_size:usize,
    download_manager: Arc<Mutex<Option<DownloadManager>>>,
    waiting_for_downloads: bool,
    checking_integrity: Arc<AtomicBool>,
    correct_files: Arc<Mutex<Vec<FFileManifest>>>,
    ignore_corrupted_files: bool,
    manifest_communication: (tokio::sync::mpsc::Sender<ParseResult<FManifest>>, tokio::sync::mpsc::Receiver<ParseResult<FManifest>>),
}

impl Default for Application {
    fn default() -> Self {
        Self {
            ignore_corrupted_files: false,
            tags: None,
            download_tags: HashMap::new(),
            manifest: None,
            max_threads: 3,
            max_chunk_concurrency: 15,
            download_path: None,
            error: None,
            base_url: String::from("https://epicgames-download1.akamaized.net/Builds/Fortnite/CloudDir/ChunksV4"),
            files_number:0,
            game_size:0,
            download_manager: Arc::new(Mutex::new(None)),
            waiting_for_downloads: false,
            checking_integrity: Arc::new(AtomicBool::new(false)),
            correct_files: Arc::new(Mutex::new(Vec::new())),
            manifest_communication: tokio::sync::mpsc::channel(1),
        }
    }
}

impl Application {
    fn calculate_game_files_size(&mut self) -> Option<(usize,usize)> {
            let files = self.get_filtered_files();

            if files.len() == 0 {
                return None;
            }

            let file_size = files.iter().map(|x| x.file_size() as usize).sum::<usize>();
            let files_number = files.len();

            self.game_size = file_size;
            self.files_number = files_number;

            return Some((files_number,file_size));
        

    }

    fn get_filtered_files(&self) -> Vec<FFileManifest> {
        if let Some(manifest) = &self.manifest {
            let tags = &self.download_tags.iter().filter(|x| x.1 == &true).map(|x| x.0.clone()).collect::<Vec<String>>();

            return manifest.file_list.entries().iter().filter(|x| {
                if x.install_tags().is_empty() { //mandatory files
                    return true;
                }

                for tag in x.install_tags() {
                    if tags.contains(&tag) {
                        return true;
                    }
                }

                return false; 
            }).cloned().collect::<Vec<FFileManifest>>();
        }

        vec![]
    
    }
}

impl eframe::App for Application {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        if let Ok(manifest_result) = self.manifest_communication.1.try_recv() {
            match manifest_result {
                Ok(manifest) => {
                    self.manifest = Some(manifest);
                    let mut tags = self.manifest.as_ref().unwrap().file_list.entries().iter().flat_map(|x| {
                        x.install_tags()
                    }).filter(|tag| !tag.contains("chunk")).map(|x| x.to_string()).collect::<Vec<String>>();

                    tags.sort();
                    tags.dedup();

                    self.tags = Some(tags.clone());

                    self.download_tags = HashMap::with_capacity(tags.len());

                    
                    for tag in tags {
                        self.download_tags.insert(tag.clone(), true);
                    }

                    self.calculate_game_files_size();

                },
                Err(error) => self.error = Some(error.into()),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.label(format!("Error: {}", error));
            }

            let download_manager = {
                self.download_manager.lock().unwrap().clone()
            };

            if self.waiting_for_downloads {
                ui.label(RichText::new("Preparing download.. please wait !").strong());
                
                if !download_manager.is_none() {
                    self.waiting_for_downloads = false;
                }
            }

            if let Some(download_manager) = &download_manager {
                let progress = download_manager.progress.read().unwrap();

                if progress.finished {
                    ui.label(RichText::new("Download finished !").strong());
                } else {
                    ui.label(format!("{}/{} files downloaded ({} failed)", progress.downloaded_files_number, progress.total_files_number, progress.failed_files_number));
                    ui.label(format!("{}%", (progress.downloaded_bytes as f64 / progress.total_bytes as f64 * 100.0).round()));
                    
                    ui.separator();
    
                    ui.label(RichText::new("Downloading files:").strong());
                    for (file, progress) in &progress.downloading_files {
                        ui.label(format!("{}: {}%", file, progress.round()));
                    }
                }
            } else {
                if let Some(manifest) = &self.manifest {
                    ui.label(format!("Selected manifest : {} ({})", manifest.meta.app_name(), manifest.meta.build_version()));
                        if let Some(tags) = self.tags.clone() {
                                ui.label("Select the tags you want to download: ");
                                
                                egui::ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                                    for tag in tags {
                                        ui.horizontal(|ui| {
                                            let mut ignored = self.download_tags.get(&tag).unwrap_or(&false).clone();
                                            if ui.checkbox(&mut ignored, &tag).changed() {
                                                self.download_tags.insert(tag, ignored);
                                                self.calculate_game_files_size();
                                            }
                                        });
                                    }
                                });
    
                                ui.label(format!("Selected files: {} ({} GB) ", self.files_number, self.game_size / 1024 / 1024 / 1024));
    
    
    
                        ui.separator();
    
                        }

                        ui.checkbox(&mut self.ignore_corrupted_files, "Ignore corrupted files");
    
                        ui.horizontal(|ui| {
                            ui.label("Max threads: ");
                            ui.add(egui::Slider::new(&mut self.max_threads, 1..=16).text("threads"));
                        });
    
                        ui.horizontal(|ui| {
                            ui.label("Max chunk concurrency: ");
                            ui.add(egui::Slider::new(&mut self.max_chunk_concurrency, 1..=30).text("chunks"));
                        });
    
                        ui.label(format!("Max downloads at the same time : {}", self.max_chunk_concurrency * self.max_threads));
    
                        ui.separator();
    
                        
                        if let Some(download_path) = &self.download_path {
                            ui.label(format!("Download path: {}", download_path.to_string_lossy()));
                        }
    
                        ui.horizontal(|ui| {
                            if ui.button("Select the download path").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    self.download_path = Some(path);
                                }
                            }



                            if let Some(download_path) = &self.download_path {
                                if self.checking_integrity.load(std::sync::atomic::Ordering::Relaxed) {
                                    ui.horizontal(|ui| {
                                        ui.label("Checking integrity.. please wait (it may take a moment) !");
                                        //circle here
                                        Spinner::new().ui(ui);
                                    });

                                } else {
                                    if ui.button("Check integrity").clicked() {
                                        self.checking_integrity.store(true, Ordering::Relaxed);
                                        let download_path = download_path.clone();
                                        let game_files = self.get_filtered_files();
                                        let correct_files = self.correct_files.clone();
                                        let checking_integrity = self.checking_integrity.clone();
                                        {
                                            correct_files.lock().unwrap().clear();
                                        }



                                        tokio::spawn(async move {
                                            
                                            for file in game_files {  
                                                let path = download_path.join(file.filename());
                                                if path.exists() {
                                                    if let Ok(data) = std::fs::read(path) {
                                                        if crate::helper::check_hash(&data, file.sha_hash()) {
                                                            correct_files.lock().unwrap().push(file);
                                                        }
                                                    }                            
                                                }
                                            }

                                            checking_integrity.store(false, Ordering::Relaxed);
                                        });
                                    }
                                }

                                if !self.checking_integrity.load(Ordering::Relaxed) {
                                    let correct_files_number = {
                                        self.correct_files.lock().unwrap().len()
                                    };
        
                                    if correct_files_number > 0 {
                         
                                        let already_installed_size = {
                                            self.correct_files.lock().unwrap().iter().map(|x| x.file_size() as usize).sum::<usize>()
                                        };
                                        ui.vertical(|ui| {
                                            ui.label(format!("{} files are already downloaded", correct_files_number));
                                            ui.label(format!("Already installed : {} GB", already_installed_size / 1024 / 1024 / 1024));
                                            ui.label(format!("Rest : {} GB",  (self.game_size - already_installed_size) / 1024 / 1024 / 1024)); 
                                        })                 ;
                                    }   
                                }


                            }
                        });
    
                        
                        ui.separator();
    
                        //changet the base url
    
                        ui.horizontal(|ui| {
                            ui.label("Base URL: ");
                            ui.add(TextEdit::singleline(&mut self.base_url).desired_width(200.0).hint_text("Base URL"));
                        });
    
                        ui.separator();
    
                        //check that everything is ready to download
    
                        if !self.base_url.is_empty() && self.download_path.is_some() && self.manifest.is_some() &&
                        self.max_chunk_concurrency != 0 && self.max_threads != 0 && self.files_number != 0 {
                            if ui.button("Download").clicked() {
                                let files = self.get_filtered_files();
                                let chunks = self.manifest.as_ref().unwrap().chunk_list.chunks().iter().cloned().collect::<Vec<FChunkInfo>>();
                                let download_manager = self.download_manager.clone();
                                let max_threads = self.max_threads;
                                let max_chunk_concurrency = self.max_chunk_concurrency;
                                let download_path = self.download_path.clone();
                                let base_url = self.base_url.clone();
                                let ignore_corrupted_files = self.ignore_corrupted_files;
                                let ctx = ctx.clone();
                                

                                self.waiting_for_downloads = true;

                                tokio::spawn(async move {
                                    //bottle neck here, might be unecessary though
                                    // let chunk_list = chunks.iter().filter(|x| {
                                    //     files.iter().any(|y| {
                                    //         y.chunk_parts().iter().any(|z| z.guid() == x.guid())
                                    //     })
                                    // }).map(|x| x.clone()).collect::<Vec<FChunkInfo>>();
        
                                    let mut download_manager = download_manager.lock().unwrap();
                                    *download_manager = Some( DownloadManager::new(max_threads, max_chunk_concurrency, files, chunks, ignore_corrupted_files, ctx));
                                    download_manager.as_ref().unwrap().start_workers(download_path.as_ref().unwrap(), base_url);
                                });
    
                            }
                        }
                    
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Select the manifest file: ");
                        if ui.button("Select").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                let mut parser = epic_manifest_parser_rs::manifest::FManifestParser::new(&std::fs::read(path).unwrap());
                                
                                let tx =  self.manifest_communication.0.clone();
                                tokio::spawn(async move {
                                   let _ = tx.send(parser.parse()).await;
                                });
                            }
                        }

                        if ui.button("Use the latest manifest").clicked() {
                            match crate::launcher::epic_get_remember_me_data() {
                                Ok(mut remember_me_data ) => {

                                    let tx: tokio::sync::mpsc::Sender<Result<FManifest, epic_manifest_parser_rs::error::ParseError>> = self.manifest_communication.0.clone();
                                    tokio::spawn(async move {
                                        if let Ok(launcher_token) = epic::token(
                                            epic::Token::RefreshToken(&remember_me_data.token),
                                            get_client!("launcherAppClient2"),
                                        )
                                        .await {
                                            remember_me_data.token = launcher_token.refresh_token.clone().unwrap();
    
                                            if let Err(e) = launcher::epic_set_remember_me_data(remember_me_data) {
                                                eprintln!("Failed to save remember me data: {}", e);
                                            }
        
                                            dbg!(&launcher_token.access_token);

                                            if let Ok(manifest_response) = launcher_token.get_manifest().await {
                                                if let Some(manifest_uri) = manifest_response.get_first_manifest() {
                                                    if let Ok(response) = reqwest::get(&manifest_uri).await {
                                                        let bytes = response.bytes().await.unwrap().to_vec();
                                                        
                                                        let mut parser = epic_manifest_parser_rs::manifest::FManifestParser::new(&bytes);
                                                        let _ = tx.send(parser.parse()).await;
                                                    }
                                                }
                                            }

                                        } else {
                                            eprintln!("Failed to login");
                                        }
                                    });
                                }, 
                                Err(e) => {
                                    eprintln!("Failed to get remember me data: {}", e);
                                }
                            }
                        }
                    });
                }
     
            }
        });
    }
}