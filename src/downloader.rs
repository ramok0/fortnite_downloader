use std::{collections::{HashMap, VecDeque}, env, fs::{read, File}, io::{Read, Seek, Write}, path::PathBuf, sync::{atomic::AtomicU64, Arc, Mutex, RwLock}};

use epic_manifest_parser_rs::{helper, manifest::{chunk_info::FChunkInfo, chunk_part::{self, FChunkPart}, chunks::chunk_header::{self, FChunkHeader}, file_manifest::FFileManifest, shared::{FGuid, FSHAHash}}, reader::ByteReader};
use sha1::{Sha1, Digest};
use tokio::{sync::Semaphore, task::JoinHandle};
#[derive(Default, Clone)]
pub struct DownloadProgress {
    pub downloaded_files_number:usize,
    pub failed_files_number:usize,
    pub downloading_files:HashMap<String, f32>,
    pub total_files_number:usize,
    pub downloaded_bytes:usize,
    pub total_bytes:usize,
    pub finished:bool,
    pub downloading:bool,
}

#[derive(Clone)]
pub struct DownloadManager {
    pub max_threads: usize,
    pub max_chunk_concurrency: usize,
    pub file_list: Arc<tokio::sync::Mutex<VecDeque<FFileManifest>>>,
    pub chunks_list: Arc<Vec<FChunkInfo>>,
    pub progress: Arc<RwLock<DownloadProgress>>,
    pub downloading:bool,
    pub ignore_corrupted_files:bool,
    pub ctx:egui::Context
}

impl DownloadManager {
    pub fn new(max_threads: usize, max_chunk_concurrency:usize, file_list: Vec<FFileManifest>, chunks_list: Vec<FChunkInfo>, ignore_corrupted_files:bool, ctx:egui::Context) -> Self {

        let mut progress = DownloadProgress::default();

        progress.total_files_number = file_list.len();
        progress.total_bytes = file_list.iter().map(|x| x.file_size() as usize).sum();

        Self {
            ignore_corrupted_files,
            max_threads,
            max_chunk_concurrency,
            file_list: Arc::new(tokio::sync::Mutex::new(file_list.into_iter().collect())),
            chunks_list: Arc::new(chunks_list),
            progress: Arc::new(RwLock::new(progress)),
            downloading:false,
            ctx: ctx
        }
    }

    async fn download_chunk(client:reqwest::Client, chunk:FChunkInfo, base_url:String) -> Option<Vec<u8>>
    {


        let url = format!("{}/{}/{}_{}.chunk", base_url, chunk.group_num_str(), chunk.hash_str(), chunk.guid().to_string());

        let response = client.get(&url).send().await;

        if response.is_err() {
            dbg!(response);
            return None;
        }

        let bytes = response.unwrap().bytes().await;

        if bytes.is_err() {
            dbg!(bytes);
            return None;
        }

        let mut reader = ByteReader::new(bytes.unwrap().to_vec());
        let header = FChunkHeader::parse(&mut reader).ok()?;
        let data = header.get_data(&mut reader);


        Some(data)
    } 

    pub fn start_workers(&self, out_path:&PathBuf, base_url:String) {
        for i in 0..self.max_threads {
            let file_list = self.file_list.clone();
            let chunks_list = self.chunks_list.clone();
            let progress = self.progress.clone();

            {
                progress.as_ref().write().unwrap().downloading = true;
            }

            
            let client = reqwest::Client::new();
            let out_path = out_path.clone();

            let base_url = base_url.clone();
            let ctx = self.ctx.clone();
            let ignore_corrupted_files = self.ignore_corrupted_files;
            let semaphore = Arc::new(Semaphore::new(self.max_chunk_concurrency));


            tokio::task::spawn(async move {
                loop {
                    let file = {
                        file_list.lock().await.pop_front()
                    };

                    if let Some(file_manifest) = file {
                        std::fs::create_dir_all(out_path.join(file_manifest.filename()).parent().unwrap()).unwrap();
                        let path = out_path.join(file_manifest.filename());

                        if path.exists() {
                            let data = std::fs::read(&path).unwrap();                            

                            if !ignore_corrupted_files && data.len() != file_manifest.file_size() as usize {
                                std::fs::remove_file(&path).unwrap();
                                println!("File {} is corrupted so it has been deleted.", file_manifest.filename());
                            } else {
                                if ignore_corrupted_files || crate::helper::check_hash(&data, file_manifest.sha_hash()) {
                                    let mut progress = progress.write().unwrap();
                                    println!("thread #{} wrote progress", i);
                                    progress.downloaded_files_number += 1;
                                    progress.downloaded_bytes += data.len();
                                    ctx.request_repaint();
                                    
                                    continue;
                                } else {
                                    std::fs::remove_file(&path).unwrap();
    
                                    println!("File {} is corrupted so it has been deleted.", file_manifest.filename());
                                }
                            }
                        }

                        println!("Downloading {}", file_manifest.filename());


                        
                        //create the temporary buffer to store the file data
                        let mut buffer:Vec<u8> = Vec::<u8>::with_capacity(file_manifest.file_size() as usize);

                        unsafe {
                            //set the buffer length to the file size so we can write to it
                            buffer.set_len(file_manifest.file_size() as usize);
                        }
                        
                        let file_manifest = file_manifest.clone();
                        let parts:Vec<FChunkPart> = file_manifest.chunk_parts().to_vec();
                        //number of handles of downloads for the file
                        let mut handles: Vec<JoinHandle<Option<(Vec<u8>, usize)>>>      = Vec::new();



                      //  let downloaded_chunks = Arc::new(Mutex::new(Vec::<FChunkInfo>::new()));
                      let atomic_number = Arc::new(AtomicU64::new(0));

                        let parts_num = parts.len();

                        for chunk_id in 0..parts_num {
                            let chunk_part = parts[chunk_id].clone();

                            let chunk_list = chunks_list.clone();
                            let client = client.clone();
                            let progress = progress.clone();
                            let file_manifest = file_manifest.clone();
                            let atomic_number = atomic_number.clone();
                            let semaphore = semaphore.clone();
                            let base_url = base_url.clone();
                            let ctx = ctx.clone();
                        
                            let handle = tokio::task::spawn(async move {
                                let permit = semaphore.acquire().await.expect("Failed to acquire semaphore");
                         //       println!("Started new download");
                                let chunk = chunk_list.iter().find(|x| x.guid() == chunk_part.guid()).unwrap();

                                let chunk_data = DownloadManager::download_chunk(client.clone(), chunk.clone(), base_url).await?;
                                let downloaded_chunks_num = atomic_number.load(std::sync::atomic::Ordering::Relaxed);
                            //    println!("Thread #{} downloaded chunk {} ({}/{}) downloaded file : {}", i, chunk.guid().to_string(), downloaded_chunks_num, file_manifest.chunk_parts().len(), file_manifest.filename());
                                //increment the number of downloaded chunks
                                atomic_number.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                let data_start = chunk_part.offset() as usize;
                                let data_end = data_start + chunk_part.size() as usize;

                                // //check if its safe to get part_data
                                // if data_end > chunk_data.len() {
                                //     eprintln!("Failed to download file {}", file_manifest.filename());
                                //     dbg!(data_end > chunk_data.len());
                                //     return None;
                                // }

                                //get part_data
                                let part_data = &chunk_data[data_start..data_end];

                                {
                                    let mut progress = progress.write().unwrap();
                                    progress.downloaded_bytes += part_data.len();
                                    progress.downloading_files.insert(file_manifest.filename().to_string(), (atomic_number.load(std::sync::atomic::Ordering::Relaxed) as f32 / parts_num as f32 * 100.0).round());    
                                    ctx.request_repaint();
                                }

                                return Some((part_data.to_vec(), chunk_part.file_offset() as usize));
                            });

                            {
                                handles.push(handle);
                            }
                        }
                        
                        {
                            for handle in handles {
                                if let Some((data, file_offset)) = handle.await.unwrap() {
                                        //write the data to the buffer
                                        buffer[file_offset..file_offset + data.len()].copy_from_slice(&data);


                                } else {
                                    println!("Failed to download file {}", file_manifest.filename());
                                    progress.write().unwrap().failed_files_number += 1;
                                }
                            }

                            
                            {
                                progress.write().unwrap().downloading_files.remove(file_manifest.filename());
                            }

                            println!("Thread #{} finished downloading {}", i, file_manifest.filename());
                        }

                        //check if the file is corrupted and then write the data to the file
                        if ignore_corrupted_files || crate::helper::check_hash(&buffer, file_manifest.sha_hash()) {
                            let mut file = File::options().write(true).truncate(true).create(true).open(&path).unwrap();
                            file.write_all(&buffer).unwrap();
                            let mut progress = progress.write().unwrap();
                            progress.downloaded_files_number += 1;
                            println!("{} downloaded successfully", file_manifest.filename());
                        } else {
                            println!("File {} is corrupted", file_manifest.filename());
                            progress.write().unwrap().failed_files_number += 1;
                        }


                    } else {
                        //check if all the files have been downloaded
                        let mut progress = progress.write().unwrap();
                        if progress.downloaded_files_number + progress.failed_files_number == progress.total_files_number {
                            progress.finished = true;
                            progress.downloading = false;
                            ctx.request_repaint();
                            break;
                        }
                    }
                }
                
            });
        }
    }
}