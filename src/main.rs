
use std::process::Command;

use app::Application;
use epic_manifest_parser_rs::manifest::chunk_info::FChunkInfo;
use epic_manifest_parser_rs::manifest::chunk_part::FChunkPart;
use epic_manifest_parser_rs::manifest::file_manifest::FFileManifest;
use epic_manifest_parser_rs::manifest::FManifestParser;
use epic_manifest_parser_rs::reader::ByteReader;
use launcher::get_decryption_keys;


pub mod downloader;
pub mod app;
pub mod helper;
pub mod launcher;
pub mod decrypt;
pub mod epic;
pub mod epic_clients;
pub mod epic_manifest_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    // //Grab the manifest
    // let manifest_data = include_bytes!("6rnGKJUE4ZR1wE8YbEipgborM43rVQ.manifest").to_vec();

    // //Parse the manifest

    // let mut parser = FManifestParser::new(&manifest_data);
    // let manifest = parser.parse()?;

    // //Print the manifest
    // println!("Downloading {} ({})", manifest.meta.app_name(), manifest.meta.build_version());

    // let mut tags = manifest.file_list.entries().iter().flat_map(|x| {
    //     x.install_tags()
    // }).collect::<Vec<&String>>();

    // tags.sort();
    // tags.dedup();

    // let ignored_modal = TagsIgnored {
    //     tags: tags.iter().map(|x| x.to_string()).collect()
    // };

    // let ignored = ignored_modal.render();

    // let mut file_list = manifest.file_list.entries().iter().filter(|x| {
    //     x.install_tags().iter().all(|tag| !ignored.contains(tag))
    // }).map(|x| x.clone()).collect::<Vec<FFileManifest>>();

    // file_list.sort_by(|a,b| a.file_size().partial_cmp(&b.file_size()).unwrap());

    // println!("{} files, {} GB",file_list.len(), file_list.iter().map(|x| x.file_size() as usize).sum::<usize>() as f64 / 1e9);

    // //Download the files

    // let chunk_list = manifest.chunk_list.chunks().iter().filter(|x| {
    //     file_list.iter().any(|y| {
    //         y.chunk_parts().iter().any(|z| z.guid() == x.guid())
    //     })
    // }).map(|x| x.clone()).collect::<Vec<FChunkInfo>>();

    // //let file_list = file_list.into_iter().filter(|x| x.filename().contains(&"pakchunk0-WindowsClient.ucas")).collect::<Vec<_>>();

    // let manager = downloader::DownloadManager::new(3, 15, file_list, chunk_list);

    // manager.start_workers();

    // loop {
    //     {
    //         let progress = manager.progress.read().unwrap();

    //         println!("{} / {} files downloaded, {} failed, {}% downloaded", progress.downloaded_files_number, progress.total_files_number, progress.failed_files_number, (progress.downloaded_bytes as f64 / progress.total_bytes as f64 * 100.0).round());
    //         progress.downloading_files.iter().for_each(|x| {
    //             println!("{}: {}%", x.0, x.1);
    //         });
    //         let finished = progress.finished;

    //         if finished {
    //             break;
    //         }
    
    //     }
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    // }

    get_decryption_keys().await;


    //kill process epicgameslauncher.exe

    Command::new("taskkill").args(&["/F", "/IM", "epicgameslauncher.exe"]).spawn().expect("failed to execute process");


    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Fortnite Downloader",
        options,
        Box::new(|cc| {
            // This gives us image support:
           // egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<Application>::default())
        }),
    );

    Ok(())
}
